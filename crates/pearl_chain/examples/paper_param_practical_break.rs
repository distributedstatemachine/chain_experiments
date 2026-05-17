use std::time::{Duration, Instant};

use pearl_chain::attack::{solve_zero_job_no_decode, solve_zero_job_shortcut};
use pearl_chain::hash::{hex, leading_zero_bits};
use pearl_chain::{
    Block, BlockHeader, Chain, CuPowParams, Matrix, MatrixJob, Result, solve, verify,
};

fn main() -> Result<()> {
    let n = 64;
    let mining_params = CuPowParams {
        tile: 4,
        rank: 4,
        difficulty_bits: 8,
    };
    let benchmark_params = CuPowParams {
        difficulty_bits: 0,
        ..mining_params
    };
    let zero = Matrix::zeros(n, n);

    let (honest_avg, shortcut_avg, no_decode_avg) =
        benchmark_proof_attempts(&zero, benchmark_params, 8)?;
    println!("valid paper params: rank == tile == {}", mining_params.rank);
    println!("zero job dimensions: {n}x{n}");
    println!(
        "honest average proof attempt: {} us",
        honest_avg.as_micros()
    );
    println!(
        "shortcut average proof attempt: {} us",
        shortcut_avg.as_micros()
    );
    println!(
        "no-decode average proof attempt: {} us",
        no_decode_avg.as_micros()
    );
    let ratio = honest_avg.as_nanos() as f64 / shortcut_avg.as_nanos().max(1) as f64;
    let no_decode_ratio = honest_avg.as_nanos() as f64 / no_decode_avg.as_nanos().max(1) as f64;
    println!("finite-size measured ratio honest/factorized: {ratio:.2}x");
    println!("finite-size measured ratio honest/no-decode: {no_decode_ratio:.2}x");

    let block = mine_with_shortcut(zero, mining_params, 20_000)?;
    println!("no-decode-mined paper-param block accepted: yes");
    if no_decode_ratio > 1.2 {
        println!("attack status: practical implementation-level advantage observed");
    } else {
        println!("attack status: no reliable finite-size advantage observed in this run");
    }
    println!("accepted nonce: {}", block.header.nonce);
    println!(
        "accepted transcript hash: {} ({} leading zero bits)",
        hex(&block.header.transcript_hash),
        leading_zero_bits(&block.header.transcript_hash)
    );

    Ok(())
}

fn benchmark_proof_attempts(
    zero: &Matrix,
    params: CuPowParams,
    rounds: u8,
) -> Result<(Duration, Duration, Duration)> {
    let mut honest_total = Duration::ZERO;
    let mut shortcut_total = Duration::ZERO;
    let mut no_decode_total = Duration::ZERO;

    for round in 0..rounds {
        let mut seed = [0_u8; 32];
        seed[0] = round;
        seed[1] = zero.rows() as u8;
        seed[2] = params.rank as u8;

        let started = Instant::now();
        let honest = solve(&seed, zero, zero, params)?;
        honest_total += started.elapsed();

        let started = Instant::now();
        let shortcut = solve_zero_job_shortcut(&seed, zero.rows(), params)?;
        shortcut_total += started.elapsed();

        let started = Instant::now();
        let no_decode = solve_zero_job_no_decode(&seed, zero.rows(), params)?;
        no_decode_total += started.elapsed();

        assert_eq!(shortcut, honest);
        assert_eq!(no_decode, honest);
        assert!(verify(&seed, zero, zero, &shortcut, params)?);
        assert!(verify(&seed, zero, zero, &no_decode, params)?);
    }

    Ok((
        honest_total / u32::from(rounds),
        shortcut_total / u32::from(rounds),
        no_decode_total / u32::from(rounds),
    ))
}

fn mine_with_shortcut(zero: Matrix, params: CuPowParams, max_nonce: u64) -> Result<Block> {
    let job = MatrixJob::new(zero.clone(), zero, params)?;
    let job_hash = job.commitment();
    let timestamp_ms = 1_700_000_000_000;

    for nonce in 0..=max_nonce {
        let mut header = BlockHeader {
            height: 0,
            prev_hash: [0; 32],
            timestamp_ms,
            nonce,
            difficulty_bits: params.difficulty_bits,
            job_hash,
            product_hash: [0; 32],
            transcript_hash: [0; 32],
        };
        let seed = header.mining_seed();
        let solution = solve_zero_job_no_decode(&seed, job.left.rows(), params)?;
        if !solution.proof.meets_difficulty(params.difficulty_bits) {
            continue;
        }

        header.product_hash = solution.product.commitment();
        header.transcript_hash = solution.proof.transcript_hash;
        let block = Block {
            header,
            job,
            solution,
        };
        assert!(block.validate(0, [0; 32])?);

        let mut chain = Chain::new();
        chain.append(block.clone())?;
        assert!(chain.validate()?);
        return Ok(block);
    }

    panic!("shortcut mining did not find a block within nonce bound");
}
