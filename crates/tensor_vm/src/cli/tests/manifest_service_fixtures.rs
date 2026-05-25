use super::*;
use crate::hash::hex;
use crate::testnet::{
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind,
};
use crate::types::hash_bytes;

pub(super) fn public_service_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
    }
}

pub(super) fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
    let service = PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[label]),
            public_service_url(kind),
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    hex(&service.health_check_signature)
}

pub(super) fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
    }
}

pub(super) fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "/chain/head",
        PublicServiceKind::Explorer => "/explorer",
        PublicServiceKind::Faucet => "/faucet/page",
        PublicServiceKind::Telemetry => "/telemetry/dashboard",
    }
}

pub(super) fn public_service_content(
    kind: PublicServiceKind,
    label: &[u8],
) -> PublicServiceContentEvidence {
    PublicServiceContentEvidence::new(
        kind,
        hash_bytes(b"test", &[label]),
        public_service_content_url(kind),
        public_service_content_path(kind),
        hash_bytes(b"test", &[label, b"content-root"]),
        1_700_000_000,
        64,
    )
}

pub(super) fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
    let content = public_service_content(kind, label);
    format!(
        "service_content={},{},{},{},{},{},{},{}",
        public_service_kind_tag(kind),
        hex(&content.endpoint_id),
        content.public_url,
        content.content_path,
        hex(&content.content_root),
        content.observed_at_unix_seconds,
        content.min_content_bytes,
        hex(&content.content_signature)
    )
}
