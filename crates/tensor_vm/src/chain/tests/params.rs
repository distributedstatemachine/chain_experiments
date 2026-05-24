use super::*;

#[test]
fn chain_params_define_tensor_retention_deadline() {
    let params = ChainParams {
        epoch_length: 50,
        reward_settlement_delay_epochs: 2,
        challenge_window_epochs: 3,
        ..ChainParams::default()
    };
    assert_eq!(params.tensor_retention_window_blocks(), 250);
    assert_eq!(params.tensor_retention_deadline(10), 260);
}
