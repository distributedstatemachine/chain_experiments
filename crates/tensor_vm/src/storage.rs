use crate::chain::{
    AccountState, BlockVote, Chain, ChainEngine, ChainParams, ChainParts, ChainState,
    ChainStateParts, HardwareClass, JobState, MinerState, ModelState, ReceiptState, RewardState,
    TensorBlock, ValidatorState,
};
use crate::codec;
use crate::error::{Result, TvmError};
use crate::p2p::PeerBookStore;
use crate::types::{Hash, hash_bytes};
use crate::verify::{FreivaldsParams, ValidatorAttestation};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod block_log;
mod snapshot;

pub use block_log::BlockLogStore;
use block_log::{BLOCK_PAYLOAD_LEN, decode_block_payload, encode_block_payload};
pub use snapshot::{ChainSnapshot, SnapshotStore};

const CHAIN_STATE_MAGIC: &[u8] = b"TENSORVM_STATE\n";
const HASH_LEN: usize = 32;
const U64_LEN: usize = 8;
const CHAIN_STATE_DIGEST_LEN: usize = HASH_LEN;
const SNAPSHOT_FILE_NAME: &str = "chain.snapshot";
const BLOCK_LOG_FILE_NAME: &str = "blocks.log";
const CHAIN_STATE_FILE_NAME: &str = "chain.state";
const PEER_BOOK_FILE_NAME: &str = "peers.book";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainStateStore {
    path: PathBuf,
}

impl ChainStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_chain(&self, chain: &Chain) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create chain state directory"))?;
        }

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, encode_chain_state_file(chain))
            .map_err(|_| TvmError::Storage("failed to write chain state"))?;
        fs::rename(&temp_path, &self.path)
            .map_err(|_| TvmError::Storage("failed to commit chain state"))?;
        Ok(())
    }

    pub fn load_chain(&self) -> Result<Chain> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read chain state"))?;
        decode_chain_state_file(&bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedNodeState {
    pub snapshot: ChainSnapshot,
    pub blocks: Vec<TensorBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeStoreStatus {
    pub data_dir: PathBuf,
    pub snapshot: ChainSnapshot,
    pub block_count: usize,
    pub latest_block_hash: Hash,
    pub block_log_root: Hash,
}

pub trait ChainStore {
    type Chain: ChainEngine;

    fn persist_chain(&self, chain: &Self::Chain) -> Result<NodeStoreStatus>;
    fn load_chain(&self) -> Result<Self::Chain>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeStore {
    data_dir: PathBuf,
    snapshot_store: SnapshotStore,
    block_log_store: BlockLogStore,
    chain_state_store: ChainStateStore,
    peer_book_store: PeerBookStore,
}

impl NodeStore {
    pub fn open(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            snapshot_store: SnapshotStore::new(data_dir.join(SNAPSHOT_FILE_NAME)),
            block_log_store: BlockLogStore::new(data_dir.join(BLOCK_LOG_FILE_NAME)),
            chain_state_store: ChainStateStore::new(data_dir.join(CHAIN_STATE_FILE_NAME)),
            peer_book_store: PeerBookStore::new(data_dir.join(PEER_BOOK_FILE_NAME)),
            data_dir,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn snapshot_store(&self) -> &SnapshotStore {
        &self.snapshot_store
    }

    pub fn block_log_store(&self) -> &BlockLogStore {
        &self.block_log_store
    }

    pub fn chain_state_store(&self) -> &ChainStateStore {
        &self.chain_state_store
    }

    pub fn peer_book_store(&self) -> &PeerBookStore {
        &self.peer_book_store
    }

    pub fn persist_chain(&self, chain: &Chain) -> Result<NodeStoreStatus> {
        let blocks = self.block_log_store.sync_chain(chain)?;
        self.chain_state_store.save_chain(chain)?;
        let snapshot = self.snapshot_store.save_chain(chain)?;
        self.validate_parts(snapshot, blocks)
    }

    pub fn load(&self) -> Result<PersistedNodeState> {
        let snapshot = self.snapshot_store.load()?;
        let blocks = self.block_log_store.load_blocks_or_empty()?;
        self.validate_parts(snapshot.clone(), blocks.clone())?;
        Ok(PersistedNodeState { snapshot, blocks })
    }

    pub fn status(&self) -> Result<NodeStoreStatus> {
        let state = self.load()?;
        self.status_from_parts(state.snapshot, &state.blocks)
    }

    pub fn recover_from_chain_state(&self) -> Result<NodeStoreStatus> {
        let chain = self.chain_state_store.load_chain()?;
        let blocks = self.block_log_store.replace_chain(&chain)?;
        let snapshot = self.snapshot_store.save_chain(&chain)?;
        self.validate_parts(snapshot, blocks)
    }

    pub fn load_chain(&self) -> Result<Chain> {
        let snapshot = self.snapshot_store.load()?;
        let blocks = self.block_log_store.load_blocks_or_empty()?;
        let chain = self.chain_state_store.load_chain()?;
        self.validate_parts(snapshot.clone(), blocks.clone())?;
        if ChainSnapshot::from_chain(&chain) != snapshot {
            return Err(TvmError::Storage("chain state snapshot mismatch"));
        }
        if chain.blocks() != blocks.as_slice() {
            return Err(TvmError::Storage("chain state block log mismatch"));
        }
        Ok(chain)
    }

    fn validate_parts(
        &self,
        snapshot: ChainSnapshot,
        blocks: Vec<TensorBlock>,
    ) -> Result<NodeStoreStatus> {
        self.status_from_parts(snapshot, &blocks)
    }

    fn status_from_parts(
        &self,
        snapshot: ChainSnapshot,
        blocks: &[TensorBlock],
    ) -> Result<NodeStoreStatus> {
        if snapshot.block_count != blocks.len() as u64 {
            return Err(TvmError::Storage("snapshot block count mismatch"));
        }
        let latest_block_hash = blocks.last().map(TensorBlock::hash).unwrap_or([0; 32]);
        if snapshot.latest_block_hash != latest_block_hash {
            return Err(TvmError::Storage("snapshot latest block mismatch"));
        }
        Ok(NodeStoreStatus {
            data_dir: self.data_dir.clone(),
            snapshot,
            block_count: blocks.len(),
            latest_block_hash,
            block_log_root: self.block_log_store.file_root()?,
        })
    }
}

impl ChainStore for NodeStore {
    type Chain = Chain;

    fn persist_chain(&self, chain: &Self::Chain) -> Result<NodeStoreStatus> {
        NodeStore::persist_chain(self, chain)
    }

    fn load_chain(&self) -> Result<Self::Chain> {
        NodeStore::load_chain(self)
    }
}

fn encode_chain_state_file(chain: &Chain) -> Vec<u8> {
    let payload = encode_chain_state_payload(chain);
    let digest = hash_bytes(b"tensor-vm-state-file-v1", &[&payload]);
    let mut encoded =
        Vec::with_capacity(CHAIN_STATE_MAGIC.len() + payload.len() + CHAIN_STATE_DIGEST_LEN);
    encoded.extend_from_slice(CHAIN_STATE_MAGIC);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&digest);
    encoded
}

fn decode_chain_state_file(bytes: &[u8]) -> Result<Chain> {
    if !bytes.starts_with(CHAIN_STATE_MAGIC) {
        return Err(TvmError::Storage("invalid chain state magic"));
    }
    if bytes.len() < CHAIN_STATE_MAGIC.len() + CHAIN_STATE_DIGEST_LEN {
        return Err(TvmError::Storage("invalid chain state length"));
    }
    let payload_end = bytes.len() - CHAIN_STATE_DIGEST_LEN;
    let payload = &bytes[CHAIN_STATE_MAGIC.len()..payload_end];
    let expected_digest = hash_bytes(b"tensor-vm-state-file-v1", &[payload]);
    if bytes[payload_end..] != expected_digest {
        return Err(TvmError::Storage("chain state checksum mismatch"));
    }
    decode_chain_state_payload(payload)
}

fn encode_chain_state_payload(chain: &Chain) -> Vec<u8> {
    let mut out = Vec::new();
    encode_chain_params(&mut out, chain.params());
    encode_chain_state(&mut out, chain.state());
    write_len(&mut out, chain.blocks().len());
    for block in chain.blocks() {
        out.extend_from_slice(&encode_block_payload(block));
    }
    out
}

fn decode_chain_state_payload(bytes: &[u8]) -> Result<Chain> {
    let mut reader = StateReader::new(bytes);
    let params = decode_chain_params(&mut reader)?;
    let state = decode_chain_state(&mut reader)?;
    let block_count = reader.read_len()?;
    let mut blocks = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        blocks.push(decode_block_payload(reader.read_exact(BLOCK_PAYLOAD_LEN)?)?);
    }
    reader.finish()?;
    Ok(Chain::from_parts(ChainParts {
        params,
        state,
        blocks,
    }))
}

fn encode_chain_params(out: &mut Vec<u8>, params: &ChainParams) {
    write_u64(out, params.block_time_seconds);
    write_u64(out, params.epoch_length);
    write_u64(out, params.receipt_submission_window);
    write_u64(out, params.verification_window);
    write_u64(out, params.reward_settlement_delay_epochs);
    write_u64(out, params.challenge_window_epochs);
    write_len(out, params.replication_factor);
    write_len(out, params.agreement_quorum);
    write_u64(out, params.finality_stake_numerator);
    write_u64(out, params.finality_stake_denominator);
    write_u64(out, params.miner_reward_bps);
    write_u64(out, params.validator_reward_bps);
    write_u64(out, params.proposer_reward_bps);
    write_u64(out, params.treasury_reward_bps);
    write_u64(out, params.miner_min_stake);
    write_u64(out, params.validator_min_stake);
    encode_freivalds_params(out, &params.freivalds);
}

fn decode_chain_params(reader: &mut StateReader<'_>) -> Result<ChainParams> {
    Ok(ChainParams {
        block_time_seconds: reader.read_u64()?,
        epoch_length: reader.read_u64()?,
        receipt_submission_window: reader.read_u64()?,
        verification_window: reader.read_u64()?,
        reward_settlement_delay_epochs: reader.read_u64()?,
        challenge_window_epochs: reader.read_u64()?,
        replication_factor: reader.read_len()?,
        agreement_quorum: reader.read_len()?,
        finality_stake_numerator: reader.read_u64()?,
        finality_stake_denominator: reader.read_u64()?,
        miner_reward_bps: reader.read_u64()?,
        validator_reward_bps: reader.read_u64()?,
        proposer_reward_bps: reader.read_u64()?,
        treasury_reward_bps: reader.read_u64()?,
        miner_min_stake: reader.read_u64()?,
        validator_min_stake: reader.read_u64()?,
        freivalds: decode_freivalds_params(reader)?,
    })
}

fn encode_freivalds_params(out: &mut Vec<u8>, params: &FreivaldsParams) {
    write_len(out, params.full_rounds);
    write_len(out, params.audit_rows);
    write_len(out, params.validators_per_job);
    write_len(out, params.minimum_validators);
    write_u64(out, params.minimum_stake_numerator);
    write_u64(out, params.minimum_stake_denominator);
}

fn decode_freivalds_params(reader: &mut StateReader<'_>) -> Result<FreivaldsParams> {
    Ok(FreivaldsParams {
        full_rounds: reader.read_len()?,
        audit_rows: reader.read_len()?,
        validators_per_job: reader.read_len()?,
        minimum_validators: reader.read_len()?,
        minimum_stake_numerator: reader.read_u64()?,
        minimum_stake_denominator: reader.read_u64()?,
    })
}

fn encode_chain_state(out: &mut Vec<u8>, state: &ChainState) {
    write_u64(out, state.height());
    write_u64(out, state.epoch());
    write_hash(out, &state.finalized_randomness());
    write_hash(out, &state.genesis_randomness());
    encode_accounts(out, state.accounts());
    encode_miners(out, state.miners());
    encode_validators(out, state.validators());
    encode_jobs(out, state.jobs());
    encode_receipts(out, state.receipts());
    encode_attestations(out, state.attestations());
    encode_block_votes(out, state.block_votes());
    encode_hash_set(out, state.finalized_blocks());
    encode_hash_set(out, state.data_unavailable_receipts());
    encode_hash_set(out, state.settled_receipts());
    encode_hash_set(out, state.included_receipts());
    encode_hash_vec_map(out, state.block_selected_receipts());
    encode_model_states(out, state.model_states());
    encode_rewards(out, state.rewards());
}

fn decode_chain_state(reader: &mut StateReader<'_>) -> Result<ChainState> {
    Ok(ChainState::from_parts(ChainStateParts {
        height: reader.read_u64()?,
        epoch: reader.read_u64()?,
        finalized_randomness: reader.read_hash()?,
        genesis_randomness: reader.read_hash()?,
        accounts: decode_accounts(reader)?,
        miners: decode_miners(reader)?,
        validators: decode_validators(reader)?,
        jobs: decode_jobs(reader)?,
        receipts: decode_receipts(reader)?,
        attestations: decode_attestations(reader)?,
        block_votes: decode_block_votes(reader)?,
        finalized_blocks: decode_hash_set(reader)?,
        data_unavailable_receipts: decode_hash_set(reader)?,
        settled_receipts: decode_hash_set(reader)?,
        included_receipts: decode_hash_set(reader)?,
        block_selected_receipts: decode_hash_vec_map(reader)?,
        model_states: decode_model_states(reader)?,
        rewards: decode_rewards(reader)?,
    }))
}

fn encode_accounts(out: &mut Vec<u8>, accounts: &BTreeMap<Hash, AccountState>) {
    write_len(out, accounts.len());
    for (address, account) in accounts {
        write_hash(out, address);
        write_hash(out, &account.address);
        write_u64(out, account.balance);
        write_u64(out, account.nonce);
    }
}

fn decode_accounts(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, AccountState>> {
    let mut accounts = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        accounts.insert(
            key,
            AccountState {
                address,
                balance: reader.read_u64()?,
                nonce: reader.read_u64()?,
            },
        );
    }
    Ok(accounts)
}

fn encode_miners(out: &mut Vec<u8>, miners: &BTreeMap<Hash, MinerState>) {
    write_len(out, miners.len());
    for (address, miner) in miners {
        write_hash(out, address);
        write_hash(out, &miner.address);
        write_hash(out, &miner.operator_id);
        write_u64(out, miner.stake);
        write_i64(out, miner.reputation);
        write_u64(out, miner.settled_tensor_work);
        write_u64(out, miner.pending_tensor_work);
        out.push(hardware_class_code(miner.hardware_class));
        write_u64(out, miner.gpu_utilization_bps);
    }
}

fn decode_miners(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, MinerState>> {
    let mut miners = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        miners.insert(
            key,
            MinerState {
                address,
                operator_id: reader.read_hash()?,
                stake: reader.read_u64()?,
                reputation: reader.read_i64()?,
                settled_tensor_work: reader.read_u64()?,
                pending_tensor_work: reader.read_u64()?,
                hardware_class: decode_hardware_class(reader.read_u8()?)?,
                gpu_utilization_bps: reader.read_u64()?,
            },
        );
    }
    Ok(miners)
}

fn encode_validators(out: &mut Vec<u8>, validators: &BTreeMap<Hash, ValidatorState>) {
    write_len(out, validators.len());
    for (address, validator) in validators {
        write_hash(out, address);
        write_hash(out, &validator.address);
        write_u64(out, validator.stake);
        write_i64(out, validator.reputation);
        write_u64(out, validator.valid_attestations);
        write_u64(out, validator.missed_assignments);
    }
}

fn decode_validators(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ValidatorState>> {
    let mut validators = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        validators.insert(
            key,
            ValidatorState {
                address,
                stake: reader.read_u64()?,
                reputation: reader.read_i64()?,
                valid_attestations: reader.read_u64()?,
                missed_assignments: reader.read_u64()?,
            },
        );
    }
    Ok(validators)
}

fn encode_jobs(out: &mut Vec<u8>, jobs: &BTreeMap<Hash, JobState>) {
    write_len(out, jobs.len());
    for (job_id, job) in jobs {
        write_hash(out, job_id);
        out.extend_from_slice(&codec::encode_job_payload(job));
    }
}

fn decode_jobs(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, JobState>> {
    let mut jobs = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let job = codec::decode_job_payload_from(reader.input, &mut reader.offset, None)
            .map_err(storage_codec_error)?;
        jobs.insert(key, job);
    }
    Ok(jobs)
}

fn storage_codec_error(error: codec::CodecError) -> TvmError {
    match error {
        codec::CodecError::Truncated => TvmError::Storage("truncated chain state"),
        codec::CodecError::TrailingBytes => TvmError::Storage("trailing chain state bytes"),
        codec::CodecError::UnknownJobTag => TvmError::Storage("unknown job tag"),
        codec::CodecError::UnknownReceiptTag => TvmError::Storage("unknown receipt tag"),
        codec::CodecError::UnknownDType => TvmError::Storage("unknown dtype"),
        codec::CodecError::UnknownPrimitiveType => TvmError::Storage("unknown primitive type"),
        codec::CodecError::UnknownVerificationResult => {
            TvmError::Storage("unknown verification result")
        }
        codec::CodecError::InvalidOptionalU64 => TvmError::Storage("invalid optional u64"),
        codec::CodecError::InvalidBool => TvmError::Storage("invalid boolean"),
        codec::CodecError::UsizeOverflow => TvmError::Storage("chain state length overflow"),
        codec::CodecError::ShapeVectorTooLarge => TvmError::Storage("shape vector too large"),
        codec::CodecError::HashVectorTooLarge => TvmError::Storage("hash vector too large"),
    }
}

fn encode_receipts(out: &mut Vec<u8>, receipts: &BTreeMap<Hash, ReceiptState>) {
    write_len(out, receipts.len());
    for (receipt_id, receipt) in receipts {
        write_hash(out, receipt_id);
        out.extend_from_slice(&codec::encode_receipt_payload(receipt));
    }
}

fn decode_receipts(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ReceiptState>> {
    let mut receipts = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let receipt = codec::decode_receipt_payload_from(reader.input, &mut reader.offset, None)
            .map_err(storage_codec_error)?;
        receipts.insert(key, receipt);
    }
    Ok(receipts)
}

fn encode_attestations(
    out: &mut Vec<u8>,
    attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>,
) {
    write_len(out, attestations.len());
    for (receipt_id, items) in attestations {
        write_hash(out, receipt_id);
        write_len(out, items.len());
        for attestation in items {
            out.extend_from_slice(&codec::encode_attestation_payload(attestation));
        }
    }
}

fn decode_attestations(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, Vec<ValidatorAttestation>>> {
    let mut attestations = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let receipt_id = reader.read_hash()?;
        let item_count = reader.read_len()?;
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            let attestation =
                codec::decode_attestation_payload_from(reader.input, &mut reader.offset)
                    .map_err(storage_codec_error)?;
            items.push(attestation);
        }
        attestations.insert(receipt_id, items);
    }
    Ok(attestations)
}

fn encode_block_votes(out: &mut Vec<u8>, votes: &BTreeMap<Hash, Vec<BlockVote>>) {
    write_len(out, votes.len());
    for (block_hash, votes) in votes {
        write_hash(out, block_hash);
        write_len(out, votes.len());
        for vote in votes {
            out.extend_from_slice(&codec::encode_block_vote_payload(vote));
        }
    }
}

fn decode_block_votes(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, Vec<BlockVote>>> {
    let mut block_votes = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let block_hash = reader.read_hash()?;
        let vote_count = reader.read_len()?;
        let mut votes = Vec::with_capacity(vote_count);
        for _ in 0..vote_count {
            let vote =
                codec::decode_block_vote_payload(reader.read_exact(codec::BLOCK_VOTE_PAYLOAD_LEN)?)
                    .ok_or(TvmError::Storage("invalid block vote payload length"))?;
            votes.push(vote);
        }
        block_votes.insert(block_hash, votes);
    }
    Ok(block_votes)
}

fn encode_model_states(out: &mut Vec<u8>, models: &BTreeMap<Hash, ModelState>) {
    write_len(out, models.len());
    for (model_id, model) in models {
        write_hash(out, model_id);
        write_hash(out, &model.model_id);
        write_hash(out, &model.architecture_hash);
        write_hash(out, &model.weight_root);
        write_option_hash(out, &model.optimizer_state_root);
        write_u64(out, model.step);
        write_hash(out, &model.config_hash);
    }
}

fn decode_model_states(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ModelState>> {
    let mut models = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let model_id = reader.read_hash()?;
        models.insert(
            key,
            ModelState {
                model_id,
                architecture_hash: reader.read_hash()?,
                weight_root: reader.read_hash()?,
                optimizer_state_root: reader.read_option_hash()?,
                step: reader.read_u64()?,
                config_hash: reader.read_hash()?,
            },
        );
    }
    Ok(models)
}

fn encode_rewards(out: &mut Vec<u8>, rewards: &RewardState) {
    write_len(out, rewards.balances().len());
    for (address, balance) in rewards.balances() {
        write_hash(out, address);
        write_u64(out, *balance);
    }
    write_u64(out, rewards.treasury());
}

fn decode_rewards(reader: &mut StateReader<'_>) -> Result<RewardState> {
    let mut balances = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        balances.insert(reader.read_hash()?, reader.read_u64()?);
    }
    Ok(RewardState::from_parts(balances, reader.read_u64()?))
}

fn encode_hash_set(out: &mut Vec<u8>, items: &BTreeSet<Hash>) {
    write_len(out, items.len());
    for item in items {
        write_hash(out, item);
    }
}

fn decode_hash_set(reader: &mut StateReader<'_>) -> Result<BTreeSet<Hash>> {
    let mut items = BTreeSet::new();
    for _ in 0..reader.read_len()? {
        items.insert(reader.read_hash()?);
    }
    Ok(items)
}

fn encode_hash_vec_map(out: &mut Vec<u8>, items: &BTreeMap<Hash, Vec<Hash>>) {
    write_len(out, items.len());
    for (key, values) in items {
        write_hash(out, key);
        write_len(out, values.len());
        for value in values {
            write_hash(out, value);
        }
    }
}

fn decode_hash_vec_map(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, Vec<Hash>>> {
    let mut items = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let mut values = Vec::new();
        for _ in 0..reader.read_len()? {
            values.push(reader.read_hash()?);
        }
        items.insert(key, values);
    }
    Ok(items)
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_len(out: &mut Vec<u8>, value: usize) {
    write_u64(out, value as u64);
}

fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn write_option_hash(out: &mut Vec<u8>, value: &Option<Hash>) {
    match value {
        Some(hash) => {
            out.push(1);
            write_hash(out, hash);
        }
        None => out.push(0),
    }
}

fn hardware_class_code(hardware_class: HardwareClass) -> u8 {
    match hardware_class {
        HardwareClass::Cpu => 1,
        HardwareClass::ConsumerGpu => 2,
        HardwareClass::DatacenterGpu => 3,
        HardwareClass::Other => 4,
    }
}

fn decode_hardware_class(tag: u8) -> Result<HardwareClass> {
    match tag {
        1 => Ok(HardwareClass::Cpu),
        2 => Ok(HardwareClass::ConsumerGpu),
        3 => Ok(HardwareClass::DatacenterGpu),
        4 => Ok(HardwareClass::Other),
        _ => Err(TvmError::Storage("unknown hardware class")),
    }
}

struct StateReader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> StateReader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.input.len().saturating_sub(self.offset) < len {
            return Err(TvmError::Storage("truncated chain state"));
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.input[start..self.offset])
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u64(&mut self) -> Result<u64> {
        let mut out = [0_u8; U64_LEN];
        out.copy_from_slice(self.read_exact(U64_LEN)?);
        Ok(u64::from_le_bytes(out))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let mut out = [0_u8; U64_LEN];
        out.copy_from_slice(self.read_exact(U64_LEN)?);
        Ok(i64::from_le_bytes(out))
    }

    fn read_len(&mut self) -> Result<usize> {
        Ok(self.read_u64()? as usize)
    }

    fn read_hash(&mut self) -> Result<Hash> {
        let mut out = [0_u8; HASH_LEN];
        out.copy_from_slice(self.read_exact(HASH_LEN)?);
        Ok(out)
    }

    fn read_option_hash(&mut self) -> Result<Option<Hash>> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_hash()?)),
            _ => Err(TvmError::Storage("invalid optional hash")),
        }
    }

    fn finish(&self) -> Result<()> {
        if self.offset != self.input.len() {
            return Err(TvmError::Storage("trailing chain state bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainCommand;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        PrimitiveType, TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{AttestationStatement, VerificationResult};

    fn chain_with_blocks(chain: &Chain, blocks: Vec<TensorBlock>) -> Chain {
        Chain::from_parts(ChainParts {
            params: chain.params().clone(),
            state: chain.state().clone(),
            blocks,
        })
    }

    fn durable_chain_fixture(label: &[u8]) -> Chain {
        let beacon = hash_bytes(b"test", &[label]);
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 1,
            freivalds: FreivaldsParams {
                full_rounds: 2,
                audit_rows: 3,
                validators_per_job: 2,
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 2,
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"durable-miner");
        let validator = address(b"durable-validator");
        chain
            .register_miner_with_profile_and_operator(
                miner,
                chain.params().miner_min_stake,
                hash_bytes(b"test", &[b"durable-operator"]),
                HardwareClass::DatacenterGpu,
                8_500,
            )
            .unwrap();
        chain
            .register_validator(validator, chain.params().validator_min_stake)
            .unwrap();
        chain.credit_account(miner, 1_000);
        chain.transfer(miner, validator, 125).unwrap();

        let matmul = MatmulJob::synthetic(0, 7, 4, 3, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&matmul, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(matmul.clone()));
        let mut no_modulus = MatmulJob::synthetic(0, 8, 2, 2, 2, &beacon, 11);
        no_modulus.modulus = None;
        chain.submit_job(JobState::TensorOp(no_modulus));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let attestation = ValidatorAttestation::new(
            validator,
            chain.params().validator_min_stake,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );
        chain.submit_attestation(attestation).unwrap();

        let model_id = hash_bytes(b"test", &[b"durable-model"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        chain
            .register_model(
                model_id,
                hash_bytes(b"test", &[b"architecture"]),
                weights.commitment_root(),
                hash_bytes(b"test", &[b"config"]),
            )
            .unwrap();
        chain
            .register_model(
                hash_bytes(b"test", &[b"durable-model-with-optimizer"]),
                hash_bytes(b"test", &[b"architecture-2"]),
                weights.commitment_root(),
                hash_bytes(b"test", &[b"config-2"]),
            )
            .unwrap();
        chain
            .set_model_optimizer_state_root_for_testing(
                model_id,
                Some(hash_bytes(b"test", &[b"optimizer"])),
            )
            .unwrap();
        let linear = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 12,
        });
        let (linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&linear, miner, &weights, 2, 7).unwrap();
        chain.submit_job(JobState::LinearTrainingStep(linear));
        chain.submit_linear_receipt(linear_receipt.clone()).unwrap();
        chain.mark_receipt_settled_for_testing(receipt.receipt_id);
        chain.mark_receipt_data_unavailable_for_testing(linear_receipt.receipt_id);
        chain
            .apply_command(ChainCommand::CreditReward {
                address: miner,
                amount: 77,
            })
            .unwrap();
        chain.set_reward_treasury_for_testing(11);

        let block = chain.produce_block(validator, 1_000).unwrap();
        chain
            .submit_block_vote(BlockVote::new(
                validator,
                chain.params().validator_min_stake,
                &block,
            ))
            .unwrap();
        chain.produce_block(validator, 1_006).unwrap();
        chain
    }

    #[test]
    fn node_store_persists_snapshot_block_log_and_peer_book_paths() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"node-store"]));
        let miner = address(b"node-store-miner");
        chain
            .register_miner(miner, chain.params().miner_min_stake)
            .unwrap();
        chain
            .register_validator(miner, chain.params().validator_min_stake)
            .unwrap();
        chain.produce_block(miner, 1_000).unwrap();
        chain.produce_block(miner, 1_006).unwrap();

        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-store-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        assert_eq!(store.data_dir(), data_dir.as_path());
        assert_eq!(
            store.snapshot_store().path(),
            data_dir.join(SNAPSHOT_FILE_NAME).as_path()
        );
        assert_eq!(
            store.block_log_store().path(),
            data_dir.join(BLOCK_LOG_FILE_NAME).as_path()
        );
        assert_eq!(
            store.chain_state_store().path(),
            data_dir.join(CHAIN_STATE_FILE_NAME).as_path()
        );
        assert_eq!(
            store.peer_book_store().path(),
            data_dir.join(PEER_BOOK_FILE_NAME).as_path()
        );

        let status = store.persist_chain(&chain).unwrap();
        assert_eq!(status.data_dir, data_dir);
        assert_eq!(status.block_count, 2);
        assert_eq!(status.latest_block_hash, chain.blocks()[1].hash());
        assert_eq!(
            status.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(status.snapshot, ChainSnapshot::from_chain(&chain));
        assert_eq!(store.status().unwrap(), status);
        let loaded = store.load().unwrap();
        assert_eq!(loaded.snapshot, status.snapshot);
        assert_eq!(loaded.blocks.as_slice(), chain.blocks());
        assert_eq!(store.load_chain().unwrap(), chain);

        chain.produce_block(miner, 1_012).unwrap();
        let updated = store.persist_chain(&chain).unwrap();
        assert_eq!(updated.block_count, 3);
        assert_ne!(updated.block_log_root, status.block_log_root);
        assert_eq!(
            updated.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(store.load().unwrap().blocks.as_slice(), chain.blocks());
        assert_eq!(store.load_chain().unwrap(), chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(store.data_dir());
    }

    #[test]
    fn node_store_recovers_snapshot_and_block_log_from_chain_state() {
        let chain = durable_chain_fixture(b"node-store-recovery");
        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-recovery-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        let original = store.persist_chain(&chain).unwrap();

        let mut stale_snapshot = ChainSnapshot::from_chain(&chain);
        stale_snapshot.block_count = stale_snapshot.block_count.saturating_sub(1);
        store.snapshot_store().save(&stale_snapshot).unwrap();
        assert_eq!(
            store.status(),
            Err(TvmError::Storage("snapshot block count mismatch"))
        );

        let recovered = store.recover_from_chain_state().unwrap();
        assert_eq!(recovered.snapshot, ChainSnapshot::from_chain(&chain));
        assert_eq!(recovered.block_count, original.block_count);
        assert_eq!(store.load_chain().unwrap(), chain);

        let mut ahead = chain.clone();
        ahead
            .produce_block(address(b"durable-validator"), 1_012)
            .unwrap();
        store
            .block_log_store()
            .append_block(ahead.blocks().last().unwrap())
            .unwrap();
        assert_eq!(
            store.status(),
            Err(TvmError::Storage("snapshot block count mismatch"))
        );

        let recovered_again = store.recover_from_chain_state().unwrap();
        assert_eq!(recovered_again.block_count, chain.blocks().len());
        assert_eq!(
            store.block_log_store().load_blocks().unwrap().as_slice(),
            chain.blocks()
        );
        assert_eq!(store.load_chain().unwrap(), chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }

    #[test]
    fn node_store_satisfies_chain_store_boundary() {
        fn persist_and_reload<S>(store: &S, chain: &Chain) -> (NodeStoreStatus, Chain)
        where
            S: ChainStore<Chain = Chain>,
        {
            let status = store.persist_chain(chain).unwrap();
            let loaded = store.load_chain().unwrap();
            (status, loaded)
        }

        let mut chain = Chain::new(hash_bytes(b"test", &[b"chain-store-boundary"]));
        let miner = address(b"chain-store-boundary-miner");
        chain
            .register_miner(miner, chain.params().miner_min_stake)
            .unwrap();
        chain
            .register_validator(miner, chain.params().validator_min_stake)
            .unwrap();
        chain.produce_block(miner, 1_000).unwrap();

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-chain-store-boundary-{}",
            std::process::id()
        ));
        let store = NodeStore::open(data_dir);
        let (status, loaded) = persist_and_reload(&store, &chain);

        assert_eq!(status.block_count, 1);
        assert_eq!(status.latest_block_hash, chain.blocks()[0].hash());
        assert_eq!(
            status.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(loaded, chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(store.data_dir());
    }

    #[test]
    fn chain_state_store_roundtrips_full_chain_and_detects_tampering() {
        let chain = durable_chain_fixture(b"chain-state-store");
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-state-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = ChainStateStore::new(path.clone());
        assert_eq!(store.path(), path.as_path());
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("failed to read chain state"))
        );

        store.save_chain(&chain).unwrap();
        assert_eq!(store.load_chain().unwrap(), chain);
        assert_eq!(
            decode_chain_state_file(&encode_chain_state_file(&chain)).unwrap(),
            chain
        );

        let mut tampered = encode_chain_state_file(&chain);
        tampered[CHAIN_STATE_MAGIC.len()] ^= 1;
        assert_eq!(
            decode_chain_state_file(&tampered),
            Err(TvmError::Storage("chain state checksum mismatch"))
        );
        assert_eq!(
            decode_chain_state_file(b"bad"),
            Err(TvmError::Storage("invalid chain state magic"))
        );
        assert_eq!(
            decode_chain_state_file(CHAIN_STATE_MAGIC),
            Err(TvmError::Storage("invalid chain state length"))
        );

        let mut trailing = encode_chain_state_payload(&chain);
        trailing.push(0);
        assert_eq!(
            decode_chain_state_payload(&trailing),
            Err(TvmError::Storage("trailing chain state bytes"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn node_store_load_chain_rejects_state_disagreement() {
        let chain = durable_chain_fixture(b"node-store-full-state");
        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-full-state-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        store.persist_chain(&chain).unwrap();
        assert_eq!(store.load_chain().unwrap(), chain);

        let mut changed_state = chain.clone();
        changed_state.credit_account(address(b"unexpected-account"), 1);
        store
            .chain_state_store()
            .save_chain(&changed_state)
            .unwrap();
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("chain state snapshot mismatch"))
        );

        let mut blocks = chain.blocks().to_vec();
        blocks[0].timestamp = blocks[0].timestamp.saturating_add(1);
        let changed_blocks = chain_with_blocks(&chain, blocks);
        store
            .chain_state_store()
            .save_chain(&changed_blocks)
            .unwrap();
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("chain state block log mismatch"))
        );

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }

    #[test]
    fn chain_state_decoder_rejects_invalid_tags_and_values() {
        const TENSOR_DTYPE_OFFSET: usize = 1 + 32 + 8 + 8 + 8 + 8;
        const TENSOR_OPTIONAL_MODULUS_OFFSET: usize = TENSOR_DTYPE_OFFSET + 1;

        assert_eq!(hardware_class_code(HardwareClass::Cpu), 1);
        assert_eq!(hardware_class_code(HardwareClass::ConsumerGpu), 2);
        assert_eq!(hardware_class_code(HardwareClass::DatacenterGpu), 3);
        assert_eq!(hardware_class_code(HardwareClass::Other), 4);
        assert_eq!(decode_hardware_class(1).unwrap(), HardwareClass::Cpu);
        assert_eq!(
            decode_hardware_class(2).unwrap(),
            HardwareClass::ConsumerGpu
        );
        assert_eq!(
            decode_hardware_class(3).unwrap(),
            HardwareClass::DatacenterGpu
        );
        assert_eq!(decode_hardware_class(4).unwrap(), HardwareClass::Other);
        assert_eq!(
            decode_hardware_class(9),
            Err(TvmError::Storage("unknown hardware class"))
        );

        assert_eq!(
            StateReader::new(&[]).read_u8(),
            Err(TvmError::Storage("truncated chain state"))
        );
        assert_eq!(
            StateReader::new(&[2]).read_option_hash(),
            Err(TvmError::Storage("invalid optional hash"))
        );
        let mut none = Vec::new();
        write_option_hash(&mut none, &None);
        assert_eq!(StateReader::new(&none).read_option_hash().unwrap(), None);

        let key = hash_bytes(b"test", &[b"bad-key"]);
        let mut bad_job = Vec::new();
        write_len(&mut bad_job, 1);
        write_hash(&mut bad_job, &key);
        bad_job.push(9);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job)),
            Err(TvmError::Storage("unknown job tag"))
        );

        let bad_job_template = JobState::TensorOp(MatmulJob::synthetic(
            0,
            2,
            2,
            2,
            2,
            &hash_bytes(b"test", &[b"bad-job-beacon"]),
            10,
        ));

        let mut bad_job_dtype = Vec::new();
        write_len(&mut bad_job_dtype, 1);
        write_hash(&mut bad_job_dtype, &key);
        let mut bad_job_dtype_payload = codec::encode_job_payload(&bad_job_template);
        bad_job_dtype_payload[TENSOR_DTYPE_OFFSET] = 9;
        bad_job_dtype.extend_from_slice(&bad_job_dtype_payload);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job_dtype)),
            Err(TvmError::Storage("unknown dtype"))
        );

        let mut bad_job_optional = Vec::new();
        write_len(&mut bad_job_optional, 1);
        write_hash(&mut bad_job_optional, &key);
        let mut bad_job_optional_payload = codec::encode_job_payload(&bad_job_template);
        bad_job_optional_payload[TENSOR_OPTIONAL_MODULUS_OFFSET] = 9;
        bad_job_optional.extend_from_slice(&bad_job_optional_payload);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job_optional)),
            Err(TvmError::Storage("invalid optional u64"))
        );

        let mut bad_receipt = Vec::new();
        write_len(&mut bad_receipt, 1);
        write_hash(&mut bad_receipt, &key);
        bad_receipt.push(9);
        assert_eq!(
            decode_receipts(&mut StateReader::new(&bad_receipt)),
            Err(TvmError::Storage("unknown receipt tag"))
        );
    }

    #[test]
    fn node_store_detects_snapshot_and_block_log_disagreement() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"node-store-mismatch"]));
        let miner = address(b"node-store-mismatch-miner");
        chain
            .register_miner(miner, chain.params().miner_min_stake)
            .unwrap();
        chain
            .register_validator(miner, chain.params().validator_min_stake)
            .unwrap();
        chain.produce_block(miner, 1_000).unwrap();

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-node-store-mismatch-{}",
            std::process::id()
        ));
        let store = NodeStore::open(data_dir.clone());
        store
            .snapshot_store()
            .save(&ChainSnapshot::from_chain(&chain))
            .unwrap();
        assert_eq!(
            store.load(),
            Err(TvmError::Storage("snapshot block count mismatch"))
        );

        store.block_log_store().append_chain(&chain).unwrap();
        let mut bad_snapshot = ChainSnapshot::from_chain(&chain);
        bad_snapshot.latest_block_hash = hash_bytes(b"test", &[b"wrong-latest"]);
        store.snapshot_store().save(&bad_snapshot).unwrap();
        assert_eq!(
            store.status(),
            Err(TvmError::Storage("snapshot latest block mismatch"))
        );

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }
}
