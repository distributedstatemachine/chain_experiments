use super::value_types::HashArg;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Args)]
pub struct OperatorIdArgs {
    #[arg(long, value_name = "HEX", help = "Public operator identifier.")]
    operator_id: HashArg,
}

impl OperatorIdArgs {
    pub fn new(operator_id: Hash) -> Self {
        Self {
            operator_id: HashArg::new(operator_id),
        }
    }

    pub fn id(&self) -> Hash {
        self.operator_id.into_hash()
    }
}
