use crate::chain::{Chain, ChainCommand, ChainEngine, JobState};
use crate::error::{Result, TvmError};
use crate::faucet::Faucet;
use crate::hash::hex;
#[cfg(test)]
use crate::jobs::PrimitiveType;
use crate::localnet::{
    produce_synthetic_cpu_round_with_profile, produce_synthetic_cpu_round_with_tensors,
};
use crate::profile::ChainProfile;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DEFAULT_CHUNK_SIZE, Tensor};
use crate::txpool::{TxPool, parse_transaction_envelope};
use crate::types::{Address, Hash};
use std::collections::BTreeMap;
use std::io::Write;
#[cfg(test)]
use std::net::SocketAddr;
use std::net::TcpStream;
use tensor_vm_explorer::{
    account_json, blocks_json, explorer_shell_html, jobs_json, miners_json, receipts_json,
    validators_json,
};

mod explorer;
mod http;
mod websocket;
use explorer::{
    explorer_account, explorer_blocks, explorer_jobs, explorer_miners, explorer_overview,
    explorer_receipts, explorer_summary, explorer_validators,
};
#[cfg(test)]
use explorer::{hardware_class_label, primitive_label};
#[cfg(test)]
use http::{
    ParsedHttpRequest, read_http_request_from, split_path_and_auth_token, try_parse_http_request,
};
pub use http::{RpcHttpServer, http_response_text};
#[cfg(test)]
use websocket::{base64_encode, websocket_accept_key, write_websocket_frame};
use websocket::{
    json_string_field, json_usize_field, read_websocket_text_frame, write_websocket_close,
    write_websocket_text,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcRequest {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct RpcNode {
    pub chain: Chain,
    pub txpool: TxPool,
    pub faucet: Option<Faucet>,
    tensors: BTreeMap<Hash, Tensor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcPolicy {
    pub auth_token: Option<String>,
    pub max_body_bytes: usize,
    pub max_requests_per_client: u64,
}

impl Default for RpcPolicy {
    fn default() -> Self {
        Self {
            auth_token: None,
            max_body_bytes: 1024 * 1024,
            max_requests_per_client: 1_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RpcGateway {
    pub node: RpcNode,
    pub policy: RpcPolicy,
    request_counts: BTreeMap<String, u64>,
}

impl RpcGateway {
    pub fn new(node: RpcNode, policy: RpcPolicy) -> Self {
        Self {
            node,
            policy,
            request_counts: BTreeMap::new(),
        }
    }

    pub fn handle(
        &mut self,
        client_id: &str,
        auth_token: Option<&str>,
        request: &RpcRequest,
    ) -> RpcResponse {
        if request.body.len() > self.policy.max_body_bytes {
            return RpcNode::response(413, "request body too large");
        }
        if let Some(response) = self.authorize_request(client_id, auth_token) {
            return response;
        }
        self.node.handle_mut(request)
    }

    fn authorize_request(
        &mut self,
        client_id: &str,
        auth_token: Option<&str>,
    ) -> Option<RpcResponse> {
        if let Some(required) = &self.policy.auth_token
            && auth_token != Some(required.as_str())
        {
            return Some(RpcNode::response(401, "unauthorized"));
        }
        let count = self.request_counts.entry(client_id.to_owned()).or_default();
        if *count >= self.policy.max_requests_per_client {
            return Some(RpcNode::response(429, "rate limit exceeded"));
        }
        *count += 1;
        None
    }

    pub fn request_count(&self, client_id: &str) -> u64 {
        self.request_counts.get(client_id).copied().unwrap_or(0)
    }
}

impl RpcNode {
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            txpool: TxPool::default(),
            faucet: None,
            tensors: BTreeMap::new(),
        }
    }

    pub fn with_faucet(chain: Chain, faucet: Faucet) -> Self {
        Self {
            chain,
            txpool: TxPool::default(),
            faucet: Some(faucet),
            tensors: BTreeMap::new(),
        }
    }

    pub fn insert_tensor(&mut self, tensor: Tensor) -> Hash {
        let id = tensor.tensor_id();
        self.tensors.insert(id, tensor);
        id
    }

    pub fn tensor_by_commitment_root(&self, commitment_root: &Hash) -> Option<&Tensor> {
        self.tensors
            .values()
            .find(|tensor| tensor.commitment_root() == *commitment_root)
    }

    pub fn contains_tensor_commitment_root(&self, commitment_root: &Hash) -> bool {
        self.tensor_by_commitment_root(commitment_root).is_some()
    }

    pub fn produce_synthetic_cpu_round(&mut self) -> Result<Option<u64>> {
        let Some(round) = produce_synthetic_cpu_round_with_tensors(&mut self.chain)? else {
            return Ok(None);
        };
        for tensor in round.tensors {
            self.insert_tensor(tensor);
        }
        Ok(Some(round.height))
    }

    pub fn produce_synthetic_cpu_round_with_profile(
        &mut self,
        profile: &ChainProfile,
    ) -> Result<Option<u64>> {
        let Some(round) = produce_synthetic_cpu_round_with_profile(&mut self.chain, profile)?
        else {
            return Ok(None);
        };
        for tensor in round.tensors {
            self.insert_tensor(tensor);
        }
        Ok(Some(round.height))
    }

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

    fn chain_block(&self, height: &str) -> RpcResponse {
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

    fn receipt(&self, receipt_id: &str) -> RpcResponse {
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

    fn explorer_account(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid account address");
        };
        self.ok(account_json(&explorer_account(&self.chain, &address)))
    }

    fn explorer_latest_blocks(&self, limit: &str) -> RpcResponse {
        let Ok(limit) = limit.parse::<usize>() else {
            return self.bad_request("invalid block limit");
        };
        self.ok(blocks_json(&explorer_blocks(&self.chain, limit)))
    }

    fn explorer_latest_receipts(&self, limit: &str) -> RpcResponse {
        let Ok(limit) = limit.parse::<usize>() else {
            return self.bad_request("invalid receipt limit");
        };
        self.ok(receipts_json(&explorer_receipts(&self.chain, limit)))
    }

    fn faucet_status(&self) -> RpcResponse {
        let Some(faucet) = &self.faucet else {
            return self.not_found("faucet not configured");
        };
        self.ok(format!(
            "{{\"balance\":{},\"drip_amount\":{}}}",
            faucet.balance(),
            faucet.drip_amount()
        ))
    }

    fn submit_faucet_claim(&mut self, request: &RpcRequest) -> RpcResponse {
        let Some(address) = request.path.strip_prefix("/faucet/claim/") else {
            return self.not_found("route not found");
        };
        let Ok(address) = parse_address(address) else {
            return self.bad_request("invalid faucet address");
        };
        let Some(faucet) = self.faucet.as_mut() else {
            return self.not_found("faucet not configured");
        };
        match faucet.claim(address, self.chain.state().epoch()) {
            Ok(amount) => match self
                .chain
                .apply_command(ChainCommand::CreditReward { address, amount })
            {
                Ok(_) => {
                    let balance = faucet.balance();
                    self.ok(format!(
                        "{{\"claimed\":{},\"address\":\"{}\",\"faucet_balance\":{}}}",
                        amount,
                        hex(&address),
                        balance
                    ))
                }
                Err(error) => self.bad_request(&error.to_string()),
            },
            Err(error) => self.bad_request(&error.to_string()),
        }
    }

    fn submit_transaction(&mut self, request: &RpcRequest) -> RpcResponse {
        let envelope = match parse_transaction_envelope(&request.body) {
            Ok(envelope) => envelope,
            Err(error) => return self.bad_request(&error.to_string()),
        };
        if envelope.transaction.is_reference_submission() {
            return if self.txpool.submit_envelope(&envelope) {
                self.accepted()
            } else {
                self.conflict("duplicate transaction")
            };
        }
        match self
            .chain
            .apply_transaction(envelope.sender, envelope.transaction.clone())
        {
            Ok(_) => {
                self.txpool.submit_envelope(&envelope);
                self.accepted()
            }
            Err(error) => self.bad_request(&error.to_string()),
        }
    }

    fn submit_receipt_reference(&mut self, request: &RpcRequest) -> RpcResponse {
        let mut body = b"submit_tensor_receipt ".to_vec();
        body.extend_from_slice(&request.body);
        self.submit_reference_payload(&body)
    }

    fn submit_attestation_reference(&mut self, request: &RpcRequest) -> RpcResponse {
        let mut body = b"submit_attestation ".to_vec();
        body.extend_from_slice(&request.body);
        self.submit_reference_payload(&body)
    }

    fn submit_reference_payload(&mut self, body: &[u8]) -> RpcResponse {
        let envelope = match parse_transaction_envelope(body) {
            Ok(envelope) => envelope,
            Err(error) => return self.bad_request(&error.to_string()),
        };
        if self.txpool.submit_envelope(&envelope) {
            self.accepted()
        } else {
            self.conflict("duplicate transaction")
        }
    }

    fn jobs_current(&self) -> RpcResponse {
        let jobs: Vec<_> = self.chain.state().jobs().values().map(job_json).collect();
        self.ok(format!("{{\"jobs\":[{}]}}", jobs.join(",")))
    }

    fn job(&self, job_id: &str) -> RpcResponse {
        let Ok(job_id) = parse_hash(job_id) else {
            return self.bad_request("invalid job id");
        };
        let Some(job) = self.chain.state().jobs().get(&job_id) else {
            return self.not_found("job not found");
        };
        self.ok(job_json(job))
    }

    fn miner(&self, address: &str) -> RpcResponse {
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

    fn validator(&self, address: &str) -> RpcResponse {
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

    fn tensor_descriptor(&self, tensor_id: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let descriptor = tensor.descriptor();
        self.ok(format!(
            "{{\"tensor_id\":\"{}\",\"shape\":{},\"byte_size\":{},\"root\":\"{}\"}}",
            hex(&descriptor.tensor_id),
            json_usize_array(&descriptor.shape),
            descriptor.byte_size,
            hex(&descriptor.commitment.root)
        ))
    }

    fn tensor_chunk(&self, tensor_id: &str, chunk_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(chunk_index) = chunk_index.parse::<u64>() else {
            return self.bad_request("invalid chunk index");
        };
        match tensor.opening(chunk_index, DEFAULT_CHUNK_SIZE) {
            Ok(opening) => self.ok(format!(
                "{{\"tensor_id\":\"{}\",\"chunk_index\":{},\"bytes\":\"{}\"}}",
                hex(&opening.tensor_id),
                opening.chunk_index,
                hex(&opening.chunk_bytes)
            )),
            Err(_) => self.not_found("chunk not found"),
        }
    }

    fn tensor_row(&self, tensor_id: &str, row_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(row_index) = row_index.parse::<usize>() else {
            return self.bad_request("invalid row index");
        };
        match tensor.row(row_index) {
            Ok(row) => self.ok(format!("{{\"row\":{}}}", json_u64_array(row))),
            Err(_) => self.not_found("row not found"),
        }
    }

    fn tensor_opening(&self, tensor_id: &str, chunk_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(chunk_index) = chunk_index.parse::<u64>() else {
            return self.bad_request("invalid chunk index");
        };
        match tensor.opening(chunk_index, DEFAULT_CHUNK_SIZE) {
            Ok(opening) => self.ok(format!(
                "{{\"tensor_id\":\"{}\",\"chunk_index\":{},\"proof_len\":{}}}",
                hex(&opening.tensor_id),
                opening.chunk_index,
                opening.merkle_proof.siblings.len()
            )),
            Err(_) => self.not_found("opening not found"),
        }
    }

    fn tensor_latest(&self) -> RpcResponse {
        let Some((tensor_id, tensor)) = self.tensors.iter().next_back() else {
            return self.not_found("tensor not found");
        };
        self.ok(format!(
            "{{\"tensor_id\":\"{}\",\"tensor_count\":{},\"root\":\"{}\"}}",
            hex(tensor_id),
            self.tensors.len(),
            hex(&tensor.commitment_root())
        ))
    }

    fn lookup_tensor(&self, tensor_id: &str) -> Option<&Tensor> {
        parse_hash(tensor_id)
            .ok()
            .and_then(|tensor_id| self.tensors.get(&tensor_id))
    }

    fn health(&self, service: &str) -> RpcResponse {
        self.ok(format!(
            "{{\"status\":\"ok\",\"service\":\"{service}\",\"height\":{},\"epoch\":{},\"block_count\":{},\"faucet_configured\":{}}}",
            self.chain.state().height(),
            self.chain.state().epoch(),
            self.chain.blocks.len(),
            self.faucet.is_some()
        ))
    }

    fn ok(&self, body: String) -> RpcResponse {
        RpcResponse { status: 200, body }
    }

    fn accepted(&self) -> RpcResponse {
        RpcResponse {
            status: 202,
            body: "{\"accepted\":true}".to_owned(),
        }
    }

    fn bad_request(&self, message: &str) -> RpcResponse {
        Self::response(400, message)
    }

    fn not_found(&self, message: &str) -> RpcResponse {
        Self::response(404, message)
    }

    fn conflict(&self, message: &str) -> RpcResponse {
        Self::response(409, message)
    }

    fn response(status: u16, message: &str) -> RpcResponse {
        RpcResponse {
            status,
            body: format!("{{\"error\":\"{message}\"}}"),
        }
    }

    fn serve_explorer_websocket_once(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let command = match read_websocket_text_frame(stream)? {
            Some(command) => command,
            None => {
                write_websocket_close(stream)?;
                return stream.flush();
            }
        };
        let body = self.explorer_websocket_response(&command);
        write_websocket_text(stream, &body)?;
        write_websocket_close(stream)?;
        stream.flush()
    }

    fn explorer_websocket_response(&self, command: &str) -> String {
        let command = command.trim();
        if command.contains("\"type\":\"account\"") || command.contains("\"type\": \"account\"") {
            let Some(address) = json_string_field(command, "address") else {
                return "{\"type\":\"error\",\"error\":\"missing account address\"}".to_owned();
            };
            let Ok(address) = parse_hash(&address) else {
                return "{\"type\":\"error\",\"error\":\"invalid account address\"}".to_owned();
            };
            return account_json(&explorer_account(&self.chain, &address));
        }
        if command == "summary" || command.contains("\"type\":\"summary\"") {
            return format!(
                "{{\"type\":\"summary\",\"summary\":{}}}",
                explorer_summary(&self.chain).to_json()
            );
        }
        if command == "miners" || command.contains("\"type\":\"miners\"") {
            return miners_json(&explorer_miners(&self.chain));
        }
        if command == "validators" || command.contains("\"type\":\"validators\"") {
            return validators_json(&explorer_validators(&self.chain));
        }
        if command == "jobs" || command.contains("\"type\":\"jobs\"") {
            let limit = json_usize_field(command, "job_limit").unwrap_or(50);
            return jobs_json(&explorer_jobs(&self.chain, limit));
        }
        if command == "receipts" || command.contains("\"type\":\"receipts\"") {
            let limit = json_usize_field(command, "receipt_limit").unwrap_or(50);
            return receipts_json(&explorer_receipts(&self.chain, limit));
        }
        if command == "blocks" || command.contains("\"type\":\"blocks\"") {
            let limit = json_usize_field(command, "block_limit").unwrap_or(25);
            return blocks_json(&explorer_blocks(&self.chain, limit));
        }
        let block_limit = json_usize_field(command, "block_limit").unwrap_or(12);
        let receipt_limit = json_usize_field(command, "receipt_limit").unwrap_or(20);
        let job_limit = json_usize_field(command, "job_limit").unwrap_or(20);
        explorer_overview(&self.chain, block_limit, receipt_limit, job_limit).to_json()
    }
}

fn parse_hash(value: &str) -> Result<Hash> {
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash length"));
    }
    let mut out = [0_u8; 32];
    for (i, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        out[i] = (high << 4) | low;
    }
    Ok(out)
}

fn parse_address(value: &str) -> Result<Address> {
    parse_hash(value)
}

fn hex_value(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid hex")),
    }
}

fn json_usize_array(values: &[usize]) -> String {
    let parts: Vec<_> = values.iter().map(|value| value.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn json_u64_array(values: &[u64]) -> String {
    let parts: Vec<_> = values.iter().map(|value| value.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn telemetry_dashboard_html(snapshot: &TelemetrySnapshot) -> String {
    html_document(
        "TensorVM Telemetry",
        format!(
            "<section><h1>Telemetry Dashboard</h1><dl>{}</dl></section>",
            metric_rows(&[
                (
                    "Block Finality Rate",
                    format!("{:.6}", snapshot.block_finality_rate),
                ),
                (
                    "Average Block Time",
                    format!("{:.6}", snapshot.average_block_time),
                ),
                (
                    "Data Availability Rate",
                    format!("{:.6}", snapshot.data_availability_rate),
                ),
                (
                    "Invalid Receipts Submitted",
                    snapshot.invalid_receipts_submitted.to_string(),
                ),
                (
                    "Validator Disagreement Rate",
                    format!("{:.6}", snapshot.validator_disagreement_rate),
                ),
                ("Total TensorWork", snapshot.total_tensor_work.to_string(),),
                (
                    "Max Miner Work Share",
                    format!("{:.6}", snapshot.max_miner_work_share),
                ),
                (
                    "GPU Utilization",
                    format!("{:.6}", snapshot.estimated_gpu_utilization),
                ),
                (
                    "Hardware Classes",
                    snapshot.hardware_class_participation.to_string(),
                ),
            ]),
        ),
    )
}

fn faucet_page_html(faucet: Option<&Faucet>) -> String {
    let rows = match faucet {
        Some(faucet) => metric_rows(&[
            ("Balance", faucet.balance().to_string()),
            ("Drip Amount", faucet.drip_amount().to_string()),
        ]),
        None => metric_rows(&[("Status", "Not configured".to_owned())]),
    };
    html_document(
        "TensorVM Faucet",
        format!("<section><h1>Faucet</h1><dl>{rows}</dl></section>"),
    )
}

fn metric_rows(rows: &[(&str, String)]) -> String {
    rows.iter()
        .map(|(name, value)| format!("<dt>{name}</dt><dd>{value}</dd>"))
        .collect::<Vec<_>>()
        .join("")
}

fn html_document(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>body{{font-family:system-ui,sans-serif;margin:0;background:#f7f7f4;color:#151515}}main{{max-width:960px;margin:0 auto;padding:32px}}section{{border-top:1px solid #d8d8d0;padding:20px 0}}dl{{display:grid;grid-template-columns:minmax(160px,260px)1fr;gap:8px 16px}}dt{{font-weight:700}}dd{{margin:0}}code{{font-size:12px;word-break:break-all}}</style></head><body><main>{body}</main></body></html>"
    )
}

fn job_json(job: &JobState) -> String {
    match job {
        JobState::TensorOp(job) => format!(
            "{{\"job_id\":\"{}\",\"primitive_type\":\"tensor_op\",\"epoch\":{},\"m\":{},\"k\":{},\"n\":{},\"deadline_block\":{},\"reward_weight\":{}}}",
            hex(&job.job_id),
            job.epoch,
            job.m,
            job.k,
            job.n,
            job.deadline_block,
            job.reward_weight
        ),
        JobState::LinearTrainingStep(job) => format!(
            "{{\"job_id\":\"{}\",\"primitive_type\":\"linear_training_step\",\"model_id\":\"{}\",\"step\":{},\"input_shape\":{},\"weight_shape\":{},\"target_shape\":{},\"deadline_block\":{},\"reward_weight\":{}}}",
            hex(&job.job_id),
            hex(&job.model_id),
            job.step,
            json_usize_array(&job.input_shape),
            json_usize_array(&job.weight_shape),
            json_usize_array(&job.target_shape),
            job.deadline_block,
            job.reward_weight
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainParams, HardwareClass, JobState};
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, TensorOpReceipt};
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

    #[test]
    fn node_rpc_serves_head_and_blocks() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let proposer = address(b"proposer");
        chain.register_validator(proposer, 10_000).unwrap();
        chain.produce_block(proposer, 1000).unwrap();
        let rpc = RpcNode::new(chain);

        let head = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/chain/head".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(head.status, 200);
        assert!(head.body.contains("\"height\":1"));
        assert!(head.body.contains("\"state_root\""));
        assert!(head.body.len() >= 64);

        let health = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/health".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(health.status, 200);
        assert!(health.body.contains("\"status\":\"ok\""));
        assert!(health.body.contains("\"service\":\"all\""));
        assert!(health.body.contains("\"block_count\":1"));

        let rpc_health = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/rpc/health".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(rpc_health.status, 200);
        assert!(rpc_health.body.contains("\"service\":\"rpc\""));

        let block = rpc.handle_http_text("GET /chain/block/0 HTTP/1.1\r\n\r\n");
        assert_eq!(block.status, 200);
        assert!(block.body.contains("\"height\":0"));
    }

    #[test]
    fn node_rpc_serves_miner_and_validator_state() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let rpc = RpcNode::new(chain);

        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path: format!("/miners/{}", hex(&miner)),
                body: Vec::new(),
            })
            .status,
            200
        );
        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path: format!("/validators/{}", hex(&validator)),
                body: Vec::new(),
            })
            .status,
            200
        );
    }

    #[test]
    fn node_rpc_serves_current_jobs_and_job_lookup() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let job = MatmulJob::synthetic(0, 9, 4, 5, 6, &beacon, 20);
        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"rpc-linear-model"]),
            step: 7,
            batch_seed: hash_bytes(b"test", &[b"rpc-linear-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 2,
            deadline_block: 30,
        });
        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_job(JobState::LinearTrainingStep(linear_job.clone()));
        let rpc = RpcNode::new(chain);

        let current = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/jobs/current".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(current.status, 200);
        assert!(current.body.contains("\"primitive_type\":\"tensor_op\""));
        assert!(
            current
                .body
                .contains("\"primitive_type\":\"linear_training_step\"")
        );
        assert!(current.body.contains("\"m\":4"));
        assert!(current.body.contains("\"input_shape\":[3,2]"));

        let response = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/jobs/{}", hex(&job.job_id)),
            body: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"deadline_block\":20"));

        let response = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/jobs/{}", hex(&linear_job.job_id)),
            body: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"step\":7"));
    }

    #[test]
    fn node_rpc_serves_explorer_telemetry_and_faucet_routes() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"rpc-service-miner");
        let user = address(b"rpc-faucet-user");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(miner, 10_000).unwrap();
        chain.produce_block(miner, 1000).unwrap();
        let mut rpc = RpcNode::with_faucet(chain, Faucet::new(1_000, 100));

        let summary = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/summary".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(summary.status, 200);
        assert!(summary.body.contains("\"miner_count\":1"));
        assert!(summary.body.contains("\"job_count\":0"));

        let overview = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/overview".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(overview.status, 200);
        assert!(overview.body.contains("\"type\":\"overview\""));
        assert!(overview.body.contains("\"blocks\""));
        assert!(overview.body.contains("\"miners\""));

        let account = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/explorer/account/{}", hex(&miner)),
            body: Vec::new(),
        });
        assert_eq!(account.status, 200);
        assert!(account.body.contains("\"is_miner\":true"));

        let blocks = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/blocks/latest/1".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(blocks.status, 200);
        assert!(blocks.body.contains("\"blocks\""));

        let miners = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/miners".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(miners.status, 200);
        assert!(miners.body.contains("\"hardware_class\":\"cpu\""));

        let validators = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/validators".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(validators.status, 200);
        assert!(validators.body.contains("\"validators\""));

        let receipts = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/receipts/latest/5".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(receipts.status, 200);
        assert!(receipts.body.contains("\"receipts\""));
        let bad_receipts = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/receipts/latest/nope".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(bad_receipts.status, 400);

        let jobs = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/jobs".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(jobs.status, 200);
        assert!(jobs.body.contains("\"jobs\""));

        let explorer_page = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(explorer_page.status, 200);
        assert!(explorer_page.body.starts_with("<!doctype html>"));
        assert!(explorer_page.body.contains("TensorVM Explorer"));
        assert!(explorer_page.body.contains("new WebSocket"));

        let explorer_health = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/health".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(explorer_health.status, 200);
        assert!(explorer_health.body.contains("\"service\":\"explorer\""));

        let telemetry = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/telemetry".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(telemetry.status, 200);
        assert!(telemetry.body.contains("\"block_finality_rate\""));

        let telemetry_page = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/telemetry/dashboard".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(telemetry_page.status, 200);
        assert!(telemetry_page.body.contains("Telemetry Dashboard"));

        let telemetry_health = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/telemetry/health".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(telemetry_health.status, 200);
        assert!(telemetry_health.body.contains("\"service\":\"telemetry\""));

        let faucet = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/faucet".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(faucet.status, 200);
        assert!(faucet.body.contains("\"drip_amount\":100"));

        let faucet_page = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/faucet/page".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(faucet_page.status, 200);
        assert!(faucet_page.body.contains("<h1>Faucet</h1>"));
        assert!(faucet_page.body.contains("<dd>100</dd>"));

        let faucet_health = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/faucet/health".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(faucet_health.status, 200);
        assert!(faucet_health.body.contains("\"service\":\"faucet\""));
        assert!(faucet_health.body.contains("\"faucet_configured\":true"));

        let claim = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: format!("/faucet/claim/{}", hex(&user)),
            body: Vec::new(),
        });
        assert_eq!(claim.status, 200);
        assert_eq!(rpc.chain.state().rewards().balance(&user), 100);
        assert_eq!(rpc.faucet.as_ref().unwrap().balance(), 900);

        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: format!("/faucet/claim/{}", hex(&user)),
            body: Vec::new(),
        });
        assert_eq!(duplicate.status, 400);
        assert_eq!(rpc.chain.state().rewards().balance(&user), 100);
        assert_eq!(rpc.faucet.as_ref().unwrap().balance(), 900);

        let missing_faucet = RpcNode::new(Chain::new(beacon)).handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/faucet".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(missing_faucet.status, 404);
    }

    #[test]
    fn mutable_rpc_applies_transactions_and_queues_submissions() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut rpc = RpcNode::new(Chain::new(beacon));
        let miner = address(b"rpc-miner");
        let receiver = address(b"rpc-receiver");

        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("register_miner {}", hex(&miner)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        assert!(rpc.chain.state().miners().contains_key(&miner));

        rpc.chain.credit_account(miner, 100);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("transfer {} {} 70", hex(&miner), hex(&receiver)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        assert_eq!(
            rpc.chain.state().accounts().get(&receiver).unwrap().balance,
            70
        );

        let tx_receipt = hash_bytes(b"test", &[b"tx-receipt"]);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_tensor_receipt {}", hex(&tx_receipt)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_tensor_receipt {}", hex(&tx_receipt)).into_bytes(),
        });
        assert_eq!(duplicate.status, 409);

        let linear_receipt = hash_bytes(b"test", &[b"tx-linear-receipt"]);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_linear_receipt {}", hex(&linear_receipt)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_linear_receipt {}", hex(&linear_receipt)).into_bytes(),
        });
        assert_eq!(duplicate.status, 409);

        let tx_attestation = hash_bytes(b"test", &[b"tx-attestation"]);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_attestation {}", hex(&tx_attestation)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_attestation {}", hex(&tx_attestation)).into_bytes(),
        });
        assert_eq!(duplicate.status, 202);
        assert!(rpc.chain.state().receipts().is_empty());
        assert!(rpc.chain.state().attestations().is_empty());

        let receipt = hash_bytes(b"test", &[b"receipt"]);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/receipt".to_owned(),
            body: hex(&receipt).into_bytes(),
        });
        assert_eq!(response.status, 202);
        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/receipt".to_owned(),
            body: hex(&receipt).into_bytes(),
        });
        assert_eq!(duplicate.status, 409);

        let attestation = hash_bytes(b"test", &[b"attestation"]);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/attestation".to_owned(),
            body: hex(&attestation).into_bytes(),
        });
        assert_eq!(response.status, 202);
        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/attestation".to_owned(),
            body: hex(&attestation).into_bytes(),
        });
        assert_eq!(duplicate.status, 202);

        let accepted_preview = rpc.handle(&RpcRequest {
            method: "POST".to_owned(),
            path: "/attestation".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(accepted_preview.status, 202);
    }

    #[test]
    fn mutable_rpc_rejects_bad_transaction_payloads_without_mutating_state() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut rpc = RpcNode::new(Chain::new(beacon));
        let sender = address(b"rpc-sender");
        let receiver = address(b"rpc-receiver");
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("transfer {} {} 1", hex(&sender), hex(&receiver)).into_bytes(),
        });
        assert_eq!(response.status, 400);
        let malformed = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: b"not_a_transaction".to_vec(),
        });
        assert_eq!(malformed.status, 400);
        let malformed_receipt = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/receipt".to_owned(),
            body: b"not-a-hex-receipt".to_vec(),
        });
        assert_eq!(malformed_receipt.status, 400);
        let malformed_attestation = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/attestation".to_owned(),
            body: b"not-a-hex-attestation".to_vec(),
        });
        assert_eq!(malformed_attestation.status, 400);
        assert!(rpc.txpool.is_empty());
        assert_eq!(
            rpc.chain
                .state()
                .accounts()
                .get(&receiver)
                .map(|account| account.balance),
            None
        );
    }

    #[test]
    fn rpc_rejects_malformed_requests_and_missing_resources() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let missing = hash_bytes(b"test", &[b"missing"]);
        let mut rpc = RpcNode::new(Chain::new(beacon));

        assert_eq!(rpc.handle_http_text("").status, 400);
        assert_eq!(rpc.handle_http_text("\r\n").status, 400);
        assert_eq!(rpc.handle_http_text("GET").status, 400);
        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path: "/not-a-route".to_owned(),
                body: Vec::new(),
            })
            .status,
            404
        );
        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path: "/epoch/current".to_owned(),
                body: Vec::new(),
            })
            .status,
            200
        );

        for (path, expected_status) in [
            ("/chain/block/nope".to_owned(), 400),
            ("/chain/block/9".to_owned(), 404),
            ("/receipts/nope".to_owned(), 400),
            (format!("/receipts/{}", hex(&missing)), 404),
            ("/explorer/account/nope".to_owned(), 400),
            ("/explorer/blocks/latest/nope".to_owned(), 400),
            ("/jobs/nope".to_owned(), 400),
            (format!("/jobs/{}", hex(&missing)), 404),
            ("/miners/nope".to_owned(), 400),
            (format!("/miners/{}", hex(&missing)), 404),
            ("/validators/nope".to_owned(), 400),
            (format!("/validators/{}", hex(&missing)), 404),
        ] {
            assert_eq!(
                rpc.handle(&RpcRequest {
                    method: "GET".to_owned(),
                    path,
                    body: Vec::new(),
                })
                .status,
                expected_status
            );
        }

        assert!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path: "/faucet/page".to_owned(),
                body: Vec::new(),
            })
            .body
            .contains("Not configured")
        );
        assert_eq!(
            rpc.handle_mut(&RpcRequest {
                method: "POST".to_owned(),
                path: "/faucet/claim/nope".to_owned(),
                body: Vec::new(),
            })
            .status,
            400
        );
        assert_eq!(
            rpc.handle_mut(&RpcRequest {
                method: "POST".to_owned(),
                path: format!("/faucet/claim/{}", hex(&missing)),
                body: Vec::new(),
            })
            .status,
            404
        );
        assert_eq!(
            rpc.submit_faucet_claim(&RpcRequest {
                method: "POST".to_owned(),
                path: "/wrong".to_owned(),
                body: Vec::new(),
            })
            .status,
            404
        );

        let user = address(b"exhausted-faucet-user");
        let mut exhausted = RpcNode::with_faucet(Chain::new(beacon), Faucet::new(50, 100));
        assert_eq!(
            exhausted
                .handle_mut(&RpcRequest {
                    method: "POST".to_owned(),
                    path: format!("/faucet/claim/{}", hex(&user)),
                    body: Vec::new(),
                })
                .status,
            400
        );

        let tensor = Tensor::from_vec(vec![1, 2], DType::FieldElement, vec![1, 2]).unwrap();
        let tensor_id = rpc.insert_tensor(tensor);
        for (path, expected_status) in [
            ("/tensor/nope/descriptor".to_owned(), 404),
            (format!("/tensor/{}/descriptor", hex(&missing)), 404),
            (format!("/tensor/{}/chunk/0", hex(&missing)), 404),
            ("/tensor/nope/chunk/0".to_owned(), 404),
            (format!("/tensor/{}/chunk/nope", hex(&tensor_id)), 400),
            (format!("/tensor/{}/chunk/99", hex(&tensor_id)), 404),
            (format!("/tensor/{}/row/0", hex(&missing)), 404),
            ("/tensor/nope/row/0".to_owned(), 404),
            (format!("/tensor/{}/row/nope", hex(&tensor_id)), 400),
            (format!("/tensor/{}/row/99", hex(&tensor_id)), 404),
            (format!("/tensor/{}/opening/0", hex(&missing)), 404),
            ("/tensor/nope/opening/0".to_owned(), 404),
            (format!("/tensor/{}/opening/nope", hex(&tensor_id)), 400),
            (format!("/tensor/{}/opening/99", hex(&tensor_id)), 404),
        ] {
            assert_eq!(
                rpc.handle(&RpcRequest {
                    method: "GET".to_owned(),
                    path,
                    body: Vec::new(),
                })
                .status,
                expected_status
            );
        }

        let receipt = hash_bytes(b"test", &[b"queued-receipt"]);
        assert_eq!(
            rpc.handle_mut(&RpcRequest {
                method: "POST".to_owned(),
                path: "/tx".to_owned(),
                body: format!("submit_tensor_receipt {}", hex(&receipt)).into_bytes(),
            })
            .status,
            202
        );
        assert_eq!(
            rpc.handle_mut(&RpcRequest {
                method: "POST".to_owned(),
                path: "/tx".to_owned(),
                body: format!("submit_tensor_receipt {}", hex(&receipt)).into_bytes(),
            })
            .status,
            409
        );
    }

    #[test]
    fn rpc_serves_receipts_and_status_text_edges() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"rpc-receipt-miner");
        chain.register_miner(miner, 100).unwrap();
        let job = MatmulJob::synthetic(0, 42, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = crate::jobs::TensorOpReceipt::from_job(&job, miner, 1, 5)
            .expect("static matmul receipt should build");
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let rpc = RpcNode::new(chain);

        let response = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/receipts/{}", hex(&receipt.receipt_id)),
            body: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"tensor_work_units\":16"));

        for (status, text) in [
            (400, "Bad Request"),
            (401, "Unauthorized"),
            (404, "Not Found"),
            (413, "Payload Too Large"),
            (999, "Unknown"),
        ] {
            let wire = http_response_text(&RpcResponse {
                status,
                body: "{\"ok\":false}".to_owned(),
            });
            assert!(wire.starts_with(&format!("HTTP/1.1 {status} {text}")));
        }
    }

    #[test]
    fn rpc_http_server_returns_bad_request_and_payload_too_large() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        fn send_raw(addr: SocketAddr, raw: &[u8]) -> String {
            let mut client = TcpStream::connect(addr).unwrap();
            client.write_all(raw).unwrap();
            client.shutdown(Shutdown::Write).unwrap();
            let mut response = String::new();
            client.read_to_string(&mut response).unwrap();
            response
        }

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let gateway = RpcGateway::new(
            RpcNode::new(Chain::new(beacon)),
            RpcPolicy {
                max_body_bytes: 2,
                ..RpcPolicy::default()
            },
        );
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_n(2).unwrap());

        let bad = send_raw(addr, b"GET");
        assert!(bad.starts_with("HTTP/1.1 400 Bad Request"));

        let too_large = send_raw(addr, b"POST /tx HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc");
        assert!(too_large.starts_with("HTTP/1.1 413 Payload Too Large"));

        server_thread.join().unwrap();
    }

    #[test]
    fn rpc_http_parser_rejects_bad_headers_and_waits_for_complete_bodies() {
        let uppercase_hash = hex(&hash_bytes(b"test", &[b"rpc-uppercase"])).to_uppercase();
        assert!(parse_hash(&uppercase_hash).is_ok());
        assert!(parse_hash(&"g".repeat(64)).is_err());

        assert!(matches!(
            try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n\r\n", 16),
            Some(ParsedHttpRequest::Request {
                auth_token: None,
                ..
            })
        ));
        assert!(matches!(
            try_parse_http_request(
                b"POST /tx HTTP/1.1\r\nauthorization: Bearer secret\r\ncontent-length: 4\r\n\r\nbody",
                16,
            ),
            Some(ParsedHttpRequest::Request {
                auth_token: Some(token),
                ..
            }) if token == "secret"
        ));
        assert!(matches!(
            try_parse_http_request(
                b"POST /tx HTTP/1.1\r\nx-tensorchain-auth: local\r\ncontent-length: 4\r\n\r\nbody",
                16,
            ),
            Some(ParsedHttpRequest::Request {
                auth_token: Some(token),
                ..
            }) if token == "local"
        ));
        assert!(matches!(
            try_parse_http_request(
                b"GET /explorer/ws?token=local HTTP/1.1\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
                16,
            ),
            Some(ParsedHttpRequest::WebSocketUpgrade {
                path,
                auth_token: Some(token),
                websocket_key,
            }) if path == "/explorer/ws" && token == "local" && websocket_key == "dGhlIHNhbXBsZSBub25jZQ=="
        ));
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"ws-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(miner, 10_000).unwrap();
        chain.produce_block(miner, 1).unwrap();
        let rpc = RpcNode::new(chain);
        let overview = rpc.explorer_websocket_response(
            "{\"type\":\"overview\",\"block_limit\":1,\"receipt_limit\":1,\"job_limit\":1}",
        );
        assert!(overview.contains("\"type\":\"overview\""));
        assert!(overview.contains("\"block_count\":1"));
        let account = rpc.explorer_websocket_response(&format!(
            "{{\"type\":\"account\",\"address\":\"{}\"}}",
            hex(&miner)
        ));
        assert!(account.contains("\"type\":\"account\""));
        assert!(account.contains("\"is_miner\":true"));
        assert!(matches!(
            try_parse_http_request(b"GET /\xff HTTP/1.1\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"GET\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"GET /chain/head HTTP/1.1\r\nhost\r\n\r\n", 16),
            Some(ParsedHttpRequest::Request { .. })
        ));
        assert!(matches!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: nope\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 17\r\n\r\n", 16),
            Some(ParsedHttpRequest::TooLarge)
        ));
        assert!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbo", 16,)
                .is_none()
        );
        assert!(try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n", 16).is_none());
        assert!(matches!(
            try_parse_http_request(
                b"GET /explorer/ws HTTP/1.1\r\nupgrade: websocket\r\n\r\n",
                16,
            ),
            Some(ParsedHttpRequest::BadRequest)
        ));
    }

    #[test]
    fn explorer_websocket_views_cover_chain_collections_and_bad_commands() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let cpu_miner = address(b"ws-cpu-miner");
        let consumer_gpu_miner = address(b"ws-consumer-gpu-miner");
        let datacenter_gpu_miner = address(b"ws-datacenter-gpu-miner");
        let other_miner = address(b"ws-other-miner");
        let validator = address(b"ws-validator");
        chain.register_miner(cpu_miner, 100).unwrap();
        chain
            .register_miner_with_profile(consumer_gpu_miner, 100, HardwareClass::ConsumerGpu, 9_000)
            .unwrap();
        chain
            .register_miner_with_profile(
                datacenter_gpu_miner,
                100,
                HardwareClass::DatacenterGpu,
                8_000,
            )
            .unwrap();
        chain
            .register_miner_with_profile(other_miner, 100, HardwareClass::Other, 0)
            .unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let matmul_job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) =
            TensorOpReceipt::from_job(&matmul_job, cpu_miner, 1, 5).unwrap();
        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"ws-linear-model"]),
            step: 3,
            batch_seed: hash_bytes(b"test", &[b"ws-linear-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 1,
            deadline_block: 20,
        });
        chain.submit_job(JobState::TensorOp(matmul_job));
        chain.submit_job(JobState::LinearTrainingStep(linear_job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain.mark_receipt_settled_for_testing(receipt.receipt_id);
        chain.register_validator(cpu_miner, 10_000).unwrap();
        chain.produce_block(cpu_miner, 1000).unwrap();
        let rpc = RpcNode::new(chain);

        let miners = rpc.explorer_websocket_response("miners");
        assert!(miners.contains("\"hardware_class\":\"cpu\""));
        assert!(miners.contains("\"hardware_class\":\"consumer_gpu\""));
        assert!(miners.contains("\"hardware_class\":\"datacenter_gpu\""));
        assert!(miners.contains("\"hardware_class\":\"other\""));
        let validators = rpc.explorer_websocket_response("{\"type\":\"validators\"}");
        assert!(validators.contains("\"valid_attestations\""));
        let jobs = rpc.explorer_websocket_response("{\"type\":\"jobs\",\"job_limit\":2}");
        assert!(jobs.contains("\"primitive_type\":\"tensor_op\""));
        assert!(jobs.contains("\"primitive_type\":\"linear_training_step\""));
        let receipts =
            rpc.explorer_websocket_response("{\"type\":\"receipts\",\"receipt_limit\":1}");
        assert!(receipts.contains("\"primitive_type\":\"tensor_op\""));
        assert!(receipts.contains("\"attestation_count\":0"));
        assert!(receipts.contains("\"validator_attestations\":[]"));
        assert!(receipts.contains("\"settled\":true"));
        let blocks = rpc.explorer_websocket_response("{\"type\":\"blocks\",\"block_limit\":1}");
        assert!(blocks.contains("\"blocks\""));
        let summary = rpc.explorer_websocket_response("summary");
        assert!(summary.contains("\"type\":\"summary\""));
        let missing_account = rpc.explorer_websocket_response("{\"type\":\"account\"}");
        assert!(missing_account.contains("missing account address"));
        let invalid_account =
            rpc.explorer_websocket_response("{\"type\":\"account\",\"address\":\"bad\"}");
        assert!(invalid_account.contains("invalid account address"));

        assert_eq!(primitive_label(PrimitiveType::TensorOp), "tensor_op");
        assert_eq!(
            primitive_label(PrimitiveType::LinearTrainingStep),
            "linear_training_step"
        );
        assert_eq!(hardware_class_label(HardwareClass::Cpu), "cpu");
        assert_eq!(
            hardware_class_label(HardwareClass::ConsumerGpu),
            "consumer_gpu"
        );
        assert_eq!(
            hardware_class_label(HardwareClass::DatacenterGpu),
            "datacenter_gpu"
        );
        assert_eq!(hardware_class_label(HardwareClass::Other), "other");
    }

    #[test]
    fn rpc_http_reader_handles_in_memory_requests_and_limits() {
        let mut get = std::io::Cursor::new(b"GET /chain/head HTTP/1.1\r\n\r\n");
        assert!(matches!(
            read_http_request_from(&mut get, 16).unwrap(),
            ParsedHttpRequest::Request {
                request,
                auth_token: None,
            } if request.method == "GET" && request.path == "/chain/head" && request.body.is_empty()
        ));

        let mut post = std::io::Cursor::new(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbody");
        assert!(matches!(
            read_http_request_from(&mut post, 16).unwrap(),
            ParsedHttpRequest::Request { request, .. } if request.body == b"body"
        ));

        let mut empty = std::io::Cursor::new(Vec::<u8>::new());
        assert!(matches!(
            read_http_request_from(&mut empty, 16).unwrap(),
            ParsedHttpRequest::BadRequest
        ));

        let too_large = vec![b'x'; 8 * 1024 + 17];
        let mut too_large = std::io::Cursor::new(too_large);
        assert!(matches!(
            read_http_request_from(&mut too_large, 16).unwrap(),
            ParsedHttpRequest::TooLarge
        ));
    }

    #[test]
    fn rpc_gateway_enforces_auth_body_limits_and_rate_limits() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut gateway = RpcGateway::new(
            RpcNode::new(Chain::new(beacon)),
            RpcPolicy {
                auth_token: Some("secret".to_owned()),
                max_body_bytes: 8,
                max_requests_per_client: 1,
            },
        );
        let request = RpcRequest {
            method: "GET".to_owned(),
            path: "/chain/head".to_owned(),
            body: Vec::new(),
        };

        assert_eq!(gateway.handle("client", None, &request).status, 401);
        assert_eq!(gateway.request_count("client"), 0);

        let oversized = RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: b"too many bytes".to_vec(),
        };
        assert_eq!(
            gateway.handle("client", Some("secret"), &oversized).status,
            413
        );
        assert_eq!(gateway.request_count("client"), 0);

        assert_eq!(
            gateway.handle("client", Some("secret"), &request).status,
            200
        );
        assert_eq!(
            gateway.handle("client", Some("secret"), &request).status,
            429
        );
    }

    #[test]
    fn tensor_rpc_serves_descriptor_rows_chunks_and_openings() {
        let chain = Chain::new(hash_bytes(b"test", &[b"beacon"]));
        let mut rpc = RpcNode::new(chain);
        let empty_latest = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/tensor/latest".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(empty_latest.status, 404);

        let tensor =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let commitment_root = tensor.commitment_root();
        let tensor_id = rpc.insert_tensor(tensor);
        assert!(rpc.contains_tensor_commitment_root(&commitment_root));
        assert_eq!(
            rpc.tensor_by_commitment_root(&commitment_root)
                .map(Tensor::tensor_id),
            Some(tensor_id)
        );

        for path in [
            "/tensor/latest".to_owned(),
            format!("/tensor/{}/descriptor", hex(&tensor_id)),
            format!("/tensor/{}/row/1", hex(&tensor_id)),
            format!("/tensor/{}/chunk/0", hex(&tensor_id)),
            format!("/tensor/{}/opening/0", hex(&tensor_id)),
        ] {
            let response = rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path,
                body: Vec::new(),
            });
            assert_eq!(response.status, 200);
        }
    }

    #[test]
    fn rpc_node_synthetic_round_retains_live_tensors_for_rpc_fetch() {
        let mut empty_rpc =
            RpcNode::new(Chain::new(hash_bytes(b"test", &[b"rpc-empty-synthetic"])));
        assert_eq!(empty_rpc.produce_synthetic_cpu_round().unwrap(), None);

        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"rpc-live-tensors"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("rpc-live-tensor-miner-{index}").as_bytes()),
                    chain.params.miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("rpc-live-tensor-validator-{index}").as_bytes()),
                    chain.params.validator_min_stake,
                )
                .unwrap();
        }
        let mut rpc = RpcNode::new(chain);

        assert_eq!(
            rpc.produce_synthetic_cpu_round_with_profile(&ChainProfile::public_testnet())
                .unwrap(),
            None
        );
        assert_eq!(
            rpc.produce_synthetic_cpu_round_with_profile(&ChainProfile::local_cpu())
                .unwrap(),
            Some(1)
        );
        assert_eq!(rpc.produce_synthetic_cpu_round().unwrap(), Some(2));
        let latest = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/tensor/latest".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(latest.status, 200);
        assert!(latest.body.contains("\"tensor_count\":9"));
    }

    #[test]
    fn rpc_formats_http_response() {
        let response = RpcResponse {
            status: 202,
            body: "{\"accepted\":true}".to_owned(),
        };
        let text = http_response_text(&response);
        assert!(text.starts_with("HTTP/1.1 202 Accepted"));
        assert!(text.ends_with("{\"accepted\":true}"));
        let conflict = http_response_text(&RpcResponse {
            status: 409,
            body: "{\"error\":\"duplicate transaction\"}".to_owned(),
        });
        assert!(conflict.starts_with("HTTP/1.1 409 Conflict"));
        let limited = http_response_text(&RpcResponse {
            status: 429,
            body: "{\"error\":\"rate limit exceeded\"}".to_owned(),
        });
        assert!(limited.starts_with("HTTP/1.1 429 Too Many Requests"));
        let html = http_response_text(&RpcResponse {
            status: 200,
            body: "<!doctype html><html></html>".to_owned(),
        });
        assert!(html.contains("content-type: text/html; charset=utf-8"));
    }

    #[test]
    fn rpc_http_server_serves_socket_request() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let gateway = RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default());
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        assert_eq!(server.gateway().request_count("unseen-client"), 0);
        server.set_nonblocking(true).unwrap();
        server.set_nonblocking(false).unwrap();
        server.gateway_mut().policy.max_body_bytes = 32;
        assert_eq!(server.gateway().policy.max_body_bytes, 32);
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"GET /chain/head HTTP/1.1\r\nhost: localhost\r\n\r\n")
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server_thread.join().unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"height\":0"));
    }

    #[test]
    fn rpc_http_server_serves_explorer_websocket_poll() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"ws-http-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(miner, 10_000).unwrap();
        chain.produce_block(miner, 1).unwrap();
        let gateway = RpcGateway::new(RpcNode::new(chain), RpcPolicy::default());
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
        let mut handshake = Vec::new();
        let mut byte = [0_u8; 1];
        while !handshake.ends_with(b"\r\n\r\n") {
            client.read_exact(&mut byte).unwrap();
            handshake.push(byte[0]);
        }
        client
            .write_all(&masked_websocket_text_frame(
                "{\"type\":\"overview\",\"block_limit\":1}",
            ))
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        server_thread.join().unwrap();
        let mut full_response = handshake;
        full_response.extend_from_slice(&response);
        let response = String::from_utf8_lossy(&full_response);

        assert!(response.contains("101 Switching Protocols"));
        assert!(response.contains("sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        assert!(response.contains("\"type\":\"overview\""));
        assert!(response.contains("\"block_count\":1"));
    }

    #[test]
    fn rpc_http_server_rejects_bad_websocket_routes_and_auth() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let bad_route = serve_one_http_request(
            RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default()),
            b"GET /wrong/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
        assert!(bad_route.starts_with("HTTP/1.1 404 Not Found"));

        let unauthorized = serve_one_http_request(
            RpcGateway::new(
                RpcNode::new(Chain::new(beacon)),
                RpcPolicy {
                    auth_token: Some("secret".to_owned()),
                    max_body_bytes: 1024,
                    max_requests_per_client: 10,
                },
            ),
            b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
        assert!(unauthorized.starts_with("HTTP/1.1 401 Unauthorized"));
    }

    #[test]
    fn websocket_frame_helpers_cover_close_errors_and_extended_lengths() {
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpListener, TcpStream};

        assert_eq!(
            read_single_websocket_frame(&[0x81, 126, 0, 126])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::UnexpectedEof
        );
        let mut extended_16 = vec![0x81, 126];
        extended_16.extend_from_slice(&(126_u16).to_be_bytes());
        extended_16.extend(std::iter::repeat_n(b'a', 126));
        assert_eq!(
            read_single_websocket_frame(&extended_16).unwrap(),
            Some("a".repeat(126))
        );
        let mut extended_64 = vec![0x81, 127];
        extended_64.extend_from_slice(&(3_u64).to_be_bytes());
        extended_64.extend_from_slice(b"hey");
        assert_eq!(
            read_single_websocket_frame(&extended_64).unwrap(),
            Some("hey".to_owned())
        );
        let mut too_large = vec![0x81, 127];
        too_large.extend_from_slice(&((64_u64 * 1024) + 1).to_be_bytes());
        assert_eq!(
            read_single_websocket_frame(&too_large).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_single_websocket_frame(&[0x81, 1, 0xff])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_single_websocket_frame(&[0x82, 0]).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(read_single_websocket_frame(&[0x88, 0]).unwrap(), None);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let writer = std::thread::spawn(move || {
            let (mut server, _) = listener.accept().unwrap();
            let small_payload = [b'a'; 126];
            let large_payload = vec![b'b'; 65_536];
            write_websocket_frame(&mut server, 0x1, &small_payload).unwrap();
            write_websocket_frame(&mut server, 0x1, &large_payload).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        let mut raw = Vec::new();
        client.read_to_end(&mut raw).unwrap();
        writer.join().unwrap();
        assert_eq!(raw[1], 126);
        assert_eq!(u16::from_be_bytes([raw[2], raw[3]]), 126);
        let second = 4 + 126;
        assert_eq!(raw[second + 1], 127);
        assert_eq!(
            u64::from_be_bytes(raw[second + 2..second + 10].try_into().unwrap()),
            65_536
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let rpc = RpcNode::new(Chain::new(hash_bytes(b"test", &[b"beacon"])));
        let server_thread = std::thread::spawn(move || {
            let (mut server, _) = listener.accept().unwrap();
            rpc.serve_explorer_websocket_once(&mut server).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(&[0x88, 0]).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut close_response = Vec::new();
        client.read_to_end(&mut close_response).unwrap();
        server_thread.join().unwrap();
        assert_eq!(close_response, vec![0x88, 0]);

        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn websocket_json_and_query_helpers_handle_escaping_and_decoding() {
        let escaped =
            json_string_field("{\"address\":\"a\\\"b\\\\c\\n\\r\\t\\x\"}", "address").unwrap();
        assert_eq!(escaped, "a\"b\\c\n\r\tx");
        assert!(json_string_field("{\"address\":\"unterminated", "address").is_none());
        assert_eq!(
            json_usize_field("{\"limit\":123,\"next\":1}", "limit"),
            Some(123)
        );
        assert!(json_usize_field("{\"limit\":nope}", "limit").is_none());
        let (path, token) = split_path_and_auth_token("/explorer/ws?x=1&token=a%20b+z%2f%ZZ");
        assert_eq!(path, "/explorer/ws");
        assert_eq!(token.as_deref(), Some("a b z/%ZZ"));
    }

    fn serve_one_http_request(gateway: RpcGateway, request: &[u8]) -> String {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                return String::new();
            }
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(request).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server_thread.join().unwrap();
        response
    }

    fn read_single_websocket_frame(frame: &[u8]) -> std::io::Result<Option<String>> {
        use std::io::Write;
        use std::net::{Shutdown, TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let mut client = TcpStream::connect(addr)?;
        let (mut server, _) = listener.accept()?;
        client.write_all(frame)?;
        client.shutdown(Shutdown::Write)?;
        read_websocket_text_frame(&mut server)
    }

    fn masked_websocket_text_frame(text: &str) -> Vec<u8> {
        let mask = [1_u8, 2, 3, 4];
        let bytes = text.as_bytes();
        assert!(bytes.len() < 126);
        let mut frame = vec![0x81, 0x80 | bytes.len() as u8];
        frame.extend_from_slice(&mask);
        for (index, byte) in bytes.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        frame
    }
}
