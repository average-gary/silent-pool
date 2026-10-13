---
title: "Analysis of Bitcoin Pooled Mining Reward Systems"
authors: [Meni Rosenfeld]
year: 2011
venue: arXiv (1112.4980)
source: https://arxiv.org/abs/1112.4980
pdf: https://arxiv.org/pdf/1112.4980
type: papers
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [pplns, PPS, geometric, DGM, pool-hopping, variance, Rosenfeld]
summary: "The canonical academic reference for Bitcoin pool reward design. First rigorous mathematical treatment; cited by virtually every later paper and by modern pool specs (OCEAN's TIDES doc, Stratum V2 design discussions)."
---

# Analysis of Bitcoin Pooled Mining Reward Systems (Rosenfeld 2011)

The canonical academic reference for Bitcoin pool reward design. First rigorous mathematical treatment; cited by virtually every later paper and by modern pool specs (OCEAN's TIDES doc, Stratum V2 design discussions).

## Schemes formalized

Closed-form expected reward and variance for:

- **Proportional** (early Slush, BTC Guild) — `R = B · (n/N)`. Hop-vulnerable.
- **PPS** — `R = B · p`, `p = 1/D`. Operator absorbs all variance; needs reserves.
- **Slush score-based** — exponential decay of share weight by submission time.
- **Geometric method** — fixed fee `f` + variable fee `c`; tunable variance.
- **PPLNS** — last-N-shares window; variants: sharp 0/1 cutoff, exponential decay, linear decay.
- **Double Geometric (DGM)** — Rosenfeld's own; parameter `o` for cross-round leakage. `o=0` ≡ Geometric; `o=1` ≡ PPLNS-with-exponential-decay. Used in production by BTCDig (2013).
- **SMPPS / ESMPPS** — shifts smoothing; risk: unbounded operator debt under bad luck.
- **CPPSRB** — capped PPS with recent backpay (Eligius, Luke-Jr 2011).

## Pool hopping

Formal proof that proportional payout is exploitable: a continuous miner can lose ~43% of fair earnings. Breakeven point ≈ 43.5% of difficulty. **Hopping-proof methods**: geometric and unit-PPLNS (per-share expected payout independent of round position).

## Risk allocation framework (still cited today)

| Scheme | Variance shifted to | Operator reserve req | Fee |
|---|---|---|---|
| Proportional | Miner | Low | Low |
| PPS | Operator | High | High (covers risk) |
| PPLNS | Miner | Low | Low |
| Geometric | Tunable | Tunable | Tunable |
| SMPPS | Miner short-run, Operator long-run | Unbounded under bad luck | — |

## Why it matters for this wiki

Every modern pool design (FPPS, PPS+, TIDES, SLICE/PPLNS-JD, hashpool eHash, p2poolv2 share-chain) is a descendant of one of Rosenfeld's eight schemes — usually a tweak to PPLNS for either auditability, custody, or fee-era economics. The variance/risk-allocation framework is the lens for comparing them.

## Key citations

- BTCDig (2013) implemented DGM in production — empirical validation.
- Schrijvers et al. (FC'16) extends to mechanism-design / incentive compatibility.
- Chatzigiannis et al. (2022) updates variance analysis to FPPS-dominated landscape.
