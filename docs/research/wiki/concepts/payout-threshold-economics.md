---
title: "Payout Threshold Economics"
category: concept
created: 2026-10-10
updated: 2026-10-10
verified: 2026-10-10
confidence: medium
volatility: warm
tags: [payout-threshold, minimum-payout, dust, coinbase-outputs, pool-size, hashrate, pplns, tides, ocean, braiins, luxor, cost-model, computed]
aliases: [minimum payout economics, coinbase threshold, 0.16% rule]
sources:
  - raw/data/2026-10-10-sub-threshold-payout-economics-tables.md
  - raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md
  - raw/articles/2026-10-10-second-docs-pricing-lifetime-server-tos.md
summary: "Who falls below a coinbase payout threshold is a function of pool size, not miner size: at 132.7 T difficulty and a 3.125 BTC subsidy a miner earns < 500,000 sats per pool-found block whenever it is < 0.16 % of pool hashrate. OCEAN's 2^20-sat threshold is explained by a ~16-slot firmware-safe coinbase, and its real coinbases carry a median of 17 payout outputs (mean 27.7, max 64) — 5–7 % of the DATUM ceiling. A 43-vB P2TR coinbase output costs 43–860 sats of block space across 1–20 sat/vB; the alternative rails cost 0 sats in fees but up to 100 % in sweep risk (Ark) or need inbound liquidity (Lightning)."
---

# Payout Threshold Economics

> Every coinbase-direct pool has a number below which it will not write a miner into the coinbase. This article collects what those numbers are, why they are where they are, and how to derive one rather than copy one. Figures are from [the 2026-10-10 computation](../../raw/data/2026-10-10-sub-threshold-payout-economics-tables.md) and [OCEAN's docs plus a 40-block coinbase scan](../../raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md); treat the tables as order-of-magnitude.

## 1. The thresholds in production

| Pool | Coinbase / on-chain threshold | Below it | Custody |
|---|---|---|---|
| **OCEAN** (TIDES) | **1 048 576 sats (2^20)**, pool-set, not miner-configurable; discretionary ≥ 65 536 (2^16) | unpaid balance accrues; BOLT12 Lightning attempted every block (since Apr 2024) | none above threshold; balance is an accrual |
| **Braiins** | 20 000 sats (fee 10 000 below 500 000; free above) | **Lightning 1 sat – 500 000 sats, no fee**, daily | custodial account |
| **Luxor** | 100 000 sats + 7 500 flat fee | held | custodial |
| **Blitzpool** | none fixed — weight-budget trim; sub-dust and trimmed → pending credit | pending credit paid in a later coinbase | pool holds pending sats |
| **ArkPool** | n/a (never coinbase) | **Ark, 1-sat minimum, hourly** | arkoor trust ≤ 28 d |
| Solo (Public Pool, CKPool) | none — finder takes all | — | — |

OCEAN's TIDES doc adds the floor below the floor: "If the miner has not submitted enough work to earn at least one full satoshi… those earnings are lost."

## 2. Why OCEAN's number is ~1M sats: slots, not sats

The threshold is a **coinbase-slot budget** in disguise. Delving (vnprc, 2025): OCEAN coinbases carried ~16 outputs max (784 B) because **stock Antminer firmware tolerates "just over a dozen outputs"**. Measured 2026-10-10 over the last 40 OCEAN blocks: payout outputs **median 17, mean 27.7, max 64**, coinbase ≈ 1.1 kvB — roughly 5–7 % of the 380–530-output DATUM ceiling this wiki records elsewhere ([[tides|TIDES]]). Only ~0.25 genuine miner outputs per block are below 500 000 sats (~1 % of outputs; smallest ≈ 75 000). With ~20 slots and ~3.13 BTC to split, the 20th-largest contributor is the threshold; at OCEAN's ~21 EH/s that lands near 2^20 sats. A pool with a weight-budget trim (Blitzpool, [[../../output/plan-silent-pool-spec-2026-10-09|silent-pool §4.8]]) gets the same effect dynamically and can add a fixed floor on top.

## 3. The 0.16 % rule — who is sub-threshold

Inputs (mempool.space, 2026-10-10): difficulty 132.716 T ⇒ ≈ 950 EH/s; subsidy 312 500 000 sats; fees (+0.6–0.8 %) and pool fee (−1–2 %) ignored. Under a steady-state 8×D PPLNS window a miner's expected share of each found block ≈ `miner_hashrate / pool_hashrate`.

| miner \ pool | 1 PH/s | 10 PH/s | 100 PH/s | 1 EH/s |
|---|---|---|---|---|
| 1 TH/s | 312 500 * | 31 250 * | 3 125 * | 312 * |
| 100 TH/s (one modern ASIC) | 31 250 000 | 3 125 000 | **312 500 *** | 31 250 * |
| 1 PH/s | 312 500 000 | 31 250 000 | 3 125 000 | **312 500 *** |
| 10 PH/s | — | 312 500 000 | 31 250 000 | 3 125 000 |

`*` below 500 000 sats. **A miner is sub-threshold at T sats whenever `miner/pool < T / 312 500 000`**; for T = 500 000 that is **0.16 % of pool hashrate** → 1.6 TH/s at a 1 PH/s pool, 16 TH/s at 10 PH/s, 160 TH/s at 100 PH/s, 1.6 PH/s at 1 EH/s. Consequences:

- One modern ASIC (100–200 TH/s) is paid in the coinbase only in pools **≤ ~60–120 PH/s**. In a 1 EH/s pool, everything under ~10 ASICs is on the secondary rail *every block*.
- Expected income does not depend on the pool: a 100 TH/s miner earns ≈ 4 737 sats/day anywhere; the pool size only changes *how often* (1 EH/s → a block every ~6.6 days; 10 PH/s → every ~660 days) and therefore the per-event size.
- For a small pool the threshold is nearly moot (a 1 PH/s pool finds a block every ~18 years; its hobbyists are above 500k when it does). The rail question is a **large-pool** question.

## 4. What a slot costs, what the alternatives cost

| | 1 sat/vB | 5 sat/vB | 20 sat/vB |
|---|---|---|---|
| Coinbase P2TR output (43 vB / 172 WU) — block-space cost | 43 | 215 | 860 |
| Recipient's later standalone sweep (111 vB) | 111 | 555 | 2 220 |
| … as % of 330 k / 500 k sats | 0.03 / 0.02 % | 0.17 / 0.11 % | 0.67 / 0.44 % |
| Ark unilateral exit, 64–1 024-leaf round (7–11 txs + CPFP) | 1 050–2 860 | 5 250–14 300 | 21 000–57 200 |
| … as % of 500 k sats | 0.2–0.6 % | 1–3 % | **4–11 %** |

Ark fees are 0 % to send and receive on Second; holding 500 k sats 30 days costs 0 sats with a refresh timed into the free window, up to 2 500 (0.5 %) otherwise. Lightning on Second carries a 20-sat minimum; on-chain the 330-sat P2TR dust floor is the hard bottom. So the direct coinbase output is **cheap in sats and free in trust**; the secondary rails are free in sats and expensive in trust (Ark: sweep after 28 days; Lightning: liquidity and client compatibility; Cashu: IOU). Exit-tx sizes above are assumptions — Second publishes none.

## 5. Deriving a threshold

1. **Slot budget** — how many outputs the pool's weight budget (and its miners' firmware) tolerates per block; this sets the *dynamic* cut at the k-th largest contributor.
2. **Loss tolerance on the secondary rail** — for Ark, the amount a miner can afford to forfeit by missing one 28-day refresh; for Lightning, the amount worth a routing attempt. This argues for a *low* fixed floor, not a high one.
3. **Pool size** — apply the 0.16 % rule to the pool's expected hashrate to see which miner class the floor actually pushes off the coinbase.
4. **Publish it**, as OCEAN does; a hidden threshold is an attribution oracle (who got an output tells you who was above it).

For a 500 000-sat floor on a ~100 PH/s SPc pool, the coinbase serves ≥ 160 TH/s and the Ark rail serves single-ASIC and Bitaxe miners — roughly the population ArkPool is already paying.

## See also

- [[ark-sub-threshold-payout-rail|Ark (bark) as a Sub-Threshold Payout Rail]] — the rail these numbers size
- [[tides|TIDES (OCEAN)]] · [[lottery-pplns|Lottery-PPLNS / Blitzpool trim]] · [[parasite-pool|Parasite Pool]] · [[ehash|eHash]]
- [[coinbase-amount-linkability|Coinbase Amount Linkability]] — the same amounts as a privacy channel
- [[ctv-coinbase-payout-tree|CTV Coinbase Payout Tree]] — how a covenant would move the slot budget to ~319

## Sources

- [Economics tables (computed 2026-10-10)](../../raw/data/2026-10-10-sub-threshold-payout-economics-tables.md)
- [OCEAN threshold, BOLT12, coinbase scan, Braiins/Luxor](../../raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md)
- [Second pricing and lifetime](../../raw/articles/2026-10-10-second-docs-pricing-lifetime-server-tos.md)
