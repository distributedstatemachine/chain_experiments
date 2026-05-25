use super::value_types::HashArg;
use clap::Args;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Args)]
pub(crate) struct OperatorIdArgs {
    #[arg(long, value_name = "HEX", help = "Public operator identifier.")]
    pub(crate) operator_id: HashArg,
}
