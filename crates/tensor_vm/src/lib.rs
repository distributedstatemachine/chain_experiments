//! Reference implementation of the TensorVM MVP.
//!
//! The crate focuses on deterministic local/testnet semantics: finite-field
//! tensors, Merkle commitments, Freivalds verification, linear training-step
//! checks, receipts, attestations, rewards, and settled TensorWork proposer
//! selection.

pub mod api;
pub mod chain;
pub mod challenge;
pub mod cli;
pub mod error;
pub mod explorer;
pub mod faucet;
pub mod field;
pub mod hash;
pub mod jobs;
pub mod merkle;
pub mod miner;
pub mod oracle;
pub mod p2p;
pub mod rpc;
pub mod runtime;
pub mod scheduler;
pub mod storage;
pub mod study;
pub mod telemetry;
pub mod tensor;
pub mod tensor_server;
pub mod testnet;
pub mod txpool;
pub mod types;
pub mod validator;
pub mod verify;
pub mod vm;
pub mod watcher;

pub use chain::{
    AccountState, BlockVote, ChainParams, ChainState, HardwareClass, JobState, LocalChain,
    MinerState, RewardAllocation, RewardState, ValidatorState,
};
pub use challenge::{ChallengeOutcome, FraudChallenge, TensorOpChallengeInput, TraceStep};
pub use cli::{
    CliCommand, execute_reference_cli_command, parse_cli_args, validate_public_evidence_manifest,
    validate_public_testnet_preflight_manifest,
};
pub use error::{Result, TvmError};
pub use faucet::Faucet;
pub use jobs::{
    LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
    PrimitiveType, TensorOpReceipt,
};
pub use miner::MinerNode;
pub use p2p::{
    GossipTopic, Libp2pControlPlaneConfig, NetworkStackRecommendation, PeerBookStore, PeerRecord,
    RequestResponseProtocol, TensorVmLibp2pNode, TensorVmNetworkBehaviour, build_libp2p_node,
    decode_message, encode_gossipsub_message, encode_message, gossip_topic_for_message,
    gossipsub_ident_topic, recommended_network_stack, request_response_protocol_for_message,
    request_response_stream_protocol,
};
pub use rpc::{RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, RpcRequest, RpcResponse};
pub use runtime::{
    BackendKind, CpuReferenceBackend, ExecutionBackend, GpuMinerBackend, cuda_device_count,
    cuda_kernels_compiled,
};
pub use scheduler::{JobScheduler, MinerAssignment, ValidatorAssignment};
pub use storage::{
    BlockLogStore, ChainSnapshot, ChainStateStore, NodeStore, NodeStoreStatus, PersistedNodeState,
    SnapshotStore,
};
pub use study::{
    CollusionSimulation, CollusionSimulationInput, DataWithholdingStudy, FreivaldsSecurity,
    RandomnessAssessment, RandomnessSource, RowSamplingStudy, TensorWorkConcentrationStudy,
    ThreatActor, ThreatActorKind, ThreatModel, VerificationCostStudy, ZeroWorkLivenessStudy,
    assess_randomness, collusion_simulation, data_withholding_study, freivalds_security,
    matmul_verification_cost_study, row_sampling_study, tensorwork_concentration,
    zero_work_liveness_study,
};
pub use tensor::{DType, Layout, Tensor, TensorDescriptor, TensorOpening};
pub use tensor_server::TensorServer;
pub use testnet::{
    LocalTestnet, PublicDeploymentServicePlan, PublicEvidencePublication,
    PublicNetworkRuntimeEvidence, PublicNodeEvidence, PublicNodeRole, PublicServiceEndpoint,
    PublicServiceEvidence, PublicServiceKind, PublicTestnetCriteria, PublicTestnetEvidence,
    PublicTestnetEvidenceBundle, PublicTestnetEvidenceBundleReport, PublicTestnetPreflightPlan,
    PublicTestnetPreflightReport, PublicTestnetRunEvidence, TestnetConfig,
    parse_public_testnet_evidence_manifest, parse_public_testnet_preflight_manifest,
};
pub use txpool::TxPool;
pub use types::{Address, Hash, Signature};
pub use validator::{MatmulVerificationInput, ValidatorNode};
pub use verify::{
    AttestationStatement, FreivaldsParams, LinearVerificationReport, TensorOpVerificationReport,
    ValidatorAttestation, VerificationResult, full_freivalds, row_sample_detection_probability,
    row_sampled_freivalds, verify_linear_training_step, verify_tensor_op,
};
pub use watcher::{ChainWatcher, WatchEvent, WatchEventKind, WatchReport, WatcherConfig};
