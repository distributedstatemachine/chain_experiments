use std::time::{Duration, Instant};

use experiments::hash::{Sha256, hex};

const N: usize = 256;
const R: usize = 64;
const BLOCKS: usize = N / R;

fn main() {
    let mut rng = XorShift64::new(0x5eed_c0de_1234_5678);
    let el = random_rows(N, &mut rng);
    let er = random_wide_rows(R, &mut rng);
    let fl = random_rows(N, &mut rng);
    let fr = random_wide_rows(R, &mut rng);

    assert_eq!(rank_narrow(&el), R);
    assert_eq!(rank_wide(&er), R);
    assert_eq!(rank_narrow(&fl), R);
    assert_eq!(rank_wide(&fr), R);

    let e = low_rank_product(&el, &er);
    let f = low_rank_product(&fl, &fr);

    let (scalar_hash, scalar_elapsed) = time(|| scalar_transcript_hash(&e, &f));
    let (packed_hash, packed_elapsed) = time(|| bitpacked_transcript_hash(&e, &f));

    assert_eq!(scalar_hash, packed_hash);

    let scalar_field_terms = (BLOCKS * BLOCKS * BLOCKS * R * R * R) as u128;
    let packed_row_xors_estimate = (BLOCKS * BLOCKS * BLOCKS * R * (R / 2)) as u128;

    println!("GF(2) bitpacked transcript break");
    println!("n={N}, rank=tile={R}, blocks={BLOCKS}");
    println!("sampled low-rank factors are full rank: yes");
    println!("same transcript hash: {}", hex(&packed_hash));
    println!("scalar field terms: {scalar_field_terms}");
    println!("packed row-xor estimate: {packed_row_xors_estimate}");
    println!(
        "estimated operation ratio scalar/packed: {:.2}x",
        scalar_field_terms as f64 / packed_row_xors_estimate as f64
    );
    println!("scalar elapsed: {} us", scalar_elapsed.as_micros());
    println!("packed elapsed: {} us", packed_elapsed.as_micros());
    println!(
        "measured ratio scalar/packed: {:.2}x",
        scalar_elapsed.as_nanos() as f64 / packed_elapsed.as_nanos().max(1) as f64
    );
}

fn time<F: FnOnce() -> [u8; 32]>(f: F) -> ([u8; 32], Duration) {
    let started = Instant::now();
    let out = f();
    (out, started.elapsed())
}

fn random_rows(rows: usize, rng: &mut XorShift64) -> Vec<u64> {
    (0..rows).map(|_| rng.next()).collect()
}

fn random_wide_rows(rows: usize, rng: &mut XorShift64) -> Vec<[u64; BLOCKS]> {
    (0..rows)
        .map(|_| {
            let mut row = [0_u64; BLOCKS];
            for word in &mut row {
                *word = rng.next();
            }
            row
        })
        .collect()
}

fn low_rank_product(left: &[u64], right: &[[u64; BLOCKS]]) -> Vec<[u64; BLOCKS]> {
    let mut out = vec![[0_u64; BLOCKS]; N];
    for row in 0..N {
        let mut bits = left[row];
        while bits != 0 {
            let bit = bits.trailing_zeros() as usize;
            for block in 0..BLOCKS {
                out[row][block] ^= right[bit][block];
            }
            bits &= bits - 1;
        }
    }
    out
}

fn scalar_transcript_hash(e: &[[u64; BLOCKS]], f: &[[u64; BLOCKS]]) -> [u8; 32] {
    let mut hasher = transcript_hasher();
    for bi in 0..BLOCKS {
        for bj in 0..BLOCKS {
            let mut cumulative = [0_u64; R];
            for bk in 0..BLOCKS {
                scalar_add_tile_product(&mut cumulative, e, f, bi, bj, bk);
                hash_tile(&mut hasher, &cumulative, bi, bj, bk);
            }
        }
    }
    hasher.finalize()
}

fn bitpacked_transcript_hash(e: &[[u64; BLOCKS]], f: &[[u64; BLOCKS]]) -> [u8; 32] {
    let mut hasher = transcript_hasher();
    for bi in 0..BLOCKS {
        for bj in 0..BLOCKS {
            let mut cumulative = [0_u64; R];
            for bk in 0..BLOCKS {
                bitpacked_add_tile_product(&mut cumulative, e, f, bi, bj, bk);
                hash_tile(&mut hasher, &cumulative, bi, bj, bk);
            }
        }
    }
    hasher.finalize()
}

fn transcript_hasher() -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update(b"pearl-chain-gf2-transcript-v1");
    hasher.update_usize(N);
    hasher.update_usize(N);
    hasher.update_usize(N);
    hasher.update_usize(R);
    hasher
}

fn scalar_add_tile_product(
    cumulative: &mut [u64; R],
    e: &[[u64; BLOCKS]],
    f: &[[u64; BLOCKS]],
    bi: usize,
    bj: usize,
    bk: usize,
) {
    let row_base = bi * R;
    let inner_base = bk * R;
    for ii in 0..R {
        let mut out_row = 0_u64;
        for jj in 0..R {
            let mut parity = 0_u32;
            for kk in 0..R {
                let lhs = (e[row_base + ii][bk] >> kk) & 1;
                let rhs = (f[inner_base + kk][bj] >> jj) & 1;
                parity ^= (lhs & rhs) as u32;
            }
            out_row |= u64::from(parity) << jj;
        }
        cumulative[ii] ^= out_row;
    }
}

fn bitpacked_add_tile_product(
    cumulative: &mut [u64; R],
    e: &[[u64; BLOCKS]],
    f: &[[u64; BLOCKS]],
    bi: usize,
    bj: usize,
    bk: usize,
) {
    let row_base = bi * R;
    let inner_base = bk * R;
    for ii in 0..R {
        let mut bits = e[row_base + ii][bk];
        let mut out_row = 0_u64;
        while bits != 0 {
            let kk = bits.trailing_zeros() as usize;
            out_row ^= f[inner_base + kk][bj];
            bits &= bits - 1;
        }
        cumulative[ii] ^= out_row;
    }
}

fn hash_tile(hasher: &mut Sha256, tile: &[u64; R], bi: usize, bj: usize, bk: usize) {
    hasher.update_usize(bi);
    hasher.update_usize(bj);
    hasher.update_usize(bk);
    for row in tile {
        hasher.update_u64(*row);
    }
}

fn rank_narrow(rows: &[u64]) -> usize {
    let mut basis = [0_u64; R];
    let mut rank = 0;
    for row in rows {
        let mut x = *row;
        while x != 0 {
            let pivot = 63 - x.leading_zeros() as usize;
            if basis[pivot] == 0 {
                basis[pivot] = x;
                rank += 1;
                break;
            }
            x ^= basis[pivot];
        }
    }
    rank
}

fn rank_wide(rows: &[[u64; BLOCKS]]) -> usize {
    let mut basis = vec![[0_u64; BLOCKS]; N];
    let mut rank = 0;
    for row in rows {
        let mut x = *row;
        while let Some(pivot) = highest_bit(&x) {
            if basis[pivot] == [0; BLOCKS] {
                basis[pivot] = x;
                rank += 1;
                break;
            }
            for (dst, src) in x.iter_mut().zip(basis[pivot]) {
                *dst ^= src;
            }
        }
    }
    rank
}

fn highest_bit(row: &[u64; BLOCKS]) -> Option<usize> {
    for block in (0..BLOCKS).rev() {
        if row[block] != 0 {
            let bit = 63 - row[block].leading_zeros() as usize;
            return Some(block * R + bit);
        }
    }
    None
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}
