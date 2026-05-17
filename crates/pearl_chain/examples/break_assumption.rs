use pearl_chain::attack::{solve_zero_job_shortcut, zero_job_shortcut_work};
use pearl_chain::hash::hex;
use pearl_chain::{CuPowParams, Matrix, Result, solve, verify};

fn main() -> Result<()> {
    unsafe_params_are_rejected();
    paper_params_still_verify()?;
    Ok(())
}

fn unsafe_params_are_rejected() {
    let unsafe_params = CuPowParams {
        tile: 32,
        rank: 1,
        difficulty_bits: 0,
    };

    let err = zero_job_shortcut_work(256, unsafe_params).unwrap_err();
    println!("legacy rank < tile attack accepted: no");
    println!("rejection: {err}");
}

fn paper_params_still_verify() -> Result<()> {
    let n = 32;
    let params = CuPowParams {
        tile: 8,
        rank: 8,
        difficulty_bits: 0,
    };
    let seed = [42_u8; 32];
    let zero = Matrix::zeros(n, n);

    let honest = solve(&seed, &zero, &zero, params)?;
    let factorized = solve_zero_job_shortcut(&seed, n, params)?;

    assert_eq!(factorized, honest);
    assert!(verify(&seed, &zero, &zero, &factorized, params)?);

    let estimate = zero_job_shortcut_work(n, params)?;
    println!("paper rank == tile transcript equality: yes");
    println!(
        "estimated scalar term ratio honest/factorized: {:.2}x",
        estimate.estimated_speedup()
    );
    println!(
        "transcript hash: {}",
        hex(&factorized.proof.transcript_hash)
    );

    Ok(())
}
