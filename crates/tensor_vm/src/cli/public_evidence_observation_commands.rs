use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ObservationTimestampArgs {
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the observation."
    )]
    pub(crate) observed_at: u64,
}
