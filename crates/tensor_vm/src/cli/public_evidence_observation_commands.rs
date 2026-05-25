use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ObservationTimestampArgs {
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    pub observed_at: u64,
}

impl ObservationTimestampArgs {
    pub fn observed_at(&self) -> u64 {
        self.observed_at
    }
}
