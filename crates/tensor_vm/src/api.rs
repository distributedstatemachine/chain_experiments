use crate::types::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    "GET /explorer/account/:address",
    "GET /explorer/blocks/latest/:limit",
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
    "tvmd miner start --wallet <key> --device <device> --node <libp2p-multiaddr>",
    "tvmd miner status",
];

pub const VALIDATOR_CLI_COMMANDS: &[&str] = &[
    "tvmd validator register --stake <tokens>",
    "tvmd validator start --wallet <key> --node <libp2p-multiaddr>",
    "tvmd validator status",
];

pub const SERVICE_CLI_COMMANDS: &[&str] = &[
    "tvmd service init --data-dir <path>",
    "tvmd service peer add --data-dir <path> --peer-id <peer-id> --address <libp2p-multiaddr>",
    "tvmd service serve --listen <addr> --p2p-listen <libp2p-multiaddr> --data-dir <path> --auth-token <token> --max-requests <n>",
];

pub const PUBLIC_EVIDENCE_CLI_COMMANDS: &[&str] = &[
    "tvmd public-evidence validate --manifest <path>",
    "tvmd public-evidence publication --bundle-id <hex> --public-uri <uri> --manifest-signer <address-hex> --manifest-signature-count <n> --independent-auditor-count <n>",
    "tvmd public-evidence auditor-record --bundle-id <hex> --public-uri <uri> --auditor-id <address-hex> --audit-uri <uri> --observed-at <unix-seconds>",
    "tvmd public-evidence run-window --bundle-id <hex> --manifest-signer <address-hex> --started-at <unix-seconds> --ended-at <unix-seconds> --observed-blocks <n>",
    "tvmd public-evidence node-heartbeat --role <miner|validator> --address <address-hex> --operator-id <hex> --first-block <n> --last-block <n> --heartbeat-count <n>",
    "tvmd public-evidence operator-attestation --role <miner|validator> --address <address-hex> --operator-id <hex> --identity-uri <uri> --observed-at <unix-seconds>",
    "tvmd public-evidence service-health --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --first-block <n> --last-block <n> --reachable-count <n> --signed-health-check-count <n>",
    "tvmd public-evidence service-content --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --content-root <hex> --observed-at <unix-seconds> --min-content-bytes <n>",
    "tvmd public-evidence service-content-from-bytes --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-hex <hex-bytes>",
    "tvmd public-evidence service-content-from-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-file <path>",
    "tvmd public-evidence network-observation --operator-id <hex> --peer-id <peer-id> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --gossip-topics <n> --request-response-protocols <n> --bootstrap-peers <n> --max-transmit-bytes <n> --request-timeout-seconds <n> --max-concurrent-streams <n> --idle-timeout-seconds <n>",
    "tvmd public-evidence network-observation-from-service-log --operator-id <hex> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --service-log <path>",
    "tvmd public-evidence record-summary --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-root <hex> --record-count <n>",
    "tvmd public-evidence record-artifact --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-root <hex> --record-count <n>",
    "tvmd public-evidence record-artifact-from-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-roots <comma-separated-roots>",
    "tvmd public-evidence record-artifact-from-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-file <path>",
    "tvmd public-evidence record-summary-from-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-roots <comma-separated-roots>",
    "tvmd public-evidence record-summary-from-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-file <path>",
];

pub const PUBLIC_TESTNET_CLI_COMMANDS: &[&str] =
    &["tvmd public-testnet preflight --manifest <path>"];

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
            VALIDATOR_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("validator start"))
        );
        assert!(
            SERVICE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service serve"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("public-evidence validate"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service-health"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("publication"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("run-window"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("node-heartbeat"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("operator-attestation"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("network-observation"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("network-observation-from-service-log"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service-content"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("service-content-from-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-summary"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-artifact"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-artifact-from-roots"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-artifact-from-file"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("auditor-record"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-summary-from-roots"))
        );
        assert!(
            PUBLIC_EVIDENCE_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("record-summary-from-file"))
        );
        assert!(
            PUBLIC_TESTNET_CLI_COMMANDS
                .iter()
                .any(|command| command.contains("public-testnet preflight"))
        );
    }
}
