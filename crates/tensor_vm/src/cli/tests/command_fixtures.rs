use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;
use std::path::PathBuf;

pub(super) fn parse_test_cli(
    args: &[&str],
) -> std::result::Result<super::TvmdCommand, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv).map(|cli| cli.command)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EvidenceFixture {
    ServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    ServiceHealthFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        observation_file: String,
    },
    ServiceContent {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    },
    ServiceContentFromBytes {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_bytes: Vec<u8>,
    },
    ServiceContentFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_file: String,
    },
    RecordSummary {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_root: Hash,
        record_count: u64,
    },
    RecordArtifact {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_root: Hash,
        record_count: u64,
    },
    RecordArtifactFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_roots: Vec<Hash>,
    },
    RecordSummaryFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_roots: Vec<Hash>,
    },
}

pub(super) fn execute_evidence_fixture(command: &EvidenceFixture) -> crate::error::Result<String> {
    let cli_command = command.clone().into_cli_command();
    execute_test_cli_command(&cli_command)
}

pub(super) fn execute_test_cli_command(
    cli_command: &super::TvmdCommand,
) -> crate::error::Result<String> {
    match cli_command {
        super::TvmdCommand::Miner(_)
        | super::TvmdCommand::Validator(_)
        | super::TvmdCommand::Proposer(_)
        | super::TvmdCommand::Node(_)
        | super::TvmdCommand::Localnet(_) => {
            super::local_execution::execute_local_cli_command(cli_command)
        }
        super::TvmdCommand::Public(super::PublicCommand::Preflight(_))
        | super::TvmdCommand::Public(super::PublicCommand::Evidence(
            super::EvidenceCommand::Validate(_),
        )) => Err(crate::error::TvmError::InvalidReceipt(
            "public artifact validation reads manifests through the app dispatcher",
        )),
        super::TvmdCommand::Public(super::PublicCommand::Evidence(command)) => {
            super::execute_public_evidence_command(command)
        }
    }
}

pub(super) fn path_arg(value: String) -> PathBuf {
    value.into()
}

pub(super) fn multiaddr_arg(value: String) -> libp2p::Multiaddr {
    value.parse().expect("fixture multiaddr must parse")
}

pub(super) fn hash_arg(value: Hash) -> HashArg {
    HashArg::new(value)
}

pub(super) fn address_arg(value: Address) -> AddressArg {
    AddressArg::new(value)
}

pub(super) fn hash_args(values: Vec<Hash>) -> Vec<HashArg> {
    values.into_iter().map(HashArg::new).collect()
}

fn public_evidence_command(command: EvidenceCommand) -> super::TvmdCommand {
    super::TvmdCommand::Public(PublicCommand::Evidence(command))
}

impl EvidenceFixture {
    fn into_cli_command(self) -> super::TvmdCommand {
        match self {
            Self::ServiceHealth {
                kind,
                endpoint_id,
                public_url,
                health_path,
                first_seen_block,
                last_seen_block,
                reachable_observation_count,
                signed_health_check_count,
            } => public_evidence_command(EvidenceCommand::Service(EvidenceServiceCommand::Health(
                ServiceHealthArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    health_path,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    reachable_count: reachable_observation_count,
                    signed_health_check_count,
                },
            ))),
            Self::ServiceHealthFromFile {
                kind,
                endpoint_id,
                public_url,
                health_path,
                observation_file,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    health_path,
                    observation_file: path_arg(observation_file),
                }),
            )),
            Self::ServiceContent {
                kind,
                endpoint_id,
                public_url,
                content_path,
                content_root,
                observed_at_unix_seconds,
                min_content_bytes,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::Content(ServiceContentArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    content_root: hash_arg(content_root),
                    observed_at: observed_at_unix_seconds,
                    min_content_bytes,
                }),
            )),
            Self::ServiceContentFromBytes {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_bytes,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    observed_at: observed_at_unix_seconds,
                    content: HexBytesArg::new(content_bytes),
                }),
            )),
            Self::ServiceContentFromFile {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_file,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    observed_at: observed_at_unix_seconds,
                    content_file: path_arg(content_file),
                }),
            )),
            Self::RecordSummary {
                kind,
                bundle_id,
                manifest_signer,
                record_root,
                record_count,
            } => public_evidence_command(EvidenceCommand::Record(EvidenceRecordCommand::Summary(
                RecordSummaryArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    record_root: hash_arg(record_root),
                    record_count,
                },
            ))),
            Self::RecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_root,
                record_count,
            } => public_evidence_command(EvidenceCommand::Record(EvidenceRecordCommand::Artifact(
                RecordArtifactArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    artifact_uri,
                    record_root: hash_arg(record_root),
                    record_count,
                },
            ))),
            Self::RecordArtifactFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_roots,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    artifact_uri,
                    record_roots: hash_args(record_roots),
                }),
            )),
            Self::RecordSummaryFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                record_roots,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    record_roots: hash_args(record_roots),
                }),
            )),
        }
    }
}

pub(super) fn service_kind_arg(kind: PublicServiceKind) -> PublicServiceKindArg {
    match kind {
        PublicServiceKind::Rpc => PublicServiceKindArg::Rpc,
        PublicServiceKind::Explorer => PublicServiceKindArg::Explorer,
        PublicServiceKind::Faucet => PublicServiceKindArg::Faucet,
        PublicServiceKind::Telemetry => PublicServiceKindArg::Telemetry,
    }
}

pub(super) fn node_role_arg(role: PublicNodeRole) -> PublicNodeRoleArg {
    match role {
        PublicNodeRole::Miner => PublicNodeRoleArg::Miner,
        PublicNodeRole::Validator => PublicNodeRoleArg::Validator,
    }
}

pub(super) fn record_kind_arg(kind: PublicEvidenceRecordKind) -> PublicEvidenceRecordKindArg {
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
