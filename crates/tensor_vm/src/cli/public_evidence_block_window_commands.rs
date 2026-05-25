use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct BlockHeightWindowArgs {
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the evidence window."
    )]
    pub(crate) first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the evidence window."
    )]
    pub(crate) last_block: u64,
}
