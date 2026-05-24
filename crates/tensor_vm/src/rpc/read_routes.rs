use super::render::job_value;
use super::{RpcNode, RpcResponse, parse_hash};
use crate::hash::hex;
use serde_json::json;

impl RpcNode {
    pub(super) fn chain_head(&self) -> RpcResponse {
        self.ok(json!({
            "height": self.chain.state().height(),
            "epoch": self.chain.state().epoch(),
            "block_count": self.chain.blocks.len(),
            "state_root": hex(&self.chain.state_root()),
        })
        .to_string())
    }

    pub(super) fn current_epoch(&self) -> RpcResponse {
        self.ok(json!({ "epoch": self.chain.state().epoch() }).to_string())
    }

    pub(super) fn chain_block(&self, height: &str) -> RpcResponse {
        let Ok(height) = height.parse::<usize>() else {
            return self.bad_request("invalid block height");
        };
        let Some(block) = self.chain.blocks.get(height) else {
            return self.not_found("block not found");
        };
        self.ok(json!({
            "height": block.height,
            "epoch": block.epoch,
            "hash": hex(&block.hash()),
        })
        .to_string())
    }

    pub(super) fn receipt(&self, receipt_id: &str) -> RpcResponse {
        let Ok(receipt_id) = parse_hash(receipt_id) else {
            return self.bad_request("invalid receipt id");
        };
        let Some(receipt) = self.chain.state().receipts().get(&receipt_id) else {
            return self.not_found("receipt not found");
        };
        self.ok(json!({
            "receipt_id": hex(&receipt.receipt_id()),
            "job_id": hex(&receipt.job_id()),
            "tensor_work_units": receipt.tensor_work_units(),
        })
        .to_string())
    }

    pub(super) fn faucet_status(&self) -> RpcResponse {
        let Some(faucet) = &self.faucet else {
            return self.not_found("faucet not configured");
        };
        self.ok(json!({
            "balance": faucet.balance(),
            "drip_amount": faucet.drip_amount(),
        })
        .to_string())
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
        self.ok(json!({
            "address": hex(&miner.address),
            "stake": miner.stake,
            "settled_tensor_work": miner.settled_tensor_work,
        })
        .to_string())
    }

    pub(super) fn validator(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid validator address");
        };
        let Some(validator) = self.chain.state().validators().get(&address) else {
            return self.not_found("validator not found");
        };
        self.ok(json!({
            "address": hex(&validator.address),
            "stake": validator.stake,
            "valid_attestations": validator.valid_attestations,
        })
        .to_string())
    }

    pub(super) fn health(&self, service: &str) -> RpcResponse {
        self.ok(json!({
            "status": "ok",
            "service": service,
            "height": self.chain.state().height(),
            "epoch": self.chain.state().epoch(),
            "block_count": self.chain.blocks.len(),
            "faucet_configured": self.faucet.is_some(),
        })
        .to_string())
    }
}
