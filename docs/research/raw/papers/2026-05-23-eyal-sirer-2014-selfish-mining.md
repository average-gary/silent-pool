---
title: "Majority is Not Enough: Bitcoin Mining is Vulnerable"
authors: [Ittay Eyal, Emin Gün Sirer]
year: 2014
venue: Financial Cryptography 2014 (also CACM 2018)
source: https://arxiv.org/abs/1311.0243
type: papers
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [selfish-mining, attack, Eyal, Sirer, threshold]
summary: "Original selfish-mining paper. Pool-vs-network attack distinct from BWH/FAW."
---

# Majority is Not Enough — Selfish Mining (Eyal & Sirer 2014)

Original selfish-mining paper. Pool-vs-network attack distinct from BWH/FAW.

## Mechanism

Selfish miner mines on a private chain. When public chain catches up, attacker selectively releases blocks to make their private chain appear longer. Honest miners' blocks become orphaned → wasted hashrate.

## Profitability threshold

Attacker is profitable when:

`(1−γ)/(3−2γ) < α < 1/2`

Where γ = fraction of honest network mining on the attacker's branch when there's a tie.

| γ | Threshold α |
|---|---|
| 0 (no tie-breaking advantage) | **1/3 (33.3%)** |
| 0.5 | **1/4 (25%)** |
| ~1 (full tie advantage) | **~0%** |

## Revenue formula

`R_pool = [α(1−α)²(4α + γ(1−2α)) − α³] / [1 − α(1+(2−α)α)]`

## Sapirshtein, Sompolinsky, Zohar (FC 2016) refinement

Reframes selfish mining as MDP, finds ε-optimal policies that **lower the threshold below SM1**:

- γ = 0 → ~**0.2321** (vs Eyal's 1/3)
- γ = 1 → 0

So even a 23% attacker can profit under optimal selfish mining.

## Payout-scheme interaction

**Eyal/Sirer assume proportional distribution; they do NOT distinguish PPS / PPLNS / FPPS.** Their model is pool-vs-network, not within-pool.

The paper's Section 8 ("Practical Considerations") discusses propagation/γ, NOT payout scheme variants. **No formal selfish-mining-vs-payout-scheme analysis exists in the canonical literature** — this is a genuine gap, worth flagging in the wiki.

## Implication for this wiki

- Selfish mining is **scheme-invariant on attacker profit**, like FAW.
- Selfish mining is also **scheme-dependent on incidence**: under FPPS, the pool eats the orphan-loss (operator pays miners regardless); under PPLNS-class, miners eat the orphan-loss directly.
- Mitigation status: **none deployed**. Same as FAW.

## See also

- [[2026-05-23-kwon-2017-faw|Kwon et al. FAW (CCS 2017)]]
- [[2026-05-23-eyal-2015-miners-dilemma|Eyal IEEE S&P'15 — Miner's Dilemma]] (BWH)
- [[../../wiki/concepts/block-withholding|Block-Withholding & FAW concept article]]
