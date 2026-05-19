use std::time::Instant;

use experiments::hash::{hex, leading_zero_bits};
use experiments::oracle::OracleRng;
use experiments::{Chain, CuPowParams, Matrix, MatrixJob};

fn main() -> experiments::Result<()> {
    let mut rng = OracleRng::new(b"pearl-chain-example", &[b"mine"]);
    let left = Matrix::random(16, 16, &mut rng);
    let right = Matrix::random(16, 16, &mut rng);
    let job = MatrixJob::new(
        left,
        right,
        CuPowParams {
            tile: 4,
            rank: 4,
            difficulty_bits: 8,
        },
    )?;

    let started = Instant::now();
    let mut chain = Chain::new();
    let block = chain.append_mined(job, 10_000)?;
    let elapsed = started.elapsed();

    println!("height: {}", block.header.height);
    println!("nonce: {}", block.header.nonce);
    println!(
        "transcript_hash: {} ({} leading zero bits)",
        hex(&block.header.transcript_hash),
        leading_zero_bits(&block.header.transcript_hash)
    );
    println!("block_hash: {}", hex(&block.hash()));
    println!("elapsed_ms: {}", elapsed.as_millis());

    Ok(())
}
