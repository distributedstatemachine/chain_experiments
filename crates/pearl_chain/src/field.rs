//! Prime-field arithmetic used by the matrix kernels.
//!
//! The modulus is the Mersenne prime 2^31 - 1. It keeps products inside u64 and
//! lets the hot dot-product paths accumulate in u128 with one reduction per cell.

pub type Elem = u64;

pub const MODULUS: Elem = 2_147_483_647;

#[inline]
pub fn normalize(value: Elem) -> Elem {
    value % MODULUS
}

#[inline]
pub fn add(lhs: Elem, rhs: Elem) -> Elem {
    let sum = lhs + rhs;
    if sum >= MODULUS { sum - MODULUS } else { sum }
}

#[inline]
pub fn sub(lhs: Elem, rhs: Elem) -> Elem {
    if lhs >= rhs {
        lhs - rhs
    } else {
        MODULUS - (rhs - lhs)
    }
}

#[inline]
pub fn mul(lhs: Elem, rhs: Elem) -> Elem {
    reduce_u128(lhs as u128 * rhs as u128)
}

#[inline]
pub fn reduce_u128(value: u128) -> Elem {
    (value % MODULUS as u128) as Elem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_wraps_correctly() {
        assert_eq!(add(MODULUS - 1, 2), 1);
        assert_eq!(sub(1, 2), MODULUS - 1);
        assert_eq!(mul(MODULUS - 1, MODULUS - 1), 1);
    }
}
