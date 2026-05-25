use super::value_types::HashArg;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct EvidenceBundleIdArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
}

impl EvidenceBundleIdArgs {
    pub fn id(&self) -> Hash {
        self.bundle_id.into_hash()
    }
}
