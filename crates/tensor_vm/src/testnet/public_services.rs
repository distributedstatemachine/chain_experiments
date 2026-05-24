use super::PUBLIC_SERVICE_MIN_CONTENT_BYTES;
use super::public_evidence_crypto::{
    public_service_content_message, public_service_health_message,
};
use super::public_urls::{public_host_is_external, public_https_host, public_https_path};
use crate::types::{Hash, Signature, sign, verify_signature};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicServiceKind {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

impl PublicServiceKind {
    pub(super) fn evidence_tag(self) -> &'static [u8] {
        match self {
            Self::Rpc => b"rpc",
            Self::Explorer => b"explorer",
            Self::Faucet => b"faucet",
            Self::Telemetry => b"telemetry",
        }
    }

    pub(super) fn content_path(self) -> &'static str {
        match self {
            Self::Rpc => "/chain/head",
            Self::Explorer => "/explorer",
            Self::Faucet => "/faucet/page",
            Self::Telemetry => "/telemetry/dashboard",
        }
    }
}

pub(super) fn public_service_kinds() -> [PublicServiceKind; 4] {
    [
        PublicServiceKind::Rpc,
        PublicServiceKind::Explorer,
        PublicServiceKind::Faucet,
        PublicServiceKind::Telemetry,
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceEndpoint {
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
}

impl PublicServiceEndpoint {
    pub fn new(
        endpoint_id: Hash,
        public_url: impl Into<String>,
        health_path: impl Into<String>,
    ) -> Self {
        Self {
            endpoint_id,
            public_url: public_url.into(),
            health_path: health_path.into(),
        }
    }

    fn has_external_health_url(&self) -> bool {
        public_https_host(&self.public_url).is_some_and(public_host_is_external)
            && self.health_path.starts_with('/')
            && self.health_path.len() > 1
            && public_https_path(&self.public_url) == Some(self.health_path.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub reachable_observation_count: u64,
    pub signed_health_check_count: u64,
    pub health_check_signature: Signature,
}

impl PublicServiceEvidence {
    pub fn new(
        kind: PublicServiceKind,
        endpoint: PublicServiceEndpoint,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    ) -> Self {
        let message = public_service_health_message(
            kind,
            &endpoint,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        );
        let endpoint_id = endpoint.endpoint_id;
        Self {
            kind,
            endpoint_id,
            public_url: endpoint.public_url,
            health_path: endpoint.health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
            health_check_signature: sign(&endpoint_id, &message),
        }
    }

    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn signed_health_check_valid(&self) -> bool {
        verify_signature(
            &self.endpoint_id,
            &public_service_health_message(
                self.kind,
                &self.endpoint(),
                self.first_seen_block,
                self.last_seen_block,
                self.reachable_observation_count,
                self.signed_health_check_count,
            ),
            &self.health_check_signature,
        )
    }

    pub fn has_reachable_endpoint_proof(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.endpoint().has_external_health_url()
            && self.last_seen_block >= self.first_seen_block
            && self.reachable_observation_count > 0
            && self.signed_health_check_count > 0
            && self.reachable_observation_count <= self.signed_health_check_count
            && self.signed_health_check_valid()
    }

    pub fn is_reachable_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks)
            && self.has_reachable_endpoint_proof()
            && self.has_run_health_coverage(observed_blocks)
    }

    fn endpoint(&self) -> PublicServiceEndpoint {
        PublicServiceEndpoint {
            endpoint_id: self.endpoint_id,
            public_url: self.public_url.clone(),
            health_path: self.health_path.clone(),
        }
    }

    fn has_run_health_coverage(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.reachable_observation_count >= observed_blocks
                && self.signed_health_check_count >= observed_blocks)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceContentEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub content_path: String,
    pub content_root: Hash,
    pub observed_at_unix_seconds: u64,
    pub min_content_bytes: u64,
    pub content_signature: Signature,
}

impl PublicServiceContentEvidence {
    pub fn new(
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: impl Into<String>,
        content_path: impl Into<String>,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    ) -> Self {
        let public_url = public_url.into();
        let content_path = content_path.into();
        let message = public_service_content_message(
            kind,
            &endpoint_id,
            &public_url,
            &content_path,
            &content_root,
            observed_at_unix_seconds,
            min_content_bytes,
        );
        Self {
            kind,
            endpoint_id,
            public_url,
            content_path,
            content_root,
            observed_at_unix_seconds,
            min_content_bytes,
            content_signature: sign(&endpoint_id, &message),
        }
    }

    pub fn content_signature_valid(&self) -> bool {
        verify_signature(
            &self.endpoint_id,
            &public_service_content_message(
                self.kind,
                &self.endpoint_id,
                &self.public_url,
                &self.content_path,
                &self.content_root,
                self.observed_at_unix_seconds,
                self.min_content_bytes,
            ),
            &self.content_signature,
        )
    }

    pub fn has_external_content_proof(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.content_root != [0; 32]
            && self.observed_at_unix_seconds > 0
            && self.min_content_bytes >= PUBLIC_SERVICE_MIN_CONTENT_BYTES
            && public_https_host(&self.public_url).is_some_and(public_host_is_external)
            && self.content_path == self.kind.content_path()
            && public_https_path(&self.public_url) == Some(self.kind.content_path())
            && self.content_signature_valid()
    }
}
