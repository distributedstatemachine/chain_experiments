use super::value_types::HashArg;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Args)]
pub struct OperatorIdArgs {
    #[arg(long, value_name = "HEX", help = "Public operator identifier.")]
    pub operator_id: HashArg,
}

impl OperatorIdArgs {
    pub fn id(&self) -> Hash {
        self.operator_id.into_hash()
    }
}
