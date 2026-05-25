use super::public_evidence_block_window_commands::BlockHeightWindowArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::value_types::{HashArg, HexBytesArg};
use crate::testnet::PublicServiceKind;
use crate::types::Hash;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::{Path, PathBuf};

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
    #[command(flatten)]
    endpoint: PublicServiceEndpointArgs,
    #[command(flatten)]
    health: ServiceHealthPathArgs,
    #[command(flatten)]
    window: BlockHeightWindowArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Successful public reachability observations."
    )]
    reachable_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Signed health checks included in the evidence."
    )]
    signed_health_check_count: u64,
}

impl ServiceHealthArgs {
    pub fn new(
        endpoint: PublicServiceEndpointArgs,
        health: ServiceHealthPathArgs,
        window: BlockHeightWindowArgs,
        reachable_count: u64,
        signed_health_check_count: u64,
    ) -> Self {
        Self {
            endpoint,
            health,
            window,
            reachable_count,
            signed_health_check_count,
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.endpoint.kind()
    }

    pub fn endpoint_id(&self) -> Hash {
        self.endpoint.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.endpoint.public_url()
    }

    pub fn health_path(&self) -> &str {
        self.health.path()
    }

    pub fn first_seen_block(&self) -> u64 {
        self.window.first_block()
    }

    pub fn last_seen_block(&self) -> u64 {
        self.window.last_block()
    }

    pub fn reachable_count(&self) -> u64 {
        self.reachable_count
    }

    pub fn signed_health_check_count(&self) -> u64 {
        self.signed_health_check_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthFromFileArgs {
    #[command(flatten)]
    endpoint: PublicServiceEndpointArgs,
    #[command(flatten)]
    health: ServiceHealthPathArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured health-observation record file."
    )]
    observation_file: PathBuf,
}

impl ServiceHealthFromFileArgs {
    pub fn new(
        endpoint: PublicServiceEndpointArgs,
        health: ServiceHealthPathArgs,
        observation_file: PathBuf,
    ) -> Self {
        Self {
            endpoint,
            health,
            observation_file,
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.endpoint.kind()
    }

    pub fn endpoint_id(&self) -> Hash {
        self.endpoint.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.endpoint.public_url()
    }

    pub fn health_path(&self) -> &str {
        self.health.path()
    }

    pub fn observation_file(&self) -> &Path {
        &self.observation_file
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthPathArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "Health-check path observed on the public service."
    )]
    health_path: String,
}

impl ServiceHealthPathArgs {
    pub fn new(health_path: impl Into<String>) -> Self {
        Self {
            health_path: health_path.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.health_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentArgs {
    #[command(flatten)]
    target: ServiceContentTargetArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Merkle root or content hash committed by the observation."
    )]
    content_root: HashArg,
    #[arg(
        long,
        value_name = "BYTES",
        help = "Minimum byte length accepted for the observed content."
    )]
    min_content_bytes: u64,
}

impl ServiceContentArgs {
    pub fn new(
        target: ServiceContentTargetArgs,
        content_root: HashArg,
        min_content_bytes: u64,
    ) -> Self {
        Self {
            target,
            content_root,
            min_content_bytes,
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.target.kind()
    }

    pub fn endpoint_id(&self) -> Hash {
        self.target.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.target.public_url()
    }

    pub fn content_path(&self) -> &str {
        self.target.content_path()
    }

    pub fn content_root(&self) -> Hash {
        self.content_root.into_hash()
    }

    pub fn observed_at(&self) -> u64 {
        self.target.observed_at()
    }

    pub fn min_content_bytes(&self) -> u64 {
        self.min_content_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromBytesArgs {
    #[command(flatten)]
    target: ServiceContentTargetArgs,
    #[arg(
        long = "content-hex",
        value_name = "HEX",
        help = "Observed response bytes encoded as hex."
    )]
    content: HexBytesArg,
}

impl ServiceContentFromBytesArgs {
    pub fn new(target: ServiceContentTargetArgs, content: HexBytesArg) -> Self {
        Self { target, content }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.target.kind()
    }

    pub fn endpoint_id(&self) -> Hash {
        self.target.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.target.public_url()
    }

    pub fn content_path(&self) -> &str {
        self.target.content_path()
    }

    pub fn observed_at(&self) -> u64 {
        self.target.observed_at()
    }

    pub fn content(&self) -> &[u8] {
        self.content.as_slice()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromFileArgs {
    #[command(flatten)]
    target: ServiceContentTargetArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing observed response bytes."
    )]
    content_file: PathBuf,
}

impl ServiceContentFromFileArgs {
    pub fn new(target: ServiceContentTargetArgs, content_file: PathBuf) -> Self {
        Self {
            target,
            content_file,
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.target.kind()
    }

    pub fn endpoint_id(&self) -> Hash {
        self.target.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.target.public_url()
    }

    pub fn content_path(&self) -> &str {
        self.target.content_path()
    }

    pub fn observed_at(&self) -> u64 {
        self.target.observed_at()
    }

    pub fn content_file(&self) -> &Path {
        &self.content_file
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentTargetArgs {
    #[command(flatten)]
    endpoint: PublicServiceEndpointArgs,
    #[arg(
        long,
        value_name = "PATH",
        help = "Content path observed on the public service."
    )]
    content_path: String,
    #[command(flatten)]
    observation: ObservationTimestampArgs,
}

impl ServiceContentTargetArgs {
    pub fn new(
        endpoint: PublicServiceEndpointArgs,
        content_path: impl Into<String>,
        observation: ObservationTimestampArgs,
    ) -> Self {
        Self {
            endpoint,
            content_path: content_path.into(),
            observation,
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.endpoint.kind()
    }

    pub fn endpoint_id(&self) -> crate::types::Hash {
        self.endpoint.endpoint_id()
    }

    pub fn public_url(&self) -> &str {
        self.endpoint.public_url()
    }

    pub fn content_path(&self) -> &str {
        &self.content_path
    }

    pub fn observed_at(&self) -> u64 {
        self.observation.observed_at()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicServiceEndpointArgs {
    #[arg(long, help = "Public service being observed.")]
    kind: PublicServiceKindArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Stable 32-byte service endpoint identifier."
    )]
    endpoint_id: HashArg,
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        help = "Public URL for the service endpoint."
    )]
    public_url: String,
}

impl PublicServiceEndpointArgs {
    pub fn new(
        kind: PublicServiceKindArg,
        endpoint_id: HashArg,
        public_url: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            endpoint_id,
            public_url: public_url.into(),
        }
    }

    pub fn kind(&self) -> PublicServiceKind {
        self.kind.into()
    }

    pub fn endpoint_id(&self) -> crate::types::Hash {
        self.endpoint_id.into_hash()
    }

    pub fn public_url(&self) -> &str {
        &self.public_url
    }
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
