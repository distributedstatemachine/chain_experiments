use super::explorer::{
    explorer_jobs, explorer_miners, explorer_overview, explorer_summary, explorer_validators,
};
use super::render::{faucet_page_html, telemetry_dashboard_html};
use super::{RpcNode, RpcRequest, RpcResponse};
use crate::hash::hex;
use crate::telemetry::TelemetrySnapshot;
use tensor_vm_explorer::{explorer_shell_html, jobs_json, miners_json, validators_json};

impl RpcNode {
    pub fn handle(&self, request: &RpcRequest) -> RpcResponse {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/health") => self.health("all"),
            ("GET", "/rpc/health") => self.health("rpc"),
            ("GET", "/chain/head") => self.ok(format!(
                "{{\"height\":{},\"epoch\":{},\"block_count\":{},\"state_root\":\"{}\"}}",
                self.chain.state().height(),
                self.chain.state().epoch(),
                self.chain.blocks.len(),
                hex(&self.chain.state_root())
            )),
            ("GET", "/epoch/current") => {
                self.ok(format!("{{\"epoch\":{}}}", self.chain.state().epoch()))
            }
            ("GET", "/jobs/current") => self.jobs_current(),
            ("GET", "/explorer/health") => self.health("explorer"),
            ("GET", "/explorer") => self.ok(explorer_shell_html("/explorer/ws")),
            ("GET", "/explorer/summary") => self.ok(explorer_summary(&self.chain).to_json()),
            ("GET", "/explorer/overview") => {
                self.ok(explorer_overview(&self.chain, 10, 20, 20).to_json())
            }
            ("GET", "/explorer/miners") => self.ok(miners_json(&explorer_miners(&self.chain))),
            ("GET", "/explorer/validators") => {
                self.ok(validators_json(&explorer_validators(&self.chain)))
            }
            ("GET", "/explorer/jobs") => self.ok(jobs_json(&explorer_jobs(&self.chain, 50))),
            ("GET", "/telemetry/health") => self.health("telemetry"),
            ("GET", "/telemetry") => self.ok(TelemetrySnapshot::from_chain(&self.chain).to_json()),
            ("GET", "/telemetry/dashboard") => self.ok(telemetry_dashboard_html(
                &TelemetrySnapshot::from_chain(&self.chain),
            )),
            ("GET", "/faucet/health") => self.health("faucet"),
            ("GET", "/faucet") => self.faucet_status(),
            ("GET", "/faucet/page") => self.ok(faucet_page_html(self.faucet.as_ref())),
            ("POST", "/tx") | ("POST", "/receipt") | ("POST", "/attestation") => self.accepted(),
            _ => self.handle_dynamic(request),
        }
    }

    pub fn handle_mut(&mut self, request: &RpcRequest) -> RpcResponse {
        match (request.method.as_str(), request.path.as_str()) {
            ("POST", "/tx") => self.submit_transaction(request),
            ("POST", "/receipt") => self.submit_receipt_reference(request),
            ("POST", "/attestation") => self.submit_attestation_reference(request),
            _ if request.method == "POST" && request.path.starts_with("/faucet/claim/") => {
                self.submit_faucet_claim(request)
            }
            _ => self.handle(request),
        }
    }

    pub fn handle_http_text(&self, raw: &str) -> RpcResponse {
        let Some(first_line) = raw.lines().next() else {
            return self.bad_request("empty request");
        };
        let mut parts = first_line.split_whitespace();
        let Some(method) = parts.next() else {
            return self.bad_request("missing method");
        };
        let Some(path) = parts.next() else {
            return self.bad_request("missing path");
        };
        self.handle(&RpcRequest {
            method: method.to_owned(),
            path: path.to_owned(),
            body: Vec::new(),
        })
    }

    fn handle_dynamic(&self, request: &RpcRequest) -> RpcResponse {
        let path_parts: Vec<&str> = request.path.trim_matches('/').split('/').collect();
        match (request.method.as_str(), path_parts.as_slice()) {
            ("GET", ["chain", "block", height]) => self.chain_block(height),
            ("GET", ["receipts", receipt_id]) => self.receipt(receipt_id),
            ("GET", ["miners", address]) => self.miner(address),
            ("GET", ["validators", address]) => self.validator(address),
            ("GET", ["explorer", "account", address]) => self.explorer_account(address),
            ("GET", ["explorer", "blocks", "latest", limit]) => self.explorer_latest_blocks(limit),
            ("GET", ["explorer", "receipts", "latest", limit]) => {
                self.explorer_latest_receipts(limit)
            }
            ("GET", ["tensor", tensor_id, "descriptor"]) => self.tensor_descriptor(tensor_id),
            ("GET", ["tensor", tensor_id, "chunk", chunk_index]) => {
                self.tensor_chunk(tensor_id, chunk_index)
            }
            ("GET", ["tensor", tensor_id, "row", row_index]) => {
                self.tensor_row(tensor_id, row_index)
            }
            ("GET", ["tensor", tensor_id, "opening", chunk_index]) => {
                self.tensor_opening(tensor_id, chunk_index)
            }
            ("GET", ["tensor", "latest"]) => self.tensor_latest(),
            ("GET", ["jobs", job_id]) => self.job(job_id),
            _ => self.not_found("route not found"),
        }
    }
}
