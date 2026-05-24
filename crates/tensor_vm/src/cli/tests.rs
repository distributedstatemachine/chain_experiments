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
mod local_validation;
mod manifest_reports;
mod network_observation;

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
fn parses_documented_miner_commands() {
    assert_eq!(
        parse_test_cli(&["miner", "register", "--stake", "100"]).unwrap(),
        ExpectedCommand::MinerRegister { stake: 100 }
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "start",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["miner", "status"]).unwrap(),
        ExpectedCommand::MinerStatus
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
}

#[test]
fn parses_documented_validator_commands() {
    assert_eq!(
        parse_test_cli(&["validator", "register", "--stake", "10000"]).unwrap(),
        ExpectedCommand::ValidatorRegister { stake: 10_000 }
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "start",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        ExpectedCommand::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["validator", "status"]).unwrap(),
        ExpectedCommand::ValidatorStatus
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ValidatorRun {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "22".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ValidatorRun {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x22; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    assert_eq!(
        parse_test_cli(&["local-testnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::LocalTestnetSeed {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "validate",
            "--manifest",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceValidate {
            manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-testnet",
            "preflight",
            "--manifest",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .unwrap(),
        ExpectedCommand::PublicTestnetPreflight {
            manifest: "docs/tensorvm/public-testnet.preflight".to_owned(),
        }
    );
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "publication",
            "--bundle-id",
            &bundle_id,
            "--public-uri",
            "https://tensorvm.net/tensorvm/public-evidence.json",
            "--manifest-signer",
            &manifest_signer,
            "--manifest-signature-count",
            "1",
            "--independent-auditor-count",
            "1",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "auditor-record",
            "--bundle-id",
            &bundle_id,
            "--public-uri",
            "https://tensorvm.net/tensorvm/public-evidence.json",
            "--auditor-id",
            &manifest_address(b"public-evidence-auditor-0"),
            "--audit-uri",
            &manifest_auditor_uri(),
            "--observed-at",
            "1700000060",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_060,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "run-window",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--started-at",
            "1700000000",
            "--ended-at",
            "1700000060",
            "--observed-blocks",
            "10",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "run-window-from-file",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--block-observation-file",
            "artifacts/block-observations.records",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: "artifacts/block-observations.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "node-heartbeat",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--first-block",
            "0",
            "--last-block",
            "9",
            "--heartbeat-count",
            "10",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "node-heartbeat-from-file",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--heartbeat-file",
            "artifacts/miner-a-heartbeats.records",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceNodeHeartbeatFromFile {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            heartbeat_file: "artifacts/miner-a-heartbeats.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "operator-attestation",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--identity-uri",
            "https://operators.tensorvm.net/miner-a",
            "--observed-at",
            "1700000000",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        }
    );
    let endpoint_id = manifest_hash(b"rpc-service");
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "service-health",
            "--kind",
            "rpc",
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            "https://rpc.tensorvm.net/health",
            "--health-path",
            "/health",
            "--first-block",
            "0",
            "--last-block",
            "9",
            "--reachable-count",
            "10",
            "--signed-health-check-count",
            "10",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "service-health-from-file",
            "--kind",
            "rpc",
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            "https://rpc.tensorvm.net/health",
            "--health-path",
            "/health",
            "--observation-file",
            "artifacts/rpc-health.records",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: "artifacts/rpc-health.records".to_owned(),
        }
    );
    let content_root = manifest_hash(b"rpc-service-content");
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "service-content",
            "--kind",
            "rpc",
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            "https://rpc.tensorvm.net/chain/head",
            "--content-path",
            "/chain/head",
            "--content-root",
            &content_root,
            "--observed-at",
            "1700000000",
            "--min-content-bytes",
            "64",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service-content"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        }
    );
    let content_hex = hex(&[42_u8; 64]);
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "service-content-from-bytes",
            "--kind",
            "rpc",
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            "https://rpc.tensorvm.net/chain/head",
            "--content-path",
            "/chain/head",
            "--observed-at",
            "1700000000",
            "--content-hex",
            &content_hex,
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "service-content-from-file",
            "--kind",
            "rpc",
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            "https://rpc.tensorvm.net/chain/head",
            "--content-path",
            "/chain/head",
            "--observed-at",
            "1700000000",
            "--content-file",
            "artifacts/rpc-chain-head.body",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceServiceContentFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_file: "artifacts/rpc-chain-head.body".to_owned(),
        }
    );
    let peer_id = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "network-observation",
            "--operator-id",
            &manifest_hash(b"network-operator"),
            "--peer-id",
            &peer_id,
            "--listen-address",
            "/dns/node-a.tensorvm.net/tcp/4001",
            "--observed-at",
            "1700000000",
            "--gossip-topics",
            "5",
            "--request-response-protocols",
            "4",
            "--bootstrap-peers",
            "2",
            "--max-transmit-bytes",
            "1048576",
            "--request-timeout-seconds",
            "10",
            "--max-concurrent-streams",
            "128",
            "--idle-timeout-seconds",
            "60",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceNetworkObservation {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            peer_id: peer_id.clone(),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            gossip_topic_count: 5,
            request_response_protocol_count: 4,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "network-observation-from-service-log",
            "--operator-id",
            &manifest_hash(b"network-operator"),
            "--listen-address",
            "/dns/node-a.tensorvm.net/tcp/4001",
            "--observed-at",
            "1700000000",
            "--service-log",
            "artifacts/node-a-service.log",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            service_log: "artifacts/node-a-service.log".to_owned(),
        }
    );
    let record_root = manifest_hash(b"network-runtime-root");
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-artifact",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        }
    );
    let record_roots = format!(
        "{},{}",
        manifest_hash(b"network-observation-a"),
        manifest_hash(b"network-observation-b")
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary-from-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-roots",
            &record_roots,
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![
                hash_bytes(b"test", &[b"network-observation-a"]),
                hash_bytes(b"test", &[b"network-observation-b"]),
            ],
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-artifact-from-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-roots",
            &record_roots,
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: vec![
                hash_bytes(b"test", &[b"network-observation-a"]),
                hash_bytes(b"test", &[b"network-observation-b"]),
            ],
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary-from-file",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-file",
            "artifacts/network-runtime.records",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "record-artifact-from-file",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-file",
            "artifacts/network-runtime.records",
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::ServiceInit {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    let bootstrap_peer = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "service",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            &bootstrap_peer,
            "--address",
            "/dns/bootstrap.tensorvm.net/tcp/4001",
        ])
        .unwrap(),
        ExpectedCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: bootstrap_peer.clone(),
            address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "readiness",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .unwrap(),
        ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
        }
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "service",
            "readiness",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
        ])
        .unwrap(),
        ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        ExpectedCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        ExpectedCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "status", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::ServiceStatus {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "block",
            "--data-dir",
            "/var/lib/tensorvm",
            "--height",
            "3"
        ])
        .unwrap(),
        ExpectedCommand::ServiceBlock {
            data_dir: "/var/lib/tensorvm".to_owned(),
            height: 3,
        }
    );
}

#[test]
fn parses_documented_proposer_commands() {
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ProposerRun {
            wallet: "proposer.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "33".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ProposerRun {
            wallet: "proposer.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x33; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
}

#[test]
fn rejects_invalid_cli() {
    assert!(parse_test_cli(&["miner", "register"]).is_err());
    assert!(parse_test_cli(&["validator", "register", "--stake", "abc"]).is_err());
    assert!(
        parse_test_cli(&[
            "service",
            "serve",
            "--listen",
            "not-a-socket",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--node",
            "not-a-multiaddr",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
}

#[test]
fn clap_cli_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["miner", "start", "--wallet", "miner.key"]).unwrap(),
        ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--auth-token",
            "secret"
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: ".tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "serve", "--auth-token", "secret"]).unwrap(),
        ExpectedCommand::ServiceServe {
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: ".tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "init"]).unwrap(),
        ExpectedCommand::ServiceInit {
            data_dir: ".tensorvm".to_owned(),
        }
    );
}

#[test]
fn execute_reference_cli_command_reports_miner_and_validator_readiness() {
    let miner_register =
        execute_reference_cli_command(&ExpectedCommand::MinerRegister { stake: 100 }).unwrap();
    assert!(miner_register.contains("command=miner_register"));
    assert!(miner_register.contains("min_stake=100"));
    assert!(miner_register.contains("stake_sufficient=true"));

    let miner_start = execute_reference_cli_command(&ExpectedCommand::MinerStart {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(miner_start.contains("command=miner_start"));
    assert!(miner_start.contains("wallet=miner.key"));
    assert!(miner_start.contains("device=cpu"));
    assert!(miner_start.contains("device_backend=cpu-reference"));
    assert!(miner_start.contains(&format!(
        "cuda_kernels_compiled={}",
        cuda_kernels_compiled()
    )));
    assert!(miner_start.contains("node=/ip4/127.0.0.1/tcp/4001"));
    assert!(miner_start.contains(&format!("address={}", hex(&address(b"miner.key")))));
    assert!(miner_start.contains("reference_backend_ready=true"));

    let miner_run = execute_reference_cli_command(&ExpectedCommand::MinerRun {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(miner_run.contains("command=miner_run"));
    assert!(miner_run.contains("role=miner"));
    assert!(miner_run.contains("device_backend=cpu-reference"));
    assert!(miner_run.contains("p2p_runtime=libp2p"));
    assert!(miner_run.contains("p2p_identity_seeded=true"));
    assert!(miner_run.contains("role_runtime_ready=true"));

    let validator_register =
        execute_reference_cli_command(&ExpectedCommand::ValidatorRegister { stake: 10_000 })
            .unwrap();
    assert!(validator_register.contains("command=validator_register"));
    assert!(validator_register.contains("min_stake=10000"));

    let validator_start = execute_reference_cli_command(&ExpectedCommand::ValidatorStart {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(validator_start.contains("command=validator_start"));
    assert!(validator_start.contains("reference_verifier_ready=true"));

    let validator_run = execute_reference_cli_command(&ExpectedCommand::ValidatorRun {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: None,
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(validator_run.contains("command=validator_run"));
    assert!(validator_run.contains("role=validator"));
    assert!(validator_run.contains("reference_verifier_ready=true"));
    assert!(validator_run.contains("p2p_runtime=libp2p"));
    assert!(validator_run.contains("p2p_identity_seeded=false"));
    assert!(validator_run.contains("role_runtime_ready=true"));

    let proposer_run = execute_reference_cli_command(&ExpectedCommand::ProposerRun {
        wallet: "proposer.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x33; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(proposer_run.contains("command=proposer_run"));
    assert!(proposer_run.contains("role=proposer"));
    assert!(proposer_run.contains("proposer_ready=true"));
    assert!(proposer_run.contains("p2p_runtime=libp2p"));
    assert!(proposer_run.contains("p2p_identity_seeded=true"));
    assert!(proposer_run.contains("role_runtime_ready=true"));

    let miner_status = execute_reference_cli_command(&ExpectedCommand::MinerStatus).unwrap();
    assert!(miner_status.contains("command=miner_status"));
    assert!(miner_status.contains("status_source=rpc_or_node_store_required"));

    let validator_status =
        execute_reference_cli_command(&ExpectedCommand::ValidatorStatus).unwrap();
    assert!(validator_status.contains("command=validator_status"));
    assert!(validator_status.contains("status_source=rpc_or_node_store_required"));

    let service_init = execute_reference_cli_command(&ExpectedCommand::ServiceInit {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_init.contains("command=service_init"));
    assert!(service_init.contains("node_store_ready=true"));

    let bootstrap_peer = PeerId::random().to_string();
    let service_peer_add = execute_reference_cli_command(&ExpectedCommand::ServicePeerAdd {
        data_dir: "/var/lib/tensorvm".to_owned(),
        peer_id: bootstrap_peer.clone(),
        address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(service_peer_add.contains("command=service_peer_add"));
    assert!(service_peer_add.contains(&format!("peer_id={bootstrap_peer}")));
    assert!(service_peer_add.contains("peer_book_ready=true"));

    let service_readiness = execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
    })
    .unwrap();
    assert!(service_readiness.contains("command=service_readiness"));
    assert!(service_readiness.contains("p2p_runtime=libp2p"));
    assert!(service_readiness.contains("p2p_gossipsub=enabled"));
    assert!(service_readiness.contains("p2p_identify=enabled"));
    assert!(service_readiness.contains("p2p_kademlia=enabled"));
    assert!(service_readiness.contains("p2p_request_response=enabled"));
    assert!(service_readiness.contains("p2p_identity_seeded=true"));
    assert!(service_readiness.contains(&format!("p2p_identity_seed={}", "11".repeat(32))));
    assert!(service_readiness.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_readiness.contains("p2p_request_timeout_seconds=10"));
    assert!(service_readiness.contains("p2p_max_concurrent_streams=128"));
    assert!(service_readiness.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_readiness.contains("node_store_required=true"));
    assert!(service_readiness.contains("libp2p_ready=true"));

    let unseeded_service_readiness =
        execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
        })
        .unwrap();
    assert!(unseeded_service_readiness.contains("p2p_identity_seeded=false"));

    let service_serve = execute_reference_cli_command(&ExpectedCommand::ServiceServe {
        listen: "0.0.0.0:8545".to_owned(),
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x22; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 0,
    })
    .unwrap();
    assert!(service_serve.contains("command=service_serve"));
    assert!(service_serve.contains("p2p_runtime=libp2p"));
    assert!(service_serve.contains("p2p_gossipsub=enabled"));
    assert!(service_serve.contains("p2p_identify=enabled"));
    assert!(service_serve.contains("p2p_kademlia=enabled"));
    assert!(service_serve.contains("p2p_request_response=enabled"));
    assert!(service_serve.contains("p2p_identity_seeded=true"));
    assert!(service_serve.contains(&format!("p2p_identity_seed={}", "22".repeat(32))));
    assert!(service_serve.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_serve.contains("p2p_request_timeout_seconds=10"));
    assert!(service_serve.contains("p2p_max_concurrent_streams=128"));
    assert!(service_serve.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_serve.contains("auth_enabled=true"));
    assert!(service_serve.contains("rpc_routes=enabled"));
    assert!(service_serve.contains("explorer_routes=enabled"));
    assert!(service_serve.contains("faucet_routes=enabled"));
    assert!(service_serve.contains("telemetry_routes=enabled"));
    assert!(service_serve.contains("node_store_required=true"));

    let service_status = execute_reference_cli_command(&ExpectedCommand::ServiceStatus {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_status.contains("command=service_status"));
    assert!(service_status.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_status.contains("status_source=node_store"));

    let service_block = execute_reference_cli_command(&ExpectedCommand::ServiceBlock {
        data_dir: "/var/lib/tensorvm".to_owned(),
        height: 3,
    })
    .unwrap();
    assert!(service_block.contains("command=service_block"));
    assert!(service_block.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_block.contains("height=3"));
    assert!(service_block.contains("status_source=node_store"));

    let local_seed = execute_reference_cli_command(&ExpectedCommand::LocalTestnetSeed {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(local_seed.contains("command=local_testnet_seed"));
    assert!(local_seed.contains("data_dir=/var/lib/tensorvm"));
    assert!(local_seed.contains("local_cpu_seed_ready=true"));

    let public_command = ExpectedCommand::PublicEvidenceValidate {
        manifest: "evidence.txt".to_owned(),
    };
    assert_eq!(
        execute_reference_cli_command(&public_command).unwrap(),
        describe_command(&public_command)
    );

    let publication = execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
        manifest_signer: address(b"public-evidence-publisher"),
        manifest_signature_count: 1,
        independent_auditor_count: 1,
    })
    .unwrap();
    assert!(publication.contains(&format!(
        "bundle_id={}",
        manifest_hash(b"public-evidence-bundle")
    )));
    assert!(publication.contains("public_uri=https://tensorvm.net/tensorvm/public-evidence.json"));
    assert!(publication.contains(&format!(
        "manifest_signer={}",
        manifest_address(b"public-evidence-publisher")
    )));
    assert!(publication.contains(&format!(
        "manifest_signature={}",
        manifest_publication_signature()
    )));
    assert!(publication.contains("manifest_signature_count=1"));
    assert!(publication.contains("independent_auditor_count=1"));

    let auditor_record =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_060,
        })
        .unwrap();
    assert_eq!(
        auditor_record,
        format!(
            "auditor={},{},1700000060,{}",
            manifest_address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            manifest_auditor_signature()
        )
    );

    let run_window = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
    })
    .unwrap();
    assert_eq!(
        run_window,
        format!(
            "run_started_at_unix_seconds=1700000000\nrun_ended_at_unix_seconds=1700000060\nrun_window_signature={}\nobserved_blocks=10",
            hex(&manifest_bundle().run_window_signature)
        )
    );
    let run_window_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-run-window-{}.records",
        std::process::id()
    ));
    let run_window_observations = (0..10)
        .map(|block| {
            let timestamp = if block == 9 {
                1_700_000_060
            } else {
                1_700_000_000 + block * 6
            };
            format!("run_window_observation={block},{timestamp}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&run_window_observation_file, run_window_observations).unwrap();
    let run_window_from_file =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: run_window_observation_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&run_window_observation_file).unwrap();
    assert_eq!(run_window_from_file, run_window);

    let node_cases = [
        (
            PublicNodeRole::Miner,
            b"miner-a".as_slice(),
            b"miner-a-operator".as_slice(),
            "miner",
        ),
        (
            PublicNodeRole::Validator,
            b"validator-a".as_slice(),
            b"validator-a-operator".as_slice(),
            "validator",
        ),
    ];
    for (role, address_label, operator_label, tag) in node_cases {
        let node = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role,
            address: address(address_label),
            operator_id: hash_bytes(b"test", &[operator_label]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .unwrap();
        assert!(node.starts_with(&format!(
            "node={tag},{},{}",
            hex(&address(address_label)),
            hex(&hash_bytes(b"test", &[operator_label]))
        )));
        assert!(node.ends_with(&manifest_node_signature(
            role,
            address_label,
            operator_label
        )));
        let heartbeat_file = std::env::temp_dir().join(format!(
            "tensor-vm-node-heartbeat-{}-{}.records",
            std::process::id(),
            tag
        ));
        let heartbeat_records = (0..10)
            .map(|block| {
                format!(
                    "node_heartbeat_observation={tag},{},{},{}",
                    hex(&address(address_label)),
                    hex(&hash_bytes(b"test", &[operator_label])),
                    block
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&heartbeat_file, heartbeat_records).unwrap();
        let node_from_file =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeatFromFile {
                role,
                address: address(address_label),
                operator_id: hash_bytes(b"test", &[operator_label]),
                heartbeat_file: heartbeat_file.to_string_lossy().into_owned(),
            })
            .unwrap();
        std::fs::remove_file(&heartbeat_file).unwrap();
        assert_eq!(node_from_file, node);
    }

    let operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
    let operator_identity_uri = manifest_operator_identity_uri(&operator_id);
    let operator_attestation =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id,
            identity_uri: operator_identity_uri.clone(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .unwrap();
    assert_eq!(
        operator_attestation,
        format!(
            "operator=miner,{},{},{operator_identity_uri},1700000000,{}",
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator")
        )
    );

    let service_health =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .unwrap();
    assert!(service_health.starts_with("service=rpc,"));
    assert!(service_health.contains("https://rpc.tensorvm.net/health,/health,0,9,10,10"));
    assert!(service_health.ends_with(&manifest_service_signature(
        PublicServiceKind::Rpc,
        b"rpc-service"
    )));
    let health_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-health-{}-{}.records",
        std::process::id(),
        manifest_hash(b"rpc-service").as_bytes()[0]
    ));
    let health_observations = (0..10)
        .map(|block| format!("service_health_observation={block},reachable"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&health_observation_file, health_observations).unwrap();
    let service_health_from_file =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: health_observation_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&health_observation_file).unwrap();
    assert_eq!(service_health_from_file, service_health);
    let additional_service_cases: [(PublicServiceKind, &[u8], &str); 3] = [
        (PublicServiceKind::Explorer, b"explorer-service", "explorer"),
        (PublicServiceKind::Faucet, b"faucet-service", "faucet"),
        (
            PublicServiceKind::Telemetry,
            b"telemetry-service",
            "telemetry",
        ),
    ];
    for (kind, label, tag) in additional_service_cases {
        let line = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
            kind,
            endpoint_id: hash_bytes(b"test", &[label]),
            public_url: public_service_url(kind).to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .unwrap();
        assert!(line.starts_with(&format!("service={tag},")));
        assert!(line.contains(public_service_url(kind)));
        assert!(line.ends_with(&manifest_service_signature(kind, label)));
    }

    let service_content =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .unwrap();
    assert!(service_content.starts_with("service_content=rpc,"));
    assert!(service_content.contains("https://rpc.tensorvm.net/chain/head,/chain/head"));
    assert_eq!(
        service_content,
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service")
    );
    let observed_content = vec![7_u8; 80];
    let observed_content_root = public_service_content_root(&observed_content);
    let service_content_from_bytes =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex: hex(&observed_content),
        })
        .unwrap();
    assert!(service_content_from_bytes.starts_with("service_content=rpc,"));
    assert!(
        service_content_from_bytes
            .contains(&format!("{},1700000000,80,", hex(&observed_content_root)))
    );
    let content_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-content-{}-{}.body",
        std::process::id(),
        observed_content_root[0]
    ));
    std::fs::write(&content_file, &observed_content).unwrap();
    let service_content_from_file =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_file: content_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&content_file).unwrap();
    assert_eq!(service_content_from_file, service_content_from_bytes);

    let peer_id = PeerId::random().to_string();
    let network_observation =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNetworkObservation {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            peer_id: peer_id.clone(),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            gossip_topic_count: 5,
            request_response_protocol_count: 4,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        })
        .unwrap();
    let observation_input = NetworkObservationEvidenceLine {
        operator_id: hash_bytes(b"test", &[b"network-operator"]),
        peer_id: &peer_id,
        listen_address: "/dns/node-a.tensorvm.net/tcp/4001",
        observed_at_unix_seconds: 1_700_000_000,
        gossip_topic_count: 5,
        request_response_protocol_count: 4,
        bootstrap_peer_count: 2,
        max_transmit_bytes: 1_048_576,
        request_timeout_seconds: 10,
        max_concurrent_streams: 128,
        idle_connection_timeout_seconds: 60,
    };
    let observation_root = network_observation_root(
        &observation_input,
        &peer_id,
        "/dns/node-a.tensorvm.net/tcp/4001",
    );
    let observation_signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&observation_input.operator_id, &observation_root],
    );
    assert_eq!(
        network_observation,
        format!(
            "network_runtime_observation={},{peer_id},/dns/node-a.tensorvm.net/tcp/4001,1700000000,5,4,2,1048576,10,128,60,{},{}",
            hex(&observation_input.operator_id),
            hex(&observation_root),
            hex(&observation_signature)
        )
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation,
        )
        .unwrap(),
        observation_root
    );
    let network_observation_bad_peer =
        network_observation.replace(&format!(",{peer_id},"), ",not-a-peer,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_bad_peer,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid libp2p peer id"
    );
    let network_observation_bad_multiaddr =
        network_observation.replace(",/dns/node-a.tensorvm.net/tcp/4001,", ",not-a-multiaddr,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_bad_multiaddr,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid libp2p multiaddr"
    );
    let network_observation_local_multiaddr = network_observation.replace(
        ",/dns/node-a.tensorvm.net/tcp/4001,",
        ",/ip4/127.0.0.1/tcp/4001,",
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_local_multiaddr,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: network observation address is not public"
    );
    let network_observation_whitespace_field =
        network_observation.replace(&format!(",{peer_id},"), &format!(", {peer_id},"));
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_whitespace_field,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_zero_operator =
        network_observation.replace(&hex(&observation_input.operator_id), &"00".repeat(32));
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_zero_operator,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: operator id argument is empty"
    );
    let network_observation_zero_count =
        network_observation.replace(",1700000000,5,4,2,", ",1700000000,0,4,2,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_zero_count,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_tampered_root = network_observation.replace(
        &hex(&observation_root),
        &hex(&hash_bytes(b"test", &[b"tampered-network-root"])),
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_tampered_root,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_tampered_signature = network_observation.replace(
        &hex(&observation_signature),
        &hex(&hash_bytes(b"test", &[b"tampered-network-signature"])),
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_tampered_signature,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let service_log = format!(
        "\
command=service_serve
p2p_runtime=libp2p
p2p_peer_id={peer_id}
p2p_gossipsub_topics=5
p2p_request_response_protocols=4
p2p_bootstrap_peers=2
p2p_max_transmit_bytes=1048576
p2p_request_timeout_seconds=10
p2p_max_concurrent_streams=128
p2p_idle_timeout_seconds=60
"
    );
    assert_eq!(
        service_log_field(&service_log, "p2p_peer_id").unwrap(),
        peer_id
    );
    let network_observation_from_service_log = network_observation_evidence_line_from_service_log(
        hash_bytes(b"test", &[b"network-operator"]),
        "/dns/node-a.tensorvm.net/tcp/4001",
        1_700_000_000,
        &service_log,
    )
    .unwrap();
    assert_eq!(network_observation_from_service_log, network_observation);

    let service_log_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-log-{}-{}.log",
        std::process::id(),
        observation_root[0]
    ));
    std::fs::write(&service_log_file, &service_log).unwrap();
    let network_observation_from_file = execute_reference_cli_command(
        &ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            service_log: service_log_file.to_string_lossy().into_owned(),
        },
    )
    .unwrap();
    std::fs::remove_file(&service_log_file).unwrap();
    assert_eq!(network_observation_from_file, network_observation);

    assert_eq!(
        execute_reference_cli_command(
            &ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                service_log: service_log_file.to_string_lossy().into_owned(),
            }
        )
        .unwrap_err()
        .to_string(),
        "storage error: failed to read service log file"
    );
    assert_eq!(
        network_observation_evidence_line_from_service_log(
            hash_bytes(b"test", &[b"network-operator"]),
            "/dns/node-a.tensorvm.net/tcp/4001",
            1_700_000_000,
            "command=service_init\np2p_runtime=libp2p\n",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: service log is not service_serve"
    );
    assert_eq!(
        network_observation_evidence_line_from_service_log(
            hash_bytes(b"test", &[b"network-operator"]),
            "/dns/node-a.tensorvm.net/tcp/4001",
            1_700_000_000,
            "command=service_serve\np2p_runtime=disabled\n",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: service log does not prove libp2p runtime"
    );
    assert_eq!(
        service_log_field("command=service_serve\n", "p2p_peer_id")
            .unwrap_err()
            .to_string(),
        "invalid receipt: missing service log field"
    );
    assert_eq!(
        service_log_field("p2p_runtime=libp2p\np2p_runtime=libp2p\n", "p2p_runtime")
            .unwrap_err()
            .to_string(),
        "invalid receipt: duplicate service log field"
    );
    assert_eq!(
        service_log_field("p2p_runtime= libp2p\n", "p2p_runtime")
            .unwrap_err()
            .to_string(),
        "invalid receipt: invalid service log field"
    );

    let record_cases: [(PublicEvidenceRecordKind, &[u8], u64, &str, String); 6] = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10,
            "block_history",
            hex(&manifest_bundle().block_history_signature),
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10,
            "finality_history",
            hex(&manifest_bundle().finality_history_signature),
        ),
        (
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            b"network-runtime-root",
            3,
            "network_runtime_observation",
            hex(&manifest_bundle().network_runtime_observation_signature),
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20,
            "data_availability_measurement",
            hex(&manifest_bundle().data_availability_measurement_signature),
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1,
            "invalid_work_rejection",
            hex(&manifest_bundle().invalid_work_rejection_signature),
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1,
            "reward_settlement",
            hex(&manifest_bundle().reward_settlement_signature),
        ),
    ];
    for (kind, root_label, count, field_prefix, expected_signature) in record_cases {
        let record_root = if matches!(kind, PublicEvidenceRecordKind::NetworkRuntimeObservations) {
            manifest_bundle().network_runtime_observation_root
        } else {
            hash_bytes(b"test", &[root_label])
        };
        let root = hex(&record_root);
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let manifest_signer = address(b"public-evidence-publisher");
        let line = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind,
            bundle_id,
            manifest_signer,
            record_root,
            record_count: count,
        })
        .unwrap();
        assert_eq!(
            line,
            format!(
                "{field_prefix}_records={count}\n{field_prefix}_root={root}\n{field_prefix}_signature={expected_signature}"
            )
        );

        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}/{}.json",
            manifest_hash(b"public-evidence-bundle"),
            public_evidence_record_kind_tag(kind)
        );
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &manifest_signer,
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            count,
        );
        let artifact_line =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri: artifact_uri.clone(),
                record_root,
                record_count: count,
            })
            .unwrap();
        assert_eq!(
            artifact_line,
            format!(
                "record_artifact={},{artifact_uri},{root},{count},{}",
                public_evidence_record_kind_tag(kind),
                hex(&artifact_signature)
            )
        );
    }

    let roots = vec![
        hash_bytes(b"test", &[b"network-observation-a"]),
        hash_bytes(b"test", &[b"network-observation-b"]),
    ];
    let aggregate_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &roots,
    )
    .unwrap();
    let aggregate_signature = sign_public_evidence_record(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &aggregate_root,
        roots.len() as u64,
    );
    let aggregate_line =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: roots.clone(),
        })
        .unwrap();
    assert_eq!(
        aggregate_line,
        format!(
            "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
            hex(&aggregate_root),
            hex(&aggregate_signature)
        )
    );
    let aggregate_artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
    let aggregate_artifact_signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        aggregate_artifact_uri,
        &aggregate_root,
        roots.len() as u64,
    );
    let aggregate_artifact_line =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_roots: roots,
        })
        .unwrap();
    assert_eq!(
        aggregate_artifact_line,
        format!(
            "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
            hex(&aggregate_root),
            hex(&aggregate_artifact_signature)
        )
    );

    let record_file_roots = vec![
        observation_root,
        hash_bytes(b"test", &[b"network-observation-b"]),
    ];
    let record_file_aggregate_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_roots,
    )
    .unwrap();
    let record_file = std::env::temp_dir().join(format!(
        "tensor-vm-network-records-{}-{}.records",
        std::process::id(),
        record_file_aggregate_root[0]
    ));
    std::fs::write(
        &record_file,
        format!(
            "# captured network-runtime records\n\n{network_observation}\nrecord_root={}\n",
            hex(&record_file_roots[1])
        ),
    )
    .unwrap();
    let record_file_path = record_file.to_string_lossy().into_owned();
    let record_file_roots_from_disk = public_evidence_record_roots_from_file(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_path,
    )
    .unwrap();
    assert_eq!(record_file_roots_from_disk, record_file_roots);
    let record_file_summary =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: record_file_path.clone(),
        })
        .unwrap();
    let record_file_signature = sign_public_evidence_record(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_aggregate_root,
        record_file_roots.len() as u64,
    );
    assert_eq!(
        record_file_summary,
        format!(
            "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
            hex(&record_file_aggregate_root),
            hex(&record_file_signature)
        )
    );
    let record_file_artifact =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_file: record_file_path.clone(),
        })
        .unwrap();
    let record_file_artifact_signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        aggregate_artifact_uri,
        &record_file_aggregate_root,
        record_file_roots.len() as u64,
    );
    assert_eq!(
        record_file_artifact,
        format!(
            "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
            hex(&record_file_aggregate_root),
            hex(&record_file_artifact_signature)
        )
    );
    std::fs::remove_file(&record_file).unwrap();
    assert_eq!(
        supporting_record_line_prefix(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        None
    );

    let supporting_record_cases = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "block_history",
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,finalized",
            "finality_history",
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,available,0",
            "data_availability_measurement",
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,rejected,0",
            "invalid_work_rejection",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,0"
            ),
            "reward_settlement",
        ),
    ];
    for (kind, raw_line, field_prefix) in supporting_record_cases {
        let raw_root = supporting_record_root_from_line(
            kind,
            raw_line,
            supporting_record_line_prefix(kind).unwrap(),
        )
        .unwrap();
        assert_eq!(
            public_evidence_record_root_from_line(kind, raw_line).unwrap(),
            raw_root
        );
        let extra_root = hash_bytes(b"test", &[public_evidence_record_kind_tag(kind).as_bytes()]);
        let roots = vec![raw_root, extra_root];
        let aggregate_root = aggregate_public_evidence_record_roots(kind, &roots).unwrap();
        let raw_record_file = std::env::temp_dir().join(format!(
            "tensor-vm-{}-records-{}-{}.records",
            public_evidence_record_kind_tag(kind),
            std::process::id(),
            aggregate_root[0]
        ));
        std::fs::write(
            &raw_record_file,
            format!(
                "# raw supporting records\n{raw_line}\nrecord_root={}\n",
                hex(&extra_root)
            ),
        )
        .unwrap();
        let raw_record_file_path = raw_record_file.to_string_lossy().into_owned();
        assert_eq!(
            public_evidence_record_roots_from_file(kind, &raw_record_file_path).unwrap(),
            roots
        );
        let summary =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
                kind,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_file: raw_record_file_path.clone(),
            })
            .unwrap();
        let signature = sign_public_evidence_record(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            summary,
            format!(
                "{field_prefix}_records=2\n{field_prefix}_root={}\n{field_prefix}_signature={}",
                hex(&aggregate_root),
                hex(&signature)
            )
        );
        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}.json",
            public_evidence_record_kind_tag(kind)
        );
        let artifact =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
                kind,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: artifact_uri.clone(),
                record_file: raw_record_file_path,
            })
            .unwrap();
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &artifact_uri,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            artifact,
            format!(
                "record_artifact={},{},{},2,{}",
                public_evidence_record_kind_tag(kind),
                artifact_uri,
                hex(&aggregate_root),
                hex(&artifact_signature)
            )
        );
        std::fs::remove_file(&raw_record_file).unwrap();
    }
    let malformed_supporting_record_cases = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0",
        ),
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0,not-a-root",
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,pending",
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,missing,0",
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,accepted,0",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            "reward_settlement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,miner,,0",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "miner,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,0"
            ),
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "validator,0"
            ),
        ),
    ];
    for (kind, raw_line) in malformed_supporting_record_cases {
        assert!(matches!(
            public_evidence_record_root_from_line(kind, raw_line),
            Err(TvmError::InvalidReceipt(_))
        ));
    }
    assert!(matches!(
        validate_supporting_record_payload(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ),
        Err(TvmError::InvalidReceipt(_))
    ));
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_file_path,
        )
        .unwrap_err()
        .to_string(),
        "storage error: failed to read public evidence record file"
    );
    let empty_record_file = std::env::temp_dir().join(format!(
        "tensor-vm-empty-records-{}-{}.records",
        std::process::id(),
        record_file_aggregate_root[1]
    ));
    std::fs::write(&empty_record_file, "# no roots yet\n\n").unwrap();
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &empty_record_file.to_string_lossy(),
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: record file has no roots"
    );
    std::fs::remove_file(&empty_record_file).unwrap();
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::FinalityHistory,
            "network_runtime_observation=bad",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: unsupported public evidence record line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::BlockHistory,
            &format!(
                "record_root= {}",
                hex(&hash_bytes(b"test", &[b"bad-whitespace"]))
            ),
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid record root file line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            "network_runtime_observation=bad",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record= ",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid public evidence supporting record line"
    );
    let whitespace_record_file = std::env::temp_dir().join(format!(
        "tensor-vm-whitespace-record-{}.records",
        std::process::id()
    ));
    std::fs::write(&whitespace_record_file, " block_history_record=0\n").unwrap();
    let whitespace_record_path = whitespace_record_file.to_string_lossy().into_owned();
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::BlockHistory,
            &whitespace_record_path,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: public evidence record line has leading or trailing whitespace"
    );
    std::fs::remove_file(&whitespace_record_file).unwrap();
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
