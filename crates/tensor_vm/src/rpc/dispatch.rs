use super::explorer::{
    explorer_jobs, explorer_miners, explorer_overview, explorer_summary, explorer_validators,
};
use super::http::{HttpRequestLineError, parse_http_request_line};
use super::render::{faucet_page_html, telemetry_dashboard_html};
use super::{RpcNode, RpcRequest, RpcResponse};
use crate::telemetry::TelemetrySnapshot;
use tensor_vm_explorer::{explorer_shell_html, jobs_json, miners_json, validators_json};

const MAX_DYNAMIC_ROUTE_SEGMENTS: usize = 4;

impl RpcNode {
    pub fn handle(&self, request: &RpcRequest) -> RpcResponse {
        self.handle_static(request)
            .unwrap_or_else(|| self.handle_dynamic(request))
    }

    fn handle_static(&self, request: &RpcRequest) -> Option<RpcResponse> {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/health") => Some(self.health("all")),
            ("GET", "/rpc/health") => Some(self.health("rpc")),
            ("GET", "/chain/head") => Some(self.chain_head()),
            ("GET", "/epoch/current") => Some(self.current_epoch()),
            ("GET", "/jobs/current") => Some(self.jobs_current()),
            ("GET", "/explorer/health") => Some(self.health("explorer")),
            ("GET", "/explorer") => Some(self.ok(explorer_shell_html("/explorer/ws"))),
            ("GET", "/explorer/summary") => Some(self.ok(explorer_summary(&self.chain).to_json())),
            ("GET", "/explorer/overview") => {
                Some(self.ok(explorer_overview(&self.chain, 10, 20, 20).to_json()))
            }
            ("GET", "/explorer/miners") => {
                Some(self.ok(miners_json(&explorer_miners(&self.chain))))
            }
            ("GET", "/explorer/validators") => {
                Some(self.ok(validators_json(&explorer_validators(&self.chain))))
            }
            ("GET", "/explorer/jobs") => Some(self.ok(jobs_json(&explorer_jobs(&self.chain, 50)))),
            ("GET", "/telemetry/health") => Some(self.health("telemetry")),
            ("GET", "/telemetry") => {
                Some(self.ok(TelemetrySnapshot::from_chain(&self.chain).to_json()))
            }
            ("GET", "/telemetry/dashboard") => Some(self.ok(telemetry_dashboard_html(
                &TelemetrySnapshot::from_chain(&self.chain),
            ))),
            ("GET", "/faucet/health") => Some(self.health("faucet")),
            ("GET", "/faucet") => Some(self.faucet_status()),
            ("GET", "/faucet/page") => Some(self.ok(faucet_page_html(self.faucet.as_ref()))),
            ("POST", "/tx") | ("POST", "/receipt") | ("POST", "/attestation") => {
                Some(self.accepted())
            }
            _ => None,
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
        let request_line = match parse_http_request_line(first_line) {
            Ok(request_line) => request_line,
            Err(HttpRequestLineError::MissingMethod) => {
                return self.bad_request("missing method");
            }
            Err(HttpRequestLineError::MissingPath) => {
                return self.bad_request("missing path");
            }
        };
        self.handle(&RpcRequest {
            method: request_line.method.to_owned(),
            path: request_line.path.to_owned(),
            body: Vec::new(),
        })
    }

    fn handle_dynamic(&self, request: &RpcRequest) -> RpcResponse {
        let path = DynamicRoutePath::parse(&request.path);
        match (request.method.as_str(), path.segments()) {
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

struct DynamicRoutePath<'a> {
    segments: [&'a str; MAX_DYNAMIC_ROUTE_SEGMENTS],
    len: usize,
    overflow: bool,
}

impl<'a> DynamicRoutePath<'a> {
    fn parse(path: &'a str) -> Self {
        let mut parsed = Self {
            segments: [""; MAX_DYNAMIC_ROUTE_SEGMENTS],
            len: 0,
            overflow: false,
        };
        for segment in path.trim_matches('/').split('/') {
            if parsed.len == MAX_DYNAMIC_ROUTE_SEGMENTS {
                parsed.overflow = true;
                return parsed;
            }
            parsed.segments[parsed.len] = segment;
            parsed.len += 1;
        }
        parsed
    }

    fn segments(&self) -> &[&'a str] {
        if self.overflow {
            &[]
        } else {
            &self.segments[..self.len]
        }
    }
}

#[cfg(test)]
mod dynamic_route_path_tests {
    use super::*;

    #[test]
    fn dynamic_route_path_preserves_current_segment_rules() {
        assert_eq!(
            DynamicRoutePath::parse("/chain/block/0/").segments(),
            &["chain", "block", "0"]
        );
        assert_eq!(
            DynamicRoutePath::parse("chain//block").segments(),
            &["chain", "", "block"]
        );
        assert_eq!(DynamicRoutePath::parse("/").segments(), &[""]);
        assert!(DynamicRoutePath::parse("/a/b/c/d/e").segments().is_empty());
    }
}
