use super::value_types::AddressArg;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ManifestSignerArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub(crate) manifest_signer: AddressArg,
}
