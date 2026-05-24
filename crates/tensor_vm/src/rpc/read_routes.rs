use super::render::job_value;
use super::{RpcNode, RpcResponse, parse_hash};
use crate::hash::hex;
use serde_json::json;

impl RpcNode {
    pub(super) fn chain_block(&self, height: &str) -> RpcResponse {
        let Ok(height) = height.parse::<usize>() else {
            return self.bad_request("invalid block height");
        };
        let Some(block) = self.chain.blocks.get(height) else {
            return self.not_found("block not found");
        };
        self.ok(format!(
            "{{\"height\":{},\"epoch\":{},\"hash\":\"{}\"}}",
            block.height,
            block.epoch,
            hex(&block.hash())
        ))
    }

    pub(super) fn receipt(&self, receipt_id: &str) -> RpcResponse {
        let Ok(receipt_id) = parse_hash(receipt_id) else {
            return self.bad_request("invalid receipt id");
        };
        let Some(receipt) = self.chain.state().receipts().get(&receipt_id) else {
            return self.not_found("receipt not found");
        };
        self.ok(format!(
            "{{\"receipt_id\":\"{}\",\"job_id\":\"{}\",\"tensor_work_units\":{}}}",
            hex(&receipt.receipt_id()),
            hex(&receipt.job_id()),
            receipt.tensor_work_units()
        ))
    }

    pub(super) fn faucet_status(&self) -> RpcResponse {
        let Some(faucet) = &self.faucet else {
            return self.not_found("faucet not configured");
        };
        self.ok(format!(
            "{{\"balance\":{},\"drip_amount\":{}}}",
            faucet.balance(),
            faucet.drip_amount()
        ))
    }

    pub(super) fn jobs_current(&self) -> RpcResponse {
        let jobs: Vec<_> = self.chain.state().jobs().values().map(job_value).collect();
        self.ok(json!({ "jobs": jobs }).to_string())
    }

    pub(super) fn job(&self, job_id: &str) -> RpcResponse {
        let Ok(job_id) = parse_hash(job_id) else {
            return self.bad_request("invalid job id");
        };
        let Some(job) = self.chain.state().jobs().get(&job_id) else {
            return self.not_found("job not found");
        };
        self.ok(job_value(job).to_string())
    }

    pub(super) fn miner(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid miner address");
        };
        let Some(miner) = self.chain.state().miners().get(&address) else {
            return self.not_found("miner not found");
        };
        self.ok(format!(
            "{{\"address\":\"{}\",\"stake\":{},\"settled_tensor_work\":{}}}",
            hex(&miner.address),
            miner.stake,
            miner.settled_tensor_work
        ))
    }

    pub(super) fn validator(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid validator address");
        };
        let Some(validator) = self.chain.state().validators().get(&address) else {
            return self.not_found("validator not found");
        };
        self.ok(format!(
            "{{\"address\":\"{}\",\"stake\":{},\"valid_attestations\":{}}}",
            hex(&validator.address),
            validator.stake,
            validator.valid_attestations
        ))
    }

    pub(super) fn health(&self, service: &str) -> RpcResponse {
        self.ok(format!(
            "{{\"status\":\"ok\",\"service\":\"{service}\",\"height\":{},\"epoch\":{},\"block_count\":{},\"faucet_configured\":{}}}",
            self.chain.state().height(),
            self.chain.state().epoch(),
            self.chain.blocks.len(),
            self.faucet.is_some()
        ))
    }
}
