---
title: "Share Accounting and Durability"
category: concept
sources:
  - raw/repos/2026-09-21-ckpool-architecture.md
  - raw/notes/2026-09-21-adjacent-systems-design-transfer.md
  - raw/papers/2026-09-21-smartpool-decentralized-pooled-mining.md
  - raw/papers/2026-09-21-withholding-and-power-splitting-attack-economics.md
  - raw/repos/2026-09-21-sri-and-sv2-apps-hardening-releases-2026.md
  - raw/repos/2026-09-21-public-pool-and-miningcore-managed-runtime-pools.md
created: 2026-09-21
updated: 2026-09-21
tags: [share-accounting, durability, batching, wal, single-writer, lock-contention, deduplication, replay, in-memory-accounting, retention, withholding-detection]
aliases: ["share storage", "share database", "accounting hot path"]
confidence: high
volatility: cold
summary: "What a pool must actually remember about shares, and where. The strongest evidence available says: far less than expected, and not in a database on the hot path — the most scaled open-source Bitcoin pool keeps share accounting entirely in memory. What genuinely must persist is driven not by payout arithmetic but by two adversarial requirements: replay prevention and long-horizon withholding detection."
---

# Share Accounting and Durability

> Shares look like financial records, so the instinct is to write each one durably. The evidence points
> the other way. The question to ask of every piece of share state is: **is it on a hot path, and does
> an adversary benefit if we forget it?**

## Round 2: eight implementations, one rule

A source read of eight implementations settles the empirical question: **nobody in production writes one
durable row per share**, and the single design that does — MPOS, with a primary key plus four secondary
indexes and no batching — is the only one with a documented scaling failure (reported to struggle above
1–2k concurrent miners). Full schemas and the write-amplification table are in
[[share-storage-architectures|Share Storage Architectures]] ([Share Storage Architectures](../references/share-storage-architectures.md)).

Two results from that survey belong here because they change this article's conclusions:

- **The SRI reference pool persists nothing at all.** No database crate in `pool-apps/pool/Cargo.toml`, no
  file I/O, no serialisation. Counters live in a per-channel `ShareAccounting` struct and the integration
  point is an HTTP polling API (`/metrics`, `/api/v1/clients/{id}/channels`). On restart every counter
  zeroes **and `seen_shares` empties, so replay protection does not survive a restart** — an adversary who
  can trigger a restart can roll over the dedup window deliberately. The reference pool is a
  share-validation frontend, not a complete pool.
- **public-pool's write load scales with sessions, not shares** — in-memory accumulation flushed as
  10-minute buckets, roughly one `UPDATE` per active session per minute regardless of share count. The
  persistence layer independently reproduces this topic's central finding about connection count.

## The reference data point: ckpool has no database

ckpool — the canonical high-performance design, running in production at solo.ckpool.org — keeps
**accounting entirely in memory**. `uthash` tables hold `user_instance`, `worker_instance`,
`stratum_instance` and `notify_instance`, with rolling share-rate averages (`dsps1/5/60/1440/10080`).
Share logging is *optional* (`-L`), written by a **dedicated logging thread**, "divided by block height
and workbase". **No SQL schema ships with it.**

This is the strongest single argument against per-share durable writes: the most scaled open-source
Bitcoin pool does not do it. **And it is stronger than round 1 recorded** — a source read found
`ckdb`, ckpool's companion **PostgreSQL** accounting daemon, referenced at `ckpool.h:361-365` but
disabled by commit `4b655e1d` (2017-05-13, *"Disable ckdb by default"*). ckpool did not merely never have
a database; it had one and switched it off almost a decade ago.

**One correction to round 1 in the other direction**: the optional `-L` sharelog is *not* written by a
dedicated thread. `stratifier.c:6246-6249` shows the **stratifier itself** doing
`fopen(fname, "ae")` → one `fwrite` per share → close, **synchronously, on the share-submit path** — one
JSON line per share including rejects, carrying both `diff` and `sdiff`. Survivable at realistic share
rates, but it is the one place ckpool's otherwise strict off-hot-path discipline breaks. Nothing in-tree
reads those logs.

It is enabled by a specific product decision. solo.ckpool has **no registration and no operator
wallet** — usernames *are* payout addresses. No accounts means no account table, no auth store, no
credential management, no KYC surface. **The absence of a user table is what removes the entire
persistence tier.** Any pool that adds accounts re-acquires all of it, which is why Miningcore requires
PostgreSQL 10+ and public-pool ships an ORM.

## The staged pipeline

Miningcore names the stages explicitly — `ShareReceiver` → `ShareRecorder` → `ShareRelay` — which is
the clearest published statement that **accepting a share and making it durable are different concerns
on different latency budgets.**

ckpool encodes the same boundary as the `unaccounted` / `accounted` split in `pool_stats_t`: count
immediately, aggregate later, elsewhere.

## Two rules from outside mining

**Batch, then fsync.** InfluxDB's storage engine batches **5,000–10,000 points** per write, serialising
and compressing into a WAL and calling fsync **once per batch, not per event**, with segments closing at
10 MB for sequential I/O. Shares *are* time-series (timestamp, worker, difficulty, job). This is the
direct answer to per-share write amplification, and ckpool's off-thread append-only log is the laziest
version of it.

**Single writer.** Martin Thompson's measurements on 500M counter increments: single thread 300 ms;
single thread with a memory barrier 4,700 ms (**15.7×**); two threads with CAS 18,000 ms (**60×**); two
threads with locks 118,000 ms (**393×**). Contention overhead exceeds the work being done.

The anti-pattern is therefore named precisely: **several connection workers contending on one shared
share/stats map under a lock.** The fix is per-worker or per-miner-shard counters with no lock, flushed
to a single accounting owner. ckpool's `ckmsgq` queues are that multiplexer; its `unaccounted` bucket is
the flush boundary. **393× is the number that justifies the architecture.**

## What must persist, and why — the adversarial requirements

Payout arithmetic needs surprisingly little. Two adversarial requirements drive the real retention.

### 1. Replay prevention, and its direct conflict with bounding

A `seen_shares` set on the submit path is what stops a share being credited twice. SRI's 2026 hardening
release documents both failure modes and the tension between their fixes:

- **Dedup keyed to the wrong thing.** The cache was flushed on **timestamp updates**, not only on
  block-header change — so resubmitting an identical share with a different timestamp got credited
  twice. Fixed by keying invalidation to **`prev_hash` only**, the actual work-uniqueness boundary.
- **Resource recycling racing job lifetime.** Rotated extranonce prefixes were released immediately
  while jobs using them were still active, enabling **cross-channel replay that per-channel duplicate
  detection structurally cannot catch**: take prefix A, submit shares, wait for A to be reassigned,
  receive A on another channel, replay. Fixed by retaining retired prefixes until all associated jobs
  are stale.
- **And then bounding — but only on the client side.** The release notes read as though caps were added
  everywhere. **A source read at `stratum@9b0454ec` shows otherwise**: `MAX_FUTURE_JOBS = 16` and the
  bucketing of `rejected_shares` into nine known error codes plus `"unknown"` live in
  `channels-sv2/src/client/`. The **pool server's** `JobStore` still uses unbounded `HashMap`s for
  `future_template_to_job_id`, `future_jobs`, `past_jobs` and `stale_jobs`
  (`server/jobs/job_store.rs:24-31`), **`stale_jobs` are never cleared**, and server-side
  `rejected_shares` is an unbounded `HashMap<String, u32>` **keyed by the `error_code` string** — a
  memory-exhaustion surface if a downstream can vary that string. Hardening the client against a hostile
  pool while leaving the pool unhardened against hostile miners is the wrong way round.

**`seen_shares` must be large enough to prevent replay and bounded enough to survive a long-lived
connection. Those requirements are in direct conflict**, and the release resolves it with a configured
cap — meaning the safe size is now an operator's problem rather than a designed invariant.

**The general rule**: deduplication state must be keyed to actual work uniqueness, never to metadata a
client can vary, and must never be scoped to something an adversary can cause to roll over. SmartPool
solves the same problem structurally rather than by cache size — an **augmented Merkle tree** whose
nodes carry `(min, hash, max)` with a **monotonic counter**, plus a persisted `last_max` rejecting any
new claim whose `min ≤ last_max`. Deliberately not resettable by a chain event.

### 2. Withholding detection needs long-horizon per-worker history

Block withholding is profitable under existing reward schemes over extended horizons (Luu et al., IEEE
CSF 2015), and a smart contract that pays others to withhold lets an adversary with **0.0000002% of
network hashpower** drive a large PPS pool's revenue to zero at no net cost (Velner et al. 2017) —
against **>1% of network hashrate** needed for a 5% dent by classical withholding.

**No protocol upgrade fixes this.** The attacker is an authenticated, well-behaved miner submitting
valid shares and silently dropping the one that is a block. SV2's encryption and authentication are
irrelevant to it.

Detection is statistical and slow: it requires comparing **expected against actual block attribution
per worker over long horizons**. That is a retention and indexing requirement on the share pipeline,
produced by a game-theory result rather than by any accounting need. It is the clearest case where
"what must we remember?" is answered by an adversary and not by payout.

**And no production pool meets it.** The round-2 survey found raw per-share history deleted at payment
(Redis pools), after 24 hours (public-pool), after 7 days (p2poolv2), or never written at all (BTCPool's
MySQL, SRI). The only indefinite raw record anywhere is ckpool's sharelog — which nothing in-tree reads.
So the retention that withholding detection requires **exists in no open implementation**. The nearest
is Blitzpool (2026-09-23 source read), which never prunes lifetime per-worker totals or per-block payout
history. But those totals have no time dimension, and its per-worker time series is kept for only 14
days. The aggregates that pools do keep (hourly difficulty sums per worker) may or may not be sufficient
for the test. That is an open question worth resolving, because it decides whether the detection requirement
costs raw retention or merely a longer aggregate history.

## Sampling instead of exhaustive verification

Because accounting is off the hot path, it can afford a different correctness model. SmartPool's
result: if a miner claims `n` shares of which only `m` are valid, sampling `k` detects the cheat with
probability `1 − (m/n)^k`; with an all-or-nothing penalty the cheater's expected payout is exactly
`m` — **what honest behaviour would have paid**. Worked: 1,000 claimed, 500 valid, 50% detection,
`0.5 × 1000 + 0.5 × 0 = 500`. So `k = 1` or `2` suffices.

**The reusable principle: verification of a high-volume accounting stream can be sampled rather than
exhaustive, provided the penalty function makes cheating expectation-neutral.** The substrate
(an Ethereum contract) does not transfer to Bitcoin; the principle does.

## Attribution is getting harder

DMND's SLICE allocates the subsidy by PPLNS but the **fees by job-declaration scoring** — according to
whose template was used. That means the pipeline must retain **template provenance per share**. Paying
miners for template contribution rather than only for hashrate adds a dimension to what accounting has
to remember, at exactly the moment template construction is moving off the pool.

## Summary

| State | Hot path? | Durable? | Driver |
|---|---|---|---|
| Per-connection difficulty, extranonce | yes | no | reconstructible |
| Rolling share rates | no | no | derived, in-memory (ckpool: 5 windows) |
| `seen_shares` dedup set | **yes** | no, but must survive `prev_hash` changes | **replay** |
| Retired extranonce prefixes | yes | no, but must outlive referencing jobs | **cross-channel replay** |
| Per-worker share history | no | **yes, long horizon** | **withholding detection** |
| Template provenance per share | no | yes, if fees are attributed | SLICE-style payout |
| Accounts / credentials | no | yes — **if you have accounts at all** | product decision |

## See Also

- [[share-storage-architectures|Share Storage Architectures]] ([Share Storage Architectures](../references/share-storage-architectures.md)) — the eight schemas, write amplification, retention primitives.
- [[the-two-hot-paths|The Two Hot Paths]] ([The Two Hot Paths](the-two-hot-paths.md))
- [[template-sourcing-and-control|Template Sourcing and Control]] ([Template Sourcing and Control](template-sourcing-and-control.md)) — where template provenance comes from.
- [[pool-implementation-survey|Pool Implementation Survey]] ([Pool Implementation Survey](../topics/pool-implementation-survey.md)) — who uses a database and who does not.
- Payout schema design itself belongs to the `bitcoin-mining-payout-schemas` hub topic; this article covers only the architectural constraints those schemas impose.
