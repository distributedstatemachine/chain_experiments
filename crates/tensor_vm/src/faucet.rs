use crate::chain::RewardState;
use crate::error::{Result, TvmError};
use crate::types::Address;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Faucet {
    balance: u64,
    drip_amount: u64,
    claims: BTreeMap<Address, u64>,
}

impl Faucet {
    pub fn new(balance: u64, drip_amount: u64) -> Self {
        Self {
            balance,
            drip_amount,
            claims: BTreeMap::new(),
        }
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }

    pub fn drip_amount(&self) -> u64 {
        self.drip_amount
    }

    pub fn claim(
        &mut self,
        address: Address,
        epoch: u64,
        rewards: &mut RewardState,
    ) -> Result<u64> {
        if self.claims.get(&address).copied() == Some(epoch) {
            return Err(TvmError::InvalidReceipt(
                "faucet already claimed this epoch",
            ));
        }
        if self.balance < self.drip_amount {
            return Err(TvmError::InvalidReceipt("faucet exhausted"));
        }
        self.balance -= self.drip_amount;
        self.claims.insert(address, epoch);
        rewards.credit(address, self.drip_amount);
        Ok(self.drip_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::address;

    #[test]
    fn faucet_drips_once_per_epoch() {
        let mut faucet = Faucet::new(1_000, 100);
        let user = address(b"user");
        let mut rewards = RewardState::default();
        assert_eq!(faucet.claim(user, 0, &mut rewards).unwrap(), 100);
        assert_eq!(rewards.balance(&user), 100);
        assert!(faucet.claim(user, 0, &mut rewards).is_err());
        assert_eq!(faucet.claim(user, 1, &mut rewards).unwrap(), 100);
        assert_eq!(faucet.balance(), 800);
    }
}
