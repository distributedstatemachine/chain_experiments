use super::*;

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

    let tensor = Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
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
    let mut empty_rpc = RpcNode::new(Chain::new(hash_bytes(b"test", &[b"rpc-empty-synthetic"])));
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
