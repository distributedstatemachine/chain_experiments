use crate::chain::{Chain, ChainParams};
use crate::scheduler::{JobScheduler, SyntheticLocalJobSource};
use crate::types::Hash;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainNetwork {
    Local,
    Testnet,
    Mainnet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeRole {
    Gateway,
    Miner,
    Validator,
    Explorer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceExposure {
    LoopbackOnly,
    PublicHttps,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainProfile {
    pub network: ChainNetwork,
    pub chain_params: ChainParams,
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
    pub synthetic_job_scheduler: Option<JobScheduler>,
    pub public_evidence_required: bool,
    pub service_exposure: ServiceExposure,
}

impl ChainProfile {
    pub fn local_cpu() -> Self {
        Self {
            network: ChainNetwork::Local,
            chain_params: ChainParams::default(),
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
            synthetic_job_scheduler: Some(JobScheduler::with_small_shape((8, 8, 8))),
            public_evidence_required: false,
            service_exposure: ServiceExposure::LoopbackOnly,
        }
    }

    pub fn public_testnet() -> Self {
        Self {
            network: ChainNetwork::Testnet,
            synthetic_job_scheduler: None,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn mainnet() -> Self {
        Self {
            network: ChainNetwork::Mainnet,
            synthetic_job_scheduler: None,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn build_chain(&self, finalized_randomness: Hash) -> Chain {
        Chain::with_params(self.chain_params.clone(), finalized_randomness)
    }

    pub fn label(&self) -> &'static str {
        match self.network {
            ChainNetwork::Local => "local_cpu",
            ChainNetwork::Testnet => "public_testnet",
            ChainNetwork::Mainnet => "mainnet",
        }
    }

    pub fn requires_public_services(&self) -> bool {
        self.service_exposure == ServiceExposure::PublicHttps
    }

    pub fn synthetic_job_source(&self) -> Option<SyntheticLocalJobSource> {
        self.synthetic_job_scheduler
            .clone()
            .map(SyntheticLocalJobSource::new)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeConfig {
    pub profile: ChainProfile,
    pub role: NodeRole,
    pub data_dir: PathBuf,
}

impl NodeConfig {
    pub fn new(profile: ChainProfile, role: NodeRole, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            profile,
            role,
            data_dir: data_dir.into(),
        }
    }

    pub fn build_chain(&self, finalized_randomness: Hash) -> Chain {
        self.profile.build_chain(finalized_randomness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainEngine;
    use crate::scheduler::JobSource;
    use crate::types::hash_bytes;

    #[test]
    fn profiles_build_the_same_chain_engine_base() {
        let beacon = hash_bytes(b"test", &[b"profile-engine"]);
        let profiles = [
            ChainProfile::local_cpu(),
            ChainProfile::public_testnet(),
            ChainProfile::mainnet(),
        ];

        for profile in profiles {
            let chain = profile.build_chain(beacon);
            assert_eq!(chain.params(), &profile.chain_params);
            assert_eq!(chain.view().finalized_randomness, beacon);
        }
    }

    #[test]
    fn node_config_keeps_role_and_profile_separate_from_chain_state() {
        let profile = ChainProfile::local_cpu();
        let config = NodeConfig::new(profile.clone(), NodeRole::Miner, "local/miner-00");
        let chain = config.build_chain(hash_bytes(b"test", &[b"profile-node-config"]));

        assert_eq!(config.profile, profile);
        assert_eq!(config.role, NodeRole::Miner);
        assert_eq!(config.data_dir, PathBuf::from("local/miner-00"));
        assert_eq!(chain.params(), &profile.chain_params);
        assert!(!profile.public_evidence_required);
        assert!(!profile.requires_public_services());
        assert!(ChainProfile::public_testnet().requires_public_services());
    }

    #[test]
    fn profiles_configure_local_synthetic_jobs_without_changing_chain_base() {
        let mut local_source = ChainProfile::local_cpu()
            .synthetic_job_source()
            .expect("local profile should enable deterministic synthetic jobs");
        let public_profile = ChainProfile::public_testnet();
        let mainnet_profile = ChainProfile::mainnet();
        let beacon = hash_bytes(b"test", &[b"profile-synthetic-jobs"]);
        let mut local_chain = ChainProfile::local_cpu().build_chain(beacon);
        local_chain.state.height = 0;

        let Some(crate::chain::JobState::TensorOp(job)) = local_source.next_job(&local_chain)
        else {
            panic!("local synthetic source should emit a matmul job first");
        };

        assert_eq!((job.m, job.k, job.n), (8, 8, 8));
        assert!(public_profile.synthetic_job_source().is_none());
        assert!(mainnet_profile.synthetic_job_source().is_none());
        assert_eq!(
            public_profile.build_chain(beacon).params(),
            mainnet_profile.build_chain(beacon).params()
        );
        assert_eq!(ChainProfile::local_cpu().label(), "local_cpu");
        assert_eq!(public_profile.label(), "public_testnet");
        assert_eq!(mainnet_profile.label(), "mainnet");
    }
}
