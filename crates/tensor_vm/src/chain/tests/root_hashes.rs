use super::*;

#[test]
fn miner_root_commits_to_operator_identity() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"operator-root-miner");
    chain
        .register_miner_with_operator(
            miner,
            chain.params().miner_min_stake,
            address(b"operator-root-a"),
        )
        .unwrap();

    let original_root = miner_root(chain.state().miners());
    let mut changed_miners = chain.state().miners().clone();
    changed_miners.get_mut(&miner).unwrap().operator_id = address(b"operator-root-b");

    assert_ne!(original_root, miner_root(&changed_miners));
}
