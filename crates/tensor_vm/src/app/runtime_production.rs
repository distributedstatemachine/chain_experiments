use std::time::{Duration, Instant};

use super::produce_and_publish_synthetic_round;
use crate::{ChainProfile, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

pub struct LocalProductionSchedule {
    block_interval: Option<Duration>,
    next_block_at: Option<Instant>,
}

impl LocalProductionSchedule {
    pub fn new(block_interval: Option<Duration>) -> Self {
        Self {
            block_interval,
            next_block_at: block_interval.map(|interval| Instant::now() + interval),
        }
    }

    pub fn produce_if_due(
        &mut self,
        profile: &ChainProfile,
        local_producer: bool,
        store: &NodeStore,
        server: &mut RpcHttpServer,
        p2p_service: &TensorVmLibp2pService,
        runtime_state: &mut NodeRuntimeState,
    ) -> std::result::Result<bool, String> {
        let Some(interval) = self.block_interval else {
            return Ok(false);
        };
        if self
            .next_block_at
            .is_none_or(|deadline| Instant::now() < deadline)
        {
            return Ok(false);
        }
        let mut status_changed = false;
        if local_producer
            && produce_and_publish_synthetic_round(server, p2p_service, profile)?.is_some()
        {
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| format!("failed to persist produced block: {error}"))?;
            runtime_state.record_produced_block();
            status_changed = true;
        }
        self.next_block_at = Some(Instant::now() + interval);
        Ok(status_changed)
    }
}
