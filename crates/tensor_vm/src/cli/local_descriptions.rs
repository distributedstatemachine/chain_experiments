use super::CliCommand;
use super::local_parser::{LocalCpuCommand, LocalTestnetCommand};
use super::local_role_descriptions::{
    describe_miner_command, describe_proposer_command, describe_validator_command,
};
use super::local_service_descriptions::describe_service_command;

pub(super) fn describe_local_command(command: &CliCommand) -> String {
    match command {
        CliCommand::Miner { command } => describe_miner_command(command),
        CliCommand::Validator { command } => describe_validator_command(command),
        CliCommand::Proposer { command } => describe_proposer_command(command),
        CliCommand::Service { command } => describe_service_command(command),
        CliCommand::LocalTestnet { command } => describe_local_testnet_command(command),
        CliCommand::LocalCpu { command } => describe_local_cpu_command(command),
        _ => unreachable!(
            "public evidence commands are handled by cli::public_evidence_descriptions"
        ),
    }
}

fn describe_local_testnet_command(command: &LocalTestnetCommand) -> String {
    match command {
        LocalTestnetCommand::Seed(args) => {
            format!("seed local CPU testnet data_dir={}", args.data_dir)
        }
    }
}

fn describe_local_cpu_command(command: &LocalCpuCommand) -> String {
    match command {
        LocalCpuCommand::Verify(args) => format!(
            "verify local CPU node evidence data_dir={} json={}",
            args.data_dir, args.json
        ),
    }
}
