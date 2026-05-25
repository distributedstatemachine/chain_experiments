use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ObservationTimestampArgs {
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    observed_at: u64,
}

impl ObservationTimestampArgs {
    pub fn new(observed_at: u64) -> Self {
        Self { observed_at }
    }

    pub fn observed_at(&self) -> u64 {
        self.observed_at
    }
}
