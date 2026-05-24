use super::TvmdCommand;
use super::commands::{LocalCpuCommand, LocalTestnetCommand};
use super::local_role_descriptions::{
    describe_miner_command, describe_proposer_command, describe_validator_command,
};
use super::local_service_descriptions::describe_service_command;
use super::validation::path_argument;

pub(super) fn describe_local_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::Miner { command } => describe_miner_command(command),
        TvmdCommand::Validator { command } => describe_validator_command(command),
        TvmdCommand::Proposer { command } => describe_proposer_command(command),
        TvmdCommand::Service { command } => describe_service_command(command),
        TvmdCommand::LocalTestnet { command } => describe_local_testnet_command(command),
        TvmdCommand::LocalCpu { command } => describe_local_cpu_command(command),
        _ => unreachable!(
            "public evidence commands are handled by cli::public_evidence_descriptions"
        ),
    }
}

fn describe_local_testnet_command(command: &LocalTestnetCommand) -> String {
    match command {
        LocalTestnetCommand::Seed(args) => {
            format!(
                "seed local CPU testnet data_dir={}",
                path_argument(&args.data_dir)
            )
        }
    }
}

fn describe_local_cpu_command(command: &LocalCpuCommand) -> String {
    match command {
        LocalCpuCommand::Verify(args) => format!(
            "verify local CPU node evidence data_dir={} json={}",
            path_argument(&args.data_dir),
            args.json
        ),
    }
}
