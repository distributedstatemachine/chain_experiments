use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct BlockHeightWindowArgs {
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the evidence window."
    )]
    pub first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the evidence window."
    )]
    pub last_block: u64,
}

impl BlockHeightWindowArgs {
    pub fn first_block(&self) -> u64 {
        self.first_block
    }

    pub fn last_block(&self) -> u64 {
        self.last_block
    }
}
