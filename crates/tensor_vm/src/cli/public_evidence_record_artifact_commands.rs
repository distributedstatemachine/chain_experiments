use super::public_evidence_record_commands::{
    PublicEvidenceRecordContextArgs, RecordFileArgs, RecordRootArgs, RecordRootsArgs,
};
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Address, Hash};
use clap::{Args, ValueHint};
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[command(flatten)]
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    root: RecordRootArgs,
}

impl RecordArtifactArgs {
    #[cfg(test)]
    pub(crate) fn new(
        context: PublicEvidenceRecordContextArgs,
        artifact: RecordArtifactLocatorArgs,
        root: RecordRootArgs,
    ) -> Self {
        Self {
            context,
            artifact,
            root,
        }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn artifact_uri(&self) -> &str {
        self.artifact.uri()
    }

    pub fn root(&self) -> Hash {
        self.root.root()
    }

    pub fn count(&self) -> u64 {
        self.root.count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[command(flatten)]
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    roots: RecordRootsArgs,
}

impl RecordArtifactFromRootsArgs {
    #[cfg(test)]
    pub(crate) fn new(
        context: PublicEvidenceRecordContextArgs,
        artifact: RecordArtifactLocatorArgs,
        roots: RecordRootsArgs,
    ) -> Self {
        Self {
            context,
            artifact,
            roots,
        }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn artifact_uri(&self) -> &str {
        self.artifact.uri()
    }

    pub fn roots(&self) -> Vec<Hash> {
        self.roots.roots()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromFileArgs {
    #[command(flatten)]
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    file: RecordFileArgs,
}

impl RecordArtifactFromFileArgs {
    #[cfg(test)]
    pub(crate) fn new(
        context: PublicEvidenceRecordContextArgs,
        artifact: RecordArtifactLocatorArgs,
        file: RecordFileArgs,
    ) -> Self {
        Self {
            context,
            artifact,
            file,
        }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn artifact_uri(&self) -> &str {
        self.artifact.uri()
    }

    pub fn file_path(&self) -> &Path {
        self.file.path()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactLocatorArgs {
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    artifact_uri: String,
}

impl RecordArtifactLocatorArgs {
    #[cfg(test)]
    pub(crate) fn new(artifact_uri: impl Into<String>) -> Self {
        Self {
            artifact_uri: artifact_uri.into(),
        }
    }

    pub fn uri(&self) -> &str {
        &self.artifact_uri
    }
}
