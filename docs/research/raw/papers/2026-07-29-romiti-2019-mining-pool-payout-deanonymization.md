---
title: "A Deep Dive into Bitcoin Mining Pools: An Empirical Analysis of Mining Shares"
authors: [Matteo Romiti, Aljosha Judmayer, Alexei Zamyatin, Bernhard Haslhofer]
year: 2019
venue: "WEIS 2019 (arXiv 1905.05999)"
source: https://arxiv.org/abs/1905.05999
type: papers
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [deanonymization, chain-analysis, clustering, payout-graph, address-reuse, gini, attribution, primary]
summary: "Empirical proof that pool payouts are **already deanonymized at scale by a third party with no pool cooperation**. This is the chain-analysis half of the attribution threat model, with hard coverage numbers."
---

# Deep Dive into Bitcoin Mining Pools (Romiti et al., WEIS 2019)

Empirical proof that pool payouts are **already deanonymized at scale by a third party with
no pool cooperation**. This is the chain-analysis half of the attribution threat model, with
hard coverage numbers.

## Attribution pipeline

1. Coinbase **reward address** — treated as authoritative, since faking it forfeits the
   reward.
2. Coinbase **markers/tags** — explicitly noted as *not cryptographically secured*, hence
   fakeable.
3. **Multiple-input clustering** via GraphSense.

Attributed 54,409 blocks in range 500000–556400 — ~2,568 blocks (~32,100 BTC) more than
blockchain.info alone, with conflicts on only 684/556,400 blocks (0.0012 %).

## Per-pool payout-graph fingerprints (blocks 510,000–514,032)

- **BTC.com** — single reward address `A₁` → one **collector address** `A₂`; payouts chained
  through it, ~10³ outputs of a few mBTC each.
- **AntPool** — payout transactions with **exactly 101 outputs**; change address always the
  largest output; a chain of never-reused change addresses.
- **ViaBTC** — a dozen rotating addresses each receiving **exactly 10 BTC** from the reward
  address.

## Coverage measured against the sum constraint

Comparing BTC paid to identified member addresses (BTC_P) against BTC mined (BTC_M):

| Pool | Individual miners identified | Blocks | Addresses | Clusters |
|---|---|---|---|---|
| BTC.com | **92 %** | 1,020 | 20,444 | 8,900 |
| ViaBTC | **75 %** | — | — | — |
| AntPool | **30 %** | — | — | — |

AntPool's low figure is attributed to the authors' strict 101-output filter, not to better
privacy.

## Address reuse is the attribution lever

Median payout-address reuse: **μ = 20 (BTC.com), 5 (ViaBTC), 2 (AntPool)**. AntPool members
rotate payout addresses fastest — the one behavioral privacy defense visible in the data, and
the direct empirical argument for per-payout address derivation.

## Concentration (the distribution shape recovered from public data)

≤20 clusters receive >50 % of payouts in each pool. 50 % of ViaBTC hashrate sits in **7
clusters**; BTC.com 20; AntPool 15. Gini on member shares: **0.945 BTC.com, 0.942 ViaBTC,
0.938 AntPool**.

Individual miners were found operating **simultaneously across all three pools** — cross-pool
linkage of the same actor from payout clustering alone.

Note also: pools "distribute the mining rewards not at every mined block, but within a longer
time period, combining payments to minimize the transaction fees" — which is why blocks mined
exceeded payouts detected, and which is the one shape-breaking mitigation visible in the data.

**Explicitly out of scope in the clustering literature**: coinbase transactions themselves.
A companion survey (arXiv 2403.00523) states "This is not true for Coinbase transactions, but
we will not consider them in this work" — so coinbase-direct payout deanonymization by amount
has no dedicated study.
