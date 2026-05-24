use super::TvmdCommand;
use super::commands::TestnetCommand;
use super::local_role_descriptions::{
    describe_miner_command, describe_proposer_command, describe_validator_command,
};
use super::local_service_descriptions::describe_service_command;
use super::validation::path_argument;

pub(super) fn describe_local_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::Miner(command) => describe_miner_command(command),
        TvmdCommand::Validator(command) => describe_validator_command(command),
        TvmdCommand::Proposer(command) => describe_proposer_command(command),
        TvmdCommand::Service(command) => describe_service_command(command),
        TvmdCommand::Testnet(command) => describe_testnet_command(command),
        _ => unreachable!(
            "public evidence commands are handled by cli::public_evidence_descriptions"
        ),
    }
}

fn describe_testnet_command(command: &TestnetCommand) -> String {
    match command {
        TestnetCommand::Seed(args) => {
            format!(
                "seed local CPU testnet data_dir={}",
                path_argument(&args.data_dir)
            )
        }
        TestnetCommand::Preflight(args) => format!(
            "run public testnet preflight manifest {}",
            path_argument(&args.manifest)
        ),
        TestnetCommand::VerifyLocalCpu(args) => format!(
            "verify local CPU node evidence data_dir={} json={}",
            path_argument(&args.data_dir),
            args.json
        ),
    }
}
