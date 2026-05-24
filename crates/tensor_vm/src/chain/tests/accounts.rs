use super::*;

#[test]
fn chain_tracks_accounts_jobs_and_transfers() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let alice = address(b"alice");
    let bob = address(b"bob");
    chain.credit_account(alice, 1_000);
    chain.transfer(alice, bob, 250).unwrap();
    assert_eq!(chain.state.accounts.get(&alice).unwrap().balance, 750);
    assert_eq!(chain.state.accounts.get(&alice).unwrap().nonce, 1);
    assert_eq!(chain.state.accounts.get(&bob).unwrap().balance, 250);

    let job = MatmulJob::synthetic(0, 3, 2, 2, 2, &beacon, 10);
    chain.submit_job(JobState::TensorOp(job.clone()));
    assert_eq!(chain.job(&job.job_id).unwrap().deadline_block(), 10);
}
