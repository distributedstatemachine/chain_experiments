use super::explorer::{
    explorer_account, explorer_blocks, explorer_jobs, explorer_miners, explorer_overview,
    explorer_receipts, explorer_summary, explorer_validators,
};
use super::websocket::{
    json_string_field, json_usize_field, read_websocket_text_frame, write_websocket_close,
    write_websocket_text,
};
use super::{RpcNode, RpcResponse, parse_hash};
use std::io::Write;
use std::net::TcpStream;
use tensor_vm_explorer::{
    account_json, blocks_json, jobs_json, miners_json, receipts_json, validators_json,
};

impl RpcNode {
    pub(super) fn explorer_account(&self, address: &str) -> RpcResponse {
        let Ok(address) = parse_hash(address) else {
            return self.bad_request("invalid account address");
        };
        self.ok(account_json(&explorer_account(&self.chain, &address)))
    }

    pub(super) fn explorer_latest_blocks(&self, limit: &str) -> RpcResponse {
        let Ok(limit) = limit.parse::<usize>() else {
            return self.bad_request("invalid block limit");
        };
        self.ok(blocks_json(&explorer_blocks(&self.chain, limit)))
    }

    pub(super) fn explorer_latest_receipts(&self, limit: &str) -> RpcResponse {
        let Ok(limit) = limit.parse::<usize>() else {
            return self.bad_request("invalid receipt limit");
        };
        self.ok(receipts_json(&explorer_receipts(&self.chain, limit)))
    }

    pub(super) fn serve_explorer_websocket_once(
        &self,
        stream: &mut TcpStream,
    ) -> std::io::Result<()> {
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

    pub(super) fn explorer_websocket_response(&self, command: &str) -> String {
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
