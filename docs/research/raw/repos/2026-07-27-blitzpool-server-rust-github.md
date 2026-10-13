---
title: "warioishere/blitzpool-server-rust — non-custodial in-coinbase payout pool (Solo / PPLNS / Group-Solo / Blockparty)"
url: https://github.com/warioishere/blitzpool-server-rust
source: "https://github.com/warioishere/blitzpool-server-rust"
type: repos
category: repo
language: Rust
license: AGPL-3.0-or-later
homepage: https://blitzpool.yourdevice.ch
operator: yourdevice.ch (Switzerland)
repo_created: 2026-06-23
last_commit: 2026-07-27
latest_release: 2.2.5
commits_on_main: 129
stars: 1
ingested: 2026-07-27
volatility: hot
quality: 4
credibility: medium
confidence: medium
tags: [blitzpool, non-custodial, pplns, coinbase-payout, group-solo, blockparty, solo-mining, stratum-v1, stratum-v2, job-declaration, template-distribution, bitcoin-core-ipc, coinbase-weight-budget, vardiff, rust, agpl]
summary: "Rust rebuild (37-crate workspace, AGPL-3.0) of the Blitzpool non-custodial mining pool by yourdevice.ch. Its defining claim: every payout — Solo, PPLNS, Group-Solo, Blockparty — is written directly as an output of the coinbase transaction of the block that earned it, so the pool never holds miner sats and there is no minimum payout. Notable accounting mechanics: multi-output PPLNS coinbase over a sliding 4×-netdiff window, a signed credit/debit ledger that carries trimmed/sub-dust sats forward as pending, a self-tuning coinbase weight-budget autoscaler, and confirmation-gated orphan-safe ledger application."
---

# warioishere/blitzpool-server-rust — non-custodial in-coinbase payout pool

Open-source Bitcoin mining pool, **AGPL-3.0-or-later**, by the Blitzpool team at [yourdevice.ch](https://yourdevice.ch) (Switzerland). Live at `blitzpool.yourdevice.ch`.

**Central claim (README, verbatim framing):** "every payout — Solo, PPLNS, Group-Solo, Blockparty — is written directly into the coinbase transaction of the block that earned it. No pool wallet, no custody period, no FPPS-style intermediate."

This repo is the **ground-up Rust rebuild** of the original TypeScript [warioishere/blitzpool](https://github.com/warioishere/blitzpool) (created 2025-05-02, 11 stars, still pushed as of 2026-06-30), which was itself a fork of [benjamin-wilson/public-pool](https://github.com/benjamin-wilson/public-pool). The README states the two are **feature-equivalent in protocol behaviour** (Stratum V1/V2, coinbase distribution) but that "the internals are not." Rust repo created 2026-06-23; 129 commits on main; release 2.2.5 on 2026-07-27 (actively developed — commits landing the same day as ingest).

## The four payout modes

All four write payout straight into the coinbase; they differ only in **how the reward is split**.

| Mode | Split rule | Coinbase reservation |
|---|---|---|
| **Solo** | Winner's share takes the entire coinbase. No fee, no custody. | fixed |
| **PPLNS** | Proportional cut per miner in the sliding window, each as **their own coinbase output**. | autoscaled |
| **Group-Solo** | Closed group; each block the group finds splits proportionally to member shares **in that round**. | fixed |
| **Blockparty** | Directed group sharing a hashpower rental; each member takes a **fixed cut in basis points** of every block the rented hashrate finds. | fixed (only when configured) |

Operator claim on censorship-resistance: because the miner address(es) are the *direct* coinbase destination, "an operator can't withhold payouts — they'd have to refuse to relay the block at all, and the miner could submit it elsewhere."

### PPLNS specifics (the accounting-relevant mode)

- **Window size:** `window_factor × networkDifficulty`, default **4×**, measured in diff-1-weighted shares. README asserts this is **anti-hop by design** — sliding window, not per-block reset.
- **Signed credit/debit ledger:** a per-miner *signed* balance keeps the pool "non-custodial to the sat." Trimmed and sub-dust sats become a **pending credit** for the miner who earned them; the **bonus recipient of the same block picks up a matching pending debit**. Pool-wide balances therefore sum to ~0, with bounded floor-rounding drift.
- **Group-Solo** trimmed sats instead stay as positive `pendingSats` and clear in a future block. **Blockparty** residuals fold into the pool-fee output.
- Per-group **opt-in** per-block round reset (Group-Solo).
- Group-Solo has address-driven routing, admin-token auth, and email-verified invitations.

## Coinbase-space mechanics (the interesting engineering)

### One IPC template stream per payout mode

Talks to **bitcoin-core v31 over its Cap'n-Proto IPC socket** (Template Distribution Protocol) — explicitly *not* `getblocktemplate` + ZMQ. Each payout mode gets its own IPC client, hence its own template stream.

Per stream, the pool tells core how much coinbase space to reserve via the IPC **`coinbase_output_max_additional_size`** field, derived from that mode's coinbase weight budget. Two claims worth noting:

- **No `blockreservedweight` in `bitcoin.conf`** — the reservation is handed to core per-stream at runtime.
- Streams are **independent**, so reservations "don't sum against a shared limit," because only one stream's block ultimately wins.

TDP streams only spawn in a process holding the `front` role.

### PPLNS coinbase autoscaler

The PPLNS stream's coinbase weight budget self-tunes between a floor (`[pplns].coinbase_weight_budget`, default **50 000 WU**) and a hard ceiling (`[coinbase_autoscale].max_weight_budget`):

- **Steps up** when utilization ≥ `up_threshold` (default **0.85**, i.e. 15 % of headroom before trimming would start), after `up_debounce` samples.
- **Steps down** when utilization ≤ `down_threshold` (default **0.50**), after `down_debounce` samples.
- Multiplicative `step_factor` (default **1.15** = ±15 %), with a `cooldown_secs` floor between changes.

Stated effect: reservation grows ahead of a growing PPLNS window, and when the window shrinks the reservation shrinks and **reclaims block space for fee-paying transactions**. Alt streams (Solo / Group-Solo / Blockparty) stay on small fixed reservations sized to the largest expected group.

### Always-valid blocks via weight-budget trim

The distribution builder reserves structural coinbase overhead, then **greedily keeps the largest payouts that fit** the stream's budget and **trims the smallest to pending** (carry-forward). Because the assembled coinbase can never exceed the reservation core was told about, the README claims **a found block is always valid** — the pool cannot build a coinbase that overflows its template and gets rejected.

Stated tradeoff: under-sizing a budget costs **fairness** (a small miner waits a block or two), never **validity**. Trimmed sats are never lost. A capacity-alert cron emails the operator before anyone is trimmed, and `GET /api/pplns/groups/coinbase-capacity` surfaces the live member ceiling.

### Confirmation-gated, orphan-safe payouts

PPLNS and Group-Solo **freeze** a block's distribution and park it in Redis keyed by block hash. A confirmation watcher applies the ledger only after `confirmation_depth` (default **3**) confirmations, and **discards orphaned candidates** — no payout is booked for a block that didn't survive on-chain. Ledger applies are idempotent (history `UNIQUE` constraint + history-gated balance upsert), so stream redelivery or a duplicate block-found can't double-credit.

## Core/Satellite split (horizontal scale)

Role-gated processes communicating over **Redis streams**:

| Role | Responsibility |
|---|---|
| `front` | Stratum listeners, template streams, job building, block submit |
| `api` | read-only HTTP API |
| `payout` | ledger apply, confirmation gating, dust sweep |
| `stats` | share-stats aggregation |
| `notify` | Telegram / ntfy / push / email fan-out |

Roles are selected with `--roles` / `BLITZPOOL_ROLES`, independent of payout mode. Block-found and accepted/rejected shares flow Core→Satellite over Redis streams with **exactly-once semantics**; routing-cache invalidation is broadcast cross-process. Runbook: `full-setup/DEPLOYMENT.md`, split validation at `full-setup/REGTEST-SPLIT-VALIDATION.md`.

## Stratum support

- **Stratum V1** — full SV1 stack with vardiff, `mining.notify` (ckpool-convention hex-padded fields), per-port difficulty.
- **Stratum V2** — Noise handshake, standard **and extended** channels (extranonce rolling, merkle reconstruction, pool-side share validation), Template Distribution Protocol, **Job Declaration Protocol**, SipHash-2-4.
- SV1 and SV2 **share the same TCP ports**; protocol detection on the first byte routes the socket.

### Endpoints (operator-configured per-port TOML)

| Role | Starting difficulty | Purpose |
|---|---|---|
| Default entry | configured (e.g. 1 000) | Solo / Group-Solo / Blockparty |
| High-diff rental | **1 000 000** | NiceHash / MRR / Braiins rentals; canonical Blockparty rental port |
| PPLNS opt-in | adaptive | explicit opt-in to PPLNS payout |
| SV2 JDP | — | Job Declaration Protocol (when enabled) |

**Routing priority on connect:** explicit PPLNS port → Blockparty admin address → Group-Solo membership → Solo. Group-Solo and Blockparty are **mutually exclusive per address**. Extranonce2 size is 8 bytes.

Recent vardiff work (2026-07-26/27) is notable for proxy interop: difficulty is assigned only as **powers of two** so a proxy has nothing to round, assigned difficulty rounds **UP** never to nearest rung, the configured SV2 difficulty floor is also rounded to a power of two, and a session is regulated **before its first share**.

## Tech stack

Rust workspace of **37 crates** (`bp-*` plus the `blitzpool` binary): tokio (async), axum (HTTP), sqlx/Postgres (DB), Redis (share/cache/stream bus). **Postgres + Redis are required — there is no SQLite path.** Schema applied via **sqlx migrations run at boot**, advisory-locked and idempotent so every process in a split can run them safely.

SV2 protocol primitives come from the SRI **`stratum-core`**; the bitcoin-core IPC bridge comes from **`sv2-apps` (`bitcoin_core_sv2`)**. Upstream pin/bump strategy documented in `UPSTREAM_DEPS.md`.

Crate list (36 `bp-*` crates): `bp-api`, `bp-bitcoin`, `bp-blockparty`, `bp-blockparty-engine`, `bp-coinbase-snapshot`, `bp-common`, `bp-config`, `bp-cron-utils`, `bp-db`, `bp-geoip`, `bp-group-mgmt`, `bp-group-mgmt-engine`, `bp-group-solo`, `bp-group-solo-engine`, `bp-inflight-cache`, `bp-job-declaration`, `bp-jobs-lifecycle`, `bp-metrics`, `bp-mining-job`, `bp-mining-mode`, `bp-notifications`, `bp-pplns`, `bp-pplns-engine`, `bp-protocol-detect`, `bp-regtest-harness`, `bp-session-persistence`, `bp-share`, `bp-share-hook`, `bp-share-stats-sink`, `bp-share-stream`, `bp-stats`, `bp-stratum-v1`, `bp-stratum-v2`, `bp-template-distribution`, `bp-test-support`, `bp-vardiff`.

## Configuration surface (TOML-first, parsed by `bp-config`)

| Section / key | Purpose |
|---|---|
| `[tdp].socket_path` | bitcoin-core IPC socket for template streams |
| `[pplns].coinbase_weight_budget` | PPLNS budget **floor** (default 50 000 WU) |
| `[coinbase_autoscale]` | `max_weight_budget` (ceiling), `up`/`down_threshold`, `step_factor`, debounce, cooldown |
| `[group_fees].coinbase_weight_budget` | Group-Solo + Blockparty fixed budget (default **10 000 WU ≈ 50 members**) |
| `[group_fees].address` / `.percent` | shared Group-Solo/Blockparty fee lane (falls back to `[pplns]` fee) |
| `[solo].coinbase_weight_budget`, `[blockparty].coinbase_weight_budget` | per-mode fixed alt-stream reservations |
| `[capacity_alert]` | operator capacity-alert email thresholds |
| `--roles` / `BLITZPOOL_ROLES` | deployment topology override |

## HTTP API

Served by the `api` role; mirrors the TS pool's surface — pool-wide (`/api/info/*`, `/api/network`), per-address (`/api/client/:address/*`, `/api/pplns/mode/:address`), PPLNS (`/api/pplns/*`), Group-Solo (`/api/pplns/groups/*`), Blockparty (`/api/blockparty/*`), plus email-binding endpoints. Rust-build additions:

- `GET /api/pplns/groups/coinbase-capacity` — worst-case member ceiling for the Group-Solo coinbase budget.
- `GET /api/pplns/groups/finder-bonus-cap` — current block subsidy, used to cap the finder-bonus input.

Frontend is a separate Angular repo: [blitzpool-ui](https://github.com/warioishere/blitzpool-ui).

## Build & test

```bash
cargo build --release            # builds the `blitzpool` binary
cargo test --workspace           # unit + integration (~40% of the tree is tests)
cargo clippy --workspace         # lints
```

Regtest-driven integration tests (TDP/IPC + RPC) spin up bitcoin-core via the in-tree `bp-regtest-harness`. The **`bp-template-distribution` and `bp-job-declaration` suites are described as the bitcoin-core compatibility canaries.** Regtest deploy helper: `build_and_deploy_regtest.sh`. Top-level dirs: `bin`, `crates`, `db`, `full-setup`, `scripts`, `.sqlx`, `.cargo`, `.github`.

## Comparison table the README asserts

| | Blitzpool | Typical FPPS / PPS+ | Custodial PPLNS |
|---|---|---|---|
| Payouts on-chain | same block as the find | batch cron, hours to days | threshold-based |
| Pool holds miner sats | never | between find & payout | until threshold |
| Minimum payout | none (it's just a coinbase output) | typically 0.001 BTC+ | same |
| Stratum V1 | ✅ | ✅ | ✅ |
| Stratum V2 (Noise + TDP + JDP + extended channels) | ✅ actively developed | rare | almost never |
| Non-custodial Group-Solo | ✅ | — | — |
| Non-custodial Blockparty (co-funded rentals) | ✅ | — | — |

## Source-criticism notes

- **Everything above is operator/README self-description**, not independently verified. Credibility set to `medium`: the claims are specific and code-backed (crate names, config keys, defaults all check out against the repo tree), but the anti-hop, always-valid-block, exactly-once, and "non-custodial to the sat" properties are **asserted, not audited**.
- **1 star, 0 forks, 0 open issues, created 2026-06-23** — very young, essentially single-team. Not evidence of production hashrate. The TS predecessor has 11 stars and a longer history.
- The FPPS/PPS+ comparison column is the README's framing of competitors, not a neutral survey.
- "37 crates" counts the `blitzpool` binary alongside 36 `bp-*` crates.
- Unverified from the README alone: whether JDP is enabled in production, how the finder bonus is calculated, and what the dust threshold actually is. **→ The finder-bonus question is now resolved** against a local clone at commit `7815884` — see [[2026-07-27-blitzpool-finder-bonus-code-read]]. Short version: the bonus is a dedicated coinbase output carved out of the miner cut *before* the proportional split (so co-miners fund it), capped at 95% of that cut, suppressed below `min_payout_sats`, with an absolute ceiling of 1 BTC; it is wired for Group-Solo only and PPLNS passes `None`. JDP-in-production and the dust threshold remain unverified.
- **Distinct from PPLNS-JD** (`demand-open-source/share-accounting-ext`, extension type 32): Blitzpool ships JDP support and pays in-coinbase, but the README describes **no miner-verifiable share-accounting extension** — no window/slice/merkle spot-check protocol, and no fee-attribution to the miner who selected the transactions. Its non-custody guarantee comes from *coinbase destination*, whereas PPLNS-JD's comes from *auditable accounting*. Worth compiling as a contrasting point in the payout design space.
