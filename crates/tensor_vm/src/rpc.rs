use crate::chain::{JobState, LocalChain, Transaction};
use crate::error::{Result, TvmError};
use crate::explorer::{ExplorerSummary, account_page, latest_blocks};
use crate::faucet::Faucet;
use crate::hash::hex;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DEFAULT_CHUNK_SIZE, Tensor};
use crate::txpool::{TxPool, parse_transaction_envelope};
use crate::types::{Address, Hash};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

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
    pub chain: LocalChain,
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

#[derive(Debug)]
pub struct RpcHttpServer {
    listener: TcpListener,
    gateway: RpcGateway,
    read_timeout: Duration,
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
        if let Some(required) = &self.policy.auth_token
            && auth_token != Some(required.as_str())
        {
            return RpcNode::response(401, "unauthorized");
        }
        let count = self.request_counts.entry(client_id.to_owned()).or_default();
        if *count >= self.policy.max_requests_per_client {
            return RpcNode::response(429, "rate limit exceeded");
        }
        *count += 1;
        self.node.handle_mut(request)
    }

    pub fn request_count(&self, client_id: &str) -> u64 {
        self.request_counts.get(client_id).copied().unwrap_or(0)
    }
}

impl RpcHttpServer {
    pub fn bind(addr: &str, gateway: RpcGateway) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            gateway,
            read_timeout: Duration::from_secs(5),
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn gateway(&self) -> &RpcGateway {
        &self.gateway
    }

    pub fn serve_next(&mut self) -> std::io::Result<()> {
        let (mut stream, peer_addr) = self.listener.accept()?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        let response = match read_http_request(&mut stream, self.gateway.policy.max_body_bytes)? {
            ParsedHttpRequest::Request {
                request,
                auth_token,
            } => self
                .gateway
                .handle(&peer_addr.to_string(), auth_token.as_deref(), &request),
            ParsedHttpRequest::BadRequest => RpcNode::response(400, "bad http request"),
            ParsedHttpRequest::TooLarge => RpcNode::response(413, "request body too large"),
        };
        stream.write_all(http_response_text(&response).as_bytes())?;
        stream.flush()
    }

    pub fn serve_n(&mut self, max_requests: usize) -> std::io::Result<()> {
        for _ in 0..max_requests {
            self.serve_next()?;
        }
        Ok(())
    }
}

impl RpcNode {
    pub fn new(chain: LocalChain) -> Self {
        Self {
            chain,
            txpool: TxPool::default(),
            faucet: None,
            tensors: BTreeMap::new(),
        }
    }

    pub fn with_faucet(chain: LocalChain, faucet: Faucet) -> Self {
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

    pub fn handle(&self, request: &RpcRequest) -> RpcResponse {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/health") => self.health("all"),
            ("GET", "/rpc/health") => self.health("rpc"),
            ("GET", "/chain/head") => self.ok(format!(
                "{{\"height\":{},\"epoch\":{},\"block_count\":{}}}",
                self.chain.state.height,
                self.chain.state.epoch,
                self.chain.blocks.len()
            )),
            ("GET", "/epoch/current") => {
                self.ok(format!("{{\"epoch\":{}}}", self.chain.state.epoch))
            }
            ("GET", "/jobs/current") => self.jobs_current(),
            ("GET", "/explorer/health") => self.health("explorer"),
            ("GET", "/explorer") => self.ok(explorer_dashboard_html(&self.chain)),
            ("GET", "/explorer/summary") => {
                self.ok(ExplorerSummary::from_chain(&self.chain).to_json())
            }
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
        let Some(receipt) = self.chain.state.receipts.get(&receipt_id) else {
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
        self.ok(account_page(&self.chain, &address))
    }

    fn explorer_latest_blocks(&self, limit: &str) -> RpcResponse {
        let Ok(limit) = limit.parse::<usize>() else {
            return self.bad_request("invalid block limit");
        };
        let blocks = latest_blocks(&self.chain, limit);
        self.ok(format!("{{\"blocks\":[{}]}}", blocks.join(",")))
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
        match faucet.claim(
            address,
            self.chain.state.epoch,
            &mut self.chain.state.rewards,
        ) {
            Ok(amount) => {
                let balance = faucet.balance();
                self.ok(format!(
                    "{{\"claimed\":{},\"address\":\"{}\",\"faucet_balance\":{}}}",
                    amount,
                    hex(&address),
                    balance
                ))
            }
            Err(error) => self.bad_request(&error.to_string()),
        }
    }

    fn submit_transaction(&mut self, request: &RpcRequest) -> RpcResponse {
        let envelope = match parse_transaction_envelope(&request.body) {
            Ok(envelope) => envelope,
            Err(error) => return self.bad_request(&error.to_string()),
        };
        if matches!(
            envelope.transaction,
            Transaction::SubmitTensorOpReceipt(_)
                | Transaction::SubmitLinearTrainingStepReceipt(_)
                | Transaction::SubmitAttestation(_)
        ) {
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
            Ok(()) => {
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
        let jobs: Vec<_> = self.chain.state.jobs.values().map(job_json).collect();
        self.ok(format!("{{\"jobs\":[{}]}}", jobs.join(",")))
    }

    fn job(&self, job_id: &str) -> RpcResponse {
        let Ok(job_id) = parse_hash(job_id) else {
            return self.bad_request("invalid job id");
        };
        let Some(job) = self.chain.state.jobs.get(&job_id) else {
            return self.not_found("job not found");
        };
        self.ok(job_json(job))
    }

    fn miner(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid miner address");
        };
        let Some(miner) = self.chain.state.miners.get(&address) else {
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
        let Some(validator) = self.chain.state.validators.get(&address) else {
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

    fn lookup_tensor(&self, tensor_id: &str) -> Option<&Tensor> {
        parse_hash(tensor_id)
            .ok()
            .and_then(|tensor_id| self.tensors.get(&tensor_id))
    }

    fn health(&self, service: &str) -> RpcResponse {
        self.ok(format!(
            "{{\"status\":\"ok\",\"service\":\"{service}\",\"height\":{},\"epoch\":{},\"block_count\":{},\"faucet_configured\":{}}}",
            self.chain.state.height,
            self.chain.state.epoch,
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
}

pub fn http_response_text(response: &RpcResponse) -> String {
    let status_text = match response.status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        _ => "Unknown",
    };
    let content_type = if response.body.starts_with("<!doctype html>") {
        "text/html; charset=utf-8"
    } else {
        "application/json"
    };
    format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\n\r\n{}",
        response.status,
        status_text,
        content_type,
        response.body.len(),
        response.body
    )
}

enum ParsedHttpRequest {
    Request {
        request: RpcRequest,
        auth_token: Option<String>,
    },
    BadRequest,
    TooLarge,
}

fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: usize,
) -> std::io::Result<ParsedHttpRequest> {
    read_http_request_from(stream, max_body_bytes)
}

fn read_http_request_from<R: Read>(
    reader: &mut R,
    max_body_bytes: usize,
) -> std::io::Result<ParsedHttpRequest> {
    let max_request_bytes = max_body_bytes.saturating_add(8 * 1024);
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 1024];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            return Ok(ParsedHttpRequest::BadRequest);
        }
        bytes.extend_from_slice(&buf[..read]);
        if bytes.len() > max_request_bytes {
            return Ok(ParsedHttpRequest::TooLarge);
        }
        if let Some(parsed) = try_parse_http_request(&bytes, max_body_bytes) {
            return Ok(parsed);
        }
    }
}

fn try_parse_http_request(bytes: &[u8], max_body_bytes: usize) -> Option<ParsedHttpRequest> {
    let header_end = find_header_end(bytes)?;
    let header_text = match std::str::from_utf8(&bytes[..header_end]) {
        Ok(text) => text,
        Err(_) => return Some(ParsedHttpRequest::BadRequest),
    };
    let mut lines = header_text.split("\r\n");
    let first_line = lines.next().unwrap_or_default();
    let mut first_parts = first_line.split_whitespace();
    let method = match first_parts.next() {
        Some(method) => method.to_owned(),
        None => return Some(ParsedHttpRequest::BadRequest),
    };
    let path = match first_parts.next() {
        Some(path) => path.to_owned(),
        None => return Some(ParsedHttpRequest::BadRequest),
    };

    let mut content_length = 0_usize;
    let mut auth_token = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = match value.parse() {
                Ok(content_length) => content_length,
                Err(_) => return Some(ParsedHttpRequest::BadRequest),
            };
        } else if name.eq_ignore_ascii_case("authorization") {
            auth_token = Some(
                value
                    .strip_prefix("Bearer ")
                    .unwrap_or(value)
                    .trim()
                    .to_owned(),
            );
        } else if name.eq_ignore_ascii_case("x-tensorchain-auth") {
            auth_token = Some(value.to_owned());
        }
    }
    if content_length > max_body_bytes {
        return Some(ParsedHttpRequest::TooLarge);
    }

    let body_start = header_end + 4;
    let body_end = body_start.checked_add(content_length)?;
    if bytes.len() < body_end {
        return None;
    }

    Some(ParsedHttpRequest::Request {
        request: RpcRequest {
            method,
            path,
            body: bytes[body_start..body_end].to_vec(),
        },
        auth_token,
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
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

fn explorer_dashboard_html(chain: &LocalChain) -> String {
    let summary = ExplorerSummary::from_chain(chain);
    let block_items = latest_blocks(chain, 10)
        .into_iter()
        .map(|block| format!("<li><code>{block}</code></li>"))
        .collect::<Vec<_>>()
        .join("");
    html_document(
        "TensorVM Explorer",
        format!(
            "<section><h1>TensorVM Explorer</h1><dl>{}</dl></section><section><h2>Latest Blocks</h2><ol>{}</ol></section>",
            metric_rows(&[
                ("Height", summary.height.to_string()),
                ("Epoch", summary.epoch.to_string()),
                ("Blocks", summary.block_count.to_string()),
                ("Miners", summary.miner_count.to_string()),
                ("Validators", summary.validator_count.to_string()),
                ("Receipts", summary.receipt_count.to_string()),
                (
                    "Settled Receipts",
                    summary.settled_receipt_count.to_string()
                ),
            ]),
            block_items
        ),
    )
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
    use crate::chain::{JobState, LocalChain};
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob};
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};

    #[test]
    fn node_rpc_serves_head_and_blocks() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let proposer = address(b"proposer");
        chain.register_miner(proposer, 100).unwrap();
        chain.produce_block(proposer, 1000);
        let rpc = RpcNode::new(chain);

        let head = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/chain/head".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(head.status, 200);
        assert!(head.body.contains("\"height\":1"));

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
        let mut chain = LocalChain::new(beacon);
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
        let mut chain = LocalChain::new(beacon);
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
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"rpc-service-miner");
        let user = address(b"rpc-faucet-user");
        chain.register_miner(miner, 100).unwrap();
        chain.produce_block(miner, 1000);
        let mut rpc = RpcNode::with_faucet(chain, Faucet::new(1_000, 100));

        let summary = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer/summary".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(summary.status, 200);
        assert!(summary.body.contains("\"miner_count\":1"));

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

        let explorer_page = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/explorer".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(explorer_page.status, 200);
        assert!(explorer_page.body.starts_with("<!doctype html>"));
        assert!(explorer_page.body.contains("TensorVM Explorer"));

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
        assert_eq!(rpc.chain.state.rewards.balance(&user), 100);

        let duplicate = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: format!("/faucet/claim/{}", hex(&user)),
            body: Vec::new(),
        });
        assert_eq!(duplicate.status, 400);

        let missing_faucet = RpcNode::new(LocalChain::new(beacon)).handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/faucet".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(missing_faucet.status, 404);
    }

    #[test]
    fn mutable_rpc_applies_transactions_and_queues_submissions() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut rpc = RpcNode::new(LocalChain::new(beacon));
        let miner = address(b"rpc-miner");
        let receiver = address(b"rpc-receiver");

        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("register_miner {}", hex(&miner)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        assert!(rpc.chain.state.miners.contains_key(&miner));

        rpc.chain.credit_account(miner, 100);
        let response = rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("transfer {} {} 70", hex(&miner), hex(&receiver)).into_bytes(),
        });
        assert_eq!(response.status, 202);
        assert_eq!(rpc.chain.state.accounts.get(&receiver).unwrap().balance, 70);

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
        let mut rpc = RpcNode::new(LocalChain::new(beacon));
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
                .state
                .accounts
                .get(&receiver)
                .map(|account| account.balance),
            None
        );
    }

    #[test]
    fn rpc_rejects_malformed_requests_and_missing_resources() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let missing = hash_bytes(b"test", &[b"missing"]);
        let mut rpc = RpcNode::new(LocalChain::new(beacon));

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
        let mut exhausted = RpcNode::with_faucet(LocalChain::new(beacon), Faucet::new(50, 100));
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
        let mut chain = LocalChain::new(beacon);
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
            RpcNode::new(LocalChain::new(beacon)),
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
            RpcNode::new(LocalChain::new(beacon)),
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
        let chain = LocalChain::new(hash_bytes(b"test", &[b"beacon"]));
        let mut rpc = RpcNode::new(chain);
        let tensor =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let tensor_id = rpc.insert_tensor(tensor);

        for path in [
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
        let gateway = RpcGateway::new(RpcNode::new(LocalChain::new(beacon)), RpcPolicy::default());
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        assert_eq!(server.gateway().request_count("unseen-client"), 0);
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
}
