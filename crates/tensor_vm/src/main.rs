use std::path::Path;
use tensor_vm::{
    CliCommand, Faucet, LocalChain, NodeStore, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    hash::hex,
    parse_cli_args,
    types::hash_bytes,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_cli_args(&args) {
        Ok(command) => match execute_command(&command) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

fn execute_command(command: &CliCommand) -> std::result::Result<String, String> {
    match command {
        CliCommand::PublicEvidenceValidate { manifest } => {
            let contents = std::fs::read_to_string(manifest)
                .map_err(|error| format!("failed to read evidence manifest {manifest}: {error}"))?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::PublicTestnetPreflight { manifest } => {
            let contents = std::fs::read_to_string(manifest).map_err(|error| {
                format!("failed to read preflight manifest {manifest}: {error}")
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::ServiceInit { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            init_service_store(data_dir)
        }
        CliCommand::ServiceServe {
            listen,
            data_dir,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            serve_service(listen, data_dir, auth_token, *max_requests)
        }
        _ => execute_reference_cli_command(command).map_err(|error| error.to_string()),
    }
}

fn init_service_store(data_dir: &str) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    if Path::new(data_dir).exists()
        && Path::new(data_dir)
            .read_dir()
            .map_err(|error| format!("failed to inspect data dir {data_dir}: {error}"))?
            .next()
            .is_some()
    {
        let status = store
            .status()
            .map_err(|error| format!("existing node store is invalid: {error}"))?;
        return Ok(format!(
            "command=service_init\ndata_dir={}\nexisting_store=true\nblock_count={}\nlatest_block_hash={}",
            status.data_dir.display(),
            status.block_count,
            hex(&status.latest_block_hash)
        ));
    }

    let chain = LocalChain::new(hash_bytes(
        b"tensor-vm-service-genesis",
        &[data_dir.as_bytes()],
    ));
    let status = store
        .persist_chain(&chain)
        .map_err(|error| format!("failed to initialize node store {data_dir}: {error}"))?;
    Ok(format!(
        "command=service_init\ndata_dir={}\nexisting_store=false\nblock_count={}\nlatest_block_hash={}",
        status.data_dir.display(),
        status.block_count,
        hex(&status.latest_block_hash)
    ))
}

fn serve_service(
    listen: &str,
    data_dir: &str,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(
        node,
        RpcPolicy {
            auth_token: Some(auth_token.to_owned()),
            ..RpcPolicy::default()
        },
    );
    let mut server = RpcHttpServer::bind(listen, gateway)
        .map_err(|error| format!("failed to bind service listener {listen}: {error}"))?;
    let mut served_requests = 0usize;
    loop {
        if max_requests != 0 && served_requests >= max_requests {
            break;
        }
        server
            .serve_next()
            .map_err(|error| format!("service request failed: {error}"))?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist service state: {error}"))?;
        served_requests = served_requests.saturating_add(1);
    }
    Ok(format!(
        "command=service_serve\nlisten={listen}\ndata_dir={data_dir}\nserved_requests={served_requests}"
    ))
}
