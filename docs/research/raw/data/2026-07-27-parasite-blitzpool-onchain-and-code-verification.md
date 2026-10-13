---
title: "Parasite Pool & Blitzpool — on-chain and code verification of finder-bonus mechanics"
url: https://mempool.space/api/v1/mining/pool/parasite/blocks
source: "mempool.space REST API + raw.githubusercontent.com/parasitepool/para @ master + blitzpool.yourdevice.ch"
type: data
category: data
ingested: 2026-07-27
volatility: hot
quality: 5
credibility: high
confidence: high
tags: [parasite-pool, blitzpool, finder-bonus, lottery-pplns, coinbase-payout, on-chain-verification, correction]
summary: "Primary-source verification pass that corrects three claims previously held in this wiki: Parasite's payout weighting is NOT decay-EMA (it is cumulative unpaid difficulty, no window); Parasite's 1 BTC bounty IS non-custodial (a coinbase output to a different finder address every block — only the remainder is custodial); and Parasite is at 5 mainnet blocks, not 2. Also records Blitzpool's live-pool status (no blocks yet) and the absence of any published critique of large flat finder bounties."
---

# Parasite Pool & Blitzpool — on-chain and code verification

Verification pass run 2026-07-27 against primary sources: the mempool.space REST API, `parasitepool/para` on `master`, and the live Blitzpool instance. Purpose was to check claims this wiki had been carrying from secondary press coverage. **Three were wrong.**

## Correction 1: Parasite's 1 BTC bounty is non-custodial

This wiki previously stated Parasite's coinbase has "a single output to a pool-controlled address." It has **three** outputs, and the first one is the finder's.

Coinbase output 0 across three blocks — **exactly 100,000,000 sats each time, to a different address each time**:

| Block | Output 0 (finder, 1.00000000 BTC) | Output 1 (remainder → pool) |
|---|---|---|
| 938,713 | `bc1qsmdhm009ukwayz90dkyydaw9qyk9zvvmz8ttae` | `bc1qkgef7pl…` 2.14229255 BTC |
| 945,601 | `bc1q2l474n3qqpnmkg0y82ydt9f9jkyhz6dhqv04lt` | `bc1qkgef7pl…` 2.12678873 BTC |
| 958,527 | `bc1qnd2xkan4kmdh6x89z2s7096n6nlkhcm2x29vkv` | `bc1qkgef7pl…` 2.13063319 BTC |

Output 2 is the zero-value OP_RETURN witness commitment. Coinbase weight 832 WU (block 958,527).

Output 1's address is **identical across all three blocks** (`bc1qkgef7pl8vdrtuc4wk8fssycz366xp5ukzsm8gp`) — that is the custodial hot wallet, consistent with the earlier on-chain analysis in this wiki. Output 0's address **varies every block**, which is only consistent with it being the finder's own address.

**So the custody critique applies to the remainder (~68% of the reward), not to the bounty.** Anyone can verify from chain data alone that the finder was paid exactly 1 BTC by the block they found. Whether the remainder reaches miners over Lightning remains unprovable on-chain.

Method: `GET /api/block-height/{h}` → `GET /api/block/{hash}/txid/0` → `GET /api/tx/{txid}`.

## Correction 2: the payout weighting is not decay-weighted

This wiki described Parasite's share weighting as a "continuous-time exponential-decay EMA." **It is not.** From `src/subcommand/server/database.rs`, the payout split is:

```sql
total_diff - already_paid_diff as unpaid_diff
...
FLOOR((pa.unpaid_diff::NUMERIC / tu.total_diff::NUMERIC) * $1)::BIGINT as amount
```

That is **cumulative unpaid difficulty since account inception** — no decay, no half-life, no sliding window. A sweep of all 162 Rust files in the repo for `decay|half.?life|exp(|lambda` returns **zero hits in any payout-path file** (`payouts.rs`, `sharediff.rs`, `pool.rs`, templates, tests).

`src/decay.rs` **does** exist and implements a `DecayingAverage` (`1 − e^(−x)` via `exp_m1`, with warmup bias correction and a clamp at x=40). Its consumers are:

- `src/metatron/stats.rs` (33 refs) — hashrate display
- `src/vardiff.rs` (12 refs) — difficulty adjustment
- `src/store/entry.rs` (14 refs) — `DecayingAverageEntry` storage
- `src/subcommand/miner/metrics.rs` (4 refs)

None of these are payout. **The decay machinery is for hashrate stats and vardiff, not reward weighting.**

Consequence: Parasite's scheme **is not PPLNS in Rosenfeld's sense and does not inherit PPLNS's hop-resistance.** Accumulated unpaid difficulty never ages out.

## Correction 3: the finder is excluded from the remainder

Parasite implements the **winner-only** variant, not bonus-on-top. Two independent confirmations:

`src/subcommand/server/database.rs` — the 1 BTC is a hardcoded constant, not config:

```rust
if total_reward <= 100_000_000 {
    // 1 BTC of coinbase value is reserved for miner who found the block
    return Ok(Vec::new());
}
```

with `WHERE total_diff - already_paid_diff > 0 AND username != COALESCE($2, '')` excluding the winner from the remainder pool.

`src/subcommand/server/payouts.rs:253-255` — the live (non-simulated) path:

```rust
let total_payment_amount = coinbasevalue.saturating_sub(COIN_VALUE.try_into().unwrap());
let payouts = database.get_payouts(blockheight, username).await?;
```

`tests/payouts.rs` asserts the intent explicitly: *"Block finder should have zero payout amount"*, *"Block finder payout should be marked success"*, plus a dedicated `test_multiple_users_with_finder_exclusion` checking the finder's amount is 0 while their diff is marked paid (4000).

Contrast with Blitzpool, which pays the finder **twice** (bonus output + proportional share) — the reason its Group-Solo ledger needs a per-address merge.

## Correction 4: Parasite is at 5 blocks, not 2

`GET /api/v1/mining/pool/parasite/blocks` returns five, total reward 15.68 BTC:

| Height | Date | Reward (sats) |
|---|---|---|
| 958,527 | 2026-07-18 | 313,063,319 |
| 958,212 | 2026-07-15 | 313,527,400 |
| 954,873 | 2026-06-22 | 314,975,385 |
| 945,601 | 2026-04-18 | 312,678,873 |
| 938,713 | 2026-02-28 | 314,229,255 |

**The cadence accelerated sharply.** First two blocks 48 days apart; last three inside ~26 days; final two only 3 days apart. This resolves CoinDesk's April framing ("a third block inside the next two months would settle the case") in Parasite's favour — and it happened while reported hashrate was *down* (~52 PH/s April 2026 vs a 182 PH/s June 2025 peak), so it is not explained by hashrate growth.

## Blitzpool live status

- Bitcoin Core v31.0.0, server **v2.2.5** (matches the cloned commit `7815884`), ~3.2 PH/s (1h), 1000+ devices (219 Bitaxe).
- **No mainnet blocks found.** Luck at 22% of expected; stated expected PPLNS block time **42.4 years** at current hashrate. Blitzpool is absent from mempool.space's 45-pool registry, corroborating zero blocks.
- PPLNS gating: min share difficulty 500, 5-share warmup → effectively ASIC-class (~500 GH/s+). Stratum `:3340`, TLS `:6640`, SV2 auto-negotiated on the same port.
- Public UI copy for Group-Solo makes **no mention of a finder bonus** — consistent with the code read finding that it is a per-group DB field, not a headline feature.
- bitcointalk announcement thread (`topic=5582011`) has **two posts, both by the operator** (2026-05-03 announce, 2026-05-10 SV2 follow-up). Zero community response.

## Comparison datum: OCEAN coinbase output count

OCEAN block 959,867: **12 coinbase outputs, 2020 WU, 532 bytes.** Top values 1.125 / 1.045 / 0.263 / 0.202 / 0.131 BTC; **smallest nonzero 0.0652 BTC**.

So OCEAN's coinbase-direct model keeps output count low via a **high effective payout threshold**, not by paying every miner every block. Useful contrast for Blitzpool's claim of "no minimum payout — it's just a coinbase output," which implies far more outputs and is what its weight-budget autoscaler exists to manage.

## Negative result: no published critique of flat finder bounties

Nine distinct search framings (bitcointalk, Delving Bitcoin, r/BitcoinMining, game theory, coefficient of variation, pool hopping, centralization, "worse than solo") found **no substantive public analysis of large flat finder bounties, for or against.**

- Delving Bitcoin's one adjacent thread — *PPLNS with Job Declaration* (`delvingbitcoin.org/t/pplns-with-job-declaration/1099`) — explicitly does not discuss finder bonuses, variance, or hopping.
- The only pro-bounty argument on record is the operator's, on behavioral grounds: *"the finder gets a guaranteed 1 BTC, satisfying that round number bias"* (zkshark Substack).
- The only quantified critique is Blockspace Media's *"effectively receiving a discounted pay-per-share of -22%"* — which averages the variance away rather than measuring it.
- Remaining hits (LeedMiner, CoinLive, Endless Mining, NewsBreak) are quality 1–2 restatements of the press release.

**The analytical gap is itself the finding**: any variance critique of this design has to be constructed rather than cited.

## See also

- [[../repos/2026-07-27-blitzpool-finder-bonus-code-read|Blitzpool finder-bonus code-level read]]
- [[../../wiki/concepts/lottery-pplns|Lottery-PPLNS (Finder-Bonus Hybrid)]]
- [[../../wiki/concepts/parasite-pool|Parasite Pool]] — corrected against this source
- [[../articles/2026-05-26-parasite-pool-coinbase-onchain-analysis|earlier Parasite on-chain analysis]] — consistent on the pool address, incomplete on output 0
