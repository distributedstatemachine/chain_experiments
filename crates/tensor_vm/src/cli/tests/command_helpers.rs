use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;

pub(super) fn parse_test_cli(
    args: &[&str],
) -> std::result::Result<super::TvmdCommand, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv).map(|cli| cli.command)
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

pub(super) fn multiaddr_arg(value: String) -> libp2p::Multiaddr {
    value.parse().expect("test multiaddr must parse")
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
