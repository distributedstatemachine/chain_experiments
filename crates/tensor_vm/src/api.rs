use crate::types::{Address, Hash};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum P2pMessage {
    NewBlock(Hash),
    NewJob(Hash),
    NewReceipt(Hash),
    NewAttestation(Hash),
    RequestTensorChunk {
        tensor_id: Hash,
        chunk_index: u64,
    },
    TensorChunkResponse {
        tensor_id: Hash,
        chunk_index: u64,
        bytes: Vec<u8>,
    },
    RequestTensorRow {
        tensor_id: Hash,
        row_index: u64,
    },
    TensorRowResponse {
        tensor_id: Hash,
        row_index: u64,
        values: Vec<u64>,
    },
    RequestProgram(Hash),
    ProgramResponse {
        program_hash: Hash,
        bytes: Vec<u8>,
    },
    PeerInfo {
        address: Address,
    },
}

pub const NODE_RPC_ROUTES: &[&str] = &[
    "GET /chain/head",
    "GET /chain/block/:height",
    "GET /epoch/current",
    "GET /jobs/current",
    "GET /jobs/:job_id",
    "GET /receipts/:receipt_id",
    "GET /miners/:address",
    "GET /validators/:address",
    "GET /explorer",
    "GET /explorer/summary",
    "GET /explorer/account/:address",
    "GET /explorer/blocks/latest/:limit",
    "GET /telemetry",
    "GET /telemetry/dashboard",
    "GET /faucet",
    "GET /faucet/page",
    "POST /faucet/claim/:address",
    "POST /tx",
    "POST /receipt",
    "POST /attestation",
];

pub const TENSOR_DATA_RPC_ROUTES: &[&str] = &[
    "GET /tensor/:tensor_id/descriptor",
    "GET /tensor/:tensor_id/chunk/:chunk_index",
    "GET /tensor/:tensor_id/row/:row_index",
    "GET /tensor/:tensor_id/opening/:chunk_index",
];

pub const MINER_CLI_COMMANDS: &[&str] = &[
    "tvmd miner register --stake <tokens>",
    "tvmd miner start --wallet <key> --device <device> --node <url>",
    "tvmd miner status",
];

pub const VALIDATOR_CLI_COMMANDS: &[&str] = &[
    "tvmd validator register --stake <tokens>",
    "tvmd validator start --wallet <key> --node <url>",
    "tvmd validator status",
];

pub const PUBLIC_EVIDENCE_CLI_COMMANDS: &[&str] =
    &["tvmd public-evidence validate --manifest <path>"];

pub const PUBLIC_TESTNET_CLI_COMMANDS: &[&str] =
    &["tvmd public-testnet preflight --manifest <path>"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_surface_includes_spec_routes() {
        assert!(NODE_RPC_ROUTES.contains(&"GET /chain/head"));
        assert!(NODE_RPC_ROUTES.contains(&"POST /attestation"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /telemetry"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /telemetry/dashboard"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /faucet/page"));
        assert!(NODE_RPC_ROUTES.contains(&"POST /faucet/claim/:address"));
        assert!(TENSOR_DATA_RPC_ROUTES.contains(&"GET /tensor/:tensor_id/opening/:chunk_index"));
        assert!(
            MINER_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("miner start"))
        );
        assert!(
            VALIDATOR_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("validator start"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("public-evidence validate"))
        );
        assert!(
            PUBLIC_TESTNET_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("public-testnet preflight"))
        );
    }
}
