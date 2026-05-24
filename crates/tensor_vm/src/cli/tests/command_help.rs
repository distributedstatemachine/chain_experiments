use super::*;
use clap::Parser;
use clap::error::ErrorKind;

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
    for command in [
        "node",
        "miner",
        "validator",
        "proposer",
        "localnet",
        "public",
    ] {
        assert!(
            help.contains(command),
            "top-level help should list {command}"
        );
    }

    let miner_run = clap_help(&["miner", "run", "--help"]);
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
fn clap_incomplete_command_groups_show_group_help() {
    for (args, commands) in [
        (&["miner"][..], &["register", "check", "run", "status"][..]),
        (
            &["validator"][..],
            &["register", "check", "run", "status"][..],
        ),
        (&["proposer"][..], &["run"][..]),
        (
            &["node"][..],
            &["init", "peer", "check", "serve", "status", "block"][..],
        ),
        (&["node", "peer"][..], &["add"][..]),
        (&["localnet"][..], &["seed", "verify"][..]),
        (&["public"][..], &["preflight", "evidence"][..]),
        (
            &["public", "evidence"][..],
            &[
                "validate", "publish", "audit", "run", "node", "service", "network", "record",
            ][..],
        ),
        (
            &["public", "evidence", "service"][..],
            &[
                "health",
                "health-file",
                "content",
                "content-bytes",
                "content-file",
            ][..],
        ),
    ] {
        let help = clap_help(args);
        assert!(
            help.contains("Usage: tvmd"),
            "incomplete command group {args:?} should show help"
        );
        for command in commands {
            assert!(
                help.contains(command),
                "incomplete command group {args:?} should list {command}"
            );
        }
    }
}

#[test]
fn clap_rejects_retired_top_level_command_families() {
    for command in [
        "role",
        "service",
        "testnet",
        "evidence",
        "public-evidence",
        "public-testnet",
        "local-testnet",
        "local-cpu",
    ] {
        let error = TvmdCli::try_parse_from(["tvmd", command])
            .expect_err("retired top-level command should not parse");
        assert!(
            matches!(error.kind(), ErrorKind::InvalidSubcommand),
            "retired top-level command {command} must not be preserved"
        );
    }
}
