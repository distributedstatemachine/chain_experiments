use super::value_types::HashArg;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct EvidenceBundleIdArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub(crate) bundle_id: HashArg,
}
