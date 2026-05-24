use super::value_types::{HashArg, HexBytesArg};
use crate::testnet::PublicServiceKind;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceServiceCommand {
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
pub struct ServiceHealthArgs {
    #[arg(long, help = "Public service being observed.")]
    pub kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub public_url: String,
    #[arg(
        long,
        value_name = "PATH",
        help = "Health-check path observed on the public service."
    )]
    pub health_path: String,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the observation window."
    )]
    pub first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the observation window."
    )]
    pub last_block: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Successful public reachability observations."
    )]
    pub reachable_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Signed health checks included in the evidence."
    )]
    pub signed_health_check_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthFromFileArgs {
    #[arg(long, help = "Public service being observed.")]
    pub kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub public_url: String,
    #[arg(
        long,
        value_name = "PATH",
        help = "Health-check path observed on the public service."
    )]
    pub health_path: String,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured health-observation record file."
    )]
    pub observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentArgs {
    #[arg(long, help = "Public service being observed.")]
    pub kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub public_url: String,
    #[arg(
        long,
        value_name = "PATH",
        help = "Content path observed on the public service."
    )]
    pub content_path: String,
    #[arg(
        long,
        value_name = "HEX",
        help = "Merkle root or content hash committed by the observation."
    )]
    pub content_root: HashArg,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    pub observed_at: u64,
    #[arg(
        long,
        value_name = "BYTES",
        help = "Minimum byte length accepted for the observed content."
    )]
    pub min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromBytesArgs {
    #[arg(long, help = "Public service being observed.")]
    pub kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub public_url: String,
    #[arg(
        long,
        value_name = "PATH",
        help = "Content path observed on the public service."
    )]
    pub content_path: String,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    pub observed_at: u64,
    #[arg(
        long = "content-hex",
        value_name = "HEX",
        help = "Observed response bytes encoded as hex."
    )]
    pub content: HexBytesArg,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromFileArgs {
    #[arg(long, help = "Public service being observed.")]
    pub kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    pub endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    pub public_url: String,
    #[arg(
        long,
        value_name = "PATH",
        help = "Content path observed on the public service."
    )]
    pub content_path: String,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    pub observed_at: u64,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing observed response bytes."
    )]
    pub content_file: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PublicServiceKindArg {
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
