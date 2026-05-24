use super::*;
use crate::hash::hex;
use crate::testnet::{
    PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceRecordKind,
    PublicEvidenceRecordSummaries, PublicNetworkRuntimeEvidence, PublicNodeEvidence,
    PublicNodeRole, PublicOperatorIdentityAttestation, PublicServiceContentEvidence,
    PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind, PublicTestnetEvidenceBundle,
    PublicTestnetRunEvidence, aggregate_public_evidence_record_roots,
    public_network_runtime_observations_for_run,
};
use crate::types::{Address, Hash, address, hash_bytes};
use clap::Parser;

mod command_descriptions;
mod execution_reports;
mod local_validation;
mod manifest_reports;
mod network_observation;
mod parser;

fn parse_test_cli(args: &[&str]) -> std::result::Result<ExpectedCommand, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    Cli::try_parse_from(argv).map(|cli| ExpectedCommand::from(cli.command))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExpectedCommand {
    MinerRegister {
        stake: u64,
    },
    MinerStart {
        wallet: String,
        device: String,
        node: String,
    },
    MinerRun {
        wallet: String,
        device: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ValidatorStatus,
    ProposerRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceInit {
        data_dir: String,
    },
    ServicePeerAdd {
        data_dir: String,
        peer_id: String,
        address: String,
    },
    ServiceReadiness {
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
    },
    ServiceServe {
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceStatus {
        data_dir: String,
    },
    ServiceBlock {
        data_dir: String,
        height: u64,
    },
    LocalTestnetSeed {
        data_dir: String,
    },
    LocalCpuVerify {
        data_dir: String,
        json: bool,
    },
    PublicEvidenceValidate {
        manifest: String,
    },
    PublicEvidenceServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    PublicEvidenceServiceHealthFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        observation_file: String,
    },
    PublicEvidenceServiceContent {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    },
    PublicEvidenceServiceContentFromBytes {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_hex: String,
    },
    PublicEvidenceServiceContentFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_file: String,
    },
    PublicEvidenceRecordSummary {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifactFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordArtifactFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_file: String,
    },
    PublicEvidenceRecordSummaryFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordSummaryFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_file: String,
    },
    PublicEvidenceNetworkObservation {
        operator_id: Hash,
        peer_id: String,
        listen_address: String,
        observed_at_unix_seconds: u64,
        gossip_topic_count: u64,
        request_response_protocol_count: u64,
        bootstrap_peer_count: u64,
        max_transmit_bytes: u64,
        request_timeout_seconds: u64,
        max_concurrent_streams: u64,
        idle_connection_timeout_seconds: u64,
    },
    PublicEvidenceNetworkObservationFromServiceLog {
        operator_id: Hash,
        listen_address: String,
        observed_at_unix_seconds: u64,
        service_log: String,
    },
    PublicEvidencePublication {
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    },
    PublicEvidenceAuditorRecord {
        bundle_id: Hash,
        public_uri: String,
        auditor_id: Address,
        audit_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicEvidenceRunWindow {
        bundle_id: Hash,
        manifest_signer: Address,
        run_started_at_unix_seconds: u64,
        run_ended_at_unix_seconds: u64,
        observed_blocks: u64,
    },
    PublicEvidenceRunWindowFromFile {
        bundle_id: Hash,
        manifest_signer: Address,
        block_observation_file: String,
    },
    PublicEvidenceNodeHeartbeat {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    },
    PublicEvidenceNodeHeartbeatFromFile {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        heartbeat_file: String,
    },
    PublicEvidenceOperatorAttestation {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicTestnetPreflight {
        manifest: String,
    },
}

fn execute_reference_cli_command(command: &ExpectedCommand) -> crate::error::Result<String> {
    super::execute_reference_cli_command(&command.clone().into_cli_command())
}

fn describe_command(command: &ExpectedCommand) -> String {
    super::describe_command(&command.clone().into_cli_command())
}

impl ExpectedCommand {
    fn into_cli_command(self) -> super::CliCommand {
        match self {
            Self::MinerRegister { stake } => super::CliCommand::Miner {
                command: MinerCommand::Register(StakeArgs { stake }),
            },
            Self::MinerStart {
                wallet,
                device,
                node,
            } => super::CliCommand::Miner {
                command: MinerCommand::Start(MinerStartArgs {
                    wallet,
                    device,
                    node,
                }),
            },
            Self::MinerRun {
                wallet,
                device,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::CliCommand::Miner {
                command: MinerCommand::Run(MinerRunArgs {
                    wallet,
                    device,
                    runtime: RoleRuntimeArgs {
                        node,
                        service: ServiceRuntimeArgs {
                            listen,
                            p2p_listen,
                            data_dir,
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::MinerStatus => super::CliCommand::Miner {
                command: MinerCommand::Status,
            },
            Self::ValidatorRegister { stake } => super::CliCommand::Validator {
                command: ValidatorCommand::Register(StakeArgs { stake }),
            },
            Self::ValidatorStart { wallet, node } => super::CliCommand::Validator {
                command: ValidatorCommand::Start(ValidatorStartArgs { wallet, node }),
            },
            Self::ValidatorRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::CliCommand::Validator {
                command: ValidatorCommand::Run(ValidatorRunArgs {
                    wallet,
                    runtime: RoleRuntimeArgs {
                        node,
                        service: ServiceRuntimeArgs {
                            listen,
                            p2p_listen,
                            data_dir,
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::ValidatorStatus => super::CliCommand::Validator {
                command: ValidatorCommand::Status,
            },
            Self::ProposerRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::CliCommand::Proposer {
                command: ProposerCommand::Run(ValidatorRunArgs {
                    wallet,
                    runtime: RoleRuntimeArgs {
                        node,
                        service: ServiceRuntimeArgs {
                            listen,
                            p2p_listen,
                            data_dir,
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::ServiceInit { data_dir } => super::CliCommand::Service {
                command: ServiceCommand::Init(DataDirArgs { data_dir }),
            },
            Self::ServicePeerAdd {
                data_dir,
                peer_id,
                address,
            } => super::CliCommand::Service {
                command: ServiceCommand::Peer {
                    command: ServicePeerCommand::Add(ServicePeerAddArgs {
                        data_dir,
                        peer_id,
                        address,
                    }),
                },
            },
            Self::ServiceReadiness {
                p2p_listen,
                data_dir,
                identity_seed,
            } => super::CliCommand::Service {
                command: ServiceCommand::Readiness(ServiceReadinessArgs {
                    p2p_listen,
                    data_dir,
                    identity_seed,
                }),
            },
            Self::ServiceServe {
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::CliCommand::Service {
                command: ServiceCommand::Serve(ServiceServeArgs {
                    runtime: ServiceRuntimeArgs {
                        listen,
                        p2p_listen,
                        data_dir,
                        identity_seed,
                        auth_token,
                        max_requests,
                    },
                }),
            },
            Self::ServiceStatus { data_dir } => super::CliCommand::Service {
                command: ServiceCommand::Status(DataDirArgs { data_dir }),
            },
            Self::ServiceBlock { data_dir, height } => super::CliCommand::Service {
                command: ServiceCommand::Block(ServiceBlockArgs { data_dir, height }),
            },
            Self::LocalTestnetSeed { data_dir } => super::CliCommand::LocalTestnet {
                command: LocalTestnetCommand::Seed(DataDirArgs { data_dir }),
            },
            Self::LocalCpuVerify { data_dir, json } => super::CliCommand::LocalCpu {
                command: LocalCpuCommand::Verify(LocalCpuVerifyArgs { data_dir, json }),
            },
            Self::PublicEvidenceValidate { manifest } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::Validate(PublicEvidenceManifestArgs { manifest }),
            },
            Self::PublicEvidenceServiceHealth {
                kind,
                endpoint_id,
                public_url,
                health_path,
                first_seen_block,
                last_seen_block,
                reachable_observation_count,
                signed_health_check_count,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceHealth(ServiceHealthArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    health_path,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    reachable_count: reachable_observation_count,
                    signed_health_check_count,
                }),
            },
            Self::PublicEvidenceServiceHealthFromFile {
                kind,
                endpoint_id,
                public_url,
                health_path,
                observation_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceHealthFromFile(ServiceHealthFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    health_path,
                    observation_file,
                }),
            },
            Self::PublicEvidenceServiceContent {
                kind,
                endpoint_id,
                public_url,
                content_path,
                content_root,
                observed_at_unix_seconds,
                min_content_bytes,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContent(ServiceContentArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    content_path,
                    content_root,
                    observed_at: observed_at_unix_seconds,
                    min_content_bytes,
                }),
            },
            Self::PublicEvidenceServiceContentFromBytes {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_hex,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContentFromBytes(
                    ServiceContentFromBytesArgs {
                        kind: service_kind_arg(kind),
                        endpoint_id,
                        public_url,
                        content_path,
                        observed_at: observed_at_unix_seconds,
                        content_hex,
                    },
                ),
            },
            Self::PublicEvidenceServiceContentFromFile {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContentFromFile(
                    ServiceContentFromFileArgs {
                        kind: service_kind_arg(kind),
                        endpoint_id,
                        public_url,
                        content_path,
                        observed_at: observed_at_unix_seconds,
                        content_file,
                    },
                ),
            },
            Self::PublicEvidenceRecordSummary {
                kind,
                bundle_id,
                manifest_signer,
                record_root,
                record_count,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummary(RecordSummaryArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    record_root,
                    record_count,
                }),
            },
            Self::PublicEvidenceRecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_root,
                record_count,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifact(RecordArtifactArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    artifact_uri,
                    record_root,
                    record_count,
                }),
            },
            Self::PublicEvidenceRecordArtifactFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_roots,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifactFromRoots(
                    RecordArtifactFromRootsArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        artifact_uri,
                        record_roots: HashList(record_roots),
                    },
                ),
            },
            Self::PublicEvidenceRecordArtifactFromFile {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifactFromFile(
                    RecordArtifactFromFileArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        artifact_uri,
                        record_file,
                    },
                ),
            },
            Self::PublicEvidenceRecordSummaryFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                record_roots,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummaryFromRoots(
                    RecordSummaryFromRootsArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        record_roots: HashList(record_roots),
                    },
                ),
            },
            Self::PublicEvidenceRecordSummaryFromFile {
                kind,
                bundle_id,
                manifest_signer,
                record_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummaryFromFile(RecordSummaryFromFileArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    record_file,
                }),
            },
            Self::PublicEvidenceNetworkObservation {
                operator_id,
                peer_id,
                listen_address,
                observed_at_unix_seconds,
                gossip_topic_count,
                request_response_protocol_count,
                bootstrap_peer_count,
                max_transmit_bytes,
                request_timeout_seconds,
                max_concurrent_streams,
                idle_connection_timeout_seconds,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::NetworkObservation(NetworkObservationArgs {
                    operator_id,
                    peer_id,
                    listen_address,
                    observed_at: observed_at_unix_seconds,
                    gossip_topics: gossip_topic_count,
                    request_response_protocols: request_response_protocol_count,
                    bootstrap_peers: bootstrap_peer_count,
                    max_transmit_bytes,
                    request_timeout_seconds,
                    max_concurrent_streams,
                    idle_timeout_seconds: idle_connection_timeout_seconds,
                }),
            },
            Self::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id,
                listen_address,
                observed_at_unix_seconds,
                service_log,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::NetworkObservationFromServiceLog(
                    NetworkObservationFromServiceLogArgs {
                        operator_id,
                        listen_address,
                        observed_at: observed_at_unix_seconds,
                        service_log,
                    },
                ),
            },
            Self::PublicEvidencePublication {
                bundle_id,
                public_uri,
                manifest_signer,
                manifest_signature_count,
                independent_auditor_count,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::Publication(PublicationArgs {
                    bundle_id,
                    public_uri,
                    manifest_signer,
                    manifest_signature_count,
                    independent_auditor_count,
                }),
            },
            Self::PublicEvidenceAuditorRecord {
                bundle_id,
                public_uri,
                auditor_id,
                audit_uri,
                observed_at_unix_seconds,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::AuditorRecord(AuditorRecordArgs {
                    bundle_id,
                    public_uri,
                    auditor_id,
                    audit_uri,
                    observed_at: observed_at_unix_seconds,
                }),
            },
            Self::PublicEvidenceRunWindow {
                bundle_id,
                manifest_signer,
                run_started_at_unix_seconds,
                run_ended_at_unix_seconds,
                observed_blocks,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RunWindow(RunWindowArgs {
                    bundle_id,
                    manifest_signer,
                    started_at: run_started_at_unix_seconds,
                    ended_at: run_ended_at_unix_seconds,
                    observed_blocks,
                }),
            },
            Self::PublicEvidenceRunWindowFromFile {
                bundle_id,
                manifest_signer,
                block_observation_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::RunWindowFromFile(RunWindowFromFileArgs {
                    bundle_id,
                    manifest_signer,
                    block_observation_file,
                }),
            },
            Self::PublicEvidenceNodeHeartbeat {
                role,
                address,
                operator_id,
                first_seen_block,
                last_seen_block,
                signed_heartbeat_count,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::NodeHeartbeat(NodeHeartbeatArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    heartbeat_count: signed_heartbeat_count,
                }),
            },
            Self::PublicEvidenceNodeHeartbeatFromFile {
                role,
                address,
                operator_id,
                heartbeat_file,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::NodeHeartbeatFromFile(NodeHeartbeatFromFileArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    heartbeat_file,
                }),
            },
            Self::PublicEvidenceOperatorAttestation {
                role,
                address,
                operator_id,
                identity_uri,
                observed_at_unix_seconds,
            } => super::CliCommand::PublicEvidence {
                command: PublicEvidenceCommand::OperatorAttestation(OperatorAttestationArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    identity_uri,
                    observed_at: observed_at_unix_seconds,
                }),
            },
            Self::PublicTestnetPreflight { manifest } => super::CliCommand::PublicTestnet {
                command: PublicTestnetCommand::Preflight(PublicTestnetManifestArgs { manifest }),
            },
        }
    }
}

impl From<super::CliCommand> for ExpectedCommand {
    fn from(command: super::CliCommand) -> Self {
        match command {
            super::CliCommand::Miner { command } => match command {
                MinerCommand::Register(args) => Self::MinerRegister { stake: args.stake },
                MinerCommand::Start(args) => Self::MinerStart {
                    wallet: args.wallet,
                    device: args.device,
                    node: args.node,
                },
                MinerCommand::Run(args) => Self::MinerRun {
                    wallet: args.wallet,
                    device: args.device,
                    node: args.runtime.node,
                    listen: args.runtime.service.listen,
                    p2p_listen: args.runtime.service.p2p_listen,
                    data_dir: args.runtime.service.data_dir,
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
                MinerCommand::Status => Self::MinerStatus,
            },
            super::CliCommand::Validator { command } => match command {
                ValidatorCommand::Register(args) => Self::ValidatorRegister { stake: args.stake },
                ValidatorCommand::Start(args) => Self::ValidatorStart {
                    wallet: args.wallet,
                    node: args.node,
                },
                ValidatorCommand::Run(args) => Self::ValidatorRun {
                    wallet: args.wallet,
                    node: args.runtime.node,
                    listen: args.runtime.service.listen,
                    p2p_listen: args.runtime.service.p2p_listen,
                    data_dir: args.runtime.service.data_dir,
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
                ValidatorCommand::Status => Self::ValidatorStatus,
            },
            super::CliCommand::Proposer { command } => match command {
                ProposerCommand::Run(args) => Self::ProposerRun {
                    wallet: args.wallet,
                    node: args.runtime.node,
                    listen: args.runtime.service.listen,
                    p2p_listen: args.runtime.service.p2p_listen,
                    data_dir: args.runtime.service.data_dir,
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
            },
            super::CliCommand::Service { command } => match command {
                ServiceCommand::Init(args) => Self::ServiceInit {
                    data_dir: args.data_dir,
                },
                ServiceCommand::Peer {
                    command: ServicePeerCommand::Add(args),
                } => Self::ServicePeerAdd {
                    data_dir: args.data_dir,
                    peer_id: args.peer_id,
                    address: args.address,
                },
                ServiceCommand::Readiness(args) => Self::ServiceReadiness {
                    p2p_listen: args.p2p_listen,
                    data_dir: args.data_dir,
                    identity_seed: args.identity_seed,
                },
                ServiceCommand::Serve(args) => Self::ServiceServe {
                    listen: args.runtime.listen,
                    p2p_listen: args.runtime.p2p_listen,
                    data_dir: args.runtime.data_dir,
                    identity_seed: args.runtime.identity_seed,
                    auth_token: args.runtime.auth_token,
                    max_requests: args.runtime.max_requests,
                },
                ServiceCommand::Status(args) => Self::ServiceStatus {
                    data_dir: args.data_dir,
                },
                ServiceCommand::Block(args) => Self::ServiceBlock {
                    data_dir: args.data_dir,
                    height: args.height,
                },
            },
            super::CliCommand::LocalTestnet { command } => match command {
                LocalTestnetCommand::Seed(args) => Self::LocalTestnetSeed {
                    data_dir: args.data_dir,
                },
            },
            super::CliCommand::LocalCpu { command } => match command {
                LocalCpuCommand::Verify(args) => Self::LocalCpuVerify {
                    data_dir: args.data_dir,
                    json: args.json,
                },
            },
            super::CliCommand::PublicEvidence { command } => match command {
                PublicEvidenceCommand::Validate(args) => Self::PublicEvidenceValidate {
                    manifest: args.manifest,
                },
                PublicEvidenceCommand::ServiceHealth(args) => Self::PublicEvidenceServiceHealth {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    health_path: args.health_path,
                    first_seen_block: args.first_block,
                    last_seen_block: args.last_block,
                    reachable_observation_count: args.reachable_count,
                    signed_health_check_count: args.signed_health_check_count,
                },
                PublicEvidenceCommand::ServiceHealthFromFile(args) => {
                    Self::PublicEvidenceServiceHealthFromFile {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        health_path: args.health_path,
                        observation_file: args.observation_file,
                    }
                }
                PublicEvidenceCommand::ServiceContent(args) => Self::PublicEvidenceServiceContent {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    content_path: args.content_path,
                    content_root: args.content_root,
                    observed_at_unix_seconds: args.observed_at,
                    min_content_bytes: args.min_content_bytes,
                },
                PublicEvidenceCommand::ServiceContentFromBytes(args) => {
                    Self::PublicEvidenceServiceContentFromBytes {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        content_path: args.content_path,
                        observed_at_unix_seconds: args.observed_at,
                        content_hex: args.content_hex,
                    }
                }
                PublicEvidenceCommand::ServiceContentFromFile(args) => {
                    Self::PublicEvidenceServiceContentFromFile {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        content_path: args.content_path,
                        observed_at_unix_seconds: args.observed_at,
                        content_file: args.content_file,
                    }
                }
                PublicEvidenceCommand::RecordSummary(args) => Self::PublicEvidenceRecordSummary {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    record_root: args.record_root,
                    record_count: args.record_count,
                },
                PublicEvidenceCommand::RecordArtifact(args) => Self::PublicEvidenceRecordArtifact {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    artifact_uri: args.artifact_uri,
                    record_root: args.record_root,
                    record_count: args.record_count,
                },
                PublicEvidenceCommand::RecordArtifactFromRoots(args) => {
                    Self::PublicEvidenceRecordArtifactFromRoots {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        artifact_uri: args.artifact_uri,
                        record_roots: args.record_roots.0,
                    }
                }
                PublicEvidenceCommand::RecordArtifactFromFile(args) => {
                    Self::PublicEvidenceRecordArtifactFromFile {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        artifact_uri: args.artifact_uri,
                        record_file: args.record_file,
                    }
                }
                PublicEvidenceCommand::RecordSummaryFromRoots(args) => {
                    Self::PublicEvidenceRecordSummaryFromRoots {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        record_roots: args.record_roots.0,
                    }
                }
                PublicEvidenceCommand::RecordSummaryFromFile(args) => {
                    Self::PublicEvidenceRecordSummaryFromFile {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        record_file: args.record_file,
                    }
                }
                PublicEvidenceCommand::NetworkObservation(args) => {
                    Self::PublicEvidenceNetworkObservation {
                        operator_id: args.operator_id,
                        peer_id: args.peer_id,
                        listen_address: args.listen_address,
                        observed_at_unix_seconds: args.observed_at,
                        gossip_topic_count: args.gossip_topics,
                        request_response_protocol_count: args.request_response_protocols,
                        bootstrap_peer_count: args.bootstrap_peers,
                        max_transmit_bytes: args.max_transmit_bytes,
                        request_timeout_seconds: args.request_timeout_seconds,
                        max_concurrent_streams: args.max_concurrent_streams,
                        idle_connection_timeout_seconds: args.idle_timeout_seconds,
                    }
                }
                PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => {
                    Self::PublicEvidenceNetworkObservationFromServiceLog {
                        operator_id: args.operator_id,
                        listen_address: args.listen_address,
                        observed_at_unix_seconds: args.observed_at,
                        service_log: args.service_log,
                    }
                }
                PublicEvidenceCommand::Publication(args) => Self::PublicEvidencePublication {
                    bundle_id: args.bundle_id,
                    public_uri: args.public_uri,
                    manifest_signer: args.manifest_signer,
                    manifest_signature_count: args.manifest_signature_count,
                    independent_auditor_count: args.independent_auditor_count,
                },
                PublicEvidenceCommand::AuditorRecord(args) => Self::PublicEvidenceAuditorRecord {
                    bundle_id: args.bundle_id,
                    public_uri: args.public_uri,
                    auditor_id: args.auditor_id,
                    audit_uri: args.audit_uri,
                    observed_at_unix_seconds: args.observed_at,
                },
                PublicEvidenceCommand::RunWindow(args) => Self::PublicEvidenceRunWindow {
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    run_started_at_unix_seconds: args.started_at,
                    run_ended_at_unix_seconds: args.ended_at,
                    observed_blocks: args.observed_blocks,
                },
                PublicEvidenceCommand::RunWindowFromFile(args) => {
                    Self::PublicEvidenceRunWindowFromFile {
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        block_observation_file: args.block_observation_file,
                    }
                }
                PublicEvidenceCommand::NodeHeartbeat(args) => Self::PublicEvidenceNodeHeartbeat {
                    role: args.role.into(),
                    address: args.address,
                    operator_id: args.operator_id,
                    first_seen_block: args.first_block,
                    last_seen_block: args.last_block,
                    signed_heartbeat_count: args.heartbeat_count,
                },
                PublicEvidenceCommand::NodeHeartbeatFromFile(args) => {
                    Self::PublicEvidenceNodeHeartbeatFromFile {
                        role: args.role.into(),
                        address: args.address,
                        operator_id: args.operator_id,
                        heartbeat_file: args.heartbeat_file,
                    }
                }
                PublicEvidenceCommand::OperatorAttestation(args) => {
                    Self::PublicEvidenceOperatorAttestation {
                        role: args.role.into(),
                        address: args.address,
                        operator_id: args.operator_id,
                        identity_uri: args.identity_uri,
                        observed_at_unix_seconds: args.observed_at,
                    }
                }
            },
            super::CliCommand::PublicTestnet { command } => match command {
                PublicTestnetCommand::Preflight(args) => Self::PublicTestnetPreflight {
                    manifest: args.manifest,
                },
            },
        }
    }
}

fn service_kind_arg(kind: PublicServiceKind) -> PublicServiceKindArg {
    match kind {
        PublicServiceKind::Rpc => PublicServiceKindArg::Rpc,
        PublicServiceKind::Explorer => PublicServiceKindArg::Explorer,
        PublicServiceKind::Faucet => PublicServiceKindArg::Faucet,
        PublicServiceKind::Telemetry => PublicServiceKindArg::Telemetry,
    }
}

fn node_role_arg(role: PublicNodeRole) -> PublicNodeRoleArg {
    match role {
        PublicNodeRole::Miner => PublicNodeRoleArg::Miner,
        PublicNodeRole::Validator => PublicNodeRoleArg::Validator,
    }
}

fn record_kind_arg(kind: PublicEvidenceRecordKind) -> PublicEvidenceRecordKindArg {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => PublicEvidenceRecordKindArg::BlockHistory,
        PublicEvidenceRecordKind::FinalityHistory => PublicEvidenceRecordKindArg::FinalityHistory,
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            PublicEvidenceRecordKindArg::NetworkRuntime
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            PublicEvidenceRecordKindArg::DataAvailability
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => PublicEvidenceRecordKindArg::InvalidWork,
        PublicEvidenceRecordKind::RewardSettlements => {
            PublicEvidenceRecordKindArg::RewardSettlement
        }
    }
}

fn manifest_hash(label: &[u8]) -> String {
    hex(&hash_bytes(b"test", &[label]))
}

fn manifest_address(label: &[u8]) -> String {
    hex(&address(label))
}

fn manifest_node_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
        PublicNodeRole::Validator => {
            PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
        }
    };
    hex(&node.heartbeat_signature)
}

fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
    format!("https://operators.tensorvm.net/{}", hex(operator_id))
}

fn manifest_operator_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        node_address,
        operator_id,
        manifest_operator_identity_uri(&operator_id),
        1_700_000_000,
    );
    hex(&attestation.operator_signature)
}

fn public_service_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
    }
}

fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
    let service = PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[label]),
            public_service_url(kind),
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    hex(&service.health_check_signature)
}

fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
    }
}

fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "/chain/head",
        PublicServiceKind::Explorer => "/explorer",
        PublicServiceKind::Faucet => "/faucet/page",
        PublicServiceKind::Telemetry => "/telemetry/dashboard",
    }
}

fn public_service_content(kind: PublicServiceKind, label: &[u8]) -> PublicServiceContentEvidence {
    PublicServiceContentEvidence::new(
        kind,
        hash_bytes(b"test", &[label]),
        public_service_content_url(kind),
        public_service_content_path(kind),
        hash_bytes(b"test", &[label, b"content-root"]),
        1_700_000_000,
        64,
    )
}

fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
    let content = public_service_content(kind, label);
    format!(
        "service_content={},{},{},{},{},{},{},{}",
        public_service_kind_tag(kind),
        hex(&content.endpoint_id),
        content.public_url,
        content.content_path,
        hex(&content.content_root),
        content.observed_at_unix_seconds,
        content.min_content_bytes,
        hex(&content.content_signature)
    )
}

fn manifest_publication_signature() -> String {
    let publication = PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    );
    hex(&publication.manifest_signature)
}

fn manifest_publication() -> PublicEvidencePublication {
    PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    )
}

fn manifest_auditor_uri() -> String {
    format!(
        "https://auditors.tensorvm.net/{}/0",
        manifest_hash(b"public-evidence-bundle")
    )
}

fn manifest_auditor_signature() -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let record = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        "https://tensorvm.net/tensorvm/public-evidence.json",
        address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        1_700_000_060,
    );
    hex(&record.auditor_signature)
}

fn manifest_artifact_line(
    kind: PublicEvidenceRecordKind,
    root_label: &[u8],
    record_count: u64,
) -> String {
    manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
}

fn manifest_artifact_line_for_root(
    kind: PublicEvidenceRecordKind,
    record_root: Hash,
    record_count: u64,
) -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let artifact_uri = format!(
        "https://evidence.tensorvm.net/{}/{}.json",
        manifest_hash(b"public-evidence-bundle"),
        public_evidence_record_kind_tag(kind)
    );
    let signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &bundle_id,
        kind,
        &artifact_uri,
        &record_root,
        record_count,
    );
    format!(
        "record_artifact={},{},{},{},{}",
        public_evidence_record_kind_tag(kind),
        artifact_uri,
        hex(&record_root),
        record_count,
        hex(&signature)
    )
}

fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
    let record_roots = public_network_runtime_observations_for_run(run)
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_roots,
    )
    .expect("generated network observation roots should aggregate")
}

fn manifest_network_observation_lines() -> String {
    public_network_runtime_observations_for_run(&manifest_bundle().run)
        .iter()
        .map(|observation| {
            format!(
                "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                hex(&observation.operator_id),
                observation.peer_id,
                observation.listen_address,
                observation.observed_at_unix_seconds,
                observation.gossip_topic_count,
                observation.request_response_protocol_count,
                observation.bootstrap_peer_count,
                observation.max_transmit_bytes,
                observation.request_timeout_seconds,
                observation.max_concurrent_streams,
                observation.idle_connection_timeout_seconds,
                hex(&observation.record_root),
                hex(&observation.observation_signature)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn manifest_bundle() -> PublicTestnetEvidenceBundle {
    let run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::miner(
                address(b"miner-b"),
                hash_bytes(b"test", &[b"miner-b-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::validator(
                address(b"validator-a"),
                hash_bytes(b"test", &[b"validator-a-operator"]),
                0,
                9,
                10,
            ),
        ],
        network_runtime: PublicNetworkRuntimeEvidence {
            libp2p_runtime_used: true,
            peer_discovery_observed: true,
            gossip_propagation_observed: true,
            request_response_observed: true,
            dos_controls_enabled: true,
        },
        services: vec![
            PublicServiceEvidence::new(
                PublicServiceKind::Rpc,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"rpc-service"]),
                    public_service_url(PublicServiceKind::Rpc),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Explorer,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"explorer-service"]),
                    public_service_url(PublicServiceKind::Explorer),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Faucet,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"faucet-service"]),
                    public_service_url(PublicServiceKind::Faucet),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Telemetry,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"telemetry-service"]),
                    public_service_url(PublicServiceKind::Telemetry),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
        ],
        service_content: vec![
            public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
            public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
            public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
            public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
        ],
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 10,
        checked_receipts: 20,
        available_receipts: 19,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
    };
    let network_runtime_observation_root = network_runtime_root_for_run(&run);
    PublicTestnetEvidenceBundle::new(
        run,
        manifest_publication(),
        PublicEvidenceRecordSummaries {
            block_history_records: 10,
            block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
            finality_history_records: 10,
            finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
            operator_identity_attestation_records: 3,
            network_runtime_observation_records: 3,
            network_runtime_observation_root,
            data_availability_measurement_records: 20,
            data_availability_measurement_root: hash_bytes(b"test", &[b"data-availability-root"]),
            invalid_work_rejection_records: 1,
            invalid_work_rejection_root: hash_bytes(b"test", &[b"invalid-work-root"]),
            reward_settlement_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
        },
    )
}

fn evidence_manifest() -> String {
    format!(
        "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
auditor={},{},1700000060,{}
{}
{}
{}
{}
{}
{}
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
operator=miner,{},{},{},1700000000,{}
operator=miner,{},{},{},1700000000,{}
operator=validator,{},{},{},1700000000,{}
{}
network_runtime_observation_records=3
network_runtime_observation_root={}
network_runtime_observation_signature={}
data_availability_measurement_records=20
data_availability_measurement_root={}
data_availability_measurement_signature={}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
run_window_signature={}
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root={}
invalid_work_rejection_signature={}
reward_settlement_records=1
reward_settlement_root={}
reward_settlement_signature={}
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.net/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.net/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.net/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,0,9,10,10,{}
{}
{}
{}
{}
",
        manifest_hash(b"public-evidence-bundle"),
        manifest_address(b"public-evidence-publisher"),
        manifest_publication_signature(),
        manifest_address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        manifest_auditor_signature(),
        manifest_artifact_line(
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10
        ),
        manifest_artifact_line_for_root(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            manifest_bundle().network_runtime_observation_root,
            3
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1
        ),
        manifest_hash(b"block-history-root"),
        hex(&manifest_bundle().block_history_signature),
        manifest_hash(b"finality-history-root"),
        hex(&manifest_bundle().finality_history_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
        manifest_operator_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_network_observation_lines(),
        hex(&manifest_bundle().network_runtime_observation_root),
        hex(&manifest_bundle().network_runtime_observation_signature),
        manifest_hash(b"data-availability-root"),
        hex(&manifest_bundle().data_availability_measurement_signature),
        hex(&manifest_bundle().run_window_signature),
        manifest_hash(b"invalid-work-root"),
        hex(&manifest_bundle().invalid_work_rejection_signature),
        manifest_hash(b"reward-settlement-root"),
        hex(&manifest_bundle().reward_settlement_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_node_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_hash(b"rpc-service"),
        manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_hash(b"explorer-service"),
        manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_hash(b"faucet-service"),
        manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_hash(b"telemetry-service"),
        manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
    )
}

fn preflight_manifest() -> String {
    format!(
            "\
version={PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION}
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,{},https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,{},https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
",
            manifest_hash(b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_hash(b"telemetry-service"),
        )
}

#[test]
fn execute_reference_cli_command_rejects_invalid_public_evidence_args() {
    let peer_id = PeerId::random().to_string();
    let make_network_observation = |operator_id,
                                    peer_id: String,
                                    listen_address: String,
                                    observed_at_unix_seconds,
                                    gossip_topic_count,
                                    request_response_protocol_count,
                                    bootstrap_peer_count,
                                    max_transmit_bytes| {
        ExpectedCommand::PublicEvidenceNetworkObservation {
            operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds,
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
            max_transmit_bytes,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    };
    let operator_id = hash_bytes(b"test", &[b"network-operator"]);
    let public_listen_address = "/dns/node-a.tensorvm.net/tcp/4001".to_owned();
    for invalid in [
        make_network_observation(
            [0; 32],
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            0,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            0,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            0,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            0,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            2,
            0,
        ),
        make_network_observation(
            operator_id,
            "not-a-peer-id".to_owned(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "not-a-multiaddr".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/127.0.0.1/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8/tcp/0".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8/udp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/203.0.113.10/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/dns/bad_host.tensorvm.net/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/dns/node.tensorvm.example/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
    ] {
        assert!(execute_reference_cli_command(&invalid).is_err());
    }
    assert!(parse_public_service_kind("archive").is_err());
    assert_eq!(
        parse_public_node_role("miner").unwrap(),
        PublicNodeRole::Miner
    );
    assert_eq!(
        parse_public_node_role("validator").unwrap(),
        PublicNodeRole::Validator
    );
    assert!(parse_public_node_role("observer").is_err());
    assert_eq!(
        parse_public_evidence_record_kind("block-history").unwrap(),
        PublicEvidenceRecordKind::BlockHistory
    );
    assert_eq!(
        parse_public_evidence_record_kind("finality-history").unwrap(),
        PublicEvidenceRecordKind::FinalityHistory
    );
    assert_eq!(
        parse_public_evidence_record_kind("network-runtime").unwrap(),
        PublicEvidenceRecordKind::NetworkRuntimeObservations
    );
    assert_eq!(
        parse_public_evidence_record_kind("data-availability").unwrap(),
        PublicEvidenceRecordKind::DataAvailabilityMeasurements
    );
    assert_eq!(
        parse_public_evidence_record_kind("invalid-work").unwrap(),
        PublicEvidenceRecordKind::InvalidWorkRejections
    );
    assert_eq!(
        parse_public_evidence_record_kind("reward-settlement").unwrap(),
        PublicEvidenceRecordKind::RewardSettlements
    );
    assert!(parse_public_evidence_record_kind("operator-identity").is_err());
    assert!(parse_hash_argument("12").is_err());
    assert!(parse_hash_argument(&"g".repeat(64)).is_err());
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "http://127.0.0.1/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.example.test/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health?probe=1".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health#probe".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 10,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: [0; 32],
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 0,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    let partial_health = service_health_observation_summary_from_file(
        "service_health_observation=0,reachable\nservice_health_observation=1,unreachable\n",
    )
    .unwrap();
    assert_eq!(partial_health.first_seen_block, 0);
    assert_eq!(partial_health.last_seen_block, 1);
    assert_eq!(partial_health.reachable_observation_count, 1);
    assert_eq!(partial_health.signed_health_check_count, 2);
    for invalid_health_observations in [
        "# no observations\n\n",
        " service_health_observation=0,reachable\n",
        "service_health_observation=0,reachable\nservice_health_observation=0,reachable\n",
        "service_health_observation=0,reachable\nservice_health_observation=2,reachable\n",
        "service_health_observation=0,ok\n",
        "service_health_observation=0\n",
        "record_root=00\n",
    ] {
        assert!(service_health_observation_summary_from_file(invalid_health_observations).is_err());
    }
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-service-health-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://localhost/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head?height=1".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head#latest".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            content_path: "/wrong".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: [0; 32],
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 0,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 63,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex: "zz".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex: "abc".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex: hex(&[1_u8; 63]),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_file: std::env::temp_dir()
                .join("tensor-vm-missing-service-content-file.body")
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://evidence.tensorvm.example/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "http://127.0.0.1/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: " https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json ".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json?download=1".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: [0; 32],
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: [0; 32],
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 0,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 2,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: [0; 32],
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://localhost/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: "https://localhost/audit.json".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: "https://auditor.tensorvm.net/".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_060,
            run_ended_at_unix_seconds: 1_700_000_000,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 0,
        })
        .is_err()
    );
    let run_window_summary = run_window_observation_summary_from_file(
        "run_window_observation=7,1700000000\nrun_window_observation=8,1700000006\n",
    )
    .unwrap();
    assert_eq!(
        run_window_summary.run_started_at_unix_seconds,
        1_700_000_000
    );
    assert_eq!(run_window_summary.run_ended_at_unix_seconds, 1_700_000_006);
    assert_eq!(run_window_summary.observed_blocks, 2);
    for invalid_run_window_observations in [
        "# no observations\n\n",
        " run_window_observation=0,1700000000\n",
        "run_window_observation=0,1700000000\nrun_window_observation=0,1700000001\n",
        "run_window_observation=0,1700000000\nrun_window_observation=2,1700000012\n",
        "run_window_observation=0,1700000006\nrun_window_observation=1,1700000000\n",
        "run_window_observation=0,0\n",
        "run_window_observation=0\n",
        "service_health_observation=0,reachable\n",
    ] {
        assert!(run_window_observation_summary_from_file(invalid_run_window_observations).is_err());
    }
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-run-window-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: [0; 32],
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: [0; 32],
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 10,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 0,
        })
        .is_err()
    );
    let miner_address_hex = manifest_address(b"miner-a");
    let miner_operator_hex = manifest_hash(b"miner-a-operator");
    let heartbeat_summary = node_heartbeat_observation_summary_from_file(
            PublicNodeRole::Miner,
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            &format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},1\n"
            ),
        )
        .unwrap();
    assert_eq!(heartbeat_summary.first_seen_block, 0);
    assert_eq!(heartbeat_summary.last_seen_block, 1);
    assert_eq!(heartbeat_summary.signed_heartbeat_count, 2);
    for invalid_heartbeat_observations in [
        "# no observations\n\n".to_owned(),
        format!(" node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},2\n"
        ),
        format!(
            "node_heartbeat_observation=validator,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!(
            "node_heartbeat_observation=miner,{},{} ,0\n",
            miner_address_hex, miner_operator_hex
        ),
        format!("node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex}\n"),
        "service_health_observation=0,reachable\n".to_owned(),
    ] {
        assert!(
            node_heartbeat_observation_summary_from_file(
                PublicNodeRole::Miner,
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                &invalid_heartbeat_observations,
            )
            .is_err()
        );
    }
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeatFromFile {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            heartbeat_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-node-heartbeat-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: [0; 32],
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: [0; 32],
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://localhost/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 0,
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-artifact",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary-from-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            "",
        ])
        .is_err()
    );
    let root_a = manifest_hash(b"network-observation-a");
    let root_b = manifest_hash(b"network-observation-b");
    let padded_roots = format!("{root_a}, {root_b}");
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary-from-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            &padded_roots,
        ])
        .is_err()
    );
    let empty_root_entry = format!("{root_a},,{root_b}");
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-artifact-from-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-roots",
            &empty_root_entry,
        ])
        .is_err()
    );
    let valid_record_summary = ExpectedCommand::PublicEvidenceRecordSummary {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_reference_cli_command(&valid_record_summary).is_ok());
    let valid_record_artifact = ExpectedCommand::PublicEvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_reference_cli_command(&valid_record_artifact).is_ok());
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: [0; 32],
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://localhost/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: [0; 32],
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![[0; 32]],
        })
        .is_err()
    );
    let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
}
