#[cfg(test)]
use crate::hash::hex;
#[cfg(test)]
use crate::jobs::PrimitiveType;
#[cfg(test)]
use std::net::SocketAddr;

mod dispatch;
mod explorer;
mod explorer_routes;
mod gateway;
mod http;
mod mutations;
mod node;
mod parse;
mod read_routes;
mod render;
mod response;
mod tensor_routes;
mod types;
mod websocket;
#[cfg(test)]
use explorer::{hardware_class_label, primitive_label};
pub use gateway::{RpcGateway, RpcPolicy};
#[cfg(test)]
use http::{
    ParsedHttpRequest, read_http_request_from, split_path_and_auth_token, try_parse_http_request,
};
pub use http::{RpcHttpServer, http_response_text};
pub use node::RpcNode;
use parse::{parse_address, parse_hash};
pub use types::{RpcRequest, RpcResponse};
#[cfg(test)]
use websocket::{
    base64_encode, json_string_field, json_usize_field, read_websocket_text_frame,
    websocket_accept_key, write_websocket_frame,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainParams, HardwareClass, JobState};
    use crate::faucet::Faucet;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, TensorOpReceipt};
    use crate::profile::ChainProfile;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

    mod routes;

    #[test]
    fn rpc_http_server_returns_bad_request_and_payload_too_large() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        fn send_raw(addr: SocketAddr, raw: &[u8]) -> String {
            let mut client = TcpStream::connect(addr).unwrap();
            client.write_all(raw).unwrap();
            client.shutdown(Shutdown::Write).unwrap();
            let mut response = String::new();
            client.read_to_string(&mut response).unwrap();
            response
        }

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let gateway = RpcGateway::new(
            RpcNode::new(Chain::new(beacon)),
            RpcPolicy {
                max_body_bytes: 2,
                ..RpcPolicy::default()
            },
        );
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_n(2).unwrap());

        let bad = send_raw(addr, b"GET");
        assert!(bad.starts_with("HTTP/1.1 400 Bad Request"));

        let too_large = send_raw(addr, b"POST /tx HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc");
        assert!(too_large.starts_with("HTTP/1.1 413 Payload Too Large"));

        server_thread.join().unwrap();
    }

    #[test]
    fn rpc_http_parser_rejects_bad_headers_and_waits_for_complete_bodies() {
        let uppercase_hash = hex(&hash_bytes(b"test", &[b"rpc-uppercase"])).to_uppercase();
        assert!(parse_hash(&uppercase_hash).is_ok());
        assert!(parse_hash(&"g".repeat(64)).is_err());

        assert!(matches!(
            try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n\r\n", 16),
            Some(ParsedHttpRequest::Request {
                auth_token: None,
                ..
            })
        ));
        assert!(matches!(
            try_parse_http_request(
                b"POST /tx HTTP/1.1\r\nauthorization: Bearer secret\r\ncontent-length: 4\r\n\r\nbody",
                16,
            ),
            Some(ParsedHttpRequest::Request {
                auth_token: Some(token),
                ..
            }) if token == "secret"
        ));
        assert!(matches!(
            try_parse_http_request(
                b"POST /tx HTTP/1.1\r\nx-tensorchain-auth: local\r\ncontent-length: 4\r\n\r\nbody",
                16,
            ),
            Some(ParsedHttpRequest::Request {
                auth_token: Some(token),
                ..
            }) if token == "local"
        ));
        assert!(matches!(
            try_parse_http_request(
                b"GET /explorer/ws?token=local HTTP/1.1\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
                16,
            ),
            Some(ParsedHttpRequest::WebSocketUpgrade {
                path,
                auth_token: Some(token),
                websocket_key,
            }) if path == "/explorer/ws" && token == "local" && websocket_key == "dGhlIHNhbXBsZSBub25jZQ=="
        ));
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"ws-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(miner, 10_000).unwrap();
        chain.produce_block(miner, 1).unwrap();
        let rpc = RpcNode::new(chain);
        let overview = rpc.explorer_websocket_response(
            "{\"type\":\"overview\",\"block_limit\":1,\"receipt_limit\":1,\"job_limit\":1}",
        );
        assert!(overview.contains("\"type\":\"overview\""));
        assert!(overview.contains("\"block_count\":1"));
        let account = rpc.explorer_websocket_response(&format!(
            "{{\"type\":\"account\",\"address\":\"{}\"}}",
            hex(&miner)
        ));
        assert!(account.contains("\"type\":\"account\""));
        assert!(account.contains("\"is_miner\":true"));
        assert!(matches!(
            try_parse_http_request(b"GET /\xff HTTP/1.1\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"GET\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"GET /chain/head HTTP/1.1\r\nhost\r\n\r\n", 16),
            Some(ParsedHttpRequest::Request { .. })
        ));
        assert!(matches!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: nope\r\n\r\n", 16),
            Some(ParsedHttpRequest::BadRequest)
        ));
        assert!(matches!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 17\r\n\r\n", 16),
            Some(ParsedHttpRequest::TooLarge)
        ));
        assert!(
            try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbo", 16,)
                .is_none()
        );
        assert!(try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n", 16).is_none());
        assert!(matches!(
            try_parse_http_request(
                b"GET /explorer/ws HTTP/1.1\r\nupgrade: websocket\r\n\r\n",
                16,
            ),
            Some(ParsedHttpRequest::BadRequest)
        ));
    }

    #[test]
    fn explorer_websocket_views_cover_chain_collections_and_bad_commands() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let cpu_miner = address(b"ws-cpu-miner");
        let consumer_gpu_miner = address(b"ws-consumer-gpu-miner");
        let datacenter_gpu_miner = address(b"ws-datacenter-gpu-miner");
        let other_miner = address(b"ws-other-miner");
        let validator = address(b"ws-validator");
        chain.register_miner(cpu_miner, 100).unwrap();
        chain
            .register_miner_with_profile(consumer_gpu_miner, 100, HardwareClass::ConsumerGpu, 9_000)
            .unwrap();
        chain
            .register_miner_with_profile(
                datacenter_gpu_miner,
                100,
                HardwareClass::DatacenterGpu,
                8_000,
            )
            .unwrap();
        chain
            .register_miner_with_profile(other_miner, 100, HardwareClass::Other, 0)
            .unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let matmul_job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) =
            TensorOpReceipt::from_job(&matmul_job, cpu_miner, 1, 5).unwrap();
        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"ws-linear-model"]),
            step: 3,
            batch_seed: hash_bytes(b"test", &[b"ws-linear-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 1,
            deadline_block: 20,
        });
        chain.submit_job(JobState::TensorOp(matmul_job));
        chain.submit_job(JobState::LinearTrainingStep(linear_job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain.mark_receipt_settled_for_testing(receipt.receipt_id);
        chain.register_validator(cpu_miner, 10_000).unwrap();
        chain.produce_block(cpu_miner, 1000).unwrap();
        let rpc = RpcNode::new(chain);

        let miners = rpc.explorer_websocket_response("miners");
        assert!(miners.contains("\"hardware_class\":\"cpu\""));
        assert!(miners.contains("\"hardware_class\":\"consumer_gpu\""));
        assert!(miners.contains("\"hardware_class\":\"datacenter_gpu\""));
        assert!(miners.contains("\"hardware_class\":\"other\""));
        let validators = rpc.explorer_websocket_response("{\"type\":\"validators\"}");
        assert!(validators.contains("\"valid_attestations\""));
        let jobs = rpc.explorer_websocket_response("{\"type\":\"jobs\",\"job_limit\":2}");
        assert!(jobs.contains("\"primitive_type\":\"tensor_op\""));
        assert!(jobs.contains("\"primitive_type\":\"linear_training_step\""));
        let receipts =
            rpc.explorer_websocket_response("{\"type\":\"receipts\",\"receipt_limit\":1}");
        assert!(receipts.contains("\"primitive_type\":\"tensor_op\""));
        assert!(receipts.contains("\"attestation_count\":0"));
        assert!(receipts.contains("\"validator_attestations\":[]"));
        assert!(receipts.contains("\"settled\":true"));
        let blocks = rpc.explorer_websocket_response("{\"type\":\"blocks\",\"block_limit\":1}");
        assert!(blocks.contains("\"blocks\""));
        let summary = rpc.explorer_websocket_response("summary");
        assert!(summary.contains("\"type\":\"summary\""));
        let missing_account = rpc.explorer_websocket_response("{\"type\":\"account\"}");
        assert!(missing_account.contains("missing account address"));
        let invalid_account =
            rpc.explorer_websocket_response("{\"type\":\"account\",\"address\":\"bad\"}");
        assert!(invalid_account.contains("invalid account address"));

        assert_eq!(primitive_label(PrimitiveType::TensorOp), "tensor_op");
        assert_eq!(
            primitive_label(PrimitiveType::LinearTrainingStep),
            "linear_training_step"
        );
        assert_eq!(hardware_class_label(HardwareClass::Cpu), "cpu");
        assert_eq!(
            hardware_class_label(HardwareClass::ConsumerGpu),
            "consumer_gpu"
        );
        assert_eq!(
            hardware_class_label(HardwareClass::DatacenterGpu),
            "datacenter_gpu"
        );
        assert_eq!(hardware_class_label(HardwareClass::Other), "other");
    }

    #[test]
    fn rpc_http_reader_handles_in_memory_requests_and_limits() {
        let mut get = std::io::Cursor::new(b"GET /chain/head HTTP/1.1\r\n\r\n");
        assert!(matches!(
            read_http_request_from(&mut get, 16).unwrap(),
            ParsedHttpRequest::Request {
                request,
                auth_token: None,
            } if request.method == "GET" && request.path == "/chain/head" && request.body.is_empty()
        ));

        let mut post = std::io::Cursor::new(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbody");
        assert!(matches!(
            read_http_request_from(&mut post, 16).unwrap(),
            ParsedHttpRequest::Request { request, .. } if request.body == b"body"
        ));

        let mut empty = std::io::Cursor::new(Vec::<u8>::new());
        assert!(matches!(
            read_http_request_from(&mut empty, 16).unwrap(),
            ParsedHttpRequest::BadRequest
        ));

        let too_large = vec![b'x'; 8 * 1024 + 17];
        let mut too_large = std::io::Cursor::new(too_large);
        assert!(matches!(
            read_http_request_from(&mut too_large, 16).unwrap(),
            ParsedHttpRequest::TooLarge
        ));
    }

    #[test]
    fn rpc_gateway_enforces_auth_body_limits_and_rate_limits() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut gateway = RpcGateway::new(
            RpcNode::new(Chain::new(beacon)),
            RpcPolicy {
                auth_token: Some("secret".to_owned()),
                max_body_bytes: 8,
                max_requests_per_client: 1,
            },
        );
        let request = RpcRequest {
            method: "GET".to_owned(),
            path: "/chain/head".to_owned(),
            body: Vec::new(),
        };

        assert_eq!(gateway.handle("client", None, &request).status, 401);
        assert_eq!(gateway.request_count("client"), 0);

        let oversized = RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: b"too many bytes".to_vec(),
        };
        assert_eq!(
            gateway.handle("client", Some("secret"), &oversized).status,
            413
        );
        assert_eq!(gateway.request_count("client"), 0);

        assert_eq!(
            gateway.handle("client", Some("secret"), &request).status,
            200
        );
        assert_eq!(
            gateway.handle("client", Some("secret"), &request).status,
            429
        );
    }

    #[test]
    fn tensor_rpc_serves_descriptor_rows_chunks_and_openings() {
        let chain = Chain::new(hash_bytes(b"test", &[b"beacon"]));
        let mut rpc = RpcNode::new(chain);
        let empty_latest = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/tensor/latest".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(empty_latest.status, 404);

        let tensor =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let commitment_root = tensor.commitment_root();
        let tensor_id = rpc.insert_tensor(tensor);
        assert!(rpc.contains_tensor_commitment_root(&commitment_root));
        assert_eq!(
            rpc.tensor_by_commitment_root(&commitment_root)
                .map(Tensor::tensor_id),
            Some(tensor_id)
        );

        for path in [
            "/tensor/latest".to_owned(),
            format!("/tensor/{}/descriptor", hex(&tensor_id)),
            format!("/tensor/{}/row/1", hex(&tensor_id)),
            format!("/tensor/{}/chunk/0", hex(&tensor_id)),
            format!("/tensor/{}/opening/0", hex(&tensor_id)),
        ] {
            let response = rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path,
                body: Vec::new(),
            });
            assert_eq!(response.status, 200);
        }
    }

    #[test]
    fn rpc_node_synthetic_round_retains_live_tensors_for_rpc_fetch() {
        let mut empty_rpc =
            RpcNode::new(Chain::new(hash_bytes(b"test", &[b"rpc-empty-synthetic"])));
        assert_eq!(empty_rpc.produce_synthetic_cpu_round().unwrap(), None);

        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"rpc-live-tensors"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("rpc-live-tensor-miner-{index}").as_bytes()),
                    chain.params.miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("rpc-live-tensor-validator-{index}").as_bytes()),
                    chain.params.validator_min_stake,
                )
                .unwrap();
        }
        let mut rpc = RpcNode::new(chain);

        assert_eq!(
            rpc.produce_synthetic_cpu_round_with_profile(&ChainProfile::public_testnet())
                .unwrap(),
            None
        );
        assert_eq!(
            rpc.produce_synthetic_cpu_round_with_profile(&ChainProfile::local_cpu())
                .unwrap(),
            Some(1)
        );
        assert_eq!(rpc.produce_synthetic_cpu_round().unwrap(), Some(2));
        let latest = rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/tensor/latest".to_owned(),
            body: Vec::new(),
        });
        assert_eq!(latest.status, 200);
        assert!(latest.body.contains("\"tensor_count\":9"));
    }

    #[test]
    fn rpc_formats_http_response() {
        let response = RpcResponse {
            status: 202,
            body: "{\"accepted\":true}".to_owned(),
        };
        let text = http_response_text(&response);
        assert!(text.starts_with("HTTP/1.1 202 Accepted"));
        assert!(text.ends_with("{\"accepted\":true}"));
        let conflict = http_response_text(&RpcResponse {
            status: 409,
            body: "{\"error\":\"duplicate transaction\"}".to_owned(),
        });
        assert!(conflict.starts_with("HTTP/1.1 409 Conflict"));
        let limited = http_response_text(&RpcResponse {
            status: 429,
            body: "{\"error\":\"rate limit exceeded\"}".to_owned(),
        });
        assert!(limited.starts_with("HTTP/1.1 429 Too Many Requests"));
        let html = http_response_text(&RpcResponse {
            status: 200,
            body: "<!doctype html><html></html>".to_owned(),
        });
        assert!(html.contains("content-type: text/html; charset=utf-8"));
    }

    #[test]
    fn rpc_http_server_serves_socket_request() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let gateway = RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default());
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        assert_eq!(server.gateway().request_count("unseen-client"), 0);
        server.set_nonblocking(true).unwrap();
        server.set_nonblocking(false).unwrap();
        server.gateway_mut().policy.max_body_bytes = 32;
        assert_eq!(server.gateway().policy.max_body_bytes, 32);
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"GET /chain/head HTTP/1.1\r\nhost: localhost\r\n\r\n")
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server_thread.join().unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"height\":0"));
    }

    #[test]
    fn rpc_http_server_serves_explorer_websocket_poll() {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"ws-http-miner");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(miner, 10_000).unwrap();
        chain.produce_block(miner, 1).unwrap();
        let gateway = RpcGateway::new(RpcNode::new(chain), RpcPolicy::default());
        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
        let mut handshake = Vec::new();
        let mut byte = [0_u8; 1];
        while !handshake.ends_with(b"\r\n\r\n") {
            client.read_exact(&mut byte).unwrap();
            handshake.push(byte[0]);
        }
        client
            .write_all(&masked_websocket_text_frame(
                "{\"type\":\"overview\",\"block_limit\":1}",
            ))
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        server_thread.join().unwrap();
        let mut full_response = handshake;
        full_response.extend_from_slice(&response);
        let response = String::from_utf8_lossy(&full_response);

        assert!(response.contains("101 Switching Protocols"));
        assert!(response.contains("sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        assert!(response.contains("\"type\":\"overview\""));
        assert!(response.contains("\"block_count\":1"));
    }

    #[test]
    fn rpc_http_server_rejects_bad_websocket_routes_and_auth() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let bad_route = serve_one_http_request(
            RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default()),
            b"GET /wrong/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
        assert!(bad_route.starts_with("HTTP/1.1 404 Not Found"));

        let unauthorized = serve_one_http_request(
            RpcGateway::new(
                RpcNode::new(Chain::new(beacon)),
                RpcPolicy {
                    auth_token: Some("secret".to_owned()),
                    max_body_bytes: 1024,
                    max_requests_per_client: 10,
                },
            ),
            b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
        assert!(unauthorized.starts_with("HTTP/1.1 401 Unauthorized"));
    }

    #[test]
    fn websocket_frame_helpers_cover_close_errors_and_extended_lengths() {
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpListener, TcpStream};

        assert_eq!(
            read_single_websocket_frame(&[0x81, 126, 0, 126])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::UnexpectedEof
        );
        let mut extended_16 = vec![0x81, 126];
        extended_16.extend_from_slice(&(126_u16).to_be_bytes());
        extended_16.extend(std::iter::repeat_n(b'a', 126));
        assert_eq!(
            read_single_websocket_frame(&extended_16).unwrap(),
            Some("a".repeat(126))
        );
        let mut extended_64 = vec![0x81, 127];
        extended_64.extend_from_slice(&(3_u64).to_be_bytes());
        extended_64.extend_from_slice(b"hey");
        assert_eq!(
            read_single_websocket_frame(&extended_64).unwrap(),
            Some("hey".to_owned())
        );
        let mut too_large = vec![0x81, 127];
        too_large.extend_from_slice(&((64_u64 * 1024) + 1).to_be_bytes());
        assert_eq!(
            read_single_websocket_frame(&too_large).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_single_websocket_frame(&[0x81, 1, 0xff])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_single_websocket_frame(&[0x82, 0]).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(read_single_websocket_frame(&[0x88, 0]).unwrap(), None);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let writer = std::thread::spawn(move || {
            let (mut server, _) = listener.accept().unwrap();
            let small_payload = [b'a'; 126];
            let large_payload = vec![b'b'; 65_536];
            write_websocket_frame(&mut server, 0x1, &small_payload).unwrap();
            write_websocket_frame(&mut server, 0x1, &large_payload).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        let mut raw = Vec::new();
        client.read_to_end(&mut raw).unwrap();
        writer.join().unwrap();
        assert_eq!(raw[1], 126);
        assert_eq!(u16::from_be_bytes([raw[2], raw[3]]), 126);
        let second = 4 + 126;
        assert_eq!(raw[second + 1], 127);
        assert_eq!(
            u64::from_be_bytes(raw[second + 2..second + 10].try_into().unwrap()),
            65_536
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let rpc = RpcNode::new(Chain::new(hash_bytes(b"test", &[b"beacon"])));
        let server_thread = std::thread::spawn(move || {
            let (mut server, _) = listener.accept().unwrap();
            rpc.serve_explorer_websocket_once(&mut server).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(&[0x88, 0]).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut close_response = Vec::new();
        client.read_to_end(&mut close_response).unwrap();
        server_thread.join().unwrap();
        assert_eq!(close_response, vec![0x88, 0]);

        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn websocket_json_and_query_helpers_handle_escaping_and_decoding() {
        let escaped =
            json_string_field("{\"address\":\"a\\\"b\\\\c\\n\\r\\t\\x\"}", "address").unwrap();
        assert_eq!(escaped, "a\"b\\c\n\r\tx");
        assert!(json_string_field("{\"address\":\"unterminated", "address").is_none());
        assert_eq!(
            json_usize_field("{\"limit\":123,\"next\":1}", "limit"),
            Some(123)
        );
        assert!(json_usize_field("{\"limit\":nope}", "limit").is_none());
        let (path, token) = split_path_and_auth_token("/explorer/ws?x=1&token=a%20b+z%2f%ZZ");
        assert_eq!(path, "/explorer/ws");
        assert_eq!(token.as_deref(), Some("a b z/%ZZ"));
    }

    fn serve_one_http_request(gateway: RpcGateway, request: &[u8]) -> String {
        use std::io::ErrorKind;
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};

        let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                return String::new();
            }
            Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || server.serve_next().unwrap());
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(request).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server_thread.join().unwrap();
        response
    }

    fn read_single_websocket_frame(frame: &[u8]) -> std::io::Result<Option<String>> {
        use std::io::Write;
        use std::net::{Shutdown, TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let mut client = TcpStream::connect(addr)?;
        let (mut server, _) = listener.accept()?;
        client.write_all(frame)?;
        client.shutdown(Shutdown::Write)?;
        read_websocket_text_frame(&mut server)
    }

    fn masked_websocket_text_frame(text: &str) -> Vec<u8> {
        let mask = [1_u8, 2, 3, 4];
        let bytes = text.as_bytes();
        assert!(bytes.len() < 126);
        let mut frame = vec![0x81, 0x80 | bytes.len() as u8];
        frame.extend_from_slice(&mask);
        for (index, byte) in bytes.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        frame
    }
}
