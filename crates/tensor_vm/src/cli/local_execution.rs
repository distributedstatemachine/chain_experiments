use super::TvmdCommand;
use super::commands::LocalnetCommand;
use super::local_role_execution::{
    execute_miner_command, execute_proposer_command, execute_validator_command,
};
use super::local_service_execution::execute_node_command;
use super::validation::{ensure_data_dir, path_argument};
use crate::app::KeyValueReportWriter;
use crate::chain::ChainParams;
use crate::error::Result;

pub(super) fn execute_local_cli_command(command: &TvmdCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        TvmdCommand::Miner(command) => execute_miner_command(command, &params),
        TvmdCommand::Validator(command) => execute_validator_command(command, &params),
        TvmdCommand::Proposer(command) => execute_proposer_command(command),
        TvmdCommand::Node(command) => execute_node_command(command),
        TvmdCommand::Localnet(command) => execute_localnet_command(command),
        TvmdCommand::Public(_) => unreachable!("public commands are handled by cli::execution"),
    }
}

fn execute_localnet_command(command: &LocalnetCommand) -> Result<String> {
    match command {
        LocalnetCommand::Seed(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "local_testnet_seed");
            report.field("data_dir", data_dir);
            report.field("local_cpu_seed_ready", true);
            Ok(report.finish())
        }
        LocalnetCommand::Verify(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            let report = LocalCpuVerifyFixtureReport {
                command: "local_cpu_verify",
                data_dir: &data_dir,
                structured_verifier_ready: true,
            };
            if args.json {
                Ok(serde_json::to_string(&report)
                    .expect("local CPU verify fixture report must serialize"))
            } else {
                Ok(report.to_key_value_report())
            }
        }
    }
}

#[derive(serde::Serialize)]
struct LocalCpuVerifyFixtureReport<'a> {
    command: &'static str,
    data_dir: &'a str,
    structured_verifier_ready: bool,
}

impl LocalCpuVerifyFixtureReport<'_> {
    fn to_key_value_report(&self) -> String {
        let mut report = KeyValueReportWriter::new();
        report.field("command", self.command);
        report.field("data_dir", self.data_dir);
        report.field("structured_verifier_ready", self.structured_verifier_ready);
        report.finish()
    }
}
