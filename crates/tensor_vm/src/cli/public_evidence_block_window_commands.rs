use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct BlockHeightWindowArgs {
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the evidence window."
    )]
    first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the evidence window."
    )]
    last_block: u64,
}

impl BlockHeightWindowArgs {
    pub fn new(first_block: u64, last_block: u64) -> Self {
        Self {
            first_block,
            last_block,
        }
    }

    pub fn first_block(&self) -> u64 {
        self.first_block
    }

    pub fn last_block(&self) -> u64 {
        self.last_block
    }
}
