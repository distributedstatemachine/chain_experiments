use super::public_evidence_commands::EvidenceServiceCommand;
use super::public_evidence_service_commands::{
    PublicServiceEndpointArgs, ServiceContentTargetArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
use super::service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
use super::validation::path_argument;
use crate::error::{Result, TvmError};
use crate::testnet::PublicServiceKind;
use crate::types::Hash;

pub(super) fn execute_public_evidence_service_command(
    command: &EvidenceServiceCommand,
) -> Result<String> {
    match command {
        EvidenceServiceCommand::Health(args) => service_health_from_counts(args),
        EvidenceServiceCommand::HealthFile(args) => service_health_from_file(args),
        EvidenceServiceCommand::Content(args) => service_content_from_root(
            service_content_target(&args.target),
            args.content_root.into_hash(),
            args.min_content_bytes,
        ),
        EvidenceServiceCommand::ContentBytes(args) => service_content_from_bytes(
            service_content_target(&args.target),
            args.content.as_slice(),
        ),
        EvidenceServiceCommand::ContentFile(args) => service_content_from_file(
            service_content_target(&args.target),
            &path_argument(&args.content_file),
        ),
    }
}

fn service_health_from_counts(args: &ServiceHealthArgs) -> Result<String> {
    let endpoint = service_endpoint(&args.endpoint);
    service_health_evidence_line(ServiceHealthEvidenceLine {
        kind: endpoint.kind,
        endpoint_id: endpoint.endpoint_id,
        public_url: endpoint.public_url,
        health_path: &args.health.health_path,
        first_seen_block: args.window.first_block,
        last_seen_block: args.window.last_block,
        reachable_observation_count: args.reachable_count,
        signed_health_check_count: args.signed_health_check_count,
    })
}

fn service_health_from_file(args: &ServiceHealthFromFileArgs) -> Result<String> {
    let endpoint = service_endpoint(&args.endpoint);
    service_health_evidence_line_from_file(
        endpoint.kind,
        endpoint.endpoint_id,
        endpoint.public_url,
        &args.health.health_path,
        &path_argument(&args.observation_file),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ServiceEndpointContext<'a> {
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ServiceContentTargetContext<'a> {
    endpoint: ServiceEndpointContext<'a>,
    content_path: &'a str,
    observed_at: u64,
}

fn service_endpoint(args: &PublicServiceEndpointArgs) -> ServiceEndpointContext<'_> {
    ServiceEndpointContext {
        kind: args.kind.into(),
        endpoint_id: args.endpoint_id.into_hash(),
        public_url: &args.public_url,
    }
}

fn service_content_target(args: &ServiceContentTargetArgs) -> ServiceContentTargetContext<'_> {
    ServiceContentTargetContext {
        endpoint: service_endpoint(&args.endpoint),
        content_path: &args.content_path,
        observed_at: args.observation.observed_at,
    }
}

fn service_content_from_root(
    target: ServiceContentTargetContext<'_>,
    content_root: Hash,
    min_content_bytes: u64,
) -> Result<String> {
    service_content_evidence_line(
        target.endpoint.kind,
        target.endpoint.endpoint_id,
        target.endpoint.public_url,
        target.content_path,
        content_root,
        target.observed_at,
        min_content_bytes,
    )
}

fn service_content_from_bytes(
    target: ServiceContentTargetContext<'_>,
    content: &[u8],
) -> Result<String> {
    service_content_evidence_line_from_bytes(
        target.endpoint.kind,
        target.endpoint.endpoint_id,
        target.endpoint.public_url,
        target.content_path,
        target.observed_at,
        content,
    )
}

fn service_content_from_file(
    target: ServiceContentTargetContext<'_>,
    content_file: &str,
) -> Result<String> {
    let content_bytes = std::fs::read(content_file)
        .map_err(|_| TvmError::Storage("failed to read service content file"))?;
    service_content_from_bytes(target, &content_bytes)
}
