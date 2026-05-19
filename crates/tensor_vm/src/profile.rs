use crate::chain::{Chain, ChainParams};
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
            public_evidence_required: false,
            service_exposure: ServiceExposure::LoopbackOnly,
        }
    }

    pub fn public_testnet() -> Self {
        Self {
            network: ChainNetwork::Testnet,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn mainnet() -> Self {
        Self {
            network: ChainNetwork::Mainnet,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn build_chain(&self, finalized_randomness: Hash) -> Chain {
        Chain::with_params(self.chain_params.clone(), finalized_randomness)
    }

    pub fn requires_public_services(&self) -> bool {
        self.service_exposure == ServiceExposure::PublicHttps
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
}
