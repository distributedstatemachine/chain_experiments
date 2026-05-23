use std::{
    io::ErrorKind,
    thread,
    time::{Duration, Instant},
};

use super::{
    miner_role::tick_miner_role_work_once,
    network::{
        chain_announcement_checkpoint, ingest_network_events, produce_and_publish_synthetic_round,
        publish_new_chain_announcements,
    },
    roles::{
        submit_validator_role_attestation, submit_validator_role_block_vote,
        validator_role_work_observation,
    },
    runtime_config::{RuntimeRole, ServiceRuntimeConfig, runtime_role_wallet_registration},
    runtime_services::{RuntimeP2pMetadata, RuntimeServices, start_runtime_services},
    runtime_status::{
        RuntimeStatusSnapshot, format_role_runtime_report, write_role_runtime_status,
    },
    validator_fetch::fetch_validator_role_missing_tensors,
};
use tensor_vm::{ChainSnapshot, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

pub(super) struct RoleRuntimeLoop {
    config: ServiceRuntimeConfig,
    store: NodeStore,
    pub(super) server: RpcHttpServer,
    p2p_service: TensorVmLibp2pService,
    local_producer: bool,
    block_interval: Option<Duration>,
    next_block_at: Option<Instant>,
    runtime_state: NodeRuntimeState,
    p2p_metadata: RuntimeP2pMetadata,
}

impl RoleRuntimeLoop {
    pub(super) fn start(config: ServiceRuntimeConfig) -> std::result::Result<Self, String> {
        let RuntimeServices {
            store,
            server,
            p2p_service,
            p2p_metadata,
        } = start_runtime_services(&config)?;
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
            p2p_metadata,
        })
    }

    pub(super) fn run_until_max_requests(&mut self) -> std::result::Result<(), String> {
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
            RuntimeRole::Miner => {
                if tick_miner_role_work_once(
                    &self.config,
                    &self.store,
                    &mut self.server,
                    &self.p2p_service,
                    &mut self.runtime_state,
                )? {
                    self.write_status()?;
                }
                Ok(())
            }
            RuntimeRole::Validator => self.tick_validator_role_work_once(),
            RuntimeRole::Proposer | RuntimeRole::Service => Ok(()),
        }
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
        write_role_runtime_status(&self.config, &self.status_snapshot())
    }

    fn status_snapshot(&self) -> RuntimeStatusSnapshot {
        RuntimeStatusSnapshot::from_runtime_state(
            &self.runtime_state,
            &self.server,
            &self.p2p_service,
            self.local_producer,
            self.config.role,
            self.config.role_wallet_address,
        )
    }

    pub(super) fn report(&self) -> String {
        let snapshot = self.status_snapshot();
        format_role_runtime_report(&self.config, &snapshot, &self.p2p_metadata.report())
    }
}
