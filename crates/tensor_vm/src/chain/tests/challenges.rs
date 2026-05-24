use super::*;

#[test]
fn challenge_outcome_slashes_miner_and_credits_treasury() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyChallengeOutcome(
                ChallengeOutcome::ProvenInvalid {
                    dishonest_party: miner,
                    slash_amount: 25,
                    reason: "invalid receipt".to_owned(),
                },
            ))
            .unwrap(),
        vec![ChainEvent::ChallengeProvenInvalid {
            dishonest_party: miner,
            slash_amount: 25,
            reason: "invalid receipt".to_owned(),
        }]
    );
    chain
        .apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
            dishonest_party: miner,
            slash_amount: 5,
            reason: "invalid receipt again".to_owned(),
        })
        .unwrap();
    assert_eq!(chain.state().miners().get(&miner).unwrap().stake, 70);
    assert_eq!(chain.state().miners().get(&miner).unwrap().reputation, -20);
    assert_eq!(chain.state().rewards().treasury(), 30);
}
