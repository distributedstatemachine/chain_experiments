use crate::chain::Transaction;
use crate::error::{Result, TvmError};
use crate::types::{Hash, parse_hash_hex};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionEnvelope {
    pub sender: Option<crate::types::Address>,
    pub transaction: Transaction,
}

#[derive(Clone, Debug, Default)]
pub struct TxPool {
    queue: VecDeque<Transaction>,
    seen_receipts: BTreeSet<Hash>,
    seen_attestations: BTreeSet<Hash>,
}

impl TxPool {
    pub fn submit(&mut self, tx: Transaction) -> bool {
        if !self.mark_seen(&tx) {
            return false;
        }
        self.queue.push_back(tx);
        true
    }

    pub fn submit_envelope(&mut self, envelope: &TransactionEnvelope) -> bool {
        self.submit(envelope.transaction.clone())
    }

    pub fn pop(&mut self) -> Option<Transaction> {
        self.queue.pop_front()
    }

    pub fn drain(&mut self) -> Vec<Transaction> {
        self.queue.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn mark_seen(&mut self, tx: &Transaction) -> bool {
        match tx {
            Transaction::SubmitTensorOpReceipt(id)
            | Transaction::SubmitLinearTrainingStepReceipt(id) => self.seen_receipts.insert(*id),
            Transaction::SubmitAttestation(id) => {
                self.seen_attestations.insert(*id);
                true
            }
            _ => true,
        }
    }
}

pub fn parse_transaction_envelope(body: &[u8]) -> Result<TransactionEnvelope> {
    let body = TransactionBody::parse(body)?;
    body.envelope()
}

struct TransactionBody<'a> {
    kind: &'a str,
    args: Vec<&'a str>,
}

impl<'a> TransactionBody<'a> {
    fn parse(body: &'a [u8]) -> Result<Self> {
        let body = std::str::from_utf8(body)
            .map_err(|_| TvmError::InvalidReceipt("transaction body is not utf-8"))?;
        let mut tokens = body.split_whitespace();
        let Some(kind) = tokens.next() else {
            return Err(TvmError::InvalidReceipt("empty transaction body"));
        };
        Ok(Self {
            kind,
            args: tokens.collect(),
        })
    }

    fn envelope(&self) -> Result<TransactionEnvelope> {
        match self.kind {
            "register_miner" => self.single_hash_transaction(Transaction::RegisterMiner),
            "register_validator" => self.single_hash_transaction(Transaction::RegisterValidator),
            "claim_reward" => self.single_hash_transaction(Transaction::ClaimReward),
            "submit_tensor_receipt" => {
                self.single_hash_transaction(Transaction::SubmitTensorOpReceipt)
            }
            "submit_linear_receipt" => {
                self.single_hash_transaction(Transaction::SubmitLinearTrainingStepReceipt)
            }
            "submit_attestation" => self.single_hash_transaction(Transaction::SubmitAttestation),
            "transfer" => self.transfer(),
            _ => Err(TvmError::InvalidReceipt("unknown transaction kind")),
        }
    }

    fn single_hash_transaction(
        &self,
        transaction: impl FnOnce(Hash) -> Transaction,
    ) -> Result<TransactionEnvelope> {
        self.reject_extra_args(1)?;
        Ok(TransactionEnvelope {
            sender: None,
            transaction: transaction(parse_hash_token(self.args.first().copied())?),
        })
    }

    fn transfer(&self) -> Result<TransactionEnvelope> {
        self.reject_extra_args(3)?;
        let sender = parse_hash_token(self.args.first().copied())?;
        let to = parse_hash_token(self.args.get(1).copied())?;
        let amount = parse_u64_token(self.args.get(2).copied())?;
        Ok(TransactionEnvelope {
            sender: Some(sender),
            transaction: Transaction::Transfer { to, amount },
        })
    }

    fn reject_extra_args(&self, expected: usize) -> Result<()> {
        if self.args.len() > expected {
            return Err(TvmError::InvalidReceipt(
                "unexpected transaction body token",
            ));
        }
        Ok(())
    }
}

fn parse_hash_token(token: Option<&str>) -> Result<Hash> {
    let token = token.ok_or(TvmError::InvalidReceipt("missing hash token"))?;
    if token.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash length"));
    }
    parse_hash_hex(token).map_err(|_| TvmError::InvalidReceipt("invalid hex"))
}

fn parse_u64_token(token: Option<&str>) -> Result<u64> {
    token
        .ok_or(TvmError::InvalidReceipt("missing integer token"))?
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid integer token"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::types::{address, hash_bytes};

    #[test]
    fn txpool_deduplicates_receipts_and_attestations() {
        let id = hash_bytes(b"test", &[b"id"]);
        let mut pool = TxPool::default();
        assert!(pool.submit(Transaction::SubmitTensorOpReceipt(id)));
        assert!(!pool.submit(Transaction::SubmitTensorOpReceipt(id)));
        assert!(pool.submit(Transaction::SubmitAttestation(id)));
        assert!(pool.submit(Transaction::SubmitAttestation(id)));
        assert_eq!(pool.len(), 3);
        assert_eq!(pool.drain().len(), 3);
        assert!(pool.is_empty());
    }

    #[test]
    fn txpool_pops_transactions_fifo() {
        let miner = address(b"pop-miner");
        let validator = address(b"pop-validator");
        let mut pool = TxPool::default();
        assert!(pool.submit(Transaction::RegisterMiner(miner)));
        assert!(pool.submit(Transaction::RegisterValidator(validator)));
        assert_eq!(pool.pop(), Some(Transaction::RegisterMiner(miner)));
        assert_eq!(pool.pop(), Some(Transaction::RegisterValidator(validator)));
        assert_eq!(pool.pop(), None);
    }

    #[test]
    fn transaction_envelope_parser_handles_reference_payloads() {
        let miner = address(b"miner");
        let receiver = address(b"receiver");
        let receipt = hash_bytes(b"test", &[b"receipt"]);

        assert_eq!(
            parse_transaction_envelope(format!("register_miner {}", hex(&miner)).as_bytes())
                .unwrap(),
            TransactionEnvelope {
                sender: None,
                transaction: Transaction::RegisterMiner(miner),
            }
        );
        assert_eq!(
            parse_transaction_envelope(
                format!("transfer {} {} 25", hex(&miner), hex(&receiver)).as_bytes(),
            )
            .unwrap(),
            TransactionEnvelope {
                sender: Some(miner),
                transaction: Transaction::Transfer {
                    to: receiver,
                    amount: 25,
                },
            }
        );
        assert_eq!(
            parse_transaction_envelope(
                format!("submit_tensor_receipt {}", hex(&receipt)).as_bytes(),
            )
            .unwrap()
            .transaction,
            Transaction::SubmitTensorOpReceipt(receipt)
        );
        assert_eq!(
            parse_transaction_envelope(format!("register_validator {}", hex(&receiver)).as_bytes())
                .unwrap()
                .transaction,
            Transaction::RegisterValidator(receiver)
        );
        assert_eq!(
            parse_transaction_envelope(format!("claim_reward {}", hex(&miner)).as_bytes())
                .unwrap()
                .transaction,
            Transaction::ClaimReward(miner)
        );
        assert_eq!(
            parse_transaction_envelope(
                format!("submit_linear_receipt {}", hex(&receipt)).as_bytes(),
            )
            .unwrap()
            .transaction,
            Transaction::SubmitLinearTrainingStepReceipt(receipt)
        );
        assert_eq!(
            parse_transaction_envelope(format!("submit_attestation {}", hex(&receipt)).as_bytes(),)
                .unwrap()
                .transaction,
            Transaction::SubmitAttestation(receipt)
        );
    }

    #[test]
    fn transaction_envelope_parser_rejects_bad_payloads() {
        let sender = address(b"bad-payload-sender");
        let receiver = address(b"bad-payload-receiver");
        assert!(parse_transaction_envelope(&[0xff]).is_err());
        assert!(parse_transaction_envelope(b"").is_err());
        assert!(parse_transaction_envelope(b"register_miner").is_err());
        assert!(parse_transaction_envelope(b"register_miner not-hex").is_err());
        assert!(
            parse_transaction_envelope(
                format!("register_miner 0x{}", hex(&address(b"prefixed"))).as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_transaction_envelope(format!("register_miner {}", "g".repeat(64)).as_bytes())
                .is_err()
        );
        assert!(
            parse_transaction_envelope(
                format!("transfer {} {} nope", hex(&sender), hex(&receiver)).as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_transaction_envelope(
                format!("transfer {} {}", hex(&sender), hex(&receiver)).as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_transaction_envelope(
                format!("register_miner {} extra", hex(&address(b"extra"))).as_bytes()
            )
            .is_err()
        );
        assert!(parse_transaction_envelope(b"unknown 00").is_err());

        let uppercase = hex(&sender).to_ascii_uppercase();
        assert_eq!(
            parse_transaction_envelope(format!("register_miner {uppercase}").as_bytes())
                .unwrap()
                .transaction,
            Transaction::RegisterMiner(sender)
        );
    }
}
