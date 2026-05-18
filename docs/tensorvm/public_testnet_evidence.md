# TensorVM Public Testnet Evidence

Status: no complete external public-testnet evidence bundle is available yet.

This document is the publication target for the independently checkable evidence bundle required before
the TensorVM MVP can be called fully complete. A complete bundle must be produced from an external public
run, not from the local harness.

## Required Bundle

A complete evidence bundle must include:

- a public `https://`, `ipfs://`, or `ar://` location for the evidence manifest
- manifest signature records
- independent auditor or verifier records
- signed miner and validator heartbeat history for the full run
- independent operator identity or attestation records
- block history for the full 7-day run
- finality history for the full 7-day run
- data-availability measurements for checked tensor receipts
- invalid-work submission and rejection evidence
- reward-settlement records for verified TensorWork
- proof that production libp2p was used for peer discovery, gossip, and request/response propagation
- reachability records for deployed RPC, explorer, faucet, and telemetry services

## Current Repository Evidence

The local reference crate exposes typed validation for this future bundle through
`PublicTestnetEvidenceBundle`. The validator intentionally separates:

- `PublicTestnetRunEvidence`, which checks run-level protocol evidence
- `PublicTestnetEvidenceBundle`, which additionally checks publication, signatures, auditors, and
  independently checkable supporting records

The current local simulation and docs do not satisfy this bundle requirement.
