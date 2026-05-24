use super::arguments::parse_u64;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::sign_public_run_window;
use crate::types::{Address, Hash};
use std::collections::BTreeMap;

pub(super) fn run_window_evidence_line(
    bundle_id: Hash,
    manifest_signer: Address,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if run_ended_at_unix_seconds < run_started_at_unix_seconds {
        return Err(TvmError::InvalidReceipt(
            "public run window block range is invalid",
        ));
    }
    if observed_blocks == 0 {
        return Err(TvmError::InvalidReceipt(
            "observed blocks argument is empty",
        ));
    }
    let signature = sign_public_run_window(
        &manifest_signer,
        &bundle_id,
        run_started_at_unix_seconds,
        run_ended_at_unix_seconds,
        observed_blocks,
    );
    Ok(format!(
        "run_started_at_unix_seconds={run_started_at_unix_seconds}\nrun_ended_at_unix_seconds={run_ended_at_unix_seconds}\nrun_window_signature={}\nobserved_blocks={observed_blocks}",
        hex(&signature)
    ))
}

pub(super) struct RunWindowObservationSummary {
    pub(super) run_started_at_unix_seconds: u64,
    pub(super) run_ended_at_unix_seconds: u64,
    pub(super) observed_blocks: u64,
}

pub(super) fn run_window_evidence_line_from_file(
    bundle_id: Hash,
    manifest_signer: Address,
    block_observation_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(block_observation_file)
        .map_err(|_| TvmError::Storage("failed to read run-window block observation file"))?;
    let summary = run_window_observation_summary_from_file(&contents)?;
    run_window_evidence_line(
        bundle_id,
        manifest_signer,
        summary.run_started_at_unix_seconds,
        summary.run_ended_at_unix_seconds,
        summary.observed_blocks,
    )
}

pub(super) fn run_window_observation_summary_from_file(
    contents: &str,
) -> Result<RunWindowObservationSummary> {
    let mut observations = BTreeMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "run-window observation line has leading or trailing whitespace",
            ));
        }
        let (block, timestamp) = parse_run_window_observation_line(line)?;
        if timestamp == 0 {
            return Err(TvmError::InvalidReceipt(
                "run-window observation timestamp is empty",
            ));
        }
        if observations.insert(block, timestamp).is_some() {
            return Err(TvmError::InvalidReceipt(
                "duplicate run-window observation block",
            ));
        }
    }
    let Some((&first_block, &run_started_at_unix_seconds)) = observations.iter().next() else {
        return Err(TvmError::InvalidReceipt(
            "run-window observation file has no observations",
        ));
    };
    let (&last_block, &run_ended_at_unix_seconds) = observations.iter().next_back().unwrap();
    let observed_blocks = observations.len() as u64;
    let expected_block_count = last_block
        .checked_sub(first_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "run-window observation block range is invalid",
        ))?;
    if observed_blocks != expected_block_count {
        return Err(TvmError::InvalidReceipt(
            "run-window observation blocks must be contiguous",
        ));
    }
    let mut previous_timestamp = None;
    for timestamp in observations.values().copied() {
        if let Some(previous) = previous_timestamp
            && timestamp < previous
        {
            return Err(TvmError::InvalidReceipt(
                "run-window observation timestamps must be monotonic",
            ));
        }
        previous_timestamp = Some(timestamp);
    }
    Ok(RunWindowObservationSummary {
        run_started_at_unix_seconds,
        run_ended_at_unix_seconds,
        observed_blocks,
    })
}

fn parse_run_window_observation_line(line: &str) -> Result<(u64, u64)> {
    let record = line
        .strip_prefix("run_window_observation=")
        .ok_or(TvmError::InvalidReceipt(
            "unsupported run-window observation line",
        ))?;
    let fields: Vec<&str> = record.split(',').collect();
    if fields.len() != 2 {
        return Err(TvmError::InvalidReceipt("malformed run-window observation"));
    }
    Ok((parse_u64(fields[0])?, parse_u64(fields[1])?))
}
