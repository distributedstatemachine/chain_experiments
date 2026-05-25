use super::public_evidence_block_window_commands::BlockHeightWindowArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::value_types::{HashArg, HexBytesArg};
use crate::testnet::PublicServiceKind;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceServiceCommand {
    #[command(about = "Generate service health evidence.")]
    Health(ServiceHealthArgs),
    #[command(about = "Generate service health evidence from captured observations.")]
    HealthFile(ServiceHealthFromFileArgs),
    #[command(about = "Generate service content evidence from a known content root.")]
    Content(ServiceContentArgs),
    #[command(about = "Generate service content evidence from observed bytes.")]
    ContentBytes(ServiceContentFromBytesArgs),
    #[command(about = "Generate service content evidence from a captured file.")]
    ContentFile(ServiceContentFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceHealthArgs {
    #[command(flatten)]
    pub(crate) endpoint: PublicServiceEndpointArgs,
    #[command(flatten)]
    pub(crate) health: ServiceHealthPathArgs,
    #[command(flatten)]
    pub(crate) window: BlockHeightWindowArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Successful public reachability observations."
    )]
    pub(crate) reachable_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Signed health checks included in the evidence."
    )]
    pub(crate) signed_health_check_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceHealthFromFileArgs {
    #[command(flatten)]
    pub(crate) endpoint: PublicServiceEndpointArgs,
    #[command(flatten)]
    pub(crate) health: ServiceHealthPathArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured health-observation record file."
    )]
    pub(crate) observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceHealthPathArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "Health-check path observed on the public service."
    )]
    pub(crate) health_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceContentArgs {
    #[command(flatten)]
    pub(crate) target: ServiceContentTargetArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Merkle root or content hash committed by the observation."
    )]
    pub(crate) content_root: HashArg,
    #[arg(
        long,
        value_name = "BYTES",
        help = "Minimum byte length accepted for the observed content."
    )]
    pub(crate) min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceContentFromBytesArgs {
    #[command(flatten)]
    pub(crate) target: ServiceContentTargetArgs,
    #[arg(
        long = "content-hex",
        value_name = "HEX",
        help = "Observed response bytes encoded as hex."
    )]
    pub(crate) content: HexBytesArg,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceContentFromFileArgs {
    #[command(flatten)]
    pub(crate) target: ServiceContentTargetArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing observed response bytes."
    )]
    pub(crate) content_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ServiceContentTargetArgs {
    #[command(flatten)]
    pub(crate) endpoint: PublicServiceEndpointArgs,
    #[arg(
        long,
        value_name = "PATH",
        help = "Content path observed on the public service."
    )]
    pub(crate) content_path: String,
    #[command(flatten)]
    pub(crate) observation: ObservationTimestampArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicServiceEndpointArgs {
    #[arg(long, help = "Public service being observed.")]
    pub(crate) kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub(crate) endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub(crate) public_url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum PublicServiceKindArg {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

impl From<PublicServiceKindArg> for PublicServiceKind {
    fn from(kind: PublicServiceKindArg) -> Self {
        match kind {
            PublicServiceKindArg::Rpc => Self::Rpc,
            PublicServiceKindArg::Explorer => Self::Explorer,
            PublicServiceKindArg::Faucet => Self::Faucet,
            PublicServiceKindArg::Telemetry => Self::Telemetry,
        }
    }
}
