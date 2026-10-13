---
title: "Parasite Pool (The Bitcoin Manual)"
author: Che Kohler
publication: thebitcoinmanual.com
source: https://thebitcoinmanual.com/articles/parasite-pool/
date: 2025-10-24
type: articles
ingested: 2026-05-26
quality: 4
credibility: medium
confidence: high
tags: [parasite-pool, practitioner, critique, economics]
summary: "Highest-signal practitioner explainer on Parasite Pool. Mixes mechanics description with quantitative economic critique — the only article in the source corpus that surfaces the bootstrapping math."
---

# Parasite Pool — The Bitcoin Manual

Highest-signal practitioner explainer on Parasite Pool. Mixes mechanics description with quantitative economic critique — the only article in the source corpus that surfaces the bootstrapping math.

## Mechanism described

- Hybrid scheme: **1 BTC finder bonus + modified-PPLNS residual** (1 / 2.125 BTC split, ≈22% finder's premium of post-halving 3.125 BTC subsidy).
- Share window = **all cumulative shares since the pool's most recent block**. Not rolling-N. Distinct from classical PPLNS.
- 0% pool fee, 10-sat minimum Lightning payout, "loyalty" metric on dashboard (definition not public).

## Critical findings (the article's value)

- **Bootstrapping math is brutal**: pool hashrate ~24.92 PH/s ≈ 0.0025% of network → expected **~291+ days to first block**. Variance fragility is structural at this scale.
- **22% reward discount vs. solo** is itself a centralization pressure: only miners with enough hash to plausibly find a block accept the discount; everyone else effectively subsidizes finders.
- **Unbounded share window** is exploitable by **late-joiners after a long dry spell** — a worst-case PPLNS-hopping pattern.
- Custody / Lightning mechanics flagged as opaque: "loyalty" is undefined, no on-chain proof of distribution provided.

## Why ingestion-worthy

Triangulation source for Parasite mechanics + the only practitioner article that makes the variance-and-discount math explicit.

## See also

- [[2026-05-26-zkshark-parasite-pool-substack]] — founder rationale being critiqued
- [[2026-05-26-parasitepool-para-github]] (repo)
- [[../papers/2026-05-23-schrijvers-2016-incentive-compatibility]] — IC framework that explains why hybrid PPLNS variants like this remain non-IC
