use std::{io::ErrorKind, thread, time::Duration};

use super::{
    miner_role::tick_miner_role_work_once,
    network::ingest_network_events,
    runtime_config::{RuntimeRole, ServiceRuntimeConfig},
    runtime_production::LocalProductionSchedule,
    runtime_services::{RuntimeP2pMetadata, RuntimeServices, start_runtime_services},
    runtime_status::{
        RuntimeStatusSnapshot, format_role_runtime_report, write_role_runtime_status,
    },
    validator_role::tick_validator_role_work_once as tick_validator_role_worker_once,
};
use tensor_vm::{ChainSnapshot, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

pub(super) struct RoleRuntimeLoop {
    config: ServiceRuntimeConfig,
    store: NodeStore,
    pub(super) server: RpcHttpServer,
    p2p_service: TensorVmLibp2pService,
    local_producer: bool,
    local_production: LocalProductionSchedule,
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
        let local_production = LocalProductionSchedule::new(config.node.synthetic_block_interval());
        let local_producer = config.node.local_synthetic_producer();
        Ok(Self {
            config,
            store,
            server,
            p2p_service,
            local_producer,
            local_production,
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
        if tick_validator_role_worker_once(
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

    fn produce_local_round_if_due(&mut self) -> std::result::Result<(), String> {
        if self.local_production.produce_if_due(
            &self.config.node.profile,
            self.local_producer,
            &self.store,
            &mut self.server,
            &self.p2p_service,
            &mut self.runtime_state,
        )? {
            self.write_status()?;
        }
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
