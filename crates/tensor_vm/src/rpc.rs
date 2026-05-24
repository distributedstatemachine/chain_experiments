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

    mod http;
    mod routes;

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
}
