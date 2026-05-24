use super::*;
use tensor_vm::app::{
    RoleServiceConfig, RoleServiceRunner, RuntimeRole, chain_profile_from_label,
    runtime_role_wallet_registered, runtime_role_wallet_registration,
};

#[test]
fn runtime_role_policy_allows_only_validator_local_production() {
    let profile = ChainProfile::local_cpu();
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Service.node_role(), "service")
            .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(
            profile.clone(),
            RuntimeRole::Proposer.node_role(),
            "proposer"
        )
        .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Miner.node_role(), "miner")
            .can_produce_local_blocks()
    );
    assert!(
        NodeConfig::new(profile, RuntimeRole::Validator.node_role(), "validator")
            .can_produce_local_blocks()
    );

    assert_eq!(RuntimeRole::Service.label(), "service");
    assert_eq!(RuntimeRole::Miner.label(), "miner");
    assert_eq!(RuntimeRole::Validator.label(), "validator");
    assert_eq!(RuntimeRole::Proposer.label(), "proposer");
}

#[test]
fn role_loop_configs_bind_expected_runtime_roles_and_wallets() {
    let cases = [
        (
            RoleServiceRunner::miner(),
            "miner_run",
            RuntimeRole::Miner,
            "miner",
        ),
        (
            RoleServiceRunner::validator(),
            "validator_run",
            RuntimeRole::Validator,
            "validator",
        ),
        (
            RoleServiceRunner::proposer(),
            "proposer_run",
            RuntimeRole::Proposer,
            "proposer",
        ),
    ];

    for (loop_config, runtime_command, role, wallet) in cases {
        let service_config = loop_config
            .service_runtime_config(RoleServiceConfig {
                wallet,
                device: Some("cpu"),
                node: "/ip4/127.0.0.1/tcp/4001",
                listen: "127.0.0.1:0",
                p2p_listen: "/ip4/127.0.0.1/tcp/0",
                data_dir: "role-loop-config-test",
                identity_seed: None,
                auth_token: "token",
                max_requests: 1,
            })
            .unwrap();

        assert_eq!(service_config.runtime_command, runtime_command);
        assert_eq!(service_config.role, role);
        assert_eq!(service_config.node.role, role.node_role());
        assert_eq!(
            service_config.node.can_produce_local_blocks(),
            matches!(role, RuntimeRole::Validator)
        );
        assert!(!service_config.node.local_synthetic_producer());
        assert_eq!(
            service_config.role_wallet_address,
            Some(address(wallet.as_bytes()))
        );
    }
}

#[test]
fn role_loop_reports_keep_role_specific_readiness_lines() {
    let config = RoleServiceConfig {
        wallet: "testnet-miner-0",
        device: Some("cpu"),
        node: "/ip4/127.0.0.1/tcp/4001",
        listen: "127.0.0.1:0",
        p2p_listen: "/ip4/127.0.0.1/tcp/0",
        data_dir: "role-loop-report-test",
        identity_seed: None,
        auth_token: "token",
        max_requests: 1,
    };

    let miner_report = RoleServiceRunner::miner().format_report(config, "service_report=true");
    assert!(miner_report.contains("command=miner_run"));
    assert!(miner_report.contains("role=miner"));
    assert!(miner_report.contains("device=cpu"));
    assert!(miner_report.contains("role_runtime_ready=true"));

    let validator_report =
        RoleServiceRunner::validator().format_report(config, "service_report=true");
    assert!(validator_report.contains("command=validator_run"));
    assert!(validator_report.contains("role=validator"));
    assert!(validator_report.contains("reference_verifier_ready=true"));
    assert!(validator_report.contains("role_runtime_ready=true"));

    let proposer_report =
        RoleServiceRunner::proposer().format_report(config, "service_report=true");
    assert!(proposer_report.contains("command=proposer_run"));
    assert!(proposer_report.contains("role=proposer"));
    assert!(proposer_report.contains("proposer_ready=true"));
    assert!(proposer_report.contains("role_runtime_ready=true"));
}

#[test]
fn role_wallet_registration_matches_loaded_chain_role() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"role-wallet-registration"]));
    let miner = address(b"runtime-wallet-miner");
    let validator = address(b"runtime-wallet-validator");
    let unknown = address(b"runtime-wallet-unknown");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();

    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(miner), &chain),
        "miner"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(validator), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(unknown), &chain),
        "unregistered"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, None, &chain),
        "none"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, Some(miner), &chain),
        "none"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Service,
        None,
        &chain
    ));
}

#[test]
fn chain_profile_labels_drive_runtime_synthetic_jobs() {
    let local = chain_profile_from_label("local_cpu").unwrap();
    let testnet = chain_profile_from_label("public_testnet").unwrap();
    let mainnet = chain_profile_from_label("mainnet").unwrap();

    assert_eq!(local.label(), "local_cpu");
    assert_eq!(testnet.label(), "public_testnet");
    assert_eq!(mainnet.label(), "mainnet");
    assert!(local.synthetic_job_source().is_some());
    assert!(testnet.synthetic_job_source().is_none());
    assert!(mainnet.synthetic_job_source().is_none());
    assert!(chain_profile_from_label("staging").is_err());
}
