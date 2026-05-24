use super::payload_application::{
    apply_network_attestation_payload, apply_network_block_payload,
    apply_network_block_vote_payload, apply_network_job_payload, apply_network_receipt_payload,
};
use crate::{chain::Chain, types::Hash};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPayloadApply {
    Applied,
    Pending,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPayloadError {
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkBlockPayloadApply {
    Applied { appended: usize },
    Pending,
    Invalid,
}

pub trait NetworkPayloadProcessor {
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply;

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply;
}

pub trait NetworkEventContext {
    fn chain(&mut self) -> &mut Chain;

    fn apply_block_payload(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply;
}

pub struct ChainNetworkPayloadProcessor<'a> {
    chain: &'a mut Chain,
}

impl<'a> ChainNetworkPayloadProcessor<'a> {
    pub fn new(chain: &'a mut Chain) -> Self {
        Self { chain }
    }
}

impl NetworkPayloadProcessor for ChainNetworkPayloadProcessor<'_> {
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_job_payload(self.chain, job_id, payload)
            .map(|_| NetworkPayloadApply::Applied)
            .unwrap_or(NetworkPayloadApply::Invalid)
    }

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        apply_network_block_payload(self.chain, height, block_hash, payload)
    }

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_vote_payload(self.chain, block_hash, validator, payload)
    }

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.chain, receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.chain, attestation_id, payload)
    }
}

pub(super) struct ContextNetworkPayloadProcessor<'a, C: NetworkEventContext + ?Sized> {
    pub(super) context: &'a mut C,
}

impl<C: NetworkEventContext + ?Sized> NetworkPayloadProcessor
    for ContextNetworkPayloadProcessor<'_, C>
{
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_job_payload(self.context.chain(), job_id, payload)
            .map(|_| NetworkPayloadApply::Applied)
            .unwrap_or(NetworkPayloadApply::Invalid)
    }

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        self.context
            .apply_block_payload(height, block_hash, payload)
    }

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_vote_payload(self.context.chain(), block_hash, validator, payload)
    }

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.context.chain(), receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.context.chain(), attestation_id, payload)
    }
}
