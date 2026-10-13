---
title: "Stratum V2 Specification (sv2-spec)"
publication: stratum-mining/sv2-spec (GitHub)
source: https://github.com/stratum-mining/sv2-spec
type: repos
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [stratum-v2, sv2-spec, mining-protocol, job-declaration]
summary: "Canonical wire-level spec. SRI v1.0 released March 2024."
---

# Stratum V2 Specification

Canonical wire-level spec. SRI v1.0 released March 2024.

## Section 5 — Mining Protocol (share accounting baseline)

Wire-level share accounting primitives every SV2 pool must implement.

- `SubmitSharesStandard` fields: `channel_id` (U32), `sequence_number` (U32), `job_id` (U32), `nonce` (U32), `ntime` (U32), `version` (U32).
- `SubmitSharesExtended` adds `extranonce` (B0_32) of negotiated size.
- Share validation gated by `SetTarget` message — submits leading to hashes higher than target are rejected.
- `SubmitShares.Success` carries: `last_sequence_number`, `new_submits_accepted_count`, `new_shares_sum` (sum of difficulty within batch). Counters reset per batch.
- Rejection codes: `stale-share`, `difficulty-too-low`, `invalid-job-id`.

## Section 6 — Job Declaration Protocol (PPLNS-JD substrate)

The protocol that decouples *block construction* (miner) from *payout accounting* (pool). Prerequisite for SLICE / PPLNS-JD / hashpool / non-custodial schemes.

- Two entities: **Job Declarator Server (JDS)** pool-side, **Job Declarator Client (JDC)** miner-side.
- Two modes:
  - **Coinbase-only Mode** — JDS only sees the coinbase; preserves miner's template privacy.
  - **Full-Template Mode** — JDC sends txids; JDS may request full txs via `ProvideMissingTransactions`.
- Message set: `AllocateMiningJobToken`, `AllocateMiningJobToken.Success`, `DeclareMiningJob`, `DeclareMiningJob.Success/Error`, `ProvideMissingTransactions(.Success)`, `PushSolution`.
- Pool payout enforced as the **first coinbase output** designated by JDS in the token allocation. Miners may add additional outputs.
- Spec text: *"Pools…are only responsible for accounting shares and distributing rewards"* — explicit decoupling.

## Why it matters for payout schemas

Stratum V2 + JD is the *infrastructure* that made the 2024-2026 payout innovations practically deployable:
- TIDES + DATUM (OCEAN)
- SLICE / PPLNS-JD (DMND/Demand Pool)
- eHash / hashpool
- p2poolv2 share-chain

Without JD, payout-scheme reform is rearranging accounting in custodial pools. With JD, miners control the block; the pool just counts shares.

## Designers

Pavel Moravec & Jan Čapek (Braiins) + Matt Corallo proposed Stratum V2 in 2019, building on Corallo's earlier BetterHash proposal.
