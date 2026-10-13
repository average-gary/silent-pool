---
title: "Blitzpool finder-bonus mechanics — code-level read of blitzpool-server-rust @ 7815884"
url: https://github.com/warioishere/blitzpool-server-rust
source: "local clone: REPOS/blitzpool-server-rust @ 7815884c7c531561e1302ca311070d09f97c1c3c (release 2.2.5, 2026-07-27)"
type: repos
category: repo
language: Rust
license: AGPL-3.0-or-later
ingested: 2026-07-27
volatility: warm
quality: 5
credibility: high
confidence: high
tags: [blitzpool, finder-bonus, lottery-pplns, coinbase-payout, per-connection-coinbase, pplns, group-solo, feasibility-assessment, rust, agpl]
summary: "Code-level read (not README) of the finder-bonus mechanic in blitzpool-server-rust at commit 7815884, answering whether a PPLNS pool can pay a configurable flat bounty (e.g. 1 BTC) to the miner that solves the block. Finding: the mechanic is already implemented in the shared pure-math distribution builder and wired for Group-Solo only; PPLNS opts out with a one-line `None`. Per-connection coinbase construction is already the pool's architecture. Includes measured cost numbers and the specific ledger gap that porting would have to close."
---

# Blitzpool finder-bonus mechanics — code-level read

Direct source read of a local clone at **`7815884c7c531561e1302ca311070d09f97c1c3c`** (release 2.2.5, committed 2026-07-27 08:31:06 +0200). This supersedes the README-only capture in [[2026-07-27-blitzpool-server-rust-github]] on one point that capture explicitly listed as unverified: *"how the finder bonus is calculated."*

Workspace: 37 crates, 142,209 lines of Rust, `rust-toolchain.toml` pinned to **1.95.0**.

## Question this answers

Can this pool run PPLNS while paying a **configurable flat bounty** (e.g. 1 BTC) to the individual miner whose share solved the block — with that miner's own address as a coinbase output?

**Answer: the mechanic already exists in the codebase.** It is implemented in the shared distribution builder, exercised by tests, exposed via an API endpoint, and wired for the Group-Solo mode. PPLNS declines it in one line.

## 1. The mechanic exists in the shared pure-math builder

`crates/bp-pplns/src/distribution.rs` is the 5-phase coinbase distribution builder, **shared by both PPLNS and Group-Solo**. Its input struct already carries the finder fields (`:59-66`):

```rust
/// Group-Solo finder bonus: paid as a dedicated coinbase output on top
/// of the finder's proportional share. Capped at 95 % of the miner-cut
/// and suppressed entirely if it would fall below `min_payout_sats`.
/// Requires both `finder_bonus_sats` AND `finder_address` to be set.
pub finder_bonus_sats: Option<Sats>,
pub finder_address: Option<&'a AddressId>,
```

The carve-out logic (`:166-175`):

```rust
let configured_bonus = input.finder_bonus_sats.map(|s| s.to_i64()).unwrap_or(0);
let want_bonus = if configured_bonus > 0 && input.finder_address.is_some() {
    configured_bonus
} else { 0 };
let bonus_cap = ((reward_for_miners as f64) * 0.95).floor() as i64;
let capped_bonus = want_bonus.min(bonus_cap);
let bonus_emitted = capped_bonus >= min_payout;
let mut bonus_sats = if bonus_emitted { capped_bonus } else { 0 };
reward_for_miners -= bonus_sats;
```

Three properties that matter:

- **The bonus is subtracted from `reward_for_miners` before the proportional split.** Every other miner in the window funds it. This is the same economic shape as [[../wiki/concepts/parasite-pool|Parasite Pool]]'s flat 1 BTC bounty.
- **Capped at 95% of the miner cut**, so an over-large configured value degrades rather than breaking the coinbase.
- **Suppressed below `min_payout_sats`**, so it never emits a dust output.

Emission as a dedicated output, Phase 6 (`:649-657`):

```rust
if bonus_emitted {
    if let Some(finder_addr) = input.finder_address {
        payouts.push(CoinbaseDistributionEntry {
            address: finder_addr.clone(),
            percent: (bonus_sats as f64 / block_reward as f64) * 100.0,
            sats: Sats(bonus_sats),
        });
    }
}
```

Weight accounting for the extra output is already handled (`:233-241`) — the bonus output's weight enters `fixed_weight` before the trim loop, so the coinbase can't overflow its reservation because of it.

## 2. PPLNS declines it in one line

`crates/bp-pplns-engine/src/distribution.rs:340-341`:

```rust
finder_bonus_sats: None, // finder-bonus is a Group-Solo feature
finder_address: None,
```

Group-Solo passes it through — `crates/bp-group-solo-engine/src/distribution.rs:260-261`:

```rust
finder_bonus_sats,
finder_address: Some(finder_address),
```

## 3. The cap is already exactly 1 BTC

`crates/bp-group-mgmt/src/constants.rs:21-25`:

```rust
/// Hard cap for the optional per-block finder-bonus output, in sats.
/// 1 BTC is already absurd as a per-block bonus on top of the
/// proportional split — anything bigger is almost certainly a config
/// typo and would strand more sats than a normal block reward.
pub const MAX_FINDER_BONUS_SATS: i64 = 100_000_000;
```

The author's own comment calls 1 BTC "absurd" as a bonus *on top of* a proportional split. Worth noting as a design opinion from the codebase itself — though Parasite Pool ships exactly that value, and the value is a config knob either way.

Configurability today is **per-group, in the database**, not pool-wide TOML: `crates/bp-group-mgmt/src/group.rs:265` (`finder_bonus_sats: Sats`), validated against both `MAX_FINDER_BONUS_SATS` and the pool minimum payout (`:278-281`). There is **no `finder_bonus` key in `[pplns]`** — `crates/bp-config/src/lib.rs:557-617` (`PplnsConfig`) has no such field, and `blitzpool.example.toml:244-262` has no such line.

API surface for the cap already exists — `crates/bp-api/src/controllers/groups.rs:443-461`, `GET /api/pplns/groups/finder-bonus-cap`, returning the current block subsidy so a UI can bound the input.

## 4. Per-connection coinbase construction is already the architecture

This is the load-bearing finding, and the thing that makes the feature tractable rather than a redesign. **The finder cannot be known at block-found time and retrofitted** — the coinbase is committed to in the block header. So the coinbase must be built *speculatively per candidate finder*, in advance.

That is already how this pool works.

`bin/blitzpool/src/payout_resolver.rs:36-44`:

> "The resolver is called at most once per `(template-broadcast × connection)` event, so per-connection per-template cadence is ~30 s. The cache compresses concurrent lookups across connections so total throughput is bounded by the cache's TTL."

The dispatch takes the miner's address as its first argument (`:123-127`), and the Group-Solo arm passes it in as the prospective finder (`:366-384`):

```rust
// The finder is the miner connecting on this share path; the
// Group-Solo engine bumps the finder's payout via the
// `finder_bonus_sats` config knob when emitting the distribution.
```

The Group-Solo distribution cache is keyed on a **per-finder triple** — `crates/bp-group-solo-engine/src/distribution.rs:67-68`:

```rust
/// Cache key — concurrent calls with the same triple share one compute.
type CacheKey = (Uuid, u64, String);
```

with the module doc stating the intent outright (`:12-19`):

> "Different finders within the same group still compute independently because every miner's session calls `build_distribution` with their own address as the prospective finder."

And the job/coinbase cache was **designed** for per-finder payout sets. `crates/bp-mining-job/src/cache.rs:92-104` puts `&'a [PayoutEntry]` directly in the key tuple, and the module doc (`:19-21`, `:33-38`) says:

> "payout sets that differ per finder (Solo / Group-Solo / Blockparty) get distinct keys by construction — no per-mode special-casing"
>
> "Solo / Group-Solo payout sets are distinct per connection — those must not serialize behind one pool-wide lock"

There is one nuance worth stating precisely: the variation is per *payout list*, not per *socket*. Two connections from the same Solo address produce an identical list and share one cache entry. So "per-connection coinbase" in practice means **per distinct connected address**.

### The block-found lookup already handles many distributions per template

The concern that a per-finder coinbase would break block-found attribution turns out to be backwards — the machinery is already fingerprint-keyed, and one production path already exercises many concurrent distinct distributions per template.

`payouts_fingerprint` (`crates/bp-share/src/lib.rs:221`) hashes reward + each `(sats, address)` in order. It has **no session or connection component** — it identifies *which payout list was paid*, which is exactly the right key. It rides the winning share to the block sink (SV1 `bin/blitzpool/src/block_sink.rs:1181`, SV2 `:1271`), and `prepare_block_found_for` hard-requires it (`crates/bp-pplns-engine/src/engine.rs:474-486`) before reading `pplns:snapshot:<fp>`.

The precedent: **Stratum V2 Job Declaration (ext `0x0003`)** lets each JDC declare its own payout outputs, tracked per token with its own fingerprint — `crates/bp-stratum-v2/src/jdp/dynamic_outputs.rs:300-301` (`by_token: HashMap<Token, Vec<EmittedPayoutOutputs>>`), with single-use consumption, epoch staleness handling, and a `coinbase_pays_a_second_distribution` double-spend guard. Group-Solo likewise writes both a fingerprint-keyed *and* a per-finder-keyed snapshot in one `tokio::join!` (`crates/bp-group-solo-engine/src/distribution.rs:299-333`).

So the lookup needs no change. What's missing today is only that PPLNS emits one list per reward value, so its fingerprint carries **zero information about who found the block** — the submitter's identity reaches the ledger only as loose `address`/`worker`/`session_id` strings on `BlockFoundEvent` (`block_sink.rs:1172-1178`), used for logging and Solo booking, not for choosing coinbase outputs. Varying the list per finder makes the existing fingerprint carry that information automatically.

### SV1 wire-level: why a shared coinbase is a real optimization

For SV1, `coinb1`/`coinb2` are pre-rendered hex cached **on the shared `MiningJob`** and borrowed by every session's notify build (`crates/bp-mining-job/src/coinbase.rs:84-86`, `crates/bp-stratum-v1/src/notify.rs:398-399`). Per-session uniqueness comes entirely from a 4-byte `extranonce1` spliced into the 12-byte reserved slot at share-validation time (`coinbase.rs:14`, `crates/bp-stratum-v1/src/submit.rs:377`) — the coinbase bytes themselves are byte-identical across all PPLNS sessions on a template. `job_id` is per-connection; the coinbase is not.

## 5. What PPLNS would give up: the shared-job optimization

Today PPLNS is the *one* mode that benefits from a pool-wide shared coinbase. `crates/bp-stratum-v1/src/client.rs:840-842`:

```rust
// Pool-wide memoized build: SV1 always uses the fixed
// EXTRANONCE_SLOT_LEN, so every connection with the same payout set
// (all of PPLNS) shares literally ONE `MiningJob` per template.
```

A per-finder PPLNS bonus moves PPLNS from *one* job per template to *one job per distinct connected address* per template. Correspondingly, `crates/bp-pplns-engine/src/distribution.rs:175` would need to widen:

```rust
cache: InflightResultCache<u64, DistributionResult, DistributionError>,
```

from a `u64` (reward only) key to a tuple including the finder — mirroring Group-Solo's existing triple.

**What blunts the cost:** the expensive half of a build is already shared. `crates/bp-pplns-engine/src/distribution.rs:176-181` keeps a second cache for the reward-independent inputs, keyed by `()`:

```rust
/// Reward-independent window+ledger inputs, shared across every
/// concurrent build. Keyed by `()` — there is exactly one payout
/// window — so the cache degenerates to "one load per invalidation
/// epoch, deduped across all in-flight builds".
inputs_cache: InflightResultCache<(), DistributionInputs, DistributionError>,
```

So the Redis window read and the Postgres ledger query stay at one-per-epoch regardless of connection count. The incremental per-connection cost is the pure math plus one coinbase assembly.

**But a second memoization layer does get defeated, and it is the more expensive one.** `crates/bp-mining-job/src/cache.rs:129` has an inner cache for parsed outputs:

```rust
type OutputsKeyTuple<'a> = (Network, u64, &'a [PayoutEntry]);
```

This memoizes `address_to_script` across jobs so each payout address is parsed once per template rather than once per job (`build_payout_outputs`, `crates/bp-mining-job/src/coinbase.rs:516`). A finder bonus changes one entry's sats, which changes the whole key — so **the script parse for every one of the hundreds of PPLNS payout outputs re-runs per finder.** Per the module doc (`cache.rs:8-10`) killing exactly that redundancy is why the cache exists. This, not the distribution math, is the dominant new CPU term, and it is the one number this assessment does not have measured.

Each per-finder `MiningJob` also holds `coinbase_prefix`, `coinbase_suffix`, **and** both pre-rendered hex strings (`coinbase.rs:80-95`) — roughly 3× the raw coinbase size per entry, on a coinbase that carries the full PPLNS payout list.

Job cache eviction is TTL-only with **no size cap and no max-entries** — `crates/bp-mining-job/src/cache.rs:79-83`: `ENTRY_TTL = 120s`, `PRUNE_INTERVAL = 10s`. At a ~30 s template cadence that means roughly **4 template generations × N distinct finder lists** resident at once, against ~4 × 1 today.

Not a blocker, and the concurrency was designed for it: `cache.rs:33-37` notes that distinct keys build in parallel precisely because "Solo / Group-Solo payout sets are distinct per connection — those must not serialize behind one pool-wide lock", with `concurrent_distinct_keys_build_independently` (`cache.rs:770`) pinning the behaviour. But it is the thing to load-test before running it on a real pool.

## 6. Measured cost of the per-connection math

Measured against this commit with a throwaway integration test (scratch copy at `.buzz/.scratch/blitzpool-spikes/spike_perconn_cost.rs`, not committed to the repo), calling `build_coinbase_distribution` 500 times with distinct finders off pre-loaded inputs, `--release`, on an Apple-silicon dev box:

| PPLNS window size | 500 per-connection builds | per build |
|---|---|---|
| 100 miners | 96.6 ms | **0.193 ms** |
| 400 miners | 369.2 ms | **0.738 ms** |
| 1000 miners | 952.7 ms | **1.905 ms** |

At a ~30 s template cadence, 500 connections against a 1000-miner window is ≈0.95 s of CPU per template broadcast — roughly **3% of one core**. Debug-build numbers are ~12× worse (2.4 / 9.0 / 23.0 ms) which is why a release measurement matters here.

**Conclusion: the distribution math is not the bottleneck.** Scope note — this measures `build_coinbase_distribution` only. It does **not** measure the coinbase assembly and per-output script parsing that also fan out per finder (§5), which is the larger unknown.

## 7. The pure math already holds under PPLNS semantics — verified

The in-tree bonus tests only cover `suppress_matching_debits: true` (the Group-Solo unsigned-pending model): `crates/bp-pplns/src/distribution.rs:1340-1367` (`finder_bonus_emitted_as_separate_output`) and `:1307-1334` (`bonus_only_coinbase_claims_the_residuum_too`). The untested combination is the one this question needs: **bonus + `suppress_matching_debits: false`** (PPLNS signed credit/debit ledger) **+ non-zero opening balances**.

A throwaway spike (`.buzz/.scratch/blitzpool-spikes/spike_pplns_finder_bonus.rs`) exercised it. **All 5 cases pass** against commit 7815884:

| Spike case | Result |
|---|---|
| 1 BTC bonus, 8 miners, PPLNS ledger — reward conserved exactly | pass |
| 1 BTC bonus + mixed signed balances (credits *and* debits) — conserved | pass |
| Bonus dilutes non-finder miners (economic check) | pass |
| 10 BTC bonus on a 3.125 BTC block — clamped by the 95% cap, still conserves, no non-positive outputs | pass |
| 1 BTC bonus under starved budgets (900 / 1200 / 2000 / 5000 WU) — trim fires, still conserves | pass |

Baseline for comparison: `cargo test -p bp-pplns` = **45 tests, all passing** at this commit (40 unit incl. 2 proptests, 5 regtest-harness).

### Measured dilution

From the spike, 8 equal-share miners, 3.125 BTC subsidy, 1.5% pool fee:

- Non-finder payout: **38,476,562 → 25,976,562 sats** (ratio **0.675**)
- Finder total: **125,976,566 sats** (1 BTC bonus + their proportional share)

So a 1 BTC bounty on a 3.125 BTC subsidy costs every non-winning miner ~**32.5%** of their PPLNS payout for that block. That is the real design tradeoff, and it is arithmetic, not opinion: see [[../wiki/concepts/variance-and-risk-shifting|Variance and Risk Shifting]].

## 8. The one real gap: PPLNS ledger apply has no duplicate-address merge

The finder legitimately appears **twice** in the payout list — once as the bonus output, once as their proportional share. The spike confirms this (`finder_outputs == 2`).

Group-Solo handles it explicitly. `crates/bp-group-solo-engine/src/engine.rs:912-936`:

```rust
// The distribution can name the same address more than once:
// Group-Solo emits the finder both as a dedicated bonus output
// AND as their proportional share output. Both are valid on-chain
// TxOuts, but the ledger keys on (address, groupId) — Postgres
// rejects a second ON CONFLICT hit for the same key in one upsert,
// and the history table's (groupId, blockHeight, address) UNIQUE
// would silently drop the duplicate. Merge per-address (summing
// sats + percent) so each address yields exactly one audit +
// balance write.
```

**The PPLNS equivalent has no such merge.** `crates/bp-pplns-engine/src/engine.rs:644-660` (`build_writes_from_snapshot`) iterates `snapshot.distribution` and pushes one `AuditRow` + one `BalanceWrite` per entry, with an `emitted` HashSet that guards only the *later* pending/late-arriver loops — not duplicates within the distribution itself.

Downstream, `crates/bp-pplns-engine/src/ledger/mod.rs:74-140` (`apply_distribution`) relies on:
- `pplns_payout_history` UNIQUE `(blockHeight, address)`
- a history-gated `bulk_upsert_pplns_balances`

So a duplicate finder address would hit that UNIQUE constraint. Per the doc comment at `:78-86`, the history insert "silently dedupes" — meaning **the bonus row would be dropped from the audit trail, and `totalPaidSats` would under-count what the chain actually paid.** The on-chain payment still lands (the coinbase is correct); the *books* would disagree with the block.

This is a bounded, well-understood fix — port the Group-Solo merge (`engine.rs:912-936`) into the PPLNS write path — but it is not optional, and it is the kind of accounting drift that is painful to detect after the fact.

## Implementation delta (assessed, not implemented)

| # | Change | File | Difficulty |
|---|---|---|---|
| 1 | Add `finder_bonus_sats` to `[pplns]` config | `crates/bp-config/src/lib.rs:557` + `blitzpool.example.toml` | trivial |
| 2 | Pass finder through instead of `None` | `crates/bp-pplns-engine/src/distribution.rs:340` | trivial |
| 3 | Widen `build_distribution` signature to take the finder | `crates/bp-pplns-engine/src/engine.rs:388` | small |
| 4 | Widen inflight cache key `u64` → `(u64, String)` | `crates/bp-pplns-engine/src/distribution.rs:175` | small |
| 5 | Thread `miner_address` as finder in the PPLNS arm | `bin/blitzpool/src/payout_resolver.rs:236` | small |
| 6 | **Port the per-address merge into the PPLNS ledger apply** | `crates/bp-pplns-engine/src/engine.rs:644` | **medium — correctness-critical** |
| 7 | Tests for bonus + signed ledger (spikes are a starting point) | `crates/bp-pplns/src/distribution.rs` tests | small |

Not needed: the distribution math, the weight/trim accounting, the 95% cap, the dust suppression, the per-connection coinbase build, the per-connection job cache, and the fingerprint→snapshot block-found binding. All present.

**Honest framing of the size of this.** Items 1-5 are each individually small, and item 6 is bounded. But the aggregate effect is not a flag flip: it converts PPLNS from *one* coinbase + *one* snapshot write per template into *N* of each, where N is the count of distinct connected addresses. Every mechanism needed to do that already exists and is exercised in production paths — the work is wiring plus load-testing the fan-out, not new design.

## Operational caveats

- **Snapshot keyspace growth.** Snapshots are keyed by payout-list fingerprint (`crates/bp-pplns-engine/src/window/mod.rs:485-500`) with `snapshot_ttl_secs` default 1200. Per-finder distributions turn one Redis write per template into one per distinct connected address, and `delete_all_fingerprinted_snapshots` (`window/mod.rs:537`) has correspondingly more keys to SCAN after each apply. Commit `44b7a6c` already excluded these keys from Redis backup on the grounds that a busy pool holds "orders of magnitude more of these than window or round state" — so the infrastructure anticipates this shape, but Redis memory needs sizing.
- **Coinbase weight budget.** The bonus output consumes a slot in a budget that also has to fit every PPLNS payout. `[pplns].coinbase_weight_budget` floor is 50,000 WU with the autoscaler stepping to `max_weight_budget`. The trim is greedy-largest-first, so the 1 BTC bonus output is never the one trimmed — small miners are, into pending credit.
- **Vouching.** `snapshot_written` / `books_without_a_snapshot` (`bin/blitzpool/src/payout_resolver.rs:113-122`, `:199-204`) gate whether a found block can be booked automatically. Any new path must preserve the invariant that a *build must never fail over a lost snapshot* — failing sends the resolver to its solo fallback, which hands that one miner the entire block (`crates/bp-pplns-engine/src/distribution.rs:369-386`).
- **Repo maturity.** Created 2026-06-23, 1 star, unaudited, single primary author. The engineering quality of the code read here is genuinely high (dense invariant comments, proptests, regtest harness, ~40% of the tree is tests), but this is not battle-tested infrastructure.

## See also

- [[2026-07-27-blitzpool-server-rust-github]] — README-level capture of the same repo
- [[../wiki/concepts/parasite-pool|Parasite Pool]] — ships a flat 1 BTC finder bounty over decay-weighted PPLNS
- [[../wiki/concepts/pplns|PPLNS]] — the base scheme
- [[../wiki/concepts/variance-and-risk-shifting|Variance and Risk Shifting]]
