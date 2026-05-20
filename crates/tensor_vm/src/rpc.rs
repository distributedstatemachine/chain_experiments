use crate::chain::{HardwareClass, JobState, LocalChain, Transaction};
use crate::error::{Result, TvmError};
use crate::faucet::Faucet;
use crate::hash::hex;
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
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use tensor_vm_explorer::{
    ExplorerAccount, ExplorerBlock, ExplorerJob, ExplorerMiner, ExplorerOverview, ExplorerReceipt,
    ExplorerSummary, ExplorerValidator, account_json, blocks_json, explorer_shell_html, jobs_json,
    miners_json, receipts_json, validators_json,
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

    pub fn gateway_mut(&mut self) -> &mut RpcGateway {
        &mut self.gateway
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        self.listener.set_nonblocking(nonblocking)
    }

    pub fn serve_next(&mut self) -> std::io::Result<()> {
        let (mut stream, peer_addr) = self.listener.accept()?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        match read_http_request(&mut stream, self.gateway.policy.max_body_bytes)? {
            ParsedHttpRequest::Request {
                request,
                auth_token,
            } => {
                let response =
                    self.gateway
                        .handle(&peer_addr.to_string(), auth_token.as_deref(), &request);
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
            ParsedHttpRequest::WebSocketUpgrade {
                path,
                auth_token,
                websocket_key,
            } => {
                if path != "/explorer/ws" {
                    stream.write_all(
                        http_response_text(&RpcNode::response(404, "websocket route not found"))
                            .as_bytes(),
                    )?;
                    return stream.flush();
                }
                if let Some(response) = self
                    .gateway
                    .authorize_request(&peer_addr.to_string(), auth_token.as_deref())
                {
                    stream.write_all(http_response_text(&response).as_bytes())?;
                    return stream.flush();
                }
                write_websocket_handshake(&mut stream, &websocket_key)?;
                self.gateway.node.serve_explorer_websocket_once(&mut stream)
            }
            ParsedHttpRequest::BadRequest => {
                let response = RpcNode::response(400, "bad http request");
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
            ParsedHttpRequest::TooLarge => {
                let response = RpcNode::response(413, "request body too large");
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
        }
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
                self.chain.state.height,
                self.chain.state.epoch,
                self.chain.blocks.len(),
                hex(&self.chain.state_root())
            )),
            ("GET", "/epoch/current") => {
                self.ok(format!("{{\"epoch\":{}}}", self.chain.state.epoch))
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
    WebSocketUpgrade {
        path: String,
        auth_token: Option<String>,
        websocket_key: String,
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
    let (path, query_auth_token) = match first_parts.next() {
        Some(path) => split_path_and_auth_token(path),
        None => return Some(ParsedHttpRequest::BadRequest),
    };

    let mut content_length = 0_usize;
    let mut auth_token = query_auth_token;
    let mut websocket_key = None;
    let mut websocket_upgrade = false;
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
        } else if name.eq_ignore_ascii_case("sec-websocket-key") {
            websocket_key = Some(value.to_owned());
        } else if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket") {
            websocket_upgrade = true;
        }
    }
    if content_length > max_body_bytes {
        return Some(ParsedHttpRequest::TooLarge);
    }

    if websocket_upgrade {
        let Some(websocket_key) = websocket_key else {
            return Some(ParsedHttpRequest::BadRequest);
        };
        return Some(ParsedHttpRequest::WebSocketUpgrade {
            path,
            auth_token,
            websocket_key,
        });
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

fn split_path_and_auth_token(path: &str) -> (String, Option<String>) {
    let Some((path_only, query)) = path.split_once('?') else {
        return (path.to_owned(), None);
    };
    let token = query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == "token").then(|| percent_decode(value))
    });
    (path_only.to_owned(), token)
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

fn explorer_summary(chain: &LocalChain) -> ExplorerSummary {
    ExplorerSummary {
        height: chain.state.height,
        epoch: chain.state.epoch,
        block_count: chain.blocks.len(),
        miner_count: chain.state.miners.len(),
        validator_count: chain.state.validators.len(),
        job_count: chain.state.jobs.len(),
        model_count: chain.state.model_states.len(),
        attestation_count: chain.state.attestations.values().map(Vec::len).sum(),
        receipt_count: chain.state.receipts.len(),
        settled_receipt_count: chain.state.settled_receipts.len(),
        finalized_block_count: chain.state.finalized_blocks.len(),
        treasury_balance: chain.state.rewards.treasury,
        total_reward_balance: chain.state.rewards.balances.values().sum(),
    }
}

fn explorer_account(chain: &LocalChain, address: &Address) -> ExplorerAccount {
    let miner = chain.state.miners.get(address);
    let validator = chain.state.validators.get(address);
    let balance = chain
        .state
        .accounts
        .get(address)
        .map(|account| account.balance)
        .unwrap_or_default();
    ExplorerAccount {
        address: hex(address),
        is_miner: miner.is_some(),
        is_validator: validator.is_some(),
        balance,
        reward_balance: chain.state.rewards.balance(address),
        stake: miner
            .map(|miner| miner.stake)
            .or_else(|| validator.map(|validator| validator.stake))
            .unwrap_or_default(),
        reputation: miner
            .map(|miner| miner.reputation)
            .or_else(|| validator.map(|validator| validator.reputation))
            .unwrap_or_default(),
        settled_tensor_work: miner
            .map(|miner| miner.settled_tensor_work)
            .unwrap_or_default(),
        pending_tensor_work: miner
            .map(|miner| miner.pending_tensor_work)
            .unwrap_or_default(),
    }
}

fn explorer_blocks(chain: &LocalChain, limit: usize) -> Vec<ExplorerBlock> {
    chain
        .blocks
        .iter()
        .rev()
        .take(limit)
        .map(|block| ExplorerBlock {
            height: block.height,
            epoch: block.epoch,
            hash: hex(&block.hash()),
            proposer: hex(&block.proposer),
            state_root: hex(&block.state_root),
            timestamp: block.timestamp,
        })
        .collect()
}

fn explorer_miners(chain: &LocalChain) -> Vec<ExplorerMiner> {
    chain
        .state
        .miners
        .values()
        .map(|miner| ExplorerMiner {
            address: hex(&miner.address),
            operator_id: hex(&miner.operator_id),
            stake: miner.stake,
            reputation: miner.reputation,
            settled_tensor_work: miner.settled_tensor_work,
            pending_tensor_work: miner.pending_tensor_work,
            hardware_class: hardware_class_label(miner.hardware_class).to_owned(),
            gpu_utilization_bps: miner.gpu_utilization_bps,
            reward_balance: chain.state.rewards.balance(&miner.address),
        })
        .collect()
}

fn explorer_validators(chain: &LocalChain) -> Vec<ExplorerValidator> {
    chain
        .state
        .validators
        .values()
        .map(|validator| ExplorerValidator {
            address: hex(&validator.address),
            stake: validator.stake,
            reputation: validator.reputation,
            valid_attestations: validator.valid_attestations,
            missed_assignments: validator.missed_assignments,
            reward_balance: chain.state.rewards.balance(&validator.address),
        })
        .collect()
}

fn explorer_receipts(chain: &LocalChain, limit: usize) -> Vec<ExplorerReceipt> {
    chain
        .state
        .receipts
        .iter()
        .rev()
        .take(limit)
        .map(|(receipt_id, receipt)| {
            let validator_attestations: Vec<_> = chain
                .state
                .attestations
                .get(receipt_id)
                .into_iter()
                .flat_map(|attestations| attestations.iter())
                .map(|attestation| hex(&attestation.validator))
                .collect();
            ExplorerReceipt {
                receipt_id: hex(receipt_id),
                job_id: hex(&receipt.job_id()),
                primitive_type: primitive_label(receipt.primitive_type()).to_owned(),
                miner: hex(&receipt.miner()),
                tensor_work_units: receipt.tensor_work_units(),
                attestation_count: validator_attestations.len(),
                validator_attestations,
                settled: chain.state.settled_receipts.contains(receipt_id),
            }
        })
        .collect()
}

fn explorer_jobs(chain: &LocalChain, limit: usize) -> Vec<ExplorerJob> {
    chain
        .state
        .jobs
        .values()
        .rev()
        .take(limit)
        .map(|job| match job {
            JobState::TensorOp(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "tensor_op".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("matmul {}x{}x{}", job.m, job.k, job.n),
            },
            JobState::LinearTrainingStep(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "linear_training_step".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("model step {} input {:?}", job.step, job.input_shape),
            },
        })
        .collect()
}

fn explorer_overview(
    chain: &LocalChain,
    block_limit: usize,
    receipt_limit: usize,
    job_limit: usize,
) -> ExplorerOverview {
    ExplorerOverview {
        summary: explorer_summary(chain),
        blocks: explorer_blocks(chain, block_limit),
        miners: explorer_miners(chain),
        validators: explorer_validators(chain),
        receipts: explorer_receipts(chain, receipt_limit),
        jobs: explorer_jobs(chain, job_limit),
    }
}

fn primitive_label(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::TensorOp => "tensor_op",
        PrimitiveType::LinearTrainingStep => "linear_training_step",
    }
}

fn hardware_class_label(hardware_class: HardwareClass) -> &'static str {
    match hardware_class {
        HardwareClass::Cpu => "cpu",
        HardwareClass::ConsumerGpu => "consumer_gpu",
        HardwareClass::DatacenterGpu => "datacenter_gpu",
        HardwareClass::Other => "other",
    }
}

fn write_websocket_handshake(stream: &mut TcpStream, websocket_key: &str) -> std::io::Result<()> {
    let accept = websocket_accept_key(websocket_key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-accept: {accept}\r\n\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn read_websocket_text_frame(stream: &mut TcpStream) -> std::io::Result<Option<String>> {
    let mut header = [0_u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = header[0] & 0x0f;
    let masked = header[1] & 0x80 != 0;
    let mut length = u64::from(header[1] & 0x7f);
    if length == 126 {
        let mut extended = [0_u8; 2];
        stream.read_exact(&mut extended)?;
        length = u64::from(u16::from_be_bytes(extended));
    } else if length == 127 {
        let mut extended = [0_u8; 8];
        stream.read_exact(&mut extended)?;
        length = u64::from_be_bytes(extended);
    }
    if length > 64 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "websocket frame too large",
        ));
    }
    let mut mask = [0_u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0_u8; length as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    match opcode {
        0x1 => String::from_utf8(payload).map(Some).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "websocket text is not utf-8",
            )
        }),
        0x8 => Ok(None),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported websocket opcode",
        )),
    }
}

fn write_websocket_text(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    write_websocket_frame(stream, 0x1, body.as_bytes())
}

fn write_websocket_close(stream: &mut TcpStream) -> std::io::Result<()> {
    write_websocket_frame(stream, 0x8, &[])
}

fn write_websocket_frame(
    stream: &mut TcpStream,
    opcode: u8,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut header = vec![0x80 | opcode];
    if payload.len() < 126 {
        header.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    stream.write_all(&header)?;
    stream.write_all(payload)
}

fn websocket_accept_key(websocket_key: &str) -> String {
    let mut input = websocket_key.trim().as_bytes().to_vec();
    input.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64_encode(&sha1_digest(&input))
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(c & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn sha1_digest(input: &[u8]) -> [u8; 20] {
    let mut h0 = 0x67452301_u32;
    let mut h1 = 0xefcdab89_u32;
    let mut h2 = 0x98badcfe_u32;
    let mut h3 = 0x10325476_u32;
    let mut h4 = 0xc3d2e1f0_u32;
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0_u8; 20];
    for (chunk, value) in out.chunks_exact_mut(4).zip([h0, h1, h2, h3, h4]) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    out
}

fn json_string_field(input: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let after_key = input.split(&key).nth(1)?;
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let value = after_colon.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for c in value.chars() {
        if escaped {
            out.push(match c {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

fn json_usize_field(input: &str, field: &str) -> Option<usize> {
    let key = format!("\"{field}\"");
    let after_key = input.split(&key).nth(1)?;
    let digits = after_key.split_once(':')?.1.trim_start().chars();
    let mut value = String::new();
    for c in digits {
        if c.is_ascii_digit() {
            value.push(c);
        } else {
            break;
        }
    }
    value.parse().ok()
}

fn percent_decode(value: &str) -> String {
    let mut out = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_nibble(bytes[index + 1]), hex_nibble(bytes[index + 2]))
        {
            out.push((high << 4) | low);
            index += 3;
        } else if bytes[index] == b'+' {
            out.push(b' ');
            index += 1;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
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
    use crate::chain::{ChainParams, HardwareClass, JobState, LocalChain};
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, TensorOpReceipt};
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

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
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"ws-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.produce_block(miner, 1);
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
        let mut chain = LocalChain::new(beacon);
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
        chain.state.settled_receipts.insert(receipt.receipt_id);
        chain.produce_block(cpu_miner, 1000);
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
        let empty_latest = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/tensor/latest".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(empty_latest.status, 404);

        let tensor =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let tensor_id = rpc.insert_tensor(tensor);

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
        let mut empty_rpc = RpcNode::new(LocalChain::new(hash_bytes(
            b"test",
            &[b"rpc-empty-synthetic"],
        )));
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
        let mut chain =
            LocalChain::with_params(params, hash_bytes(b"test", &[b"rpc-live-tensors"]));
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
        let gateway = RpcGateway::new(RpcNode::new(LocalChain::new(beacon)), RpcPolicy::default());
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
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"ws-http-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.produce_block(miner, 1);
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
            RpcGateway::new(RpcNode::new(LocalChain::new(beacon)), RpcPolicy::default()),
            b"GET /wrong/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
        assert!(bad_route.starts_with("HTTP/1.1 404 Not Found"));

        let unauthorized = serve_one_http_request(
            RpcGateway::new(
                RpcNode::new(LocalChain::new(beacon)),
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
        let rpc = RpcNode::new(LocalChain::new(hash_bytes(b"test", &[b"beacon"])));
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
