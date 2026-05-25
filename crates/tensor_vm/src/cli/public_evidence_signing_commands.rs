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
    manifest_signer: AddressArg,
}

impl ManifestSignerArgs {
    #[cfg(test)]
    pub(crate) fn new(manifest_signer: Address) -> Self {
        Self {
            manifest_signer: AddressArg::new(manifest_signer),
        }
    }

    pub fn signer(&self) -> Address {
        self.manifest_signer.into_address()
    }
}
