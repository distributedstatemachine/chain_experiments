use crate::chain::LocalChain;
use crate::types::{Address, address, hash_bytes};
use crate::verify::row_sample_detection_probability;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreatActorKind {
    Miner,
    Validator,
    Proposer,
    TensorServer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatActor {
    pub kind: ThreatActorKind,
    pub controls: Vec<&'static str>,
    pub risks: Vec<&'static str>,
    pub mitigations: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatModel {
    pub actors: Vec<ThreatActor>,
}

impl ThreatModel {
    pub fn mvp() -> Self {
        Self {
            actors: vec![
                ThreatActor {
                    kind: ThreatActorKind::Miner,
                    controls: vec!["tensor execution", "receipt submission", "tensor serving"],
                    risks: vec![
                        "invalid tensor roots",
                        "sparse output corruption",
                        "data withholding after commitment",
                    ],
                    mitigations: vec![
                        "full-output Freivalds checks",
                        "server-backed tensor availability attestations",
                        "retention through settlement and challenge windows",
                    ],
                },
                ThreatActor {
                    kind: ThreatActorKind::Validator,
                    controls: vec!["attestations", "sample reveal timing"],
                    risks: vec![
                        "bad attestations",
                        "duplicate quorum votes",
                        "premature sample disclosure",
                    ],
                    mitigations: vec![
                        "signature checks",
                        "registered-stake quorum",
                        "one attestation per validator per receipt",
                    ],
                },
                ThreatActor {
                    kind: ThreatActorKind::Proposer,
                    controls: vec!["block assembly", "block timestamp", "transaction ordering"],
                    risks: vec![
                        "current-block randomness grinding",
                        "current-epoch TensorWork circularity",
                    ],
                    mitigations: vec![
                        "receipt-bound finalized randomness",
                        "settled prior TensorWork proposer selection",
                        "zero-work fallback proposer selection",
                    ],
                },
                ThreatActor {
                    kind: ThreatActorKind::TensorServer,
                    controls: vec!["tensor chunks", "rows", "Merkle openings"],
                    risks: vec![
                        "verification-time unavailability",
                        "post-settlement disappearance",
                    ],
                    mitigations: vec![
                        "availability attestations",
                        "retention-window pruning",
                        "future external content-addressed replication",
                    ],
                },
            ],
        }
    }

    pub fn actor(&self, kind: ThreatActorKind) -> Option<&ThreatActor> {
        self.actors.iter().find(|actor| actor.kind == kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomnessSource {
    CurrentBlockHash,
    FinalizedBeacon,
    ReceiptBoundFinalizedBeacon,
    CommitReveal {
        committed_before_receipt: bool,
        revealed_after_receipt_root: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomnessAssessment {
    pub source: RandomnessSource,
    pub proposer_can_bias_after_receipt_commitment: bool,
    pub acceptable_for_mvp: bool,
}

pub fn assess_randomness(source: RandomnessSource) -> RandomnessAssessment {
    let proposer_can_bias_after_receipt_commitment = match source {
        RandomnessSource::CurrentBlockHash => true,
        RandomnessSource::FinalizedBeacon | RandomnessSource::ReceiptBoundFinalizedBeacon => false,
        RandomnessSource::CommitReveal {
            committed_before_receipt,
            revealed_after_receipt_root,
        } => !(committed_before_receipt && revealed_after_receipt_root),
    };
    RandomnessAssessment {
        source,
        proposer_can_bias_after_receipt_commitment,
        acceptable_for_mvp: !proposer_can_bias_after_receipt_commitment,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreivaldsSecurity {
    pub rounds: usize,
    pub field_size: u64,
    pub false_accept_bound: f64,
}

pub fn freivalds_security(rounds: usize, field_size: u64) -> FreivaldsSecurity {
    let rounds = rounds.max(1);
    let field_size = field_size.max(2);
    FreivaldsSecurity {
        rounds,
        field_size,
        false_accept_bound: (field_size as f64).recip().powi(rounds as i32),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowSamplingStudy {
    pub total_rows: usize,
    pub corrupted_rows: usize,
    pub sampled_rows: usize,
    pub detection_probability: f64,
    pub target_detection_probability: f64,
    pub row_sampled_only_allowed: bool,
}

pub fn row_sampling_study(
    total_rows: usize,
    corrupted_rows: usize,
    sampled_rows: usize,
    target_detection_probability: f64,
) -> RowSamplingStudy {
    let detection_probability =
        row_sample_detection_probability(total_rows, corrupted_rows, sampled_rows);
    RowSamplingStudy {
        total_rows,
        corrupted_rows,
        sampled_rows,
        detection_probability,
        target_detection_probability,
        row_sampled_only_allowed: detection_probability >= target_detection_probability,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataWithholdingStudy {
    pub replicas: usize,
    pub per_replica_availability: f64,
    pub at_least_one_available_probability: f64,
}

pub fn data_withholding_study(
    replicas: usize,
    per_replica_availability: f64,
) -> DataWithholdingStudy {
    let p = per_replica_availability.clamp(0.0, 1.0);
    DataWithholdingStudy {
        replicas,
        per_replica_availability: p,
        at_least_one_available_probability: if replicas == 0 {
            0.0
        } else {
            1.0 - (1.0 - p).powi(replicas as i32)
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TensorWorkConcentrationStudy {
    pub total_work: u64,
    pub max_work_share: f64,
    pub effective_miners: f64,
}

pub fn tensorwork_concentration<I>(work: I) -> TensorWorkConcentrationStudy
where
    I: IntoIterator<Item = u64>,
{
    let values: Vec<u64> = work.into_iter().collect();
    let total_work: u64 = values.iter().copied().sum();
    if total_work == 0 {
        return TensorWorkConcentrationStudy {
            total_work,
            max_work_share: 0.0,
            effective_miners: 0.0,
        };
    }
    let mut hhi = 0.0;
    let mut max_work_share = 0.0_f64;
    for value in values {
        let share = value as f64 / total_work as f64;
        hhi += share * share;
        max_work_share = max_work_share.max(share);
    }
    TensorWorkConcentrationStudy {
        total_work,
        max_work_share,
        effective_miners: hhi.recip(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerificationCostStudy {
    pub execution_ops: u64,
    pub verification_ops: u64,
    pub verification_to_execution_ratio: f64,
}

pub fn matmul_verification_cost_study(
    m: usize,
    k: usize,
    n: usize,
    full_rounds: usize,
) -> VerificationCostStudy {
    let m = m as u64;
    let k = k as u64;
    let n = n as u64;
    let rounds = full_rounds.max(1) as u64;
    let execution_ops = 2_u64.saturating_mul(m).saturating_mul(k).saturating_mul(n);
    let verification_ops = 2_u64
        .saturating_mul(rounds)
        .saturating_mul(k.saturating_mul(n) + m.saturating_mul(k) + m.saturating_mul(n));
    VerificationCostStudy {
        execution_ops,
        verification_ops,
        verification_to_execution_ratio: if execution_ops == 0 {
            0.0
        } else {
            verification_ops as f64 / execution_ops as f64
        },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollusionSimulationInput {
    pub validator_stakes: Vec<u64>,
    pub colluding_validator_indices: Vec<usize>,
    pub miner_count: usize,
    pub colluding_miners: usize,
    pub finality_stake_numerator: u64,
    pub finality_stake_denominator: u64,
    pub attestation_stake_numerator: u64,
    pub attestation_stake_denominator: u64,
    pub minimum_validators: usize,
    pub agreement_quorum: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollusionSimulation {
    pub total_validator_stake: u64,
    pub colluding_validator_stake: u64,
    pub validator_stake_share: f64,
    pub colluding_validator_count: usize,
    pub colluding_miners: usize,
    pub reaches_finality_threshold: bool,
    pub reaches_attestation_threshold: bool,
    pub reaches_agreement_quorum: bool,
    pub can_finalize_invalid_block: bool,
    pub can_attest_invalid_receipt: bool,
    pub can_fake_redundant_agreement: bool,
}

pub fn collusion_simulation(input: CollusionSimulationInput) -> CollusionSimulation {
    let total_validator_stake: u64 = input.validator_stakes.iter().copied().sum();
    let mut seen = std::collections::BTreeSet::new();
    let mut colluding_validator_stake = 0_u64;
    for index in input.colluding_validator_indices {
        if seen.insert(index)
            && let Some(stake) = input.validator_stakes.get(index)
        {
            colluding_validator_stake = colluding_validator_stake.saturating_add(*stake);
        }
    }
    let colluding_validator_count = seen.len();
    let reaches_finality_threshold = threshold_reached(
        colluding_validator_stake,
        total_validator_stake,
        input.finality_stake_numerator,
        input.finality_stake_denominator,
    );
    let reaches_attestation_stake_threshold = threshold_reached(
        colluding_validator_stake,
        total_validator_stake,
        input.attestation_stake_numerator,
        input.attestation_stake_denominator,
    );
    let reaches_attestation_threshold = reaches_attestation_stake_threshold
        && colluding_validator_count >= input.minimum_validators;
    let reaches_agreement_quorum =
        input.colluding_miners.min(input.miner_count) >= input.agreement_quorum.max(1);

    CollusionSimulation {
        total_validator_stake,
        colluding_validator_stake,
        validator_stake_share: ratio_u64(colluding_validator_stake, total_validator_stake),
        colluding_validator_count,
        colluding_miners: input.colluding_miners.min(input.miner_count),
        reaches_finality_threshold,
        reaches_attestation_threshold,
        reaches_agreement_quorum,
        can_finalize_invalid_block: reaches_finality_threshold,
        can_attest_invalid_receipt: reaches_attestation_threshold,
        can_fake_redundant_agreement: reaches_agreement_quorum,
    }
}

fn threshold_reached(stake: u64, total: u64, numerator: u64, denominator: u64) -> bool {
    total > 0 && stake.saturating_mul(denominator.max(1)) >= total.saturating_mul(numerator)
}

fn ratio_u64(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZeroWorkLivenessStudy {
    pub blocks_requested: u64,
    pub blocks_produced: u64,
    pub proposers: Vec<Address>,
}

pub fn zero_work_liveness_study(
    miner_count: usize,
    validator_count: usize,
    blocks: u64,
) -> ZeroWorkLivenessStudy {
    let beacon = hash_bytes(b"tensor-vm-zero-work-study-v1", &[]);
    let mut chain = LocalChain::new(beacon);
    for i in 0..miner_count {
        chain
            .register_miner(address(format!("study-miner-{i}").as_bytes()), 100)
            .expect("study miner stake meets default minimum");
    }
    for i in 0..validator_count {
        chain
            .register_validator(address(format!("study-validator-{i}").as_bytes()), 10_000)
            .expect("study validator stake meets default minimum");
    }
    let mut proposers = Vec::with_capacity(blocks as usize);
    for height in 0..blocks {
        let proposer = chain
            .proposer_for_next_epoch(&chain.state.finalized_randomness)
            .unwrap_or([0; 32]);
        proposers.push(proposer);
        chain.produce_block(
            proposer,
            height.saturating_mul(chain.params.block_time_seconds),
        );
    }
    ZeroWorkLivenessStudy {
        blocks_requested: blocks,
        blocks_produced: chain.blocks.len() as u64,
        proposers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::MODULUS;

    #[test]
    fn threat_model_names_all_mvp_actors() {
        let model = ThreatModel::mvp();
        assert!(model.actor(ThreatActorKind::Miner).is_some());
        assert!(model.actor(ThreatActorKind::Validator).is_some());
        assert!(model.actor(ThreatActorKind::Proposer).is_some());
        assert!(model.actor(ThreatActorKind::TensorServer).is_some());
        assert!(
            model
                .actor(ThreatActorKind::Proposer)
                .unwrap()
                .mitigations
                .contains(&"settled prior TensorWork proposer selection")
        );
    }

    #[test]
    fn freivalds_security_reports_round_bound() {
        let one_round = freivalds_security(1, MODULUS);
        let two_rounds = freivalds_security(2, MODULUS);
        assert!(one_round.false_accept_bound < 5.0e-10);
        assert!(two_rounds.false_accept_bound < one_round.false_accept_bound);
    }

    #[test]
    fn row_sampling_study_blocks_sparse_row_sampled_only_acceptance() {
        let sparse = row_sampling_study(1024, 1, 16, 0.95);
        assert!(!sparse.row_sampled_only_allowed);
        assert!((sparse.detection_probability - 16.0 / 1024.0).abs() < 1e-12);

        let dense = row_sampling_study(1024, 512, 16, 0.95);
        assert!(dense.row_sampled_only_allowed);
    }

    #[test]
    fn randomness_assessment_rejects_current_block_hash() {
        assert!(!assess_randomness(RandomnessSource::CurrentBlockHash).acceptable_for_mvp);
        assert!(
            !assess_randomness(RandomnessSource::FinalizedBeacon)
                .proposer_can_bias_after_receipt_commitment
        );
        assert!(
            assess_randomness(RandomnessSource::ReceiptBoundFinalizedBeacon).acceptable_for_mvp
        );
        assert!(
            assess_randomness(RandomnessSource::CommitReveal {
                committed_before_receipt: true,
                revealed_after_receipt_root: true,
            })
            .acceptable_for_mvp
        );
        assert!(
            !assess_randomness(RandomnessSource::CommitReveal {
                committed_before_receipt: false,
                revealed_after_receipt_root: true,
            })
            .acceptable_for_mvp
        );
    }

    #[test]
    fn data_withholding_study_reports_replication_gain() {
        let single = data_withholding_study(1, 0.90);
        let replicated = data_withholding_study(5, 0.90);
        let none = data_withholding_study(0, 2.0);
        assert_eq!(single.at_least_one_available_probability, 0.90);
        assert!(replicated.at_least_one_available_probability > 0.999);
        assert_eq!(none.per_replica_availability, 1.0);
        assert_eq!(none.at_least_one_available_probability, 0.0);
    }

    #[test]
    fn tensorwork_concentration_reports_effective_miners() {
        let empty = tensorwork_concentration([]);
        assert_eq!(empty.total_work, 0);
        assert_eq!(empty.effective_miners, 0.0);

        let equal = tensorwork_concentration([10, 10, 10, 10]);
        assert_eq!(equal.total_work, 40);
        assert!((equal.max_work_share - 0.25).abs() < 1e-12);
        assert!((equal.effective_miners - 4.0).abs() < 1e-12);

        let concentrated = tensorwork_concentration([97, 1, 1, 1]);
        assert!(concentrated.effective_miners < 2.0);
    }

    #[test]
    fn matmul_verification_cost_is_lower_than_execution_for_mvp_shape() {
        let study = matmul_verification_cost_study(1024, 1024, 1024, 1);
        assert!(study.verification_to_execution_ratio < 0.01);
        assert!(study.verification_ops < study.execution_ops);

        let degenerate = matmul_verification_cost_study(0, 1024, 1024, 0);
        assert_eq!(degenerate.execution_ops, 0);
        assert_eq!(degenerate.verification_to_execution_ratio, 0.0);
    }

    #[test]
    fn collusion_simulation_reports_threshold_crossings() {
        let empty = collusion_simulation(CollusionSimulationInput {
            validator_stakes: Vec::new(),
            colluding_validator_indices: vec![0],
            miner_count: 0,
            colluding_miners: 3,
            finality_stake_numerator: 2,
            finality_stake_denominator: 3,
            attestation_stake_numerator: 2,
            attestation_stake_denominator: 3,
            minimum_validators: 1,
            agreement_quorum: 0,
        });
        assert_eq!(empty.validator_stake_share, 0.0);
        assert_eq!(empty.colluding_miners, 0);
        assert!(!empty.can_finalize_invalid_block);

        let below = collusion_simulation(CollusionSimulationInput {
            validator_stakes: vec![10, 10, 10, 10, 10],
            colluding_validator_indices: vec![0, 1],
            miner_count: 5,
            colluding_miners: 2,
            finality_stake_numerator: 2,
            finality_stake_denominator: 3,
            attestation_stake_numerator: 2,
            attestation_stake_denominator: 3,
            minimum_validators: 3,
            agreement_quorum: 3,
        });
        assert_eq!(below.colluding_validator_stake, 20);
        assert!(!below.can_finalize_invalid_block);
        assert!(!below.can_attest_invalid_receipt);
        assert!(!below.can_fake_redundant_agreement);

        let above = collusion_simulation(CollusionSimulationInput {
            validator_stakes: vec![10, 10, 10, 10, 10],
            colluding_validator_indices: vec![0, 1, 2, 3],
            miner_count: 5,
            colluding_miners: 3,
            finality_stake_numerator: 2,
            finality_stake_denominator: 3,
            attestation_stake_numerator: 2,
            attestation_stake_denominator: 3,
            minimum_validators: 3,
            agreement_quorum: 3,
        });
        assert_eq!(above.validator_stake_share, 0.8);
        assert!(above.can_finalize_invalid_block);
        assert!(above.can_attest_invalid_receipt);
        assert!(above.can_fake_redundant_agreement);
    }

    #[test]
    fn zero_work_liveness_study_produces_blocks_from_fallback() {
        let study = zero_work_liveness_study(0, 5, 12);
        assert_eq!(study.blocks_produced, 12);
        assert_eq!(study.proposers.len(), 12);
        assert!(study.proposers.iter().all(|proposer| *proposer != [0; 32]));

        let no_participants = zero_work_liveness_study(0, 0, 1);
        assert_eq!(no_participants.blocks_produced, 1);
        assert_eq!(no_participants.proposers, vec![[0; 32]]);

        let miner_fallback = zero_work_liveness_study(1, 0, 1);
        assert_eq!(miner_fallback.blocks_produced, 1);
        assert_ne!(miner_fallback.proposers, vec![[0; 32]]);
    }
}
