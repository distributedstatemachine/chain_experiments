use crate::error::{Result, TvmError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    MinerRegister {
        stake: u64,
    },
    MinerStart {
        wallet: String,
        device: String,
        node: String,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorStatus,
    PublicEvidenceValidate {
        manifest: String,
    },
}

pub fn parse_cli_args(args: &[String]) -> Result<CliCommand> {
    let parts: Vec<&str> = args.iter().map(String::as_str).collect();
    parse_cli_parts(&parts)
}

pub fn parse_cli_parts(args: &[&str]) -> Result<CliCommand> {
    match args {
        ["miner", "register", "--stake", stake] => Ok(CliCommand::MinerRegister {
            stake: parse_u64(stake)?,
        }),
        [
            "miner",
            "start",
            "--wallet",
            wallet,
            "--device",
            device,
            "--node",
            node,
        ] => Ok(CliCommand::MinerStart {
            wallet: (*wallet).to_owned(),
            device: (*device).to_owned(),
            node: (*node).to_owned(),
        }),
        ["miner", "status"] => Ok(CliCommand::MinerStatus),
        ["validator", "register", "--stake", stake] => Ok(CliCommand::ValidatorRegister {
            stake: parse_u64(stake)?,
        }),
        ["validator", "start", "--wallet", wallet, "--node", node] => {
            Ok(CliCommand::ValidatorStart {
                wallet: (*wallet).to_owned(),
                node: (*node).to_owned(),
            })
        }
        ["validator", "status"] => Ok(CliCommand::ValidatorStatus),
        ["public-evidence", "validate", "--manifest", manifest] => {
            Ok(CliCommand::PublicEvidenceValidate {
                manifest: (*manifest).to_owned(),
            })
        }
        _ => Err(TvmError::InvalidReceipt("invalid cli command")),
    }
}

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { stake } => format!("register miner with stake {stake}"),
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => format!("start miner wallet={wallet} device={device} node={node}"),
        CliCommand::MinerStatus => "show miner status".to_owned(),
        CliCommand::ValidatorRegister { stake } => format!("register validator with stake {stake}"),
        CliCommand::ValidatorStart { wallet, node } => {
            format!("start validator wallet={wallet} node={node}")
        }
        CliCommand::ValidatorStatus => "show validator status".to_owned(),
        CliCommand::PublicEvidenceValidate { manifest } => {
            format!("validate public evidence manifest {manifest}")
        }
    }
}

fn parse_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_miner_commands() {
        assert_eq!(
            parse_cli_parts(&["miner", "register", "--stake", "100"]).unwrap(),
            CliCommand::MinerRegister { stake: 100 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "miner",
                "start",
                "--wallet",
                "miner.key",
                "--device",
                "cuda:0",
                "--node",
                "http://localhost:8545"
            ])
            .unwrap(),
            CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cuda:0".to_owned(),
                node: "http://localhost:8545".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["miner", "status"]).unwrap(),
            CliCommand::MinerStatus
        );
    }

    #[test]
    fn parses_documented_validator_commands() {
        assert_eq!(
            parse_cli_parts(&["validator", "register", "--stake", "10000"]).unwrap(),
            CliCommand::ValidatorRegister { stake: 10_000 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "validator",
                "start",
                "--wallet",
                "validator.key",
                "--node",
                "http://localhost:8545"
            ])
            .unwrap(),
            CliCommand::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "http://localhost:8545".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["validator", "status"]).unwrap(),
            CliCommand::ValidatorStatus
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "validate",
                "--manifest",
                "docs/tensorvm/public-testnet.evidence"
            ])
            .unwrap(),
            CliCommand::PublicEvidenceValidate {
                manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_invalid_cli() {
        assert!(parse_cli_parts(&["miner", "register"]).is_err());
        assert!(parse_cli_parts(&["validator", "register", "--stake", "abc"]).is_err());
    }

    #[test]
    fn parse_cli_args_and_describe_commands() {
        let args = vec![
            "miner".to_owned(),
            "register".to_owned(),
            "--stake".to_owned(),
            "250".to_owned(),
        ];
        let command = parse_cli_args(&args).unwrap();
        assert_eq!(command, CliCommand::MinerRegister { stake: 250 });

        let commands = [
            (
                CliCommand::MinerRegister { stake: 1 },
                "register miner with stake 1",
            ),
            (
                CliCommand::MinerStart {
                    wallet: "miner.key".to_owned(),
                    device: "cuda:0".to_owned(),
                    node: "http://localhost:8545".to_owned(),
                },
                "start miner wallet=miner.key device=cuda:0 node=http://localhost:8545",
            ),
            (CliCommand::MinerStatus, "show miner status"),
            (
                CliCommand::ValidatorRegister { stake: 10 },
                "register validator with stake 10",
            ),
            (
                CliCommand::ValidatorStart {
                    wallet: "validator.key".to_owned(),
                    node: "http://localhost:8545".to_owned(),
                },
                "start validator wallet=validator.key node=http://localhost:8545",
            ),
            (CliCommand::ValidatorStatus, "show validator status"),
            (
                CliCommand::PublicEvidenceValidate {
                    manifest: "evidence.txt".to_owned(),
                },
                "validate public evidence manifest evidence.txt",
            ),
        ];
        for (command, description) in commands {
            assert_eq!(describe_command(&command), description);
        }
    }
}
