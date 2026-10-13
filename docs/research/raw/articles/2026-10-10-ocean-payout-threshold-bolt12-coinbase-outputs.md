---
title: "OCEAN payout threshold (2^20 sats), BOLT12 Lightning fallback, and measured coinbase output counts; Braiins/Luxor thresholds"
url: https://ocean.xyz/docs/lightning
source: "https://ocean.xyz/docs/lightning (2026-04-14) ; https://ocean.xyz/docs/tides ; https://ocean.xyz/template/workers/payoutcards?user=<addr> (live 2026-10-10) ; https://www.nobsbitcoin.com/ocean-pool-integrates-bolt12-lightning-payouts/ (2024-04-30) ; https://blockdyor.com/ocean-review/ (2024-11-23) ; https://delvingbitcoin.org/t/scaling-noncustodial-mining-payouts-with-ctv/1753 (2025-06-04) ; https://delvingbitcoin.org/t/fee-selection-for-ctv-based-mining-pool-fanouts/2698 (2026-07-08) ; mempool.space /api/v1/mining/pool/ocean/blocks (last 40 OCEAN coinbases, heights 969,185–970,656, fetched 2026-10-10) ; https://academy.braiins.com/braiins-pool/rewards-and-payouts ; https://docs.luxor.tech/platform/mining/revenue-payments"
type: articles
category: pool-docs+chain-data
date: 2026-10-10
ingested: 2026-10-10
research_path: applied+economics
volatility: warm
quality: 5
credibility: high
confidence: high
tags: [ocean, tides, payout-threshold, bolt12, lightning-payouts, coinbase-outputs, dust, braiins, luxor, minimum-payout, chain-data, mempool-space]
summary: "OCEAN's on-chain coinbase threshold is pool-set at 0.01048576 BTC = 1,048,576 sats (2^20), with discretionary payouts at ≥ 65,536 sats (2^16) and BOLT12 Lightning attempted every block below threshold (launched Apr 2024; fragile client compatibility). Measured over the last 40 OCEAN coinbases: mean 27.7 payout outputs/block, median 17, max 64, ~1.1 kvB — far below the 380–530 DATUM ceiling; ~0.25 genuine miner outputs/block below 500k sats. Delving: stock Antminer firmware tolerates 'just over a dozen' outputs. Braiins: LN min 1 sat / max 500,000 sats free; on-chain min 20,000 sats. Luxor: 100,000 sats + 7,500-sat fee, no Lightning."
---

# OCEAN threshold, BOLT12 fallback, and what coinbases actually carry

## OCEAN (primary docs + live dashboard)

- **On-chain threshold**: "Currently, the threshold is 0.01048576 BTC or 1,048,576 sats" (TIDES doc); dashboard payout card: "The on-chain payout threshold is 0.01048576 BTC"; "Estimated Time Until Minimum Payout … : Never" for a low-hashrate address. **Pool-wide, not miner-configurable** — no threshold setting exists in Dashboard Settings or `/config/<addr>`; the only miner config is template selection and the BOLT12 offer.
- **Discretionary tier**: TIDES doc — "discretionary payouts for balances at or above 0.00065536 BTC, should it be practical" (65 536 sats = 2^16; blockdyor had reported this as the tier for addresses inactive > 1 week).
- "Miners are paid in the generation transaction whenever possible." Fees 2 % (1 % DATUM). TIDES window 8 × network difficulty; shares get ~8 rewards each; "If the miner has not submitted enough work to earn at least one full satoshi… those earnings are lost."
- **Lightning (BOLT12)**: launched ~2024-04-29/30 (NoBS, X post). Mechanism (docs, 2026-04-14): miner pastes a BOLT12 offer (amount "any", description "OCEAN Payouts for [address]"), signs "Configure OCEAN payout to [BOLT12] at block [height]" with the payout-address key (BIP-322/legacy), submits. **Payouts attempted every block.** Fallback: "If a Lightning payout fails… OCEAN will retry paying the owed amount via Lightning every block until the accumulated earnings reach the on-chain threshold (currently 0.01048576 BTC). At that point, earnings will payout on-chain." No Lightning minimum/maximum or OCEAN-side fee stated; needs inbound liquidity; large offers with blinded paths break Coldcard signing.
- Compatibility (Mar 2026 note): Alby Hub ≥ v1.21.2 **incompatible** ("error decoding lightning_bolt12"); recommended CLN, Lexe, Eclair, Coinos (custodial), Alby Hub v1.21.0 fresh install; Phoenix not compatible. Alby blog (2025-12-02): "BOLT12 offers created in Alby Hub are not accepted by Ocean."
- OCEAN's rationale (NoBS): "BOLT12 offers allow us to request multiple invoices for any amount while only requiring the miner to set things up once"; auditability — prove "a payment was made… and that it was paid by us".

## Measured coinbase composition (mempool.space, last 40 OCEAN blocks, 2026-10-10)

| Metric | Value |
|---|---|
| Non-`OP_RETURN` payout outputs per block | **mean 27.7, median 17, min 16, max 64** |
| Coinbase vsize | mean 1 124 vB, max 2 316 vB |
| Outputs < 500 000 sats | 0.625/block (25 total); ~60 % are OCEAN's own residual to its pool address `37dvwZZ…` → **genuine miner outputs < 500k ≈ 0.25/block (~1 % of outputs)**, smallest ~75 000 sats |
| Script mix | ≈ 60–70 % P2WPKH, plus P2WSH, P2SH, P2PKH, 2–4 P2TR per block |
| Fees | median feerate 1–2 sat/vB; total fees ≈ 0.018–0.025 BTC/block (≈ 0.6–0.8 % of subsidy) |

Context: OCEAN 1 561 blocks all-time; 27 of 1 040 in the last week (2.6 % ⇒ ~23 EH/s implied; self-reported 21.35 EH/s). Network: tip 970 770, difficulty 132.716 T, 3-day hashrate 997.3 EH/s, recommended fee 1 sat/vB.

## Delving: why OCEAN's threshold is ~1M sats

"Scaling Noncustodial Mining Payouts with CTV" (vnprc, 2025-06-04): observed ~**16 outputs max (784 bytes)** in OCEAN coinbases; **Antminer stock firmware tolerates "just over a dozen outputs"**; a CTV design would allow 319 outputs from a 179-byte coinbase. **Erik De Smedt (Second), 2025-06-05**: "We have been thinking about a similar approach using Ark... The leaf of this tree is called a VTXO... Only a single UTXO worth 0.01 BTC will hit the chain." — the only Second-authored statement on mining payouts found. Follow-up "Fee Selection for CTV-Based Mining Pool Fanouts" (djh58, 2026-07-08; qbit pool): fee chosen ~17 h before maturity; vnprc (07-09) concludes sub-fee payouts need ecash or "ark service providers" long-term; raises Sybil-dust attacks on fanouts. No Second participation in 2026.

## Other pools' thresholds

| Pool | On-chain | Lightning | Custody |
|---|---|---|---|
| **Braiins** | min 0.0002 BTC (20 000 sats); free ≥ 0.005 BTC, else 0.0001 BTC fee; max 5 BTC; daily 09:00 UTC; threshold configurable per Financial Account | **min 1 sat, max 0.005 BTC (500 000 sats), no fee**; daily or manual once/24 h (processed ≤ 60 min) | custodial account |
| **Luxor** | min 0.001 BTC + flat 0.000075 BTC fee (needs 0.001075); daily/weekly/monthly after 04:00 UTC | none | custodial |
| Public Pool / Solo CKPool | solo — full reward to finder, no threshold by construction | — | — |
| NiceHash | — | LN min 0.00001 BTC | custodial |

Not ingested: D-Central's claim that OCEAN has "no minimum" (contradicted by OCEAN's own payout card).

## Source criticism

OCEAN docs and the live dashboard are primary; the coinbase scan is a 40-block sample on 2026-10-10 (scratch script `/tmp/ocean_scan/scan2.py`, not committed). OCEAN publishes no count of miners below threshold or using BOLT12; the "~16 outputs" Delving figure (2025) is consistent with the measured median of 17 but the mean (27.7) and max (64) show the ceiling has moved. The 2^16 discretionary tier is now confirmed from OCEAN's TIDES page, upgrading blockdyor's secondary report.

## See also

- [[../../wiki/concepts/tides|TIDES (OCEAN)]]
- [[2026-10-10-arkpool-bitaxe-pool-ark-payouts|ArkPool]]
- [[../data/2026-10-10-sub-threshold-payout-economics-tables|Economics tables]]
- [[2026-05-26-vnprc-ctv-coinbase-delving|vnprc CTV-coinbase Delving thread (earlier ingest)]]
