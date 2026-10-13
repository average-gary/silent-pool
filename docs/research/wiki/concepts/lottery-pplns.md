---
title: Lottery-PPLNS (Finder-Bonus Hybrid)
category: concept
created: 2026-07-27
confidence: high
tags: [lottery-pplns, finder-bonus, pplns, coinbase-payout, per-connection-coinbase, variance, blitzpool, parasite-pool]
volatility: warm
updated: 2026-07-29
verified: 2026-07-29
summary: "Payout family that carves a flat bounty out of the block reward for whichever miner's share solved the block, then splits the remainder by PPLNS weight. Covers the mechanism, the speculative-per-finder-coinbase requirement it imposes on the pool, the variance re-introduction it causes, and the two known implementations (Parasite Pool, Blitzpool)."
sources:
  - "raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md"
  - "raw/data/2026-07-27-parasite-blitzpool-onchain-and-code-verification.md"
  - "raw/repos/2026-07-27-blitzpool-server-rust-github.md"
  - "raw/repos/2026-05-26-parasitepool-para-github.md"
  - "raw/articles/2026-05-26-bitcoin-manual-parasite-pool.md"
  - "raw/articles/2026-05-26-zkshark-parasite-pool-substack.md"
  - "raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md"
---

# Lottery-PPLNS (Finder-Bonus Hybrid)

A payout family that sits between [[pplns|PPLNS]] and solo mining: a **flat bounty** goes to whichever miner's share actually solved the block, and the **remainder** is split among all window contributors by share weight.

It is the deliberate re-introduction of a controlled amount of lottery variance into a scheme whose entire purpose is to remove it.

## The mechanism

For block reward `R` (subsidy + fees), pool fee `f`, configured bonus `B`, and miner `i`'s window share weight `w_i`:

```
miner_cut     = R · (1 − f)
finder        = B + (w_finder / Σw) · (miner_cut − B)
everyone else =     (w_i      / Σw) · (miner_cut − B)
```

The load-bearing detail is **the bonus is carved out before the proportional split** — note `miner_cut − B` in *both* rows — so it is funded by every other miner in the window, not by the pool and not out of fees. There is no external subsidy. This is a redistribution from the many to the one who got lucky.

In Blitzpool's shared math (`crates/bp-pplns/src/distribution.rs:166-175`) the subtraction is literal and one line:

```rust
reward_for_miners -= bonus_sats;
```

### Two variants: bonus-on-top vs. winner-takes-bonus-only

The formula above is the **bonus-on-top** variant, where the finder collects the bounty *and* their proportional share. Blitzpool implements this — which is why the finder appears twice in its payout list.

Parasite Pool implements the other variant: **the finder is excluded from the remainder entirely.** Verified in `src/subcommand/server/database.rs` (`WHERE username != COALESCE($2, '')`) and in the live payout path `src/subcommand/server/payouts.rs:253-255`, which computes `total_payment_amount = coinbasevalue − COIN_VALUE` and then fetches payouts excluding the winning `username`. Its test suite asserts the intent directly: *"Block finder should have zero payout amount"*, plus a dedicated `test_multiple_users_with_finder_exclusion`.

The distinction matters for the finder and barely at all for everyone else:

```
bonus-on-top      finder = B + (w_f/Σw)·(cut − B)
winner-only       finder = B
```

Exclusion is simpler to reason about (the finder's outcome doesn't depend on their own window weight) and slightly cheaper for co-miners, since the finder's weight leaves the denominator. It also makes the bounty a genuine either/or rather than a bonus, which sharpens the variance effect discussed below.

### Guard rails a sane implementation needs

- **Cap as fraction of the miner cut.** Blitzpool clamps to 95%, so an over-large configured value degrades gracefully instead of producing a coinbase with non-positive outputs. A cap is mandatory: $B$ can exceed $R(1-f)$ whenever the subsidy halves or a low-fee block lands.
- **Dust suppression.** Suppress the bonus output entirely below the pool's minimum payout rather than emitting an unspendable output.
- **Absolute ceiling.** Blitzpool's `MAX_FINDER_BONUS_SATS = 100_000_000` (1 BTC) exists to catch config typos, with the in-code justification that anything larger "would strand more sats than a normal block reward." Note `R` includes fees, so a cap expressed against the subsidy alone is not equivalent.
- **Duplicate-address merge.** The finder legitimately appears **twice** in the payout list — once for the bonus, once for their proportional share. Both are valid on-chain `TxOut`s, but any ledger keyed on `(block, address)` will reject or silently drop the second. See § Implementation hazards.

## Why it forces per-finder coinbase construction

This is the structural consequence that makes lottery-PPLNS architecturally different from plain PPLNS, and it is the part most easily underestimated.

**The bonus recipient cannot be determined after the fact.** The coinbase transaction is committed to in the block header via the merkle root. By the time you know which miner found the block, the coinbase paying them is already fixed. So the pool must build the coinbase **speculatively, per candidate finder, in advance** — every connected miner is mining a *different* coinbase, each one naming that miner as the prospective bonus recipient.

Consequences:

| | Plain PPLNS | Lottery-PPLNS |
|---|---|---|
| Coinbase per template | 1, shared pool-wide | 1 per distinct connected address |
| Distribution computes per template | 1 (memoizable on reward alone) | N (must key on finder) |
| Payout-snapshot writes per template | 1 | N |
| Script-parse memoization across jobs | effective | defeated (any sats change invalidates the key) |

Note the granularity is per distinct *payout list*, not per socket — two connections from the same address produce byte-identical coinbases and share one entry.

### Block-found attribution comes free

Counter-intuitively, the attribution problem solves itself. If the coinbase's payout list already encodes the finder, then a **content hash of the payout list** uniquely identifies which distribution the winning block paid. Blitzpool calls this `payouts_fingerprint` (`crates/bp-share/src/lib.rs:221`: hash of reward + each `(sats, address)` in coinbase order); it rides the winning share to the block sink and keys the Redis snapshot that the ledger apply reads back.

No session ID or connection identity is needed anywhere in the lookup — and shouldn't be. The question the ledger asks is "which distribution did this coinbase pay?", not "who submitted it." The same machinery independently supports [[pplns-jd|SV2 Job Declaration]] clients that supply their own payout outputs.

Worth noting what this fingerprint does *not* depend on: the **stability** of any address in the list. It hashes the list as content, so it keeps working even if every miner's address changes every block. The component that does assume address permanence is the balance ledger, not the attribution path — see [[coinbase-address-rotation|Coinbase Address Rotation]] § Per-miner xpub usernames.

## The economic cost, quantified

At the post-2024 subsidy of 3.125 BTC with a 1 BTC bonus and a 1.5% fee, measured against Blitzpool's actual distribution builder with 8 equal-weight miners:

- Non-finder payout: **38,476,562 → 25,976,562 sats** — a **32.5% haircut**
- Finder total: **125,976,566 sats** (bonus + own proportional share)

So a 1 BTC bounty on a 3.125 BTC subsidy means **every non-winning miner gives up about a third of their PPLNS payout for that block.** That ratio worsens at every halving unless $B$ is reduced in step: at a 1.5625 BTC subsidy the same 1 BTC bonus consumes ~65% of the miner cut, and past that the 95% cap starts binding.

### Who this is rational for

A miner should accept the discount iff their probability of being the finder is high enough that the expected bonus exceeds the expected haircut.

**In the bonus-on-top variant it is a wash in expectation, for every miner at every size.** The probability of being the finder equals your share of pool hashrate, which is the same quantity that sets your proportional payout. So the bonus you expect to *collect* (`w_i/Σw · B`) exactly equals what you expect to *forfeit* to the carve-out (`w_i/Σw · B`). Lottery-PPLNS in this form changes nothing about expected return — it is a pure **variance** trade, converting a smooth income stream into a lumpier one with a fat right tail.

*Caveat on the identity:* this holds when share weight tracks hashrate, which is PPLNS's design intent but not an exact identity — a miner who joined mid-window has weight below their hashrate share and is therefore a net funder, while a long-tenured miner is a net collector. Under a cumulative-since-inception window (Parasite's) that skew is large and persistent, not a rounding error.

**In the winner-only variant it is not a wash** — excluding the finder from the remainder makes the finder's expected take strictly `B`, while their forgone proportional share is redistributed to everyone else. This is EV-negative for a miner whose proportional share would have exceeded... nothing, actually: it is EV-negative for a large miner (who gives up a big proportional share to collect a fixed `B`) and EV-positive for a small one. So exclusion is mildly progressive on EV while being sharply regressive on variance.

Either way the headline is the same: **the mechanism sells variance, not yield.**

So it is rational for exactly the population it attracts: **hobbyist and lottery miners who want a shot at a life-changing payout and are indifferent to — or actively want — the variance.** It is irrational for anyone with debt service, hosting bills, or a hashprice hedge, i.e. all commercial miners. Same expected value, worse risk profile. See [[variance-and-risk-shifting|Variance & Risk-Shifting]].

The distributional subtlety the expectation math hides: **the variance is not shared evenly.** A miner with 30% of pool hashrate collects the bonus roughly every third block and experiences it as a modest income bump; a miner with 0.1% collects it essentially never and experiences it purely as a haircut on every block. Both have the same expected value, but the small miner is carrying nearly all of the variance the mechanism creates. Lottery-PPLNS is therefore *most* attractive to the participants it treats worst — which is the ordinary structure of a lottery, and worth naming plainly rather than describing as a benefit.

The Bitcoin Manual's critique of Parasite Pool sharpens this: at low pool hashrate expected time-to-block runs **months to a year**, so the discount is paid continuously against a bonus almost no participant will ever collect within their hardware's service life. Expectation-neutrality is cold comfort over a horizon shorter than the convergence time.

## Implementation hazards

**1. Duplicate finder address in the ledger apply.** The finder appears twice in the payout list. On-chain that is fine. In the books it is not, if the schema keys on `(blockHeight, address)`. Blitzpool's Group-Solo path merges per-address before writing (`crates/bp-group-solo-engine/src/engine.rs:912-936`, summing sats and percent so each address yields exactly one audit + balance row); its PPLNS path has **no such merge**, because PPLNS never emits duplicates today. Porting the bonus to PPLNS without porting the merge produces a coinbase that pays correctly while the audit trail under-counts — the failure mode is silent accounting drift, not a crash.

**2. Weight-budget interaction.** The bonus output competes for coinbase weight against every PPLNS payout output. Where the trim is greedy-largest-first, the 1 BTC bonus is never the output trimmed — small miners are, deferred into pending credit. The bonus therefore *increases* the number of miners paid in credit rather than coin.

**3. Fan-out load, not fan-out correctness.** In a codebase whose job cache is already content-addressed (payout list in the key) and whose snapshots are already fingerprint-keyed, correctness under fan-out is free. The open question is throughput: N coinbase assemblies + N script-parse passes + N Redis writes per template, against a TTL-only job cache holding roughly `4 × N` entries at a 30 s cadence with a 120 s TTL. That wants load-testing before production, and it is the number both known implementations are quietest about.

**4. Halving-relative configuration.** A bonus configured as an absolute sats value silently becomes a larger fraction of the reward at every halving. Configuring it as a *fraction of the miner cut* avoids a scheduled cliff; neither known implementation does this.

**5. Address-derivation cost, if payout addresses are derived rather than stored.** Per-finder coinbase construction is a *per-template, per-miner* cadence, which turns any per-miner cryptographic work into a per-template storm. This matters if miners are identified by a descriptor the pool derives from rather than by a literal address — see [[coinbase-address-rotation|Coinbase Address Rotation]]. Concretely, sv2-apps' derivation primitive re-parses its descriptor on *every* derivation (a deliberate `Send + Sync` workaround for a `RefCell` in miniscript's taproot cache); free at once-per-block, not free at once-per-miner-per-template. Storing each miner's `scriptPubKey` at registration sidesteps it entirely, at the cost of giving up rotation.

## Known implementations

| | [[parasite-pool\|Parasite Pool]] | [[../../raw/repos/2026-07-27-blitzpool-finder-bonus-code-read\|Blitzpool]] |
|---|---|---|
| Bonus | Flat 1 BTC, **hardcoded** (`COIN_VALUE`, not config) | Configurable sats, capped at 1 BTC |
| Variant | Winner-only (finder excluded from remainder) | Bonus-on-top (finder paid twice) |
| Remainder weighting | Cumulative unpaid difficulty since account inception | Fixed sliding window (4× netdiff) |
| Fee | 0% | Configurable (PPLNS 1%) |
| **Bonus** payout rail | **Coinbase output direct to the finder** | Coinbase output direct to the finder |
| **Remainder** payout rail | Single pool-controlled output → operator Lightning fanout (**custodial**) | Coinbase outputs, no pool wallet |
| Status | Production — **5 mainnet blocks** (938713, 945601, 954873, 958212, 958527) | **Implemented for Group-Solo only; PPLNS opts out in one line.** No mainnet blocks yet |

**The bonus half of Parasite is already non-custodial**, which is easy to miss and worth stating precisely. Verified on-chain: each of blocks 938713 / 945601 / 958527 carries a coinbase output of exactly 100,000,000 sats to a **different address each time** (`bc1qsmdhm00…`, `bc1q2l474n3…`, `bc1qnd2xkan…`) — the finder's own address, paid by the block that earned it. The custody problem is confined to the **remainder**, which goes to one persistent pool address (`bc1qkgef7pl8vdrtuc4wk8fssycz366xp5ukzsm8gp`, unchanged across all three) and is fanned out over Lightning afterward. So Parasite is a 3-output coinbase: finder, pool, OP_RETURN.

Blitzpool would differ by paying the *remainder* in the coinbase too — every PPLNS recipient as their own output — which is what makes the whole distribution auditable rather than just the bounty.

**What no one currently ships is a fully coinbase-direct lottery-PPLNS**: bounty *and* remainder both paid on-chain in the block that earned them. Parasite does the bounty that way but custodies the remainder; Blitzpool pays everything that way but doesn't offer the bounty in PPLNS. Blitzpool has all the parts — the bonus math, the per-connection coinbase machinery, the fingerprint-keyed attribution — just not connected for PPLNS. See the [[../../raw/repos/2026-07-27-blitzpool-finder-bonus-code-read|code-level read]] for the delta.

A caveat on Parasite's remainder weighting worth noting for anyone copying the design: because it splits on `total_diff − already_paid_diff` **since account inception** with no decay or window, it is not PPLNS in Rosenfeld's sense and does not inherit PPLNS's hop-resistance. Accumulated unpaid difficulty never ages out. (Its `src/decay.rs` implements a decaying average, but it is used for hashrate stats and vardiff — `src/metatron/stats.rs`, `src/vardiff.rs` — not for payout weighting.) [[parasite-pool|The Parasite article]] describes the payout as decay-weighted; that is incorrect and is flagged there.

## Position in the taxonomy

| Axis | Lottery-PPLNS |
|---|---|
| Variance to | Miner, **deliberately amplified** — PPLNS smoothing on the remainder, pure lottery on the bonus |
| Hop-resistant | Inherits the base window's properties; the bonus itself is hop-neutral (it depends on luck, not window position) |
| IC-provable | No — inherits PPLNS's non-IC status under [[../../raw/papers/2026-05-23-schrijvers-2016-incentive-compatibility\|Schrijvers 2016]] |
| Custody | Independent of the mechanism — Parasite custodial, Blitzpool non-custodial |
| Coinbase outputs | Base payout set **+1** for the bonus, with the finder named twice |
| Template control | Independent of the mechanism |
| Scaling cost | **Per-connection coinbase construction** — the defining architectural cost |

## Open questions

1. **Measured fan-out cost at scale.** Neither implementation publishes coinbase-assembly throughput for N distinct per-finder payout sets. The distribution math is cheap (~1.9 ms at a 1000-miner window, release build); the assembly and script-parse fan-out is unmeasured.
2. **Does the lottery actually retain miners?** The claimed benefit is retention through excitement; the founder-side justification is explicitly behavioral rather than economic (*"the finder gets a guaranteed 1 BTC, satisfying that round number bias"*). No pool has published cohort retention data against a plain-PPLNS control, and Parasite's hashrate fell from a ~182 PH/s peak (June 2025) to ~52 PH/s (April 2026) while the bounty was live — weak evidence against, though confounded by the broader 2025–26 hobbyist-mining cycle.
3. **No published critique exists.** A nine-framing search sweep (bitcointalk, Delving Bitcoin, Reddit, game-theory literature, "worse than solo") found **no substantive public analysis of large flat finder bounties** — for or against. The only pro-bounty argument on record is the operator's behavioral one; the only quantified critique is Blockspace Media's "-22% discounted PPS" framing, which averages the variance away rather than measuring it. Delving Bitcoin's one adjacent thread (*PPLNS with Job Declaration*) doesn't touch finder bonuses. **This wiki's treatment is therefore constructed, not cited** — worth knowing when weighing it.
4. **Optimal bonus size.** Is there a principled way to pick `B`, or is it purely a marketing parameter? Since the trade is expectation-neutral, the only defensible criterion is the risk preference of the target miner population — which no pool has attempted to elicit. A fraction-of-miner-cut formulation would at least be halving-stable.
5. **Interaction with block withholding.** Worth flagging as an open question rather than a finding: a finder bonus concentrates value in the finder's own coinbase output, which changes the payoff structure of [[block-withholding|withholding]] relative to plain PPLNS. Whether that opens a new profitable deviation depends on details this wiki has no analysis of. No published treatment exists for either implementation.

## See also

- [[pplns|PPLNS]] — the base scheme
- [[parasite-pool|Parasite Pool]] — production implementation (5 blocks); bounty coinbase-direct, remainder custodial
- [[variance-and-risk-shifting|Variance & Risk-Shifting]] — why this trade is a variance trade, not a return trade
- [[payout-schema-taxonomy|Payout Schema Taxonomy]]
- [[tides|TIDES]] — non-custodial PPLNS without a lottery component, for contrast
- [[block-withholding|Block Withholding]] — open question, not an established finding here
- [[coinbase-address-rotation|Coinbase Address Rotation]] — the per-template/per-miner cadence this scheme imposes is what makes derived (rather than stored) payout addresses expensive; also the ledger-identity wall for xpub-keyed miners
