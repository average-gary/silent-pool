---
title: "DMND / Demand Pool — first production Stratum V2 pool, SLICE payout"
publication: dmnd.work
source: https://dmnd.work
type: articles
ingested: 2026-05-23
quality: 4
credibility: medium
confidence: medium
tags: [dmnd, demand-pool, SLICE, pplns-jd, stratum-v2]
summary: "Production Stratum V2 pool. Markets itself as the first SV2 pool with end-to-end encrypted binary protocol and miner-side block templates. Operator: Guru Protocol Ltd (England & Wales)."
---

# Demand Pool (DMND) — SLICE / PPLNS-JD

Production Stratum V2 pool. Markets itself as the first SV2 pool with end-to-end encrypted binary protocol and miner-side block templates. Operator: Guru Protocol Ltd (England & Wales).

## Leadership

- **Alejandro De La Torre** — CEO. Ex-VP at Poolin.
- **Filippo Merli** — CTO. Long-time Stratum Reference Implementation (SRI) contributor.

## Payout system: SLICE (a.k.a. PPLNS-JD)

- Branded as "PPLNS with Job Declarator." 
- Every valid share recorded against miner profile; payout proportional to share contribution at block discovery.
- Bound to JD-declared jobs — i.e. shares are tied to the miner's *own* declared job (block template), not the pool's.
- **Slice removes the FPPS subsidy** where miners pay (via fee) for the pool's transaction-filter policies. Under SLICE, the miner picks the transactions.
- No published fee % or window N parameter on landing pages (gap — needs deeper docs).

## Operational claims

- **Rejection rate: 0.0151%** (vs. 0.5–2% on Stratum V1 deployments).
- Zero hash-hijack incidents since launch (binary encrypted framing eliminates the SV1 plaintext attack surface).
- "SOC 2 Type 2 ready" positioning.
- Tagline: *"Miners build the block. Miners keep the rBTC."*

## Architecture

- Miner-side block templates via SV2 Job Declaration.
- End-to-end encrypted binary protocol (Noise-over-TCP).
- ~1–3 ms job-switch latency vs. ~200–300 ms on Stratum V1 — direct profitability lift via lower stales.

## Position in the landscape

DMND is the **PPLNS-JD reference deployment**. Where OCEAN's TIDES is "PPLNS done right at the accounting layer," SLICE is "PPLNS done right at the protocol layer (JD-bound shares)." Both are non-custodial alternatives to FPPS, but they make different bets:

- TIDES: keep coinbase-output payout, perfect the share log.
- SLICE: bind shares to JD jobs, let miners control templates.

These are complementary, not competing, in principle — TIDES + DATUM is OCEAN's version of the same idea.

## Gaps

- Fee schedule not on landing page.
- N parameter for SLICE not published. Worth following up via DMND blog/Discord/GitHub.
