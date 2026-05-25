use super::value_types::AddressArg;
use crate::types::Address;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ManifestSignerArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
}

impl ManifestSignerArgs {
    pub fn signer(&self) -> Address {
        self.manifest_signer.into_address()
    }
}
