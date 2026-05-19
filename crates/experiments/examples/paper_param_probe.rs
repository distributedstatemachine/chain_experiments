use std::time::{Duration, Instant};

use experiments::attack::{solve_zero_job_shortcut, zero_job_shortcut_work};
use experiments::{CuPowParams, Matrix, Result, solve, verify};

fn main() -> Result<()> {
    for (n, r, rounds) in [
        (64, 4, 5),
        (128, 4, 4),
        (128, 8, 3),
        (256, 8, 2),
        (256, 16, 1),
    ] {
        probe(n, r, rounds)?;
    }
    Ok(())
}

fn probe(n: usize, r: usize, rounds: u8) -> Result<()> {
    let params = CuPowParams {
        tile: r,
        rank: r,
        difficulty_bits: 0,
    };
    let zero = Matrix::zeros(n, n);
    let estimate = zero_job_shortcut_work(n, params)?;
    let mut honest_total = Duration::ZERO;
    let mut shortcut_total = Duration::ZERO;

    for round in 0..rounds {
        let mut seed = [0_u8; 32];
        seed[0] = round;
        seed[1] = n as u8;
        seed[2] = r as u8;

        let started = Instant::now();
        let honest = solve(&seed, &zero, &zero, params)?;
        honest_total += started.elapsed();

        let started = Instant::now();
        let shortcut = solve_zero_job_shortcut(&seed, n, params)?;
        shortcut_total += started.elapsed();

        assert_eq!(shortcut, honest);
        assert!(verify(&seed, &zero, &zero, &shortcut, params)?);
    }

    let honest_us = honest_total.as_micros() as f64 / f64::from(rounds);
    let shortcut_us = shortcut_total.as_micros() as f64 / f64::from(rounds);
    println!(
        "n={n:<3} r=tile={r:<2} rounds={rounds}: honest={honest_us:>10.1}us shortcut={shortcut_us:>10.1}us measured_ratio={:.3}x scalar_ratio={:.3}x",
        honest_us / shortcut_us.max(1.0),
        estimate.estimated_speedup(),
    );

    Ok(())
}
