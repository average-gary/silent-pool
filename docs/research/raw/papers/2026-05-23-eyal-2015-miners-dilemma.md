---
title: "The Miner's Dilemma"
authors: [Ittay Eyal]
year: 2015
venue: IEEE Symposium on Security and Privacy
source: https://arxiv.org/abs/1411.7099
type: papers
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [block-withholding, BWH, FAW, pool-vs-pool, Nash, attack]
summary: "Models inter-pool **block withholding** (BWH) as a non-cooperative game where pools infiltrate each other."
---

# The Miner's Dilemma (Eyal IEEE S&P 2015)

Models inter-pool **block withholding** (BWH) as a non-cooperative game where pools infiltrate each other.

## Core results

- **"No attacks" is never a Nash equilibrium** when more than one open pool exists.
- With two or identical pools, the equilibrium is a tragedy-of-the-commons: all pools attack and all earn less than baseline.
- Attacker pool gains short-run revenue at the cost of the victim pool's PPS/PPLNS payout. Direct mapping from reward-scheme design to attack incentives.

## Two attack flavors

1. **Classical BWH** — pure sabotage (Rosenfeld 2011 noted; can't be modeled in his single-pool frame).
2. **Profitable BWH** — attacker shares attack revenue with own loyal miners. Eyal's contribution: shows this is positive-EV under realistic parameters.

## Implication for payout-scheme analysis

Reward functions cannot be analyzed in isolation. **Cross-pool attacks** make the system-wide equilibrium worse than any single-pool design predicts. PPS pools are the most attractive sabotage targets (operator absorbs the loss); PPLNS pools shift the loss to miners but become unstable.

## Connection to modern landscape

- 2017 Kwon et al. extends to **FAW** (Fork After Withholding) — even more profitable than pure BWH.
- 2024-2026 decentralized pools (OCEAN, DMND, hashpool, p2poolv2) sidestep some BWH dynamics by changing the trust model: you can't profitably infiltrate a pool that has no internal ledger to manipulate (eHash) or that lets miners declare their own jobs (PPLNS-JD).
