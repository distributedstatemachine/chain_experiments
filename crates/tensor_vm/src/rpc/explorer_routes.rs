use super::explorer::{
    explorer_account, explorer_blocks, explorer_jobs, explorer_miners, explorer_overview,
    explorer_receipts, explorer_summary, explorer_validators,
};
use super::websocket::{read_websocket_text_frame, write_websocket_close, write_websocket_text};
use super::{RpcNode, RpcResponse, parse_hash};
use serde_json::{Value, json};
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
        match ExplorerWebsocketCommand::parse(command) {
            ExplorerWebsocketCommand::Account { address } => {
                let Some(address) = address else {
                    return explorer_websocket_error("missing account address");
                };
                let Ok(address) = parse_hash(&address) else {
                    return explorer_websocket_error("invalid account address");
                };
                account_json(&explorer_account(&self.chain, &address))
            }
            ExplorerWebsocketCommand::Summary => format!(
                "{{\"type\":\"summary\",\"summary\":{}}}",
                explorer_summary(&self.chain).to_json()
            ),
            ExplorerWebsocketCommand::Miners => miners_json(&explorer_miners(&self.chain)),
            ExplorerWebsocketCommand::Validators => {
                validators_json(&explorer_validators(&self.chain))
            }
            ExplorerWebsocketCommand::Jobs { limit } => {
                jobs_json(&explorer_jobs(&self.chain, limit))
            }
            ExplorerWebsocketCommand::Receipts { limit } => {
                receipts_json(&explorer_receipts(&self.chain, limit))
            }
            ExplorerWebsocketCommand::Blocks { limit } => {
                blocks_json(&explorer_blocks(&self.chain, limit))
            }
            ExplorerWebsocketCommand::Overview {
                block_limit,
                receipt_limit,
                job_limit,
            } => explorer_overview(&self.chain, block_limit, receipt_limit, job_limit).to_json(),
        }
    }
}

enum ExplorerWebsocketCommand {
    Account {
        address: Option<String>,
    },
    Summary,
    Miners,
    Validators,
    Jobs {
        limit: usize,
    },
    Receipts {
        limit: usize,
    },
    Blocks {
        limit: usize,
    },
    Overview {
        block_limit: usize,
        receipt_limit: usize,
        job_limit: usize,
    },
}

impl ExplorerWebsocketCommand {
    fn parse(command: &str) -> Self {
        match command.trim() {
            "summary" => return Self::Summary,
            "miners" => return Self::Miners,
            "validators" => return Self::Validators,
            "jobs" => return Self::Jobs { limit: 50 },
            "receipts" => return Self::Receipts { limit: 50 },
            "blocks" => return Self::Blocks { limit: 25 },
            _ => {}
        }

        let Ok(command) = serde_json::from_str::<Value>(command) else {
            return Self::default_overview();
        };
        match command.get("type").and_then(Value::as_str) {
            Some("account") => Self::Account {
                address: command
                    .get("address")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            Some("summary") => Self::Summary,
            Some("miners") => Self::Miners,
            Some("validators") => Self::Validators,
            Some("jobs") => Self::Jobs {
                limit: command_limit(&command, "job_limit").unwrap_or(50),
            },
            Some("receipts") => Self::Receipts {
                limit: command_limit(&command, "receipt_limit").unwrap_or(50),
            },
            Some("blocks") => Self::Blocks {
                limit: command_limit(&command, "block_limit").unwrap_or(25),
            },
            Some("overview") | None | Some(_) => Self::Overview {
                block_limit: command_limit(&command, "block_limit").unwrap_or(12),
                receipt_limit: command_limit(&command, "receipt_limit").unwrap_or(20),
                job_limit: command_limit(&command, "job_limit").unwrap_or(20),
            },
        }
    }

    fn default_overview() -> Self {
        Self::Overview {
            block_limit: 12,
            receipt_limit: 20,
            job_limit: 20,
        }
    }
}

fn command_limit(command: &Value, field: &str) -> Option<usize> {
    command
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn explorer_websocket_error(error: &str) -> String {
    json!({
        "type": "error",
        "error": error
    })
    .to_string()
}
