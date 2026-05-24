use super::public_evidence_crypto::{
    public_node_heartbeat_message, public_operator_identity_message,
};
use super::public_urls::public_evidence_uri_is_external;
use crate::types::{Address, Hash, Signature, sign, verify_signature};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PublicNodeRole {
    Miner,
    Validator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicNodeEvidence {
    pub address: Address,
    pub operator_id: Hash,
    pub role: PublicNodeRole,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub signed_heartbeat_count: u64,
    pub heartbeat_signature: Signature,
}

impl PublicNodeEvidence {
    pub fn miner(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Miner,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn validator(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Validator,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn has_external_operator_proof(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && self.last_seen_block >= self.first_seen_block
            && self.signed_heartbeat_count > 0
            && self.heartbeat_signature_valid()
    }

    pub fn is_live_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks)
            && self.has_external_operator_proof()
            && self.has_run_heartbeat_coverage(observed_blocks)
    }

    pub fn heartbeat_signature_valid(&self) -> bool {
        verify_signature(
            &self.address,
            &public_node_heartbeat_message(
                &self.address,
                &self.operator_id,
                self.role,
                self.first_seen_block,
                self.last_seen_block,
                self.signed_heartbeat_count,
            ),
            &self.heartbeat_signature,
        )
    }

    fn new(
        address: Address,
        operator_id: Hash,
        role: PublicNodeRole,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        let message = public_node_heartbeat_message(
            &address,
            &operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        );
        Self {
            address,
            operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
            heartbeat_signature: sign(&address, &message),
        }
    }

    fn has_run_heartbeat_coverage(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0 || self.signed_heartbeat_count >= observed_blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicOperatorIdentityAttestation {
    pub role: PublicNodeRole,
    pub address: Address,
    pub operator_id: Hash,
    pub identity_uri: String,
    pub observed_at_unix_seconds: u64,
    pub operator_signature: Signature,
}

impl PublicOperatorIdentityAttestation {
    pub fn new(
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: impl Into<String>,
        observed_at_unix_seconds: u64,
    ) -> Self {
        let identity_uri = identity_uri.into();
        let message = public_operator_identity_message(
            role,
            &address,
            &operator_id,
            &identity_uri,
            observed_at_unix_seconds,
        );
        Self {
            role,
            address,
            operator_id,
            identity_uri,
            observed_at_unix_seconds,
            operator_signature: sign(&operator_id, &message),
        }
    }

    pub fn operator_signature_valid(&self) -> bool {
        verify_signature(
            &self.operator_id,
            &public_operator_identity_message(
                self.role,
                &self.address,
                &self.operator_id,
                &self.identity_uri,
                self.observed_at_unix_seconds,
            ),
            &self.operator_signature,
        )
    }

    pub fn has_external_identity_proof(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && self.observed_at_unix_seconds > 0
            && public_evidence_uri_is_external(&self.identity_uri)
            && self.operator_signature_valid()
    }
}
