use crate::chain::{BlockVote, ChainParams, JobState, LocalChain, TensorBlock, Transaction};
use crate::explorer::ExplorerSummary;
use crate::faucet::Faucet;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
use crate::miner::MinerNode;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::JobScheduler;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DType, Tensor};
use crate::tensor_server::TensorServer;
use crate::txpool::TxPool;
use crate::types::{Address, Hash, address, hash_bytes};
use crate::validator::ValidatorNode;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestnetConfig {
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
}

impl Default for TestnetConfig {
    fn default() -> Self {
        Self {
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetCriteria {
    pub min_miners: usize,
    pub min_validators: usize,
    pub duration_days: u64,
    pub min_finality_rate_bps: u64,
    pub min_data_availability_bps: u64,
    pub min_invalid_work_rejections: u64,
    pub min_reward_settlement_records: u64,
}

impl Default for PublicTestnetCriteria {
    fn default() -> Self {
        Self {
            min_miners: 10,
            min_validators: 5,
            duration_days: 7,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidence {
    pub miner_count: usize,
    pub validator_count: usize,
    pub observed_blocks: u64,
    pub required_blocks: u64,
    pub finality_rate_bps: u64,
    pub data_availability_bps: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub invalid_work_rejection_rate_bps: u64,
    pub reward_settlement_records: u64,
    pub external_operator_evidence: bool,
    pub has_production_libp2p_runtime: bool,
    pub has_deployed_rpc_service: bool,
    pub has_deployed_explorer_service: bool,
    pub has_deployed_faucet_service: bool,
    pub has_deployed_telemetry_service: bool,
    pub has_deployed_public_services: bool,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_required_block_count: bool,
    pub has_required_finality: bool,
    pub has_required_data_availability: bool,
    pub has_invalid_work_rejection_evidence: bool,
    pub has_reward_settlement_records: bool,
    pub public_criterion_met: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicServiceKind {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicNetworkRuntimeEvidence {
    pub libp2p_runtime_used: bool,
    pub peer_discovery_observed: bool,
    pub gossip_propagation_observed: bool,
    pub request_response_observed: bool,
    pub dos_controls_enabled: bool,
}

impl PublicNetworkRuntimeEvidence {
    pub fn has_production_libp2p_runtime(&self) -> bool {
        self.libp2p_runtime_used
            && self.peer_discovery_observed
            && self.gossip_propagation_observed
            && self.request_response_observed
            && self.dos_controls_enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub reachable_observation_count: u64,
    pub signed_health_check_count: u64,
}

impl PublicServiceEvidence {
    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn has_reachable_endpoint_proof(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.reachable_observation_count > 0
            && self.signed_health_check_count > 0
    }

    pub fn is_reachable_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks) && self.has_reachable_endpoint_proof()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidencePublication {
    pub bundle_id: Hash,
    pub public_uri: String,
    pub manifest_signature_count: u64,
    pub independent_auditor_count: u64,
}

impl PublicEvidencePublication {
    pub fn is_published_and_independently_checkable(&self) -> bool {
        let uri = self.public_uri.trim();
        self.bundle_id != [0; 32]
            && (uri.starts_with("https://")
                || uri.starts_with("ipfs://")
                || uri.starts_with("ar://"))
            && self.manifest_signature_count > 0
            && self.independent_auditor_count > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
        self.operator_id != [0; 32] && self.signed_heartbeat_count > 0
    }

    fn new(
        address: Address,
        operator_id: Hash,
        role: PublicNodeRole,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self {
            address,
            operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetRunEvidence {
    pub nodes: Vec<PublicNodeEvidence>,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicServiceEvidence>,
    pub observed_blocks: u64,
    pub finalized_blocks: u64,
    pub checked_receipts: u64,
    pub available_receipts: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub reward_settlement_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundle {
    pub run: PublicTestnetRunEvidence,
    pub publication: PublicEvidencePublication,
    pub block_history_records: u64,
    pub finality_history_records: u64,
    pub operator_identity_attestation_records: u64,
    pub data_availability_measurement_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundleReport {
    pub run_evidence: PublicTestnetEvidence,
    pub has_published_evidence_bundle: bool,
    pub has_block_history: bool,
    pub has_finality_history: bool,
    pub has_operator_identity_attestations: bool,
    pub has_data_availability_measurements: bool,
    pub independently_checkable: bool,
    pub full_spec_evidence_met: bool,
}

impl PublicTestnetEvidenceBundle {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidenceBundleReport {
        let run_evidence =
            self.run
                .evaluate(criteria, block_time_seconds, external_operator_evidence);
        let has_published_evidence_bundle =
            self.publication.is_published_and_independently_checkable();
        let has_block_history =
            self.run.observed_blocks > 0 && self.block_history_records >= self.run.observed_blocks;
        let has_finality_history = self.run.observed_blocks > 0
            && self.finality_history_records >= self.run.observed_blocks;
        let required_operator_attestations =
            (run_evidence.miner_count + run_evidence.validator_count) as u64;
        let has_operator_identity_attestations = required_operator_attestations > 0
            && self.operator_identity_attestation_records >= required_operator_attestations;
        let has_data_availability_measurements = self.run.checked_receipts > 0
            && self.data_availability_measurement_records >= self.run.checked_receipts;
        let independently_checkable = has_published_evidence_bundle
            && has_block_history
            && has_finality_history
            && has_operator_identity_attestations
            && has_data_availability_measurements
            && run_evidence.has_invalid_work_rejection_evidence
            && run_evidence.has_reward_settlement_records;
        let full_spec_evidence_met = run_evidence.public_criterion_met && independently_checkable;
        PublicTestnetEvidenceBundleReport {
            run_evidence,
            has_published_evidence_bundle,
            has_block_history,
            has_finality_history,
            has_operator_identity_attestations,
            has_data_availability_measurements,
            independently_checkable,
            full_spec_evidence_met,
        }
    }
}

impl PublicTestnetRunEvidence {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let (miner_count, validator_count) = self.independent_operator_counts();
        let required_blocks =
            required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
        let finality_rate_bps = ratio_parts_to_bps(self.finalized_blocks, self.observed_blocks);
        let data_availability_bps =
            ratio_parts_to_bps(self.available_receipts, self.checked_receipts);
        let invalid_work_rejection_rate_bps = ratio_parts_to_bps(
            self.invalid_receipts_rejected,
            self.invalid_receipts_submitted,
        );
        let has_required_miners = miner_count >= criteria.min_miners;
        let has_required_validators = validator_count >= criteria.min_validators;
        let has_required_block_count = self.observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = self.invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected <= self.invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            self.reward_settlement_records >= criteria.min_reward_settlement_records;
        let external_operator_evidence =
            external_operator_evidence && miner_count > 0 && validator_count > 0;
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_deployed_rpc_service = self.has_reachable_service(PublicServiceKind::Rpc);
        let has_deployed_explorer_service = self.has_reachable_service(PublicServiceKind::Explorer);
        let has_deployed_faucet_service = self.has_reachable_service(PublicServiceKind::Faucet);
        let has_deployed_telemetry_service =
            self.has_reachable_service(PublicServiceKind::Telemetry);
        let has_deployed_public_services = has_deployed_rpc_service
            && has_deployed_explorer_service
            && has_deployed_faucet_service
            && has_deployed_telemetry_service;
        let public_criterion_met = has_required_miners
            && has_required_validators
            && has_required_block_count
            && has_required_finality
            && has_required_data_availability
            && has_invalid_work_rejection_evidence
            && has_reward_settlement_records
            && has_production_libp2p_runtime
            && has_deployed_public_services
            && external_operator_evidence;
        PublicTestnetEvidence {
            miner_count,
            validator_count,
            observed_blocks: self.observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted: self.invalid_receipts_submitted,
            invalid_receipts_rejected: self.invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records: self.reward_settlement_records,
            external_operator_evidence,
            has_production_libp2p_runtime,
            has_deployed_rpc_service,
            has_deployed_explorer_service,
            has_deployed_faucet_service,
            has_deployed_telemetry_service,
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            public_criterion_met,
        }
    }

    fn has_reachable_service(&self, kind: PublicServiceKind) -> bool {
        self.services.iter().any(|service| {
            service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
        })
    }

    fn independent_operator_counts(&self) -> (usize, usize) {
        let mut miners = BTreeSet::new();
        let mut validators = BTreeSet::new();
        for node in &self.nodes {
            if !node.covers_run(self.observed_blocks) || !node.has_external_operator_proof() {
                continue;
            }
            match node.role {
                PublicNodeRole::Miner => {
                    miners.insert(node.operator_id);
                }
                PublicNodeRole::Validator => {
                    validators.insert(node.operator_id);
                }
            }
        }
        (miners.len(), validators.len())
    }
}

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: LocalChain,
    pub faucet: Faucet,
    pub miners: Vec<Address>,
    pub validators: Vec<Address>,
}

impl LocalTestnet {
    pub fn new(config: TestnetConfig, finalized_randomness: Hash) -> Self {
        let params = ChainParams::default();
        let mut chain = LocalChain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain.register_miner(miner, config.miner_stake).unwrap();
            miners.push(miner);
        }
        for i in 0..config.validator_count {
            let validator = address(format!("testnet-validator-{i}").as_bytes());
            chain
                .register_validator(validator, config.validator_stake)
                .unwrap();
            validators.push(validator);
        }
        Self {
            chain,
            faucet: Faucet::new(config.faucet_balance, config.faucet_drip),
            miners,
            validators,
        }
    }

    pub fn run_blocks(&mut self, count: u64) {
        for i in 0..count {
            let beacon = self.chain.state.finalized_randomness;
            let proposer = self
                .chain
                .proposer_for_next_epoch(&beacon)
                .or_else(|| self.miners.first().copied())
                .or_else(|| self.validators.first().copied())
                .unwrap_or([0; 32]);
            let timestamp = i.saturating_mul(self.chain.params.block_time_seconds);
            let block = self.chain.produce_block(proposer, timestamp);
            self.finalize_block(&block);
        }
    }

    pub fn run_matmul_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let job = scheduler.generate_small_matmul(
            self.chain.state.epoch,
            self.chain.state.height,
            &beacon,
            self.chain.state.height + self.chain.params.receipt_submission_window,
        );
        let mut txpool = TxPool::default();
        self.chain.submit_job(JobState::TensorOp(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, _a, _b, _c) = miner
                .solve_matmul_job(&job, self.chain.state.height, 1 + index as u64)
                .expect("reference miner should solve generated job");
            assert!(txpool.submit(Transaction::SubmitTensorOpReceipt(receipt.receipt_id)));
            self.chain
                .submit_tensor_op_receipt(receipt.clone())
                .expect("registered miner receipt should be accepted");
            receipts.push((receipt, miner.tensor_server.clone()));
        }

        self.attest_matmul_receipts(scheduler, &job, &receipts, &beacon, &mut txpool);

        self.chain.settle_epoch(1_000, 500);
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn run_linear_training_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let model_id = hash_bytes(b"tensor-vm-testnet-model-v1", &[&beacon]);
        let architecture = hash_bytes(b"tensor-vm-testnet-architecture-v1", &[]);
        let config = hash_bytes(b"tensor-vm-testnet-config-v1", &[]);
        let weights = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static weights should be valid");
        self.chain
            .register_model(model_id, architecture, weights.commitment_root(), config);
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-testnet-batch-v1", &[&beacon]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: self.chain.state.height + self.chain.params.receipt_submission_window,
        });
        let mut txpool = TxPool::default();
        self.chain
            .submit_job(JobState::LinearTrainingStep(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, output) = miner
                .solve_linear_training_step(
                    &job,
                    &weights,
                    self.chain.state.height,
                    1 + index as u64,
                )
                .expect("reference miner should solve generated training step");
            assert!(txpool.submit(Transaction::SubmitLinearTrainingStepReceipt(
                receipt.receipt_id
            )));
            self.chain
                .submit_linear_receipt(receipt.clone())
                .expect("registered miner linear receipt should be accepted");
            receipts.push((receipt, output));
        }

        for (receipt, output) in &receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, &beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state
                    .validators
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_linear_training_step(
                        &job,
                        receipt,
                        &weights,
                        output,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated training step");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .submit_attestation(attestation)
                    .expect("registered validator attestation should be accepted");
            }
        }

        let canonical_receipt = &receipts[0].0;
        assert!(
            self.chain
                .has_attestation_quorum(&canonical_receipt.receipt_id)
        );
        assert!(
            self.chain
                .has_redundant_agreement(&canonical_receipt.receipt_id)
        );
        self.chain.settle_epoch(1_000, 500);
        assert!(
            self.chain
                .state
                .settled_receipts
                .contains(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_model_transition(
                &model_id,
                0,
                &weights.commitment_root(),
                canonical_receipt.weight_root_after,
            )
            .expect("verified training receipt should advance model state");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn expected_blocks_for_days(&self, days: u64) -> u64 {
        required_blocks_for_days(days, self.chain.params.block_time_seconds.max(1))
    }

    pub fn telemetry(&self) -> TelemetrySnapshot {
        TelemetrySnapshot::from_chain(&self.chain)
    }

    pub fn explorer_summary(&self) -> ExplorerSummary {
        ExplorerSummary::from_chain(&self.chain)
    }

    pub fn public_testnet_evidence(
        &self,
        criteria: &PublicTestnetCriteria,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let telemetry = self.telemetry();
        let required_blocks = self.expected_blocks_for_days(criteria.duration_days);
        let observed_blocks = self.chain.blocks.len() as u64;
        let finality_rate_bps = ratio_to_bps(telemetry.block_finality_rate);
        let data_availability_bps = ratio_to_bps(telemetry.data_availability_rate);
        let invalid_receipts_submitted = telemetry.invalid_receipts_submitted as u64;
        let invalid_receipts_rejected =
            invalid_receipts_submitted.saturating_sub(telemetry.invalid_receipts_accepted);
        let invalid_work_rejection_rate_bps =
            ratio_parts_to_bps(invalid_receipts_rejected, invalid_receipts_submitted);
        let reward_settlement_records = telemetry.settled_receipt_count as u64;
        let has_required_miners = self.miners.len() >= criteria.min_miners;
        let has_required_validators = self.validators.len() >= criteria.min_validators;
        let has_required_block_count = observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected <= invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            reward_settlement_records >= criteria.min_reward_settlement_records;
        let has_production_libp2p_runtime = false;
        let has_deployed_rpc_service = false;
        let has_deployed_explorer_service = false;
        let has_deployed_faucet_service = false;
        let has_deployed_telemetry_service = false;
        let has_deployed_public_services = false;
        let public_criterion_met = false;
        PublicTestnetEvidence {
            miner_count: self.miners.len(),
            validator_count: self.validators.len(),
            observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted,
            invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records,
            external_operator_evidence,
            has_production_libp2p_runtime,
            has_deployed_rpc_service,
            has_deployed_explorer_service,
            has_deployed_faucet_service,
            has_deployed_telemetry_service,
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            public_criterion_met,
        }
    }

    fn attest_matmul_receipts(
        &mut self,
        scheduler: &JobScheduler,
        job: &crate::jobs::MatmulJob,
        receipts: &[(crate::jobs::TensorOpReceipt, TensorServer)],
        beacon: &Hash,
        txpool: &mut TxPool,
    ) {
        for (receipt, tensor_server) in receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state
                    .validators
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_matmul_from_server(
                        job,
                        receipt,
                        tensor_server,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated job");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .submit_attestation(attestation)
                    .expect("registered validator attestation should be accepted");
            }
        }
    }

    fn finalize_block(&mut self, block: &TensorBlock) {
        for validator in self.validators.clone() {
            let stake = self
                .chain
                .state
                .validators
                .get(&validator)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            self.chain
                .submit_block_vote(BlockVote::new(validator, stake, block))
                .expect("registered validator vote should finalize local block");
            if self.chain.is_block_finalized(&block.hash()) {
                break;
            }
        }
    }
}

fn ratio_to_bps(value: f64) -> u64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn ratio_parts_to_bps(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    let numerator = u128::from(numerator.min(denominator));
    let denominator = u128::from(denominator);
    (((numerator * 10_000) + (denominator / 2)) / denominator) as u64
}

fn required_blocks_for_days(days: u64, block_time_seconds: u64) -> u64 {
    days.saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        / block_time_seconds.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash_bytes;

    fn production_runtime_evidence() -> PublicNetworkRuntimeEvidence {
        PublicNetworkRuntimeEvidence {
            libp2p_runtime_used: true,
            peer_discovery_observed: true,
            gossip_propagation_observed: true,
            request_response_observed: true,
            dos_controls_enabled: true,
        }
    }

    fn public_service(
        kind: PublicServiceKind,
        label: &[u8],
        first_seen_block: u64,
        last_seen_block: u64,
    ) -> PublicServiceEvidence {
        PublicServiceEvidence {
            kind,
            endpoint_id: hash_bytes(b"test", &[label]),
            first_seen_block,
            last_seen_block,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        }
    }

    fn deployed_public_services(last_seen_block: u64) -> Vec<PublicServiceEvidence> {
        vec![
            public_service(PublicServiceKind::Rpc, b"rpc-service", 0, last_seen_block),
            public_service(
                PublicServiceKind::Explorer,
                b"explorer-service",
                0,
                last_seen_block,
            ),
            public_service(
                PublicServiceKind::Faucet,
                b"faucet-service",
                0,
                last_seen_block,
            ),
            public_service(
                PublicServiceKind::Telemetry,
                b"telemetry-service",
                0,
                last_seen_block,
            ),
        ]
    }

    fn complete_public_run_evidence() -> PublicTestnetRunEvidence {
        PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"miner-a"),
                    hash_bytes(b"test", &[b"miner-a-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::miner(
                    address(b"miner-b"),
                    hash_bytes(b"test", &[b"miner-b-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    hash_bytes(b"test", &[b"validator-a-operator"]),
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: production_runtime_evidence(),
            services: deployed_public_services(9),
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
            invalid_receipts_submitted: 1,
            invalid_receipts_rejected: 1,
            reward_settlement_records: 1,
        }
    }

    fn complete_public_evidence_bundle() -> PublicTestnetEvidenceBundle {
        PublicTestnetEvidenceBundle {
            run: complete_public_run_evidence(),
            publication: PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: String::from("https://example.test/tensorvm/public-evidence.json"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            },
            block_history_records: 10,
            finality_history_records: 10,
            operator_identity_attestation_records: 3,
            data_availability_measurement_records: 20,
        }
    }

    #[test]
    fn local_testnet_bootstraps_required_public_shape() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        assert_eq!(testnet.miners.len(), 10);
        assert_eq!(testnet.validators.len(), 5);
        testnet.run_blocks(12);
        let summary = testnet.explorer_summary();
        assert_eq!(summary.block_count, 12);
        assert_eq!(testnet.expected_blocks_for_days(7), 100_800);
        assert_eq!(testnet.telemetry().block_finality_rate, 1.0);
        let public_evidence =
            testnet.public_testnet_evidence(&PublicTestnetCriteria::default(), false);
        assert_eq!(public_evidence.required_blocks, 100_800);
        assert!(public_evidence.has_required_miners);
        assert!(public_evidence.has_required_validators);
        assert!(!public_evidence.has_required_block_count);
        assert!(!public_evidence.external_operator_evidence);
        assert!(!public_evidence.has_production_libp2p_runtime);
        assert!(!public_evidence.has_deployed_public_services);
        assert!(!public_evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_full_matmul_receipt_attestation_settlement_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert!(testnet.telemetry().total_tensor_work > 0);
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);

        let evidence = testnet.public_testnet_evidence(
            &PublicTestnetCriteria {
                duration_days: 0,
                min_finality_rate_bps: 10_000,
                min_data_availability_bps: 9_500,
                ..PublicTestnetCriteria::default()
            },
            true,
        );
        assert_eq!(evidence.observed_blocks, 1);
        assert_eq!(evidence.required_blocks, 0);
        assert_eq!(evidence.finality_rate_bps, 10_000);
        assert_eq!(evidence.data_availability_bps, 10_000);
        assert!(evidence.has_reward_settlement_records);
        assert!(!evidence.has_invalid_work_rejection_evidence);
        assert!(!evidence.public_criterion_met);

        let invalid_receipt_id = hash_bytes(b"test", &[b"public-invalid-receipt"]);
        let invalid_statement = crate::verify::AttestationStatement {
            receipt_id: invalid_receipt_id,
            job_id: hash_bytes(b"test", &[b"public-invalid-job"]),
            primitive_type: crate::jobs::PrimitiveType::TensorOp,
            result: crate::verify::VerificationResult::Invalid,
            checks_root: hash_bytes(b"test", &[b"public-invalid-checks"]),
            data_availability_passed: true,
        };
        let invalid_validator = testnet.validators[0];
        let invalid_stake = testnet
            .chain
            .state
            .validators
            .get(&invalid_validator)
            .unwrap()
            .stake;
        testnet
            .chain
            .state
            .attestations
            .entry(invalid_receipt_id)
            .or_default()
            .push(crate::verify::ValidatorAttestation::new(
                invalid_validator,
                invalid_stake,
                invalid_statement,
            ));

        let complete_local_evidence = testnet.public_testnet_evidence(
            &PublicTestnetCriteria {
                duration_days: 0,
                min_finality_rate_bps: 10_000,
                min_data_availability_bps: 9_500,
                ..PublicTestnetCriteria::default()
            },
            true,
        );
        assert_eq!(complete_local_evidence.invalid_receipts_submitted, 1);
        assert_eq!(complete_local_evidence.invalid_receipts_rejected, 1);
        assert_eq!(
            complete_local_evidence.invalid_work_rejection_rate_bps,
            10_000
        );
        assert!(complete_local_evidence.has_invalid_work_rejection_evidence);
        assert!(complete_local_evidence.has_reward_settlement_records);
        assert!(!complete_local_evidence.has_production_libp2p_runtime);
        assert!(!complete_local_evidence.has_deployed_rpc_service);
        assert!(!complete_local_evidence.has_deployed_explorer_service);
        assert!(!complete_local_evidence.has_deployed_faucet_service);
        assert!(!complete_local_evidence.has_deployed_telemetry_service);
        assert!(!complete_local_evidence.has_deployed_public_services);
        assert!(!complete_local_evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_linear_training_receipt_state_transition_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_linear_training_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert_eq!(testnet.chain.state.model_states.len(), 1);
        assert_eq!(
            testnet
                .chain
                .state
                .model_states
                .values()
                .next()
                .unwrap()
                .step,
            1
        );
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);
    }

    #[test]
    fn public_testnet_run_evidence_requires_independent_external_operators() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 2,
            min_reward_settlement_records: 3,
        };
        let shared_operator = hash_bytes(b"test", &[b"shared-operator"]);
        let validator_operator = hash_bytes(b"test", &[b"validator-operator"]);
        let mut run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(address(b"miner-a"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::miner(address(b"miner-b"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    validator_operator,
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: production_runtime_evidence(),
            services: deployed_public_services(9),
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
            invalid_receipts_submitted: 2,
            invalid_receipts_rejected: 2,
            reward_settlement_records: 3,
        };

        let insufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(insufficient.miner_count, 1);
        assert_eq!(insufficient.validator_count, 1);
        assert_eq!(insufficient.required_blocks, 0);
        assert_eq!(insufficient.finality_rate_bps, 10_000);
        assert_eq!(insufficient.data_availability_bps, 9_500);
        assert_eq!(insufficient.invalid_work_rejection_rate_bps, 10_000);
        assert!(insufficient.external_operator_evidence);
        assert!(insufficient.has_production_libp2p_runtime);
        assert!(insufficient.has_deployed_public_services);
        assert!(!insufficient.has_required_miners);
        assert!(insufficient.has_invalid_work_rejection_evidence);
        assert!(insufficient.has_reward_settlement_records);
        assert!(!insufficient.public_criterion_met);

        run.nodes[1].operator_id = hash_bytes(b"test", &[b"miner-b-operator"]);
        let no_external_flag = run.evaluate(&criteria, 6, false);
        assert!(!no_external_flag.external_operator_evidence);
        assert!(!no_external_flag.public_criterion_met);

        let sufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(sufficient.miner_count, 2);
        assert!(sufficient.has_required_miners);
        assert!(sufficient.has_required_validators);
        assert!(sufficient.has_required_block_count);
        assert!(sufficient.has_required_finality);
        assert!(sufficient.has_required_data_availability);
        assert!(sufficient.has_invalid_work_rejection_evidence);
        assert!(sufficient.has_reward_settlement_records);
        assert!(sufficient.has_production_libp2p_runtime);
        assert!(sufficient.has_deployed_public_services);
        assert!(sufficient.public_criterion_met);

        run.invalid_receipts_rejected = 1;
        let accepted_invalid_work = run.evaluate(&criteria, 6, true);
        assert_eq!(accepted_invalid_work.invalid_work_rejection_rate_bps, 5_000);
        assert!(!accepted_invalid_work.has_invalid_work_rejection_evidence);
        assert!(!accepted_invalid_work.public_criterion_met);
    }

    #[test]
    fn public_testnet_run_evidence_requires_production_runtime_and_reachable_services() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let mut run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"miner-a"),
                    hash_bytes(b"test", &[b"miner-a-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::miner(
                    address(b"miner-b"),
                    hash_bytes(b"test", &[b"miner-b-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    hash_bytes(b"test", &[b"validator-a-operator"]),
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: production_runtime_evidence(),
            services: deployed_public_services(9),
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
            invalid_receipts_submitted: 1,
            invalid_receipts_rejected: 1,
            reward_settlement_records: 1,
        };

        assert!(run.services[0].covers_run(0));
        let complete = run.evaluate(&criteria, 6, true);
        assert!(complete.has_production_libp2p_runtime);
        assert!(complete.has_deployed_rpc_service);
        assert!(complete.has_deployed_explorer_service);
        assert!(complete.has_deployed_faucet_service);
        assert!(complete.has_deployed_telemetry_service);
        assert!(complete.has_deployed_public_services);
        assert!(complete.public_criterion_met);

        run.network_runtime.request_response_observed = false;
        let no_request_response = run.evaluate(&criteria, 6, true);
        assert!(!no_request_response.has_production_libp2p_runtime);
        assert!(no_request_response.has_deployed_public_services);
        assert!(!no_request_response.public_criterion_met);
        run.network_runtime = production_runtime_evidence();

        run.services
            .retain(|service| service.kind != PublicServiceKind::Telemetry);
        let missing_telemetry = run.evaluate(&criteria, 6, true);
        assert!(missing_telemetry.has_production_libp2p_runtime);
        assert!(!missing_telemetry.has_deployed_telemetry_service);
        assert!(!missing_telemetry.has_deployed_public_services);
        assert!(!missing_telemetry.public_criterion_met);

        run.services.push(public_service(
            PublicServiceKind::Telemetry,
            b"late-telemetry-service",
            1,
            9,
        ));
        let late_telemetry = run.evaluate(&criteria, 6, true);
        assert!(!late_telemetry.has_deployed_telemetry_service);
        assert!(!late_telemetry.public_criterion_met);

        run.services.pop();
        let mut unsigned_telemetry = public_service(
            PublicServiceKind::Telemetry,
            b"unsigned-telemetry-service",
            0,
            9,
        );
        unsigned_telemetry.signed_health_check_count = 0;
        assert!(!unsigned_telemetry.has_reachable_endpoint_proof());
        run.services.push(unsigned_telemetry);
        let unsigned_telemetry = run.evaluate(&criteria, 6, true);
        assert!(!unsigned_telemetry.has_deployed_telemetry_service);
        assert!(!unsigned_telemetry.public_criterion_met);
    }

    #[test]
    fn public_testnet_evidence_bundle_requires_publication_and_audit_records() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let mut bundle = complete_public_evidence_bundle();

        let complete = bundle.evaluate(&criteria, 6, true);
        assert!(complete.run_evidence.public_criterion_met);
        assert!(complete.has_published_evidence_bundle);
        assert!(complete.has_block_history);
        assert!(complete.has_finality_history);
        assert!(complete.has_operator_identity_attestations);
        assert!(complete.has_data_availability_measurements);
        assert!(complete.independently_checkable);
        assert!(complete.full_spec_evidence_met);

        bundle.publication.public_uri = String::from("http://localhost:8545/evidence.json");
        let local_uri = bundle.evaluate(&criteria, 6, true);
        assert!(!local_uri.has_published_evidence_bundle);
        assert!(!local_uri.independently_checkable);
        assert!(!local_uri.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.block_history_records = 9;
        let missing_block_history = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_block_history.has_block_history);
        assert!(!missing_block_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.finality_history_records = 9;
        let missing_finality_history = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_finality_history.has_finality_history);
        assert!(!missing_finality_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestation_records = 2;
        let missing_operator_attestations = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_operator_attestations.has_operator_identity_attestations);
        assert!(!missing_operator_attestations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.data_availability_measurement_records = 19;
        let missing_data_availability_measurements = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_data_availability_measurements.has_data_availability_measurements);
        assert!(!missing_data_availability_measurements.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.run.services.clear();
        let missing_services = bundle.evaluate(&criteria, 6, true);
        assert!(missing_services.independently_checkable);
        assert!(!missing_services.run_evidence.public_criterion_met);
        assert!(!missing_services.full_spec_evidence_met);
    }

    #[test]
    fn public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes() {
        let criteria = PublicTestnetCriteria {
            min_miners: 1,
            min_validators: 1,
            duration_days: 1,
            min_finality_rate_bps: 1,
            min_data_availability_bps: 1,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"unsigned-miner"),
                    hash_bytes(b"test", &[b"unsigned-miner-operator"]),
                    0,
                    9,
                    0,
                ),
                PublicNodeEvidence::miner(
                    address(b"late-miner"),
                    hash_bytes(b"test", &[b"late-miner-operator"]),
                    1,
                    9,
                    8,
                ),
                PublicNodeEvidence::validator(
                    address(b"zero-operator-validator"),
                    [0; 32],
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: PublicNetworkRuntimeEvidence::default(),
            services: Vec::new(),
            observed_blocks: 10,
            finalized_blocks: 11,
            checked_receipts: 0,
            available_receipts: 0,
            invalid_receipts_submitted: 0,
            invalid_receipts_rejected: 0,
            reward_settlement_records: 0,
        };

        assert!(run.nodes[0].covers_run(0));
        assert!(!run.nodes[0].has_external_operator_proof());
        assert!(!run.nodes[1].covers_run(run.observed_blocks));
        assert!(!run.nodes[2].has_external_operator_proof());

        let report = run.evaluate(&criteria, 6, true);
        assert_eq!(report.miner_count, 0);
        assert_eq!(report.validator_count, 0);
        assert_eq!(report.required_blocks, 14_400);
        assert_eq!(report.finality_rate_bps, 10_000);
        assert_eq!(report.data_availability_bps, 0);
        assert_eq!(report.invalid_work_rejection_rate_bps, 0);
        assert!(!report.external_operator_evidence);
        assert!(!report.has_production_libp2p_runtime);
        assert!(!report.has_deployed_rpc_service);
        assert!(!report.has_deployed_explorer_service);
        assert!(!report.has_deployed_faucet_service);
        assert!(!report.has_deployed_telemetry_service);
        assert!(!report.has_deployed_public_services);
        assert!(!report.has_invalid_work_rejection_evidence);
        assert!(!report.has_reward_settlement_records);
        assert!(!report.public_criterion_met);
    }
}
