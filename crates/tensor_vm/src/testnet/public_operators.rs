use super::{
    PublicNodeRole, PublicTestnetCriteria, PublicTestnetRunEvidence, public_node_role_tag,
};
use crate::types::{Address, Hash, hash_bytes};
use std::collections::{BTreeMap, BTreeSet};

impl PublicTestnetRunEvidence {
    pub(super) fn matched_independent_public_operators_for_criteria(
        &self,
        criteria: &PublicTestnetCriteria,
    ) -> (MatchedPublicOperators, MatchedPublicOperators) {
        let miner_first =
            self.matched_independent_public_operators_starting_with(PublicNodeRole::Miner);
        let validator_first =
            self.matched_independent_public_operators_starting_with(PublicNodeRole::Validator);
        let miner_first_score = public_operator_matching_score(criteria, &miner_first);
        let validator_first_score = public_operator_matching_score(criteria, &validator_first);
        let best_greedy = if validator_first_score > miner_first_score {
            validator_first
        } else {
            miner_first
        };
        if public_operator_matching_satisfies_criteria(criteria, &best_greedy) {
            return best_greedy;
        }
        self.find_public_operator_quota_matching(criteria)
            .unwrap_or(best_greedy)
    }

    pub(super) fn find_public_operator_quota_matching(
        &self,
        criteria: &PublicTestnetCriteria,
    ) -> Option<(MatchedPublicOperators, MatchedPublicOperators)> {
        let candidates = self.public_operator_candidates();
        let miner_quota = criteria.min_miners;
        let validator_quota = criteria.min_validators;
        if miner_quota == 0 && validator_quota == 0 {
            return Some((
                MatchedPublicOperators::default(),
                MatchedPublicOperators::default(),
            ));
        }
        let mut suffix_miners = vec![0; candidates.len() + 1];
        let mut suffix_validators = vec![0; candidates.len() + 1];
        for index in (0..candidates.len()).rev() {
            suffix_miners[index] = suffix_miners[index + 1]
                + usize::from(candidates[index].role == PublicNodeRole::Miner);
            suffix_validators[index] = suffix_validators[index + 1]
                + usize::from(candidates[index].role == PublicNodeRole::Validator);
        }
        let mut search = PublicOperatorQuotaSearch {
            candidates: &candidates,
            suffix_miners: &suffix_miners,
            suffix_validators: &suffix_validators,
            used_operator_ids: BTreeSet::new(),
            used_addresses: BTreeSet::new(),
            selected: Vec::with_capacity(miner_quota + validator_quota),
        };
        search
            .find(0, miner_quota, validator_quota)
            .map(|selected| {
                (
                    MatchedPublicOperators::from_candidates(PublicNodeRole::Miner, &selected),
                    MatchedPublicOperators::from_candidates(PublicNodeRole::Validator, &selected),
                )
            })
    }

    fn public_operator_candidates(&self) -> Vec<PublicOperatorCandidate> {
        let mut candidates = BTreeSet::new();
        for node in &self.nodes {
            if node.is_live_for_run(self.observed_blocks) {
                candidates.insert(PublicOperatorCandidate {
                    role: node.role,
                    operator_id: node.operator_id,
                    address: node.address,
                });
            }
        }
        candidates.into_iter().collect()
    }

    fn matched_independent_public_operators_starting_with(
        &self,
        first_role: PublicNodeRole,
    ) -> (MatchedPublicOperators, MatchedPublicOperators) {
        let first_operators =
            self.matched_public_operators_for_role(first_role, &BTreeSet::new(), &BTreeSet::new());
        let second_role = match first_role {
            PublicNodeRole::Miner => PublicNodeRole::Validator,
            PublicNodeRole::Validator => PublicNodeRole::Miner,
        };
        let second_operators = self.matched_public_operators_for_role(
            second_role,
            &first_operators.operator_ids,
            &first_operators.addresses,
        );
        match first_role {
            PublicNodeRole::Miner => (first_operators, second_operators),
            PublicNodeRole::Validator => (second_operators, first_operators),
        }
    }

    fn matched_public_operators_for_role(
        &self,
        role: PublicNodeRole,
        forbidden_operator_ids: &BTreeSet<Hash>,
        forbidden_addresses: &BTreeSet<Address>,
    ) -> MatchedPublicOperators {
        let mut candidate_addresses_by_operator: BTreeMap<Hash, BTreeSet<Address>> =
            BTreeMap::new();
        for node in &self.nodes {
            if node.role != role
                || !node.is_live_for_run(self.observed_blocks)
                || forbidden_operator_ids.contains(&node.operator_id)
                || forbidden_addresses.contains(&node.address)
            {
                continue;
            }
            candidate_addresses_by_operator
                .entry(node.operator_id)
                .or_default()
                .insert(node.address);
        }
        let mut address_to_operator = BTreeMap::new();
        for operator_id in candidate_addresses_by_operator.keys().copied() {
            let mut seen_addresses = BTreeSet::new();
            match_public_operator_address(
                operator_id,
                &candidate_addresses_by_operator,
                &mut address_to_operator,
                &mut seen_addresses,
            );
        }
        MatchedPublicOperators::from_address_matching(address_to_operator)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MatchedPublicOperators {
    pub(super) operator_ids: BTreeSet<Hash>,
    pub(super) addresses: BTreeSet<Address>,
    address_to_operator: BTreeMap<Address, Hash>,
}

impl MatchedPublicOperators {
    fn from_address_matching(address_to_operator: BTreeMap<Address, Hash>) -> Self {
        let mut matched = Self::default();
        for (address, operator_id) in &address_to_operator {
            matched.operator_ids.insert(*operator_id);
            matched.addresses.insert(*address);
        }
        matched.address_to_operator = address_to_operator;
        matched
    }

    fn from_candidates(role: PublicNodeRole, candidates: &[PublicOperatorCandidate]) -> Self {
        let address_to_operator = candidates
            .iter()
            .filter(|candidate| candidate.role == role)
            .map(|candidate| (candidate.address, candidate.operator_id))
            .collect();
        Self::from_address_matching(address_to_operator)
    }

    pub(super) fn attestation_keys_for_role(&self, role: PublicNodeRole) -> BTreeSet<Hash> {
        let mut attestation_keys = BTreeSet::new();
        for (address, operator_id) in &self.address_to_operator {
            attestation_keys.insert(public_operator_attestation_key(role, address, operator_id));
        }
        attestation_keys
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PublicOperatorCandidate {
    role: PublicNodeRole,
    operator_id: Hash,
    address: Address,
}

struct PublicOperatorQuotaSearch<'a> {
    candidates: &'a [PublicOperatorCandidate],
    suffix_miners: &'a [usize],
    suffix_validators: &'a [usize],
    used_operator_ids: BTreeSet<Hash>,
    used_addresses: BTreeSet<Address>,
    selected: Vec<PublicOperatorCandidate>,
}

impl PublicOperatorQuotaSearch<'_> {
    fn find(
        &mut self,
        index: usize,
        miners_needed: usize,
        validators_needed: usize,
    ) -> Option<Vec<PublicOperatorCandidate>> {
        if miners_needed == 0 && validators_needed == 0 {
            return Some(self.selected.clone());
        }
        if index >= self.candidates.len()
            || self.suffix_miners[index] < miners_needed
            || self.suffix_validators[index] < validators_needed
        {
            return None;
        }
        let candidate = self.candidates[index];
        let candidate_needed = match candidate.role {
            PublicNodeRole::Miner => miners_needed > 0,
            PublicNodeRole::Validator => validators_needed > 0,
        };
        if candidate_needed
            && !self.used_operator_ids.contains(&candidate.operator_id)
            && !self.used_addresses.contains(&candidate.address)
        {
            self.used_operator_ids.insert(candidate.operator_id);
            self.used_addresses.insert(candidate.address);
            self.selected.push(candidate);
            let (next_miners_needed, next_validators_needed) = match candidate.role {
                PublicNodeRole::Miner => (miners_needed.saturating_sub(1), validators_needed),
                PublicNodeRole::Validator => (miners_needed, validators_needed.saturating_sub(1)),
            };
            if let Some(selection) =
                self.find(index + 1, next_miners_needed, next_validators_needed)
            {
                return Some(selection);
            }
            self.selected.pop();
            self.used_addresses.remove(&candidate.address);
            self.used_operator_ids.remove(&candidate.operator_id);
        }
        self.find(index + 1, miners_needed, validators_needed)
    }
}

fn public_operator_matching_satisfies_criteria(
    criteria: &PublicTestnetCriteria,
    operators: &(MatchedPublicOperators, MatchedPublicOperators),
) -> bool {
    operators.0.operator_ids.len() >= criteria.min_miners
        && operators.1.operator_ids.len() >= criteria.min_validators
}

fn public_operator_matching_score(
    criteria: &PublicTestnetCriteria,
    operators: &(MatchedPublicOperators, MatchedPublicOperators),
) -> (bool, usize, usize, usize, usize) {
    let miner_count = operators.0.operator_ids.len();
    let validator_count = operators.1.operator_ids.len();
    (
        public_operator_matching_satisfies_criteria(criteria, operators),
        miner_count.min(criteria.min_miners) + validator_count.min(criteria.min_validators),
        miner_count + validator_count,
        miner_count,
        validator_count,
    )
}

pub(super) fn public_operator_attestation_key(
    role: PublicNodeRole,
    address: &Address,
    operator_id: &Hash,
) -> Hash {
    hash_bytes(
        b"tensor-vm-public-operator-attestation-key-v1",
        &[public_node_role_tag(role), address, operator_id],
    )
}

pub(super) fn match_public_operator_address(
    operator_id: Hash,
    candidate_addresses_by_operator: &BTreeMap<Hash, BTreeSet<Address>>,
    address_to_operator: &mut BTreeMap<Address, Hash>,
    seen_addresses: &mut BTreeSet<Address>,
) -> bool {
    let Some(candidate_addresses) = candidate_addresses_by_operator.get(&operator_id) else {
        return false;
    };
    for address in candidate_addresses {
        if !seen_addresses.insert(*address) {
            continue;
        }
        if let Some(existing_operator_id) = address_to_operator.get(address).copied()
            && !match_public_operator_address(
                existing_operator_id,
                candidate_addresses_by_operator,
                address_to_operator,
                seen_addresses,
            )
        {
            continue;
        }
        address_to_operator.insert(*address, operator_id);
        return true;
    }
    false
}
