use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn repo_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

fn trimmed_yaml_scalar(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

fn has_trimmed_line(text: &str, expected: &str) -> bool {
    text.lines().any(|line| line.trim() == expected)
}

fn trimmed_lines(text: &str) -> BTreeSet<&str> {
    text.lines().map(str::trim).collect()
}

fn prefixed_trimmed_values<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .collect()
}

fn shell_logical_lines(script: &str) -> Vec<String> {
    let mut logical_lines = Vec::new();
    let mut current = String::new();

    for line in script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let (segment, continues) = match line.strip_suffix('\\') {
            Some(segment) => (segment.trim_end(), true),
            None => (line, false),
        };
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(segment);
        if !continues {
            logical_lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        logical_lines.push(current);
    }

    logical_lines
}

fn assert_shell_logical_lines(script: &str, expected_lines: &[&str]) {
    let actual_lines = shell_logical_lines(script);
    for expected in expected_lines {
        assert!(
            actual_lines.iter().any(|line| line == expected),
            "script should contain logical line {expected}"
        );
    }
}

fn compose_service_section<'a>(compose: &'a str, service: &str) -> &'a str {
    let marker = format!("  {service}:\n");
    let start = compose
        .find(&marker)
        .unwrap_or_else(|| panic!("compose service {service} must exist"));
    let body_start = start + marker.len();
    let rest = &compose[body_start..];
    let next_service = rest.match_indices("\n  ").find_map(|(idx, _)| {
        rest[idx + 3..]
            .chars()
            .next()
            .is_some_and(|character| !character.is_whitespace())
            .then_some(idx)
    });
    let networks = rest.find("\nnetworks:");
    let end = match (next_service, networks) {
        (Some(next_service), Some(networks)) => next_service.min(networks),
        (Some(next_service), None) => next_service,
        (None, Some(networks)) => networks,
        (None, None) => rest.len(),
    };
    &rest[..end]
}

fn compose_env_value<'a>(service_section: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}: ");
    service_section
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(trimmed_yaml_scalar))
        .unwrap_or_else(|| panic!("compose service missing environment field {key}"))
}

fn env_file_value<'a>(env_file: &'a str, key: &str) -> &'a str {
    env_file
        .lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(field, value)| (field == key).then_some(value))
        .unwrap_or_else(|| panic!("env file missing field {key}"))
}

#[test]
fn local_cpu_compose_bundle_matches_spec_artifact_shape() {
    for path in [
        "deploy/tensorvm/local-cpu/docker-compose.yml",
        "deploy/tensorvm/local-cpu/Dockerfile",
        "deploy/tensorvm/local-cpu/README.md",
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
        "deploy/tensorvm/local-cpu/scripts/entrypoint.sh",
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
        ".dockerignore",
    ] {
        assert!(Path::new(&repo_path(path)).exists(), "missing {path}");
    }

    let compose = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/docker-compose.yml"))
        .expect("compose file should be readable");
    let dockerfile = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/Dockerfile"))
        .expect("Dockerfile should be readable");
    let entrypoint =
        fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/scripts/entrypoint.sh"))
            .expect("entrypoint should be readable");
    let env_file = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
    ))
    .expect("local CPU env file should be readable");
    let check_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
    ))
    .expect("check script should be readable");
    let restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
    ))
    .expect("restart continuity script should be readable");
    let rolling_restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
    ))
    .expect("rolling restart continuity script should be readable");
    let spec = fs::read_to_string(repo_path("docs/tensorvm/local_cpu_testnet_spec.md"))
        .expect("local CPU spec should be readable");
    let dockerignore =
        fs::read_to_string(repo_path(".dockerignore")).expect(".dockerignore should be readable");

    let miners = [
        "miner-00", "miner-01", "miner-02", "miner-03", "miner-04", "miner-05", "miner-06",
        "miner-07", "miner-08", "miner-09",
    ];
    let validators = [
        "validator-00",
        "validator-01",
        "validator-02",
        "validator-03",
        "validator-04",
    ];
    for service in miners.into_iter().chain(validators) {
        compose_service_section(&compose, service);
        assert!(
            spec.contains(service),
            "spec should name required service {service}"
        );
    }

    assert!(has_trimmed_line(&compose, "name: tensorvm-local-cpu"));
    assert!(has_trimmed_line(&compose, "tensorvm-local:"));
    assert!(has_trimmed_line(&compose, "driver: bridge"));
    assert!(!has_trimmed_line(
        &compose,
        "TENSORVM_ROLE_RUNTIME_COMMAND: proposer_run"
    ));

    let miner_sections = miners
        .iter()
        .map(|service| (*service, compose_service_section(&compose, service)))
        .collect::<Vec<_>>();
    let validator_sections = validators
        .iter()
        .map(|service| (*service, compose_service_section(&compose, service)))
        .collect::<Vec<_>>();
    assert_eq!(
        miner_sections
            .iter()
            .filter(|(_, section)| compose_env_value(section, "TENSORVM_ROLE") == "miner")
            .count(),
        10
    );
    assert_eq!(
        validator_sections
            .iter()
            .filter(|(_, section)| compose_env_value(section, "TENSORVM_ROLE") == "validator")
            .count(),
        5
    );
    for (idx, (service, section)) in miner_sections.iter().enumerate() {
        assert_eq!(
            compose_env_value(section, "TENSORVM_OPERATOR_NAME"),
            *service
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_WALLET"),
            format!("testnet-miner-{idx}")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_NODE_MULTIADDR"),
            format!("/dns4/{service}/tcp/4001")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_P2P_LISTEN"),
            "/ip4/0.0.0.0/tcp/4001"
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_RPC_LISTEN"),
            "0.0.0.0:8545"
        );
        assert!(
            has_trimmed_line(section, &format!("- {service}-data:/var/lib/tensorvm")),
            "compose service {service} must mount its data volume"
        );
    }
    for (idx, (service, section)) in validator_sections.iter().enumerate() {
        assert_eq!(
            compose_env_value(section, "TENSORVM_OPERATOR_NAME"),
            *service
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_WALLET"),
            format!("testnet-validator-{idx}")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_NODE_MULTIADDR"),
            format!("/dns4/{service}/tcp/4001")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_P2P_LISTEN"),
            "/ip4/0.0.0.0/tcp/4001"
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_RPC_LISTEN"),
            "0.0.0.0:8545"
        );
        assert!(
            has_trimmed_line(section, &format!("- {service}-data:/var/lib/tensorvm")),
            "compose service {service} must mount its data volume"
        );
    }
    let bootstrap_miner = compose_service_section(&compose, "miner-00");
    assert_eq!(
        compose_env_value(bootstrap_miner, "TENSORVM_IS_BOOTSTRAP"),
        "true"
    );
    assert_eq!(
        compose_env_value(bootstrap_miner, "TENSORVM_SEED_LOCAL_TESTNET"),
        "true"
    );
    assert!(has_trimmed_line(
        bootstrap_miner,
        r#"- "127.0.0.1:8545:8545""#
    ));
    assert!(has_trimmed_line(
        bootstrap_miner,
        r#"- "127.0.0.1:4001:4001""#
    ));
    let producer_validator = compose_service_section(&compose, "validator-00");
    assert_eq!(
        compose_env_value(producer_validator, "TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS"),
        "1000"
    );
    assert_eq!(
        compose_env_value(producer_validator, "TENSORVM_LOCAL_CPU_ROLE_PRODUCER"),
        "true"
    );
    for service in validators.iter().skip(1) {
        let section = compose_service_section(&compose, service);
        assert!(
            section
                .lines()
                .all(|line| !line.trim().starts_with("TENSORVM_LOCAL_CPU_ROLE_PRODUCER:")),
            "only validator-00 should enable local production"
        );
    }

    let explorer = compose_service_section(&compose, "explorer");
    assert!(has_trimmed_line(
        explorer,
        r#"entrypoint: ["/usr/local/bin/tensorvm-explorer", "serve"]"#
    ));
    assert_eq!(
        compose_env_value(explorer, "TENSORVM_EXPLORER_LISTEN"),
        "0.0.0.0:8080"
    );
    assert_eq!(
        compose_env_value(explorer, "TENSORVM_EXPLORER_WS_URL"),
        "ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token"
    );
    assert!(has_trimmed_line(
        explorer,
        r#"- "127.0.0.1:${TENSORVM_LOCAL_CPU_EXPLORER_PORT:-8080}:8080""#
    ));
    assert!(has_trimmed_line(explorer, r#""health-check","#));
    assert!(has_trimmed_line(explorer, "condition: service_healthy"));

    assert_eq!(
        env_file_value(&env_file, "TENSORVM_SEED_LOCAL_TESTNET"),
        "true"
    );
    assert_eq!(
        env_file_value(&env_file, "TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS"),
        "1000"
    );
    assert_eq!(
        env_file_value(&env_file, "TENSORVM_LOCAL_CPU_ROLE_PRODUCER"),
        "false"
    );
    assert_eq!(
        env_file_value(&env_file, "TENSORVM_BOOTSTRAP_PEER_ID"),
        "12D3KooWS2oXcVvmNNWTiUzwDWJavRHQmewe1NDfJB7SxP43jA7s"
    );
    let compose_lines = trimmed_lines(&compose);
    assert!(compose_lines.contains("dockerfile: deploy/tensorvm/local-cpu/Dockerfile"));

    let operator_ids = miner_sections
        .iter()
        .chain(validator_sections.iter())
        .map(|(_, section)| compose_env_value(section, "TENSORVM_OPERATOR_ID"))
        .collect::<BTreeSet<_>>();
    assert_eq!(operator_ids.len(), 15);

    assert_eq!(
        prefixed_trimmed_values(&dockerfile, "RUN "),
        [
            "cargo build -p tensor_vm --release",
            "cargo build -p tensor_vm_explorer --release",
            r#"useradd --system --home-dir /var/lib/tensorvm --shell /usr/sbin/nologin tensorvm \"#,
            "chmod 0755 /usr/local/bin/tvmd /usr/local/bin/tensorvm-explorer /usr/local/bin/tensorvm-local-entrypoint",
        ]
    );
    assert_eq!(
        prefixed_trimmed_values(&dockerfile, "COPY --from=builder "),
        [
            "/workspace/target/release/tvmd /usr/local/bin/tvmd",
            "/workspace/target/release/tensorvm-explorer /usr/local/bin/tensorvm-explorer",
        ]
    );
    assert!(has_trimmed_line(
        &dockerfile,
        "COPY deploy/tensorvm/local-cpu/scripts/entrypoint.sh /usr/local/bin/tensorvm-local-entrypoint"
    ));
    let dockerignore_lines = trimmed_lines(&dockerignore);
    assert!(dockerignore_lines.contains("target"));
    assert!(dockerignore_lines.contains(".git"));
    assert!(
        prefixed_trimmed_values(&dockerfile, "RUN ")
            .iter()
            .all(|command| !command
                .split_whitespace()
                .any(|token| token == "--features"))
    );
    assert!(!compose_lines.iter().any(|line| line.starts_with("NVIDIA_")));
    assert!(!compose_lines.contains("runtime: nvidia"));
    assert!(!compose_lines.contains("devices:"));

    assert_shell_logical_lines(
        &entrypoint,
        &[
            r#"RUNTIME_COMMAND="${TENSORVM_ROLE_RUNTIME_COMMAND:-${ROLE}_run}""#,
            r#"LOCAL_CPU_ROLE_PRODUCER="${TENSORVM_LOCAL_CPU_ROLE_PRODUCER:-false}""#,
            r#"tvmd service init --data-dir "$DATA_DIR" > "$INIT_OUT""#,
            r#"tvmd service peer add --data-dir "$DATA_DIR" --peer-id "$BOOTSTRAP_PEER_ID" --address "$BOOTSTRAP_ADDRESS" > "$DATA_DIR/service-peer-add.out""#,
            r#"tvmd miner register --stake "$MINER_STAKE" > "$DATA_DIR/role-register.out""#,
            r#"tvmd miner start --wallet "$WALLET" --device cpu --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out""#,
            r#"tvmd validator register --stake "$VALIDATOR_STAKE" > "$DATA_DIR/role-register.out""#,
            r#"tvmd validator start --wallet "$WALLET" --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out""#,
            r#"tvmd testnet seed --data-dir "$DATA_DIR" > "$DATA_DIR/local-testnet-seed.out""#,
            r#"tvmd service readiness --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" > "$DATA_DIR/service-readiness.out""#,
            r#"echo "runtime_command=$RUNTIME_COMMAND""#,
            r#"echo "local_cpu_role_producer=$LOCAL_CPU_ROLE_PRODUCER""#,
            r#"echo "public_evidence_full_spec=false""#,
            r#"echo "independently_checkable=false""#,
            r#"exec tvmd proposer run --wallet "$WALLET" --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
            r#"exec tvmd miner run --wallet "$WALLET" --device cpu --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
            r#"exec tvmd validator run --wallet "$WALLET" --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
        ],
    );

    for required in [
        "docker compose",
        "compose config --quiet",
        "ready_miners=10",
        "ready_validators=5",
        "distinct_operator_ids=15",
        "distinct_libp2p_peer_ids=15",
        "libp2p_ready_node_count=15",
        "cpu_ready_miner_count=10",
        "cuda_required_miner_count=0",
        "p2p_identity_seeded=true",
        "settled_receipts=10",
        "matmul_settled=true",
        "linear_training_settled=true",
        "rewarded_miners=",
        "total_reward_balance",
        "attestation_count",
        "finality_rate_bps=10000",
        "data_availability_bps=10000",
        "gateway chain head did not advance past seeded height 2",
        "protocol did not generate synthetic jobs after seed",
        "settled_receipt_count",
        "standalone_explorer_ready=true",
        "standalone_explorer_websocket_polling=true",
        "live_block_production=true",
        "live_synthetic_jobs=true",
        "live_linear_training_jobs=true",
        "live_attestations=true",
        "live_receipt_attestations=true",
        "live receipt details did not include post-seed TensorOp receipts",
        "live receipt details did not include post-seed LinearTrainingStep receipts",
        "live_tensor_op_receipts=true",
        "live_linear_training_receipts=true",
        "live_tensor_op_block_evidence=true",
        "live_tensor_op_block_height=",
        "live_linear_training_block_evidence=true",
        "live_linear_training_block_height=",
        "live_tensor_fetch=true",
        "live_rewards=true",
        "tvmd service status",
        "tvmd testnet verify-local-cpu",
        "structured_verifier_ready",
        "all_operator_status_count=15",
        "--max-time 15",
        "CANDIDATE_NETWORK_HEAD_HEIGHT",
        "role_can_produce_blocks",
        "role_wallet_address",
        "role_wallet_registration",
        "role_wallet_registered",
        "role_miner_work_ready",
        "role_miner_assigned_jobs_seen",
        "role_miner_unreceipted_jobs",
        "role_miner_receipts_submitted",
        "role_miner_tensors_inserted",
        "role_validator_work_ready",
        "role_validator_assigned_receipts_seen",
        "role_validator_unattested_receipts",
        "role_validator_artifact_ready_receipts",
        "role_validator_artifact_missing_receipts",
        "role_validator_remote_tensor_fetch_attempts",
        "role_validator_remote_tensor_fetch_successes",
        "role_validator_remote_tensor_fetch_failures",
        "role_validator_remote_tensor_fetch_bytes",
        "role_validator_remote_tensors_inserted",
        "role_validator_attestations_submitted",
        "role_validator_block_votes_submitted",
        "role_chain_profile",
        "role_local_producer",
        "role_network_applied_blocks",
        "role_network_events_ingested",
        "role_network_block_headers_ingested",
        "role_network_block_payloads_ingested",
        "role_network_block_payloads_applied",
        "role_network_block_votes_ingested",
        "role_network_block_votes_applied",
        "role_network_job_events_ingested",
        "role_network_job_payloads_ingested",
        "role_network_job_payloads_applied",
        "role_network_receipt_payloads_ingested",
        "role_network_receipt_payloads_applied",
        "role_network_attestation_payloads_ingested",
        "role_network_attestation_payloads_applied",
        "role_network_receipt_events_ingested",
        "role_network_attestation_events_ingested",
        "role_network_invalid_events",
        "role_p2p_latest_observed_block_height",
        "role_p2p_observed_block_payloads",
        "role_p2p_observed_block_votes",
        "role_p2p_latest_observed_block_payload_height",
        "role_p2p_latest_observed_block_payload_hash",
        "role_p2p_observed_block_payload_hashes",
        "all_operator_min_height=",
        "latest_block_height",
        "block_log_root",
        "all_operator_first_live_block_hash=",
        "all_operator_live_block_convergence=true",
        "tvmd service block",
        "block_validation",
        "useful_verification_pow",
        "settled_receipt_set_root",
        "selected_receipt_count",
        "checks_root_recomputed",
        "block_vote_count",
        "block_vote_validators",
        "block_vote_stake",
        "finality_threshold_stake",
        "pow_valid",
        "canonical_blockspace_valid",
        "finality_validated_block",
        "tensor_op_receipt_count",
        "linear_training_receipt_count",
        "all_operator_common_head_height=",
        "all_operator_common_head_hash=",
        "all_operator_common_head_convergence=true",
        "all_operator_target_head_height=",
        "all_operator_target_head_hash=",
        "all_operator_target_state_root=",
        "all_operator_target_head_convergence=true",
        "all_operator_network_head_height=",
        "all_operator_network_head_hash=",
        "all_operator_network_state_root=",
        "all_operator_network_head_convergence=true",
        "all_operator_role_status=true",
        "all_operator_role_runtime_commands=true",
        "all_operator_role_wallets_registered=true",
        "all_operator_miner_work_status=true",
        "all_operator_miner_receipt_status=true",
        "all_operator_validator_attestation_status=true",
        "all_operator_validator_remote_tensor_fetch_status=true",
        "all_operator_chain_profiles=true",
        "all_operator_role_production_policy=true",
        "all_operator_role_runtime_counters=true",
        "single_local_producer=true",
        "local_proposer_runtime=false",
        "local_validator_producer=true",
        "useful_pow_block_evidence=",
        "canonical_blockspace_evidence=",
        "block_checks_root_evidence=",
        "validator_proposer_evidence=",
        "tensorwork_proposer_selection_removed=true",
        "finality_requires_useful_pow=",
        "live_validator_proposer_networking=false",
        "live_validator_block_vote_networking=true",
        "all_non_producer_network_applied_blocks=true",
        "all_non_producer_network_block_payload_ingestion=true",
        "all_non_producer_network_block_payload_application=true",
        "all_non_producer_network_block_vote_ingestion=true",
        "all_non_producer_network_block_vote_application=true",
        "all_non_producer_network_event_ingestion=true",
        "all_non_producer_network_payload_announcements=true",
        "all_non_producer_network_job_payload_application=true",
        "all_non_producer_network_receipt_payload_application=true",
        "all_non_producer_network_attestation_payload_application=true",
        "all_operator_p2p_connected_peers=true",
        "all_operator_p2p_block_gossip=true",
        "all_operator_p2p_block_payload_gossip=true",
        "all_operator_p2p_block_vote_gossip=true",
        "all_operator_p2p_block_payload_head_observed=true",
        "all_operator_p2p_job_gossip=true",
        "all_operator_p2p_receipt_gossip=true",
        "all_operator_p2p_attestation_gossip=true",
        "all_operator_p2p_target_head_observed=true",
        "all_operator_p2p_latest_head_observed=true",
        "all_operator_chain_counters=true",
        "all_operator_block_log_roots_observed=true",
        "data-ui=\"ratzilla-tui\"",
        "new WebSocket",
        "cargo test -p tensor_vm local_testnet --release",
        "public_evidence_full_spec=false",
        "independently_checkable=false",
    ] {
        assert!(
            check_script.contains(required),
            "check script should contain {required}"
        );
    }

    assert_shell_logical_lines(
        &restart_script,
        &[
            r#"CHECK_SCRIPT="$SCRIPT_DIR/check-local-testnet.sh""#,
            r#"EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"RESTART_SERVICES="${*:-miner-03 validator-02}""#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd service status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then"#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd service block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then"#,
            r#"[ -x "$CHECK_SCRIPT" ] || fail "check-local-testnet.sh is not executable""#,
            r#"timeout 60s docker compose -f "$COMPOSE_FILE" restart $RESTART_SERVICES"#,
            r#"timeout 600s "$CHECK_SCRIPT""#,
            r#"local_cpu_restart_continuity_ready=true"#,
            r#"restart_services=${RESTART_SERVICE_LIST}"#,
            r#"before_common_head_height=${BEFORE_COMMON_HEIGHT}"#,
            r#"before_common_head_hash=${BEFORE_COMMON_HASH}"#,
            r#"before_common_state_root=${BEFORE_COMMON_STATE_ROOT}"#,
            r#"after_common_head_height=${AFTER_COMMON_HEIGHT}"#,
            r#"after_common_head_hash=${AFTER_COMMON_HASH}"#,
            r#"after_common_state_root=${AFTER_COMMON_STATE_ROOT}"#,
            r#"restart_peer_ids_stable=true"#,
            r#"restart_heights_non_decreasing=true"#,
            r#"restart_heights_advance=true"#,
            r#"restart_block_counts_non_decreasing=true"#,
            r#"restart_block_counts_advance=true"#,
            r#"restart_state_roots_observed=true"#,
            r#"restart_state_roots_advance=true"#,
            r#"restart_block_log_roots_observed=true"#,
            r#"restart_block_log_roots_advance=true"#,
            r#"restart_previous_common_head_preserved=true"#,
            r#"restart_previous_common_state_root_preserved=true"#,
            r#"restart_blocks_continue=true"#,
            r#"restart_common_head_convergence=true"#,
        ],
    );

    assert_shell_logical_lines(
        &rolling_restart_script,
        &[
            r#"RESTART_SCRIPT="$SCRIPT_DIR/check-restart-continuity.sh""#,
            r#"EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"ROLLING_SERVICES="${*:-$EXPECTED_SERVICES}""#,
            r#"[ -x "$RESTART_SCRIPT" ] || fail "check-restart-continuity.sh is not executable""#,
            r#"if "$RESTART_SCRIPT" "$service"; then"#,
            r#"printf 'rolling_restart_service=%s,ready\n' "$service""#,
            r#"local_cpu_rolling_restart_continuity_ready=true"#,
            r#"rolling_restart_services=${ROLLING_SERVICE_LIST}"#,
            r#"rolling_restart_service_count=${ROLLING_COUNT}"#,
            r#"rolling_restart_peer_ids_stable=true"#,
            r#"rolling_restart_heights_advance=true"#,
            r#"rolling_restart_block_counts_advance=true"#,
            r#"rolling_restart_state_roots_advance=true"#,
            r#"rolling_restart_block_log_roots_advance=true"#,
            r#"rolling_restart_previous_common_head_preserved=true"#,
            r#"rolling_restart_previous_common_state_root_preserved=true"#,
            r#"rolling_restart_blocks_continue=true"#,
            r#"rolling_restart_common_head_convergence=true"#,
        ],
    );
}
