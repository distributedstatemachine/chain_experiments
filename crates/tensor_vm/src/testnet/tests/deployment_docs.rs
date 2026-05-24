#[test]
fn public_deployment_templates_require_libp2p_and_https_surfaces() {
    let env = include_str!("../../../../../deploy/tensorvm/env/public-testnet.env.example");
    for required in [
        "TVMD_LISTEN=127.0.0.1:8545",
        "TVMD_P2P_LISTEN=/ip4/0.0.0.0/tcp/4001",
        "TVMD_DATA_DIR=/var/lib/tensorvm",
        "TVMD_AUTH_TOKEN=replace-with-high-entropy-token",
        "tvmd service peer add --data-dir \"$TVMD_DATA_DIR\"",
    ] {
        assert!(
            env.contains(required),
            "deployment env template should contain {required}"
        );
    }

    let systemd = include_str!("../../../../../deploy/tensorvm/systemd/tensorvm.service");
    for required in [
        "ExecStartPre=/usr/local/bin/tvmd service init --data-dir ${TVMD_DATA_DIR}",
        "ExecStart=/usr/local/bin/tvmd service serve",
        "--p2p-listen ${TVMD_P2P_LISTEN}",
        "--data-dir ${TVMD_DATA_DIR}",
        "--auth-token ${TVMD_AUTH_TOKEN}",
        "ReadWritePaths=/var/lib/tensorvm",
        "NoNewPrivileges=true",
        "ProtectSystem=strict",
    ] {
        assert!(
            systemd.contains(required),
            "systemd service template should contain {required}"
        );
    }

    let nginx = include_str!("../../../../../deploy/tensorvm/nginx/tensorvm.conf");
    for required in [
        "listen 443 ssl http2;",
        "server_name rpc.example.test explorer.example.test faucet.example.test telemetry.example.test;",
        "proxy_set_header X-Forwarded-Proto https;",
        "client_max_body_size 2m;",
        "proxy_pass http://tensorvm_service;",
        "return 301 https://$host$request_uri;",
    ] {
        assert!(
            nginx.contains(required),
            "nginx template should contain {required}"
        );
    }
}

#[test]
fn public_deployment_runbook_records_required_evidence_flow() {
    let runbook = include_str!("../../../../../deploy/tensorvm/RUNBOOK.md");
    for required in [
        "tvmd testnet preflight deploy/tensorvm/manifests/public-testnet.preflight.example",
        "public_testnet_preflight_ready=true",
        "deployment_plan_ready=true",
        "cuda_ready_miners=true",
        "libp2p_ready_nodes=true",
        "production_libp2p_runtime=true",
        "public_service_content_planned=true",
        "public_services_planned=true",
    ] {
        assert!(
            runbook.contains(required),
            "runbook should guard preflight requirement {required}"
        );
    }

    for command in [
        "tvmd evidence publish ...",
        "tvmd evidence audit ...",
        "tvmd evidence run window ...",
        "tvmd evidence run window-file ...",
        "tvmd evidence node heartbeat ...",
        "tvmd evidence node heartbeat-file ...",
        "tvmd evidence node operator-attestation ...",
        "tvmd evidence service health ...",
        "tvmd evidence service health-file ...",
        "tvmd evidence service content ...",
        "tvmd evidence service content-bytes ...",
        "tvmd evidence service content-file ...",
        "tvmd evidence network observation ...",
        "tvmd evidence network from-service-log ...",
        "tvmd evidence record summary ...",
        "tvmd evidence record artifact ...",
        "tvmd evidence record artifact-roots ...",
        "tvmd evidence record artifact-file ...",
        "tvmd evidence record summary-roots ...",
        "tvmd evidence record summary-file ...",
    ] {
        assert!(
            runbook.contains(command),
            "runbook should list evidence command {command}"
        );
    }

    for required in [
        "The collected records must cover the full 7-day window, not only a final snapshot.",
        "node heartbeats for every active miner and validator",
        "exactly one service-health record for each public RPC, explorer, faucet, and telemetry service",
        "exactly one service-content record for each public RPC, explorer, faucet, and telemetry service",
        "libp2p network-observation records from independent observers, one per counted public operator",
        "Do not backfill",
        "public_evidence_full_spec=true",
        "independently_checkable=true",
        "supporting_record_artifacts=true",
        "exactly one signed artifact locator line for each required raw supporting-record kind",
        "After validation returns `public_evidence_full_spec=true`, link the published bundle from",
        "It does not contain a real external 7-day public run or a published independently checkable",
    ] {
        assert!(
            runbook.contains(required),
            "runbook should record external evidence requirement {required}"
        );
    }
}

#[test]
fn public_deployment_readme_records_scaffold_boundary_and_operator_flow() {
    let readme = include_str!("../../../../../deploy/tensorvm/README.md");
    for required in [
        "These files are not public-testnet evidence by themselves",
        "env/public-testnet.env.example",
        "RUNBOOK.md",
        "systemd/tensorvm.service",
        "nginx/tensorvm.conf",
        "manifests/public-testnet.preflight.example",
        "manifests/public-testnet.evidence.example",
    ] {
        assert!(
            readme.contains(required),
            "deployment README should list scaffold artifact {required}"
        );
    }

    for route in [
        "GET /health",
        "GET /rpc/health",
        "GET /explorer/health",
        "GET /faucet/health",
        "GET /telemetry/health",
        "GET /chain/head",
        "GET /explorer",
        "GET /faucet/page",
        "GET /telemetry/dashboard",
    ] {
        assert!(
            readme.contains(route),
            "deployment README should record public route {route}"
        );
    }

    for required in [
        "signed service-health records",
        "signed service-content records",
        "one signed `network_runtime_observation=...` record per counted public operator",
        "evidence record summary-file",
        "evidence record artifact-file",
        "exactly one artifact locator for each of those six supporting",
        "cargo build -p tensor_vm --release --features cuda-kernels",
        "target/release/tvmd miner start --wallet miner.key --device cuda:0 --node",
        "tvmd service peer add",
        "tvmd service readiness",
        "cuda_ready_miner_count",
        "libp2p_ready_node_count",
        "independently_checkable=false",
        "public_evidence_full_spec=false",
        "real 7-day public run",
    ] {
        assert!(
            readme.contains(required),
            "deployment README should record operator-flow requirement {required}"
        );
    }
}
