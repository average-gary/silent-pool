---
title: "Incentive Compatibility of Bitcoin Mining Pool Reward Functions"
authors: [Okke Schrijvers, Joseph Bonneau, Dan Boneh, Tim Roughgarden]
year: 2016
venue: Financial Cryptography (FC'16), Springer LNCS
source: https://fc16.ifca.ai/preproceedings/28_Schrijvers.pdf
type: papers
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [pplns, incentive-compatibility, mechanism-design, block-withholding, Nash]
summary: "Formal mechanism-design counterpart to Rosenfeld 2011. Reframes pool reward design as a mechanism-design problem: which reward functions make truthful share submission a Nash equilibrium?"
---

# Incentive Compatibility of Bitcoin Mining Pool Reward Functions (Schrijvers et al. FC'16)

Formal mechanism-design counterpart to Rosenfeld 2011. Reframes pool reward design as a mechanism-design problem: which reward functions make truthful share submission a Nash equilibrium?

## Definitions

**Incentive compatibility (IC)** for a pool reward function: every miner truthfully reporting every share they find (no block withholding) is a Nash equilibrium given the reward function and the strategies of all other miners.

## Key results

- **Proportional is NOT incentive compatible.** Miners can profit by withholding full blocks under proportional payout.
- **PPS is IC** but at high cost to operator (variance-bearing → reserve requirement).
- **PPLNS is IC only under specific parameter regimes.** Compatibility depends on the window size N relative to round length. Small-N PPLNS approaches proportional and loses IC.
- **Authors construct a novel IC reward function** that preserves miners' steady-payout property while remaining provably IC.

## Implications for modern schemes

- **TIDES** (8× difficulty window) sits in a regime where N is large relative to round length → likely IC, but no formal proof in TIDES doc.
- **PPLNS-JD / SLICE** inherits PPLNS's IC parameter sensitivity.
- **eHash / hashpool** — IC analysis is open; bearer-token issuance changes the game-theoretic surface (the share *is* the reward, not a claim against future reward).

## Why it matters for this wiki

Pairs with Rosenfeld (intra-pool variance) and Eyal "Miner's Dilemma" (inter-pool BWH) to give the full game-theoretic frame: a payout scheme must be variance-tolerable AND IC AND robust to inter-pool sabotage. Few schemes satisfy all three.
