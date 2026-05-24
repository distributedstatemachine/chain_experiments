use super::*;
use clap::Parser;

fn clap_help(args: &[&str]) -> String {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv)
        .expect_err("help should stop parsing with a Clap display error")
        .to_string()
}

#[test]
fn clap_help_exposes_the_tvmd_command_tree() {
    let help = clap_help(&["--help"]);
    assert!(help.contains("Usage: tvmd <COMMAND>"));
    for command in ["node", "role", "localnet", "public"] {
        assert!(
            help.contains(command),
            "top-level help should list {command}"
        );
    }

    let role = clap_help(&["role", "--help"]);
    for command in ["miner", "validator", "proposer"] {
        assert!(role.contains(command), "role help should list {command}");
    }

    let miner_run = clap_help(&["role", "miner", "run", "--help"]);
    for argument in [
        "--wallet <PATH>",
        "--device <DEVICE>",
        "--node <MULTIADDR>",
        "--listen <ADDR>",
        "--p2p-listen <MULTIADDR>",
        "--data-dir <DIR>",
        "--identity-seed <HEX>",
        "--auth-token <TOKEN>",
        "--max-requests <N>",
    ] {
        assert!(
            miner_run.contains(argument),
            "miner run help should list {argument}"
        );
    }

    let evidence = clap_help(&["public", "evidence", "--help"]);
    for command in [
        "validate", "publish", "audit", "run", "node", "service", "network", "record",
    ] {
        assert!(
            evidence.contains(command),
            "evidence help should list {command}"
        );
    }
}

#[test]
fn clap_rejects_retired_top_level_command_families() {
    for command in [
        "miner",
        "validator",
        "proposer",
        "service",
        "testnet",
        "evidence",
        "public-evidence",
        "public-testnet",
        "local-testnet",
        "local-cpu",
    ] {
        assert!(
            parse_test_cli(&[command, "--help"]).is_err(),
            "retired top-level command {command} must not be preserved"
        );
    }
}
