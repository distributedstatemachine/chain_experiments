use super::value_types::HashArg;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct EvidenceBundleIdArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    bundle_id: HashArg,
}

impl EvidenceBundleIdArgs {
    #[cfg(test)]
    pub(crate) fn new(bundle_id: Hash) -> Self {
        Self {
            bundle_id: HashArg::new(bundle_id),
        }
    }

    pub fn id(&self) -> Hash {
        self.bundle_id.into_hash()
    }
}
