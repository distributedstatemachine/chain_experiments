use crate::types::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum P2pMessage {
    NewBlock(Hash),
    NewBlockHeader {
        height: u64,
        block_hash: Hash,
    },
    NewBlockPayload {
        height: u64,
        block_hash: Hash,
        payload: Vec<u8>,
    },
    NewBlockVotePayload {
        block_hash: Hash,
        validator: Address,
        payload: Vec<u8>,
    },
    NewJob(Hash),
    NewJobPayload {
        job_id: Hash,
        payload: Vec<u8>,
    },
    NewReceipt(Hash),
    NewReceiptPayload {
        receipt_id: Hash,
        payload: Vec<u8>,
    },
    NewAttestation(Hash),
    NewAttestationPayload {
        attestation_id: Hash,
        payload: Vec<u8>,
    },
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
    RequestTensorByCommitmentRoot {
        commitment_root: Hash,
    },
    TensorByCommitmentRootResponse {
        commitment_root: Hash,
        payload: Option<Vec<u8>>,
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
    "GET /health",
    "GET /rpc/health",
    "GET /chain/head",
    "GET /chain/block/:height",
    "GET /epoch/current",
    "GET /jobs/current",
    "GET /jobs/:job_id",
    "GET /receipts/:receipt_id",
    "GET /miners/:address",
    "GET /validators/:address",
    "GET /explorer",
    "GET /explorer/health",
    "GET /explorer/summary",
    "GET /explorer/overview",
    "GET /explorer/miners",
    "GET /explorer/validators",
    "GET /explorer/jobs",
    "GET /explorer/account/:address",
    "GET /explorer/blocks/latest/:limit",
    "GET /explorer/receipts/latest/:limit",
    "WS /explorer/ws",
    "GET /telemetry",
    "GET /telemetry/health",
    "GET /telemetry/dashboard",
    "GET /faucet",
    "GET /faucet/health",
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
    "tvmd miner start --wallet <key> [--device cpu|cuda:N] [--node <libp2p-multiaddr>]",
    "tvmd miner run --wallet <key> --auth-token <token> [--device cpu|cuda:N] [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
    "tvmd miner status",
];

pub const VALIDATOR_CLI_COMMANDS: &[&str] = &[
    "tvmd validator register --stake <tokens>",
    "tvmd validator start --wallet <key> [--node <libp2p-multiaddr>]",
    "tvmd validator run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
    "tvmd validator status",
];

pub const PROPOSER_CLI_COMMANDS: &[&str] = &[
    "tvmd proposer run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
];

pub const SERVICE_CLI_COMMANDS: &[&str] = &[
    "tvmd service init [--data-dir <path>]",
    "tvmd service peer add --peer-id <peer-id> --address <libp2p-multiaddr> [--data-dir <path>]",
    "tvmd service readiness [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>]",
    "tvmd service serve --auth-token <token> [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
];

pub const PUBLIC_EVIDENCE_CLI_COMMANDS: &[&str] = &[
    "tvmd evidence validate <path>",
    "tvmd evidence publish --bundle-id <hex> --public-uri <uri> --manifest-signer <address-hex> --manifest-signature-count <n> --independent-auditor-count <n>",
    "tvmd evidence audit --bundle-id <hex> --public-uri <uri> --auditor-id <address-hex> --audit-uri <uri> --observed-at <unix-seconds>",
    "tvmd evidence run window --bundle-id <hex> --manifest-signer <address-hex> --started-at <unix-seconds> --ended-at <unix-seconds> --observed-blocks <n>",
    "tvmd evidence run window-file --bundle-id <hex> --manifest-signer <address-hex> --block-observation-file <path>",
    "tvmd evidence node heartbeat --role <miner|validator> --address <address-hex> --operator-id <hex> --first-block <n> --last-block <n> --heartbeat-count <n>",
    "tvmd evidence node heartbeat-file --role <miner|validator> --address <address-hex> --operator-id <hex> --heartbeat-file <path>",
    "tvmd evidence node operator-attestation --role <miner|validator> --address <address-hex> --operator-id <hex> --identity-uri <uri> --observed-at <unix-seconds>",
    "tvmd evidence service health --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --first-block <n> --last-block <n> --reachable-count <n> --signed-health-check-count <n>",
    "tvmd evidence service health-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --observation-file <path>",
    "tvmd evidence service content --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --content-root <hex> --observed-at <unix-seconds> --min-content-bytes <n>",
    "tvmd evidence service content-bytes --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-hex <hex-bytes>",
    "tvmd evidence service content-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-file <path>",
    "tvmd evidence network observation --operator-id <hex> --peer-id <peer-id> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --gossip-topics <n> --request-response-protocols <n> --bootstrap-peers <n> --max-transmit-bytes <n> --request-timeout-seconds <n> --max-concurrent-streams <n> --idle-timeout-seconds <n>",
    "tvmd evidence network from-service-log --operator-id <hex> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --service-log <path>",
    "tvmd evidence record summary --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-root <hex> --record-count <n>",
    "tvmd evidence record artifact --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-root <hex> --record-count <n>",
    "tvmd evidence record artifact-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-roots <comma-separated-roots>",
    "tvmd evidence record artifact-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-file <path>",
    "tvmd evidence record summary-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-roots <comma-separated-roots>",
    "tvmd evidence record summary-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-file <path>",
];

pub const PUBLIC_TESTNET_CLI_COMMANDS: &[&str] = &["tvmd testnet preflight <path>"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_surface_includes_spec_routes() {
        assert!(NODE_RPC_ROUTES.contains(&"GET /health"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /rpc/health"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /chain/head"));
        assert!(NODE_RPC_ROUTES.contains(&"POST /attestation"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/health"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/overview"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/miners"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/validators"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/jobs"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /explorer/receipts/latest/:limit"));
        assert!(NODE_RPC_ROUTES.contains(&"WS /explorer/ws"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /telemetry"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /telemetry/health"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /telemetry/dashboard"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /faucet/health"));
        assert!(NODE_RPC_ROUTES.contains(&"GET /faucet/page"));
        assert!(NODE_RPC_ROUTES.contains(&"POST /faucet/claim/:address"));
        assert!(TENSOR_DATA_RPC_ROUTES.contains(&"GET /tensor/:tensor_id/opening/:chunk_index"));
        assert!(
            MINER_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("miner start"))
        );
        assert!(
            MINER_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("miner run"))
        );
        assert!(
            VALIDATOR_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("validator start"))
        );
        assert!(
            VALIDATOR_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("validator run"))
        );
        assert!(
            PROPOSER_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("proposer run"))
        );
        assert!(
            SERVICE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service serve"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("evidence validate"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service health"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service health-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("evidence publish"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("run window"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("run window-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("node heartbeat"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("node heartbeat-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("operator-attestation"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("network observation"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("network from-service-log"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service content"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service content-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record summary"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record artifact"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record artifact-roots"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record artifact-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("evidence audit"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record summary-roots"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record summary-file"))
        );
        assert!(
            PUBLIC_TESTNET_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("testnet preflight"))
        );
    }
}
