---
title: "Economics of a 500k-sat coinbase/Ark split — per-block earnings by pool size, on-chain vs Ark exit costs, Second vs Arkade fee schedules, liquidity (computed 2026-10-10)"
url: https://mempool.space/api/v1/difficulty-adjustment
source: "mempool.space /api/v1/mining/hashrate/3d, /api/v1/difficulty-adjustment, /api/fees/recommended (2026-10-10) ; https://second.tech/pricing ; https://second.tech/docs/learn/liquidity.md ; https://arkade.computer/v1/info (live JSON 2026-10-10) ; https://arkexplorer.blockonomics.co/api/stats (2026-10-10) ; https://second.tech/blog/ark-liquidity-research-01/ (2025-05-01, Matthew Vuk) ; Optech transaction-size components (P2TR 43 vB output, 57.5 vB input)"
type: data
category: computed
date: 2026-10-10
ingested: 2026-10-10
research_path: economics
volatility: hot
quality: 4
credibility: medium
confidence: medium
bias_note: "Tables (c) and (d) are in-session arithmetic on primary inputs; exit-tx sizes are assumptions (Second publishes none). Treat as order-of-magnitude."
tags: [economics, payout-threshold, dust, coinbase-output-cost, exit-cost, liquidity, hashrate, pplns, ark, second-tech, arkade, computed]
summary: "At 132.716 T difficulty (≈950 EH/s) and a 3.125 BTC subsidy, a miner lands under 500,000 sats per pool-found block whenever it is < 0.16% of pool hashrate — one 100 TH/s ASIC clears the line only in pools ≤ ~60 PH/s; in a 1 EH/s pool everything below 1.6 PH/s goes sub-threshold. A 43-vB P2TR coinbase output costs 43/215/860 sats of block space at 1/5/20 sat/vB; the recipient's later sweep costs 0.02–0.67% of a 330–500k UTXO, versus an Ark unilateral exit at 0.2–0.6% (1 sat/vB) to 4–11% (20 sat/vB). Holding 500k sats on Second for 30 days costs 0 sats with a timed refresh, up to 2,500 (0.5%). Arkade mainnet: dust/vtxoMin 330, vtxoMax 0.5 BTC, no off-chain fees, 200 sats per on-chain output."
---

# Sub-threshold payout economics (computed 2026-10-10)

## Inputs

- Network (mempool.space, 2026-10-10): tip **970 770**; difficulty **132.716 T**; 3-day avg hashrate 997.3 EH/s, 30-day 957.6 EH/s; implied `D·2³²/600` = **950 EH/s** (used as the deterministic base); next retarget ≈ +2.98 %. Recommended fee **1 sat/vB** at all tiers.
- Subsidy 3.125 BTC = 312 500 000 sats; fees ≈ +0.6–0.8 % today (ignored); pool fee 1–2 % (ignored).
- Under a steady-state 8×D PPLNS window a miner's expected share of each found block ≈ `miner_hashrate / pool_hashrate` (the window smooths variance, not expectation).

## (c) Expected payout per pool-found block (sats)

| miner \ pool | 1 PH/s | 10 PH/s | 100 PH/s | 1 EH/s |
|---|---|---|---|---|
| 1 TH/s | 312 500 * | 31 250 * | 3 125 * | 312 * |
| 100 TH/s (one modern ASIC) | 31 250 000 | 3 125 000 | **312 500 *** | 31 250 * |
| 1 PH/s | 312 500 000 | 31 250 000 | 3 125 000 | **312 500 *** |
| 10 PH/s | n/a | 312 500 000 | 31 250 000 | 3 125 000 |

`*` = below 500 000 sats → sub-threshold rail. **Rule**: sub-threshold whenever `miner / pool < 500 000 / 312 500 000 = 0.16 %` of pool hashrate → cut-offs 1.6 TH/s (1 PH pool), 16 TH/s (10 PH), 160 TH/s (100 PH), 1.6 PH/s (1 EH). Block cadence at 950 EH/s: 1 PH pool → 1 block / 6 597 d; 10 PH → 1 / 660 d; 100 PH → 1 / 66 d; 1 EH → 1 / 6.6 d. Expected income of a 100 TH/s miner is ≈ 4 737 sats/day regardless of pool.

**Reading**: the threshold is a function of pool size, not miner size. A pool that wants most single-ASIC hobbyists paid in the coinbase at 500k sats must stay under ~60 PH/s; at 1 EH/s the coinbase rail serves only ≥ 10-ASIC operations and *everyone else* is on the sub-threshold rail every block.

## (d) On-chain vs Ark cost (sats)

Sizes: P2TR output 43 vB (172 WU); P2TR keypath input 57.5 vB; 1-in/1-out P2TR tx 111 vB. Ark exit sizes are **assumptions**: ~150 vB per tree tx, O(log n) txs, plus ~110 vB CPFP children (P2A / zero-fee presigned).

| | 1 sat/vB | 5 sat/vB | 20 sat/vB |
|---|---|---|---|
| Coinbase P2TR output (43 vB) — implicit cost to the block | 43 | 215 | 860 |
| Recipient spends it later, input only (57.5 vB) | 57.5 | 288 | 1 150 |
| Recipient's standalone 1-in/1-out sweep (111 vB) | 111 | 555 | 2 220 |
| … as % of a 330 k-sat UTXO | 0.03 % | 0.17 % | 0.67 % |
| … as % of a 500 k-sat UTXO | 0.02 % | 0.11 % | 0.44 % |
| Ark unilateral exit, 64-leaf round (7 × 150 vB) | 1 050 | 5 250 | 21 000 |
| … + CPFP children ≈ 1 820 vB | 1 820 | 9 100 | 36 400 |
| Ark unilateral exit, 1 024-leaf round (11 × 150 vB) | 1 650 | 8 250 | 33 000 |
| … + CPFP children ≈ 2 860 vB | 2 860 | 14 300 | 57 200 |

Exit of a 500 k VTXO ≈ 0.2–0.6 % at 1 sat/vB, **4–11 % at 20 sat/vB** — 10–25× the cost of spending a direct coinbase UTXO; cheap only while the operator cooperates (offboard = one tx). Arkade caps tx weight at 40 000 WU and uses a 7-day unilateral exit delay.

Liquidity cost of parking 500 k sats 30 days on Second (28-day life ⇒ ≥ 1 refresh): **0** if refreshed inside the 2-day free window; 1 000 (0.2 %); 2 000 (0.4 %); 2 500 (0.5 % if refreshed with ≥ 14 d left). The docs' 5 %/yr formula gives 2 055. On Arkade: 0 (no refresh fee published).

## (b) Capital requirement for the paying pool

- Float of X BTC in VTXOs is locked for up to 28 days per cycle; rounds hourly. Refresh cost per cycle: 0 if timed into the last 2 days (bark auto-threshold 1 day), else 0.2–0.5 % of X (1 BTC → 200 k–500 k sats; ~13 cycles/yr → up to 6.5 %/yr if always refreshed fresh).
- Arkoor sends are free, but every payout is a leaf of the sender's VTXO inside an exit chain whose depth the server limits (0.6.x), so a high-volume payout VTXO must be refreshed more often than expiry alone requires.
- Sizing: float ≥ (hashrate share of sub-threshold miners) × ~3.13 BTC per block found; a 1 EH/s small-miner pool could need ~3 BTC every 6.6 days, replenished by boarding the coinbase pool output (board = on-chain fee only; coinbase needs 100 confirmations regardless). On Arkade `vtxoMaxAmount` 0.5 BTC forces ≥ 6 VTXOs per 3 BTC.
- Published liquidity: **none from Second**. Blockonomics explorer (Arkade's server `arkade.money`): `networkLiquidity 212.53 BTC`, 24 h `virtualTxCount 57`, `virtualTxVolume 0.0213 BTC`, `onboardingVolume 0.0035 BTC` — metric definitions undocumented; `timeframe` param ignored. The critic's "10 BTC for 10 000 users" remains unrefuted by data.
- Second liquidity research (2025-05-01, simulation): liquidity-days per unit — weekly top-up LSP 4.09 vs Ark 10.13 (LSP 2.49× better); monthly 16.24 vs 17.01; quarterly 64.09 vs 17.17 (Ark 3.73× better); DCA 210.27 vs 31.60 (Ark 6.65×). "Ark server liquidity requirements are driven by payment volume" (not balances). Assumes 28-day expiry, 14-day average age at receipt, refresh in last 2 days.

## (a) Fee schedules side by side

| | Second (hosted) | Arkade (`/v1/info`, v0.9.21) |
|---|---|---|
| Off-chain send / receive | 0 % / 0 % | 0 / 0 |
| Refresh / liquidity | 0 % (< 2 d) · 0.2 % (< 7 d) · 0.4 % (< 14 d) · 0.5 % (≥ 14 d) + ~221 vB on-chain share | none published |
| Dust / min VTXO | bark auto-refreshes VTXOs below P2TR dust (330); `min_board_amount` unpublished | `dust 330`, `vtxoMinAmount 330` |
| Max VTXO | `max_vtxo_amount` unpublished | `vtxoMaxAmount 50 000 000` (0.5 BTC) |
| On-chain output in settlement/offboard | 0.2–0.5 % + chain fee | `intentFee.onchainOutput 200` sats |
| Lightning send | 0.2–0.5 %, min 20 sats | via Boltz — suspended since 2026-08-03 |
| VTXO lifetime | 28 d (LN-receive 1–3 d) | `unilateralExitDelay ≈ 7 d`, `boardingExitDelay ≈ 90 d` |

## Source criticism

Chain data and fee pages are primary and dated. Tables (c)/(d) are the agent's arithmetic; the per-block-share approximation ignores window-edge effects, fees and pool fee (all ≤ 2 %). Exit-tx sizes are the task's assumption — Second publishes no vbyte figures; the covenantless-ark wiki's formal-model figure (~150 vB per exit tx) is the closest published number. A local `bark 0.1.0-beta.1` could not pull Second's machine-readable `ArkInfo` ("Client max is: 1, server min is: 4").

## See also

- [[../articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs|OCEAN threshold and measured coinbase outputs]]
- [[../articles/2026-10-10-second-docs-pricing-lifetime-server-tos|Second pricing and lifetime docs]]
- [[../../wiki/concepts/coinbase-amount-linkability|Coinbase Amount Linkability]] — the privacy side of the same amounts
