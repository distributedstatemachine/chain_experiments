use std::{
    io::ErrorKind,
    thread,
    time::{Duration, Instant},
};

use super::{
    RuntimeRole, ServiceRuntimeConfig, hex_hash_list,
    network::{
        chain_announcement_checkpoint, ingest_network_events, produce_and_publish_synthetic_round,
        publish_new_chain_announcements,
    },
    p2p_identity_report,
    roles::{
        fetch_validator_role_missing_tensors, miner_role_work_observation,
        submit_miner_role_receipt, submit_validator_role_attestation,
        submit_validator_role_block_vote, validator_role_work_observation,
    },
    runtime_role_wallet_address_text, runtime_role_wallet_registered,
    runtime_role_wallet_registration,
    status::{RuntimeStatusSnapshot, write_role_runtime_status},
};
use tensor_vm::{
    ChainSnapshot, Faucet, Libp2pControlPlaneConfig, NodeRuntimeState, NodeStore, RpcGateway,
    RpcHttpServer, RpcNode, RpcPolicy, TensorVmLibp2pService, hash::hex, spawn_libp2p_service,
};

pub(super) fn run_role_runtime_loop(
    config: ServiceRuntimeConfig,
) -> std::result::Result<String, String> {
    let mut runtime = RoleRuntimeLoop::start(config)?;
    runtime.run_until_max_requests()?;
    Ok(runtime.report())
}

pub(super) struct RoleRuntimeLoop {
    config: ServiceRuntimeConfig,
    store: NodeStore,
    pub(super) server: RpcHttpServer,
    p2p_service: TensorVmLibp2pService,
    local_producer: bool,
    block_interval: Option<Duration>,
    next_block_at: Option<Instant>,
    runtime_state: NodeRuntimeState,
    p2p_peer_id: String,
    p2p_topics: usize,
    p2p_request_response_protocols: usize,
    bootstrap_peer_count: usize,
    identity: String,
    max_transmit_bytes: usize,
    request_timeout_seconds: u64,
    max_concurrent_streams: usize,
    idle_timeout_seconds: u64,
}

impl RoleRuntimeLoop {
    pub(super) fn start(config: ServiceRuntimeConfig) -> std::result::Result<Self, String> {
        let network = &config.node.network;
        let store = NodeStore::open(config.node.data_dir());
        let chain = store.load_chain().map_err(|error| {
            format!(
                "failed to load node store {}: {error}",
                config.node.data_dir().display()
            )
        })?;
        let bootstrap_addresses = if store.peer_book_store().path().exists() {
            store
                .peer_book_store()
                .load_bootstrap_addresses()
                .map_err(|error| {
                    format!(
                        "failed to load libp2p peer book {}: {error}",
                        config.node.data_dir().display()
                    )
                })?
        } else {
            Vec::new()
        };
        let bootstrap_peer_count = bootstrap_addresses.len();
        let p2p_config = Libp2pControlPlaneConfig {
            listen_addresses: vec![network.p2p_listen.clone()],
            bootstrap_addresses,
            identity_seed: network.identity_seed,
            ..Libp2pControlPlaneConfig::default()
        };
        let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
        let request_timeout_seconds = p2p_config.request_timeout_seconds;
        let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
        let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
        let p2p_service = spawn_libp2p_service(p2p_config)
            .map_err(|error| format!("failed to start mandatory libp2p service: {error}"))?;
        let p2p_peer_id = p2p_service.peer_id().to_string();
        let p2p_topics = p2p_service.info().subscribed_topics.len();
        let p2p_request_response_protocols = p2p_service.info().request_response_protocols.len();
        let identity = p2p_identity_report(network.identity_seed);
        let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
        let gateway = RpcGateway::new(
            node,
            RpcPolicy {
                auth_token: Some(network.auth_token.clone()),
                ..RpcPolicy::default()
            },
        );
        let server = RpcHttpServer::bind(&network.rpc_listen, gateway).map_err(|error| {
            format!(
                "failed to bind service listener {}: {error}",
                network.rpc_listen
            )
        })?;
        let block_interval = config.node.synthetic_block_interval();
        let next_block_at = block_interval.map(|interval| Instant::now() + interval);
        let local_producer = config.node.local_synthetic_producer();
        Ok(Self {
            config,
            store,
            server,
            p2p_service,
            local_producer,
            block_interval,
            next_block_at,
            runtime_state: NodeRuntimeState::default(),
            p2p_peer_id,
            p2p_topics,
            p2p_request_response_protocols,
            bootstrap_peer_count,
            identity,
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_timeout_seconds,
        })
    }

    fn run_until_max_requests(&mut self) -> std::result::Result<(), String> {
        self.write_status()?;
        self.server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
        loop {
            if self.max_requests_reached() {
                break;
            }
            self.serve_rpc_once()?;
            self.ingest_network_once()?;
            self.tick_role_work_once()?;
            self.produce_local_round_if_due()?;
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    fn max_requests_reached(&self) -> bool {
        let max_requests = self.config.node.network.max_requests;
        max_requests != 0 && self.runtime_state.served_requests() >= max_requests
    }

    pub(super) fn serve_rpc_once(&mut self) -> std::result::Result<(), String> {
        let chain_snapshot_before = ChainSnapshot::from_chain(&self.server.gateway().node.chain);
        match self.server.serve_next() {
            Ok(()) => {
                let chain_changed = ChainSnapshot::from_chain(&self.server.gateway().node.chain)
                    != chain_snapshot_before;
                self.record_served_request(chain_changed)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(format!("service request failed: {error}")),
        }
    }

    fn record_served_request(&mut self, chain_changed: bool) -> std::result::Result<(), String> {
        if chain_changed {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist service state: {error}"))?;
        }
        self.runtime_state.record_served_request();
        self.write_status()
    }

    fn ingest_network_once(&mut self) -> std::result::Result<(), String> {
        let ingested = ingest_network_events(
            &mut self.server,
            &self.p2p_service,
            self.local_producer,
            self.runtime_state.pending_payloads_mut(),
        )?;
        if !ingested.has_activity() {
            return Ok(());
        }
        let should_persist = ingested.applied_blocks > 0
            || ingested.job_payloads_applied > 0
            || ingested.receipt_payloads_applied > 0
            || ingested.attestation_payloads_applied > 0
            || ingested.block_votes_applied > 0;
        self.runtime_state.record_network_ingest(ingested);
        if should_persist {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist network-applied state: {error}"))?;
        }
        self.write_status()
    }

    fn tick_role_work_once(&mut self) -> std::result::Result<(), String> {
        match self.config.role {
            RuntimeRole::Miner => self.tick_miner_role_work_once(),
            RuntimeRole::Validator => self.tick_validator_role_work_once(),
            RuntimeRole::Proposer | RuntimeRole::Service => Ok(()),
        }
    }

    fn tick_miner_role_work_once(&mut self) -> std::result::Result<(), String> {
        let Some(miner) = self.config.role_wallet_address else {
            return Ok(());
        };
        if runtime_role_wallet_registration(
            self.config.role,
            self.config.role_wallet_address,
            &self.server.gateway().node.chain,
        ) != "miner"
        {
            return Ok(());
        }
        let observation = miner_role_work_observation(&self.server.gateway().node.chain, miner);
        let job_to_submit = observation.unreceipted_jobs.iter().next().copied();
        let mut status_changed = false;
        if self
            .runtime_state
            .record_miner_work_observation(observation.assigned_jobs, observation.unreceipted_jobs)
        {
            status_changed = true;
        }
        if let Some(job_id) = job_to_submit {
            let announcement_checkpoint =
                chain_announcement_checkpoint(&self.server.gateway().node.chain);
            if let Some(submission) =
                submit_miner_role_receipt(&mut self.server.gateway_mut().node, miner, job_id)?
            {
                publish_new_chain_announcements(
                    &self.p2p_service,
                    &announcement_checkpoint,
                    &self.server.gateway().node.chain,
                )?;
                self.store
                    .persist_chain(&self.server.gateway().node.chain)
                    .map_err(|error| format!("failed to persist miner receipt state: {error}"))?;
                self.runtime_state.record_miner_receipt_submission(
                    submission.receipts_submitted,
                    submission.tensors_inserted,
                );
                for tensor in submission.served_tensors {
                    self.p2p_service.register_tensor(tensor);
                }
                let observation =
                    miner_role_work_observation(&self.server.gateway().node.chain, miner);
                self.runtime_state.record_miner_work_observation(
                    observation.assigned_jobs,
                    observation.unreceipted_jobs,
                );
                status_changed = true;
            }
        }
        if status_changed {
            self.write_status()?;
        }
        Ok(())
    }

    pub(super) fn tick_validator_role_work_once(&mut self) -> std::result::Result<(), String> {
        let Some(validator) = self.config.role_wallet_address else {
            return Ok(());
        };
        if runtime_role_wallet_registration(
            self.config.role,
            self.config.role_wallet_address,
            &self.server.gateway().node.chain,
        ) != "validator"
        {
            return Ok(());
        }
        let observation = validator_role_work_observation(&self.server.gateway().node, validator);
        let receipt_to_fetch = observation.artifact_missing_receipts.iter().next().copied();
        let mut receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
        let mut status_changed = false;
        if self.runtime_state.record_validator_work_observation(
            observation.assigned_receipts,
            observation.unattested_receipts,
            observation.artifact_ready_receipts,
            observation.artifact_missing_receipts,
        ) {
            status_changed = true;
        }
        if receipt_to_submit.is_none()
            && let Some(receipt_id) = receipt_to_fetch
        {
            let fetch_report = fetch_validator_role_missing_tensors(
                &mut self.server.gateway_mut().node,
                &self.p2p_service,
                receipt_id,
            )?;
            if fetch_report.attempts > 0
                || fetch_report.successes > 0
                || fetch_report.failures > 0
                || fetch_report.tensors_inserted > 0
            {
                self.runtime_state.record_validator_remote_tensor_fetch(
                    fetch_report.attempts,
                    fetch_report.successes,
                    fetch_report.failures,
                    fetch_report.bytes,
                    fetch_report.tensors_inserted,
                );
                let observation =
                    validator_role_work_observation(&self.server.gateway().node, validator);
                receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
                self.runtime_state.record_validator_work_observation(
                    observation.assigned_receipts,
                    observation.unattested_receipts,
                    observation.artifact_ready_receipts,
                    observation.artifact_missing_receipts,
                );
                status_changed = true;
            }
        }
        if let Some(receipt_id) = receipt_to_submit {
            let announcement_checkpoint =
                chain_announcement_checkpoint(&self.server.gateway().node.chain);
            if let Some(submission) = submit_validator_role_attestation(
                &mut self.server.gateway_mut().node,
                validator,
                receipt_id,
            )? {
                publish_new_chain_announcements(
                    &self.p2p_service,
                    &announcement_checkpoint,
                    &self.server.gateway().node.chain,
                )?;
                self.store
                    .persist_chain(&self.server.gateway().node.chain)
                    .map_err(|error| {
                        format!("failed to persist validator attestation state: {error}")
                    })?;
                self.runtime_state
                    .record_validator_attestation_submission(submission.attestations_submitted);
                let observation =
                    validator_role_work_observation(&self.server.gateway().node, validator);
                self.runtime_state.record_validator_work_observation(
                    observation.assigned_receipts,
                    observation.unattested_receipts,
                    observation.artifact_ready_receipts,
                    observation.artifact_missing_receipts,
                );
                status_changed = true;
            }
        }
        let announcement_checkpoint =
            chain_announcement_checkpoint(&self.server.gateway().node.chain);
        if let Some(submission) =
            submit_validator_role_block_vote(&mut self.server.gateway_mut().node, validator)?
        {
            publish_new_chain_announcements(
                &self.p2p_service,
                &announcement_checkpoint,
                &self.server.gateway().node.chain,
            )?;
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| {
                    format!("failed to persist validator block vote state: {error}")
                })?;
            self.runtime_state
                .record_validator_block_vote_submission(submission.block_votes_submitted);
            status_changed = true;
        }
        if status_changed {
            self.write_status()?;
        }
        Ok(())
    }

    fn produce_local_round_if_due(&mut self) -> std::result::Result<(), String> {
        let Some(interval) = self.block_interval else {
            return Ok(());
        };
        if self
            .next_block_at
            .is_none_or(|deadline| Instant::now() < deadline)
        {
            return Ok(());
        }
        if self.local_producer
            && produce_and_publish_synthetic_round(
                &mut self.server,
                &self.p2p_service,
                &self.config.node.profile,
            )?
            .is_some()
        {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist produced block: {error}"))?;
            self.runtime_state.record_produced_block();
            self.write_status()?;
        }
        self.next_block_at = Some(Instant::now() + interval);
        Ok(())
    }

    fn write_status(&self) -> std::result::Result<(), String> {
        write_role_runtime_status(
            &self.config,
            &RuntimeStatusSnapshot::from_runtime_state(
                &self.runtime_state,
                &self.server,
                &self.p2p_service,
                self.local_producer,
                self.config.role,
                self.config.role_wallet_address,
            ),
        )
    }

    fn report(&self) -> String {
        let network = &self.config.node.network;
        let network_events = self.runtime_state.network_events();
        format!(
            "command=service_serve\nruntime_command={}\nrole={}\nchain_profile={}\nrole_loop_ready=true\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nminer_work_ready={}\nminer_assigned_jobs_seen={}\nminer_unreceipted_jobs={}\nminer_receipts_submitted={}\nminer_tensors_inserted={}\nvalidator_work_ready={}\nvalidator_assigned_receipts_seen={}\nvalidator_unattested_receipts={}\nvalidator_artifact_ready_receipts={}\nvalidator_artifact_missing_receipts={}\nvalidator_remote_tensor_fetch_attempts={}\nvalidator_remote_tensor_fetch_successes={}\nvalidator_remote_tensor_fetch_failures={}\nvalidator_remote_tensor_fetch_bytes={}\nvalidator_remote_tensors_inserted={}\nvalidator_attestations_submitted={}\nvalidator_block_votes_submitted={}\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_block_payload_gossip_count={}\np2p_observed_block_vote_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_latest_observed_block_payload_height={}\np2p_latest_observed_block_payload_hash={}\np2p_observed_block_payload_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}\nnetwork_events_ingested={}\nnetwork_block_events_ingested={}\nnetwork_block_headers_ingested={}\nnetwork_block_payloads_ingested={}\nnetwork_block_payloads_applied={}\nnetwork_block_votes_ingested={}\nnetwork_block_votes_applied={}\nnetwork_job_events_ingested={}\nnetwork_job_payloads_ingested={}\nnetwork_job_payloads_applied={}\nnetwork_receipt_events_ingested={}\nnetwork_receipt_payloads_ingested={}\nnetwork_receipt_payloads_applied={}\nnetwork_attestation_events_ingested={}\nnetwork_attestation_payloads_ingested={}\nnetwork_attestation_payloads_applied={}\nnetwork_peer_events_ingested={}\nnetwork_invalid_events={}",
            self.config.runtime_command,
            self.config.role.label(),
            self.config.node.profile.label(),
            self.config.node.can_produce_local_blocks(),
            runtime_role_wallet_address_text(self.config.role_wallet_address),
            runtime_role_wallet_registration(
                self.config.role,
                self.config.role_wallet_address,
                &self.server.gateway().node.chain
            ),
            runtime_role_wallet_registered(
                self.config.role,
                self.config.role_wallet_address,
                &self.server.gateway().node.chain
            ),
            self.runtime_state.miner_work_ready(),
            self.runtime_state.miner_assigned_jobs_seen(),
            self.runtime_state.miner_unreceipted_jobs(),
            self.runtime_state.miner_receipts_submitted(),
            self.runtime_state.miner_tensors_inserted(),
            self.runtime_state.validator_work_ready(),
            self.runtime_state.validator_assigned_receipts_seen(),
            self.runtime_state.validator_unattested_receipts(),
            self.runtime_state.validator_artifact_ready_receipts(),
            self.runtime_state.validator_artifact_missing_receipts(),
            self.runtime_state.validator_remote_tensor_fetch_attempts(),
            self.runtime_state.validator_remote_tensor_fetch_successes(),
            self.runtime_state.validator_remote_tensor_fetch_failures(),
            self.runtime_state.validator_remote_tensor_fetch_bytes(),
            self.runtime_state.validator_remote_tensors_inserted(),
            self.runtime_state.validator_attestations_submitted(),
            self.runtime_state.validator_block_votes_submitted(),
            network.rpc_listen,
            network.p2p_listen,
            self.p2p_service.connected_peer_count(),
            self.p2p_service.observed_block_gossip_count(),
            self.p2p_service.observed_block_payload_gossip_count(),
            self.p2p_service.observed_block_vote_gossip_count(),
            self.p2p_service.observed_job_gossip_count(),
            self.p2p_service.observed_receipt_gossip_count(),
            self.p2p_service.observed_attestation_gossip_count(),
            self.p2p_service.latest_observed_block_height(),
            hex(&self.p2p_service.latest_observed_block_hash()),
            hex_hash_list(&self.p2p_service.observed_block_hashes()),
            self.p2p_service.latest_observed_block_payload_height(),
            hex(&self.p2p_service.latest_observed_block_payload_hash()),
            hex_hash_list(&self.p2p_service.observed_block_payload_hashes()),
            self.config.node.data_dir().display(),
            network_events.events,
            network_events.block_announcements,
            network_events.block_headers,
            network_events.block_payloads,
            network_events.block_payloads_applied,
            network_events.block_votes,
            network_events.block_votes_applied,
            network_events.jobs,
            network_events.job_payloads,
            network_events.job_payloads_applied,
            network_events.receipts,
            network_events.receipt_payloads,
            network_events.receipt_payloads_applied,
            network_events.attestations,
            network_events.attestation_payloads,
            network_events.attestation_payloads_applied,
            network_events.peers,
            network_events.invalid_events,
            local_producer = self.local_producer,
            p2p_peer_id = self.p2p_peer_id,
            p2p_topics = self.p2p_topics,
            p2p_request_response_protocols = self.p2p_request_response_protocols,
            bootstrap_peer_count = self.bootstrap_peer_count,
            identity = self.identity,
            max_transmit_bytes = self.max_transmit_bytes,
            request_timeout_seconds = self.request_timeout_seconds,
            max_concurrent_streams = self.max_concurrent_streams,
            idle_timeout_seconds = self.idle_timeout_seconds,
            served_requests = self.runtime_state.served_requests(),
            produced_blocks = self.runtime_state.produced_blocks(),
            network_applied_blocks = self.runtime_state.network_applied_blocks()
        )
    }
}
