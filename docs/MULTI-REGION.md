# Multi-Region Deployment

Date: 2026-10-14 · Status: design note (reviewed). Grounds: `docs/SPEC.md` (v0.1 draft), `docs/ARCHITECTURE.md`, decision `2026-10-13-jds-in-pool-binary-jd-always-on.md` (multi-region caveat), ROADMAP Decision 6, decision `2026-10-14-multi-region-amendments-adopted.md`. The §10 deltas were adopted into SPEC/ARCHITECTURE on 2026-10-14 except 3, 6, 7 and 8 (see §10); where this file and the spec disagree, the spec wins.

Reviewed by two independent models after drafting; the load-bearing changes from review: the pool-wide window is *designed* (§1), not assumed; block-found events carry their snapshot (§7); registrations settle to `pending`, never forfeit, on pool-side lag (§6); one broadcast stream replaces bespoke fan-out (§6, §8); ids are time-seeded and leased (§4).

## 0. Reference topology

```
 REGION A                              CENTRAL (primary)                     REGION B
┌───────────────────┐   XADD: slices/blocks(+entries)/stats/arkreg  ┌───────────────────┐
│ front (incl.      │ ════════════════════════════════════════════▶ │ front (incl.      │
│  spc-jds module)  │ ◀════════ XREAD: slices (live tail, §1),      │  spc-jds module)  │
│ a_send_A[H]       │            spc:front:events (§6)              │ a_send_B[H]       │
│ bitcoin-core (TDP │   ┌──────────────── CENTRAL ──────────────┐   │ bitcoin-core (TDP │
│  or sv2-tp, local)│   │ payout (sole Postgres writer)         │   │  or sv2-tp, local)│
└───────────────────┘   │ ark (own host) · bitcoin-core          │   └───────────────────┘
       ▲ miners A       │   (chain view for settlement)          │          ▲ miners B
      peered            │ Postgres · Redis (streams)             │        peered
      bitcoinds ◀──────▶└────────────────────────────────────────┘◀──────▶ bitcoinds
                        (all pool bitcoinds addnode-peer with each other; fronts
                         submit found blocks to local AND central node, §2)
```

One central primary site: `payout`, `ark` (own host, SPEC §11.3), Postgres, Redis, one bitcoin-core (chain view for settlement: height, coinbase reads, orphan watch). Two regional front sites: each a `front` role (including the in-process `spc-jds` module, decision 2026-10-13) plus a regional bitcoin-core for fast templates. N `api` replicas wherever (read-only; not discussed further).

No multi-master anything, no consensus, no active-active payout, no Redis cross-region replication. One writer (central `payout`); fronts produce, `payout` consumes, idempotently (SPEC §7.4). This is Blitzpool's Core/Satellite shape stretched over a WAN ([research/raw/repos/2026-07-27-blitzpool-server-rust-github.md](research/raw/repos/2026-07-27-blitzpool-server-rust-github.md)); ckpool's discipline — count on the spot, make durable later, elsewhere — is why the share path can live at the edge ([research/sibling](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md)).

`front` keeps its own region's bitcoin-core so `NewTemplate`/`SetNewPrevHash` never wait on a WAN; the central bitcoin-core is **not** a template source for the regions — it exists so `payout` can check confirmations and read mined coinbases (SPEC §4.6) and so `ark` can watch pool outputs (§11.3). Whether central and regional nodes must agree is §2.

Single-site is `front_id = 0` and needs no other changes.

## 1. The window is pool-wide, and it is designed, not assumed

**Decision (load-bearing; reviewed): there is one pool-wide PPLNS window.** Miners are paid on the pool's global window regardless of which region they hashed through — a pool with regional windows would give small regions worse variance for no operator benefit, and SPEC §3.7's type-32 window, §4.3's `select_window`, and the §11.6 audit are all written against one window anchored at the finding slice. (The rejected alternative, per-region windows, removes §1's feeds but punishes the smaller region's miners; rejected on fairness, not mechanics.)

**Mechanism.** Each front live-tails `spc:slices` with plain `XREAD` (no consumer group — the front's cursor is RAM-only and re-primed at start) so its `window_cache` reflects every region's sealed slices seconds after they seal. From the same stream's slice records each front also learns `sp_id` → `(B_scan, B_spend)` for identities that have never connected to it (needed by §4.7's distribution builder to derive `P_i`), and applies pending deltas from `spc:front:events` (§6) so `owed(id) = raw + pending` is current. A front's distribution (SPEC §4.7) is built from this pool-wide window-in-progress.

**High-water vector.** Every slice XADD returns a central stream entry id, assigned by the single Redis server — a free total order over all slices, immune to regional clock skew. Each distribution snapshot records `{front_id → last slice stream-id included}` (a two-entry vector in practice). At settlement, `payout` reconstructs the paid window as exactly the slices at or below the finding front's high-water at build time, plus the sealed slices the finding block names directly — never a post-hoc re-derivation from Postgres, which may already contain slices the finding front could not have seen. This is what makes the §11.6 audit close to zero and makes §4.3's `select_window` mean the same thing to the builder, the settler, and the auditor. SPEC amendment: §4.3/§5.1 (§10).

**Stale-fork shares.** Slices are keyed by `prev_hash` (§4.2). A slice whose `prev_hash` is not an ancestor of the finding block's `prev_hash` is excluded from that window — it belongs to the fork it built on and is credited if that fork wins instead (both regions' bitcoinds are peered, §2, so forks beyond one block are rare). Shares sealed into an old-`prev_hash` slice *after* the finding block at the next height are ineligible for the finding block's window by the same rule; they are exactly the kind of loss any pool takes at a height boundary, bounded by the sealing latency the high-water vector already records honestly.

## 2. Template supply

**Decision: each region runs its own bitcoin-core, `front` talks TDP/IPC locally.** A remote template source would put WAN round-trips (and the central node as a single point of failure) on the most latency-sensitive path in the system (§9). Regional nodes are ordinary full nodes; the central node is the settlement view.

**Consequence: regions may briefly work on different tips.** Accounting-side this is fine (§1's stale-fork rule). What must **not** happen is a front building on a stale tip for long: fronts should `waitfornewblock`-equivalent aggressively and alarm on template age, same as single-site.

**Self-orphaning is lost revenue, so peer the nodes (config, not mechanism):**
- all pool bitcoinds `addnode`-peer with each other (compact blocks on);
- each front submits a found block to its local node **and** to the central node;
- central `payout` publishes its tip in a heartbeat on `spc:front:events`; fronts alarm when their tip diverges from central's for more than `tip_divergence_secs` (default 30).

`spc:blocks` carries `{block_hash, height, front_id, entries}` (§7). `front_id` costs one byte and removes the need to infer origin from the fingerprint, which fails exactly when the snapshot is lost.

## 3. `a_send` per region

With the JDS in `front` (decision 2026-10-13), each regional front draws its own `a_send[H]` per SPEC §2.2. This is safe by construction: miners scan `A_send` from the coinbase carrier (§2.5), channels are per-front (§3.1), distributions are per-front. Two regions at height `H` simply run two independent instances of the §2.2 schedule; there is no cross-region use of any region's key.

**The §2.2 alarm.** "`front` MUST alarm if `ser_x(A_send)` repeats across heights." Scope it **per front** (`spc_a_send_repeat_total{front_id}`). A cross-front repeat is a 2⁻²⁵⁶ event under honest CSPRNGs — or proof of a shared broken entropy source. That failure is detectable **on-chain for free**: every found block's `a_send_x` is persisted in `distribution_snapshot` (§5.1), so one query — `SELECT a_send_x FROM distribution_snapshot GROUP BY a_send_x HAVING count(*) > 1` — is the entire "central registry" (across heights, as the alarm requires; it covers found blocks only, since unfound snapshots' entries are scrubbed). Do not build a live cross-region key registry: a coordination service to detect a non-event, when the durable record already answers it retrospectively and completely.

## 4. Id spaces: `distribution_id` and `slice_id`

SPEC §3.5 wants `distribution_id` "strictly increasing, pool-wide"; §5.1 wants `slice_id` "monotone, assigned by front". Check what actually consumes monotonicity:

| Consumer | What it needs |
|---|---|
| JDC validation (`distribution_id` TLV, §3.5) | comparison against ids **on its own connection** — the JDC only ever talks to one front's JDS |
| Grace rule (previous id of the same front on the same connection, same height only, §3.5 R4) | predecessor relation **within one publisher's sequence** — looked up from the JDS's own record of what it sent, never computed as `id − 1` |
| `slice` window walk / `slice_height_idx` (§5.1, §4.3) | replaced by the high-water vector + stream order (§1); Postgres ordering only serves audit displays |
| `distribution_snapshot.distribution_id` (§5.1) | nullable audit column; the PK is `fingerprint` |

Nothing compares ids across fronts. So: **`id = front_id << 56 | local_seq`**, with `front_id` a 7-bit tag (the signed `BIGINT` stays positive) and **`local_seq = max(last + 1, unix_time_ms)`** — restart-safe with no reads (a counter from 0 collides after restart; a `max(slice_id)` query misses slices still buffered in Redis), monotone per front, fitting U56 for millennia, and roughly comparable across fronts (NTP assumed, §12). Per front both ids are strictly increasing; globally both are unique. Every consumer above is satisfied; the spec's "pool-wide" phrase gets a one-line amendment (§10). No central sequence service — that would put the control plane on the JD hot path for zero consumer benefit.

## 5. Pool output indexing collision

SPEC §11.3 indexes `pool_payout_descriptor` by block height: `pool_output(H) = descriptor.at_derivation_index(H)`. If two fronts find competing blocks at the same `H`, both coinbases pay `P2TR(pool_output(H))` — the same key.

**Consequences, checked:**
- Only one block confirms. The orphan path (§7.4: `status='orphaned'`, no ledger rows) is unchanged.
- `ark`'s per-height discovery (§11.3) still works: it scans the **confirmed** block at each height. One amendment for honesty: `ark` records pool outputs by **outpoint**, not by height alone, and re-checks the active chain at maturity — a 1-block reorg between the pool's own blocks changes the txid behind the same script.
- Value accounting: `pool_output(H)` receives the winner's `pay_P`. There is exactly one confirmed coinbase at `H`; no double-spend, no double-credit.
- The only true loss case is a >`confirmation_depth` reorg after an Ark send — SPEC §11.3's existing accepted loss, unrelated to region count.

**Decision: accept as-is; do not partition the index space.** A region bit would make `ark`'s discovery scan 2× the script space per height forever, to solve a problem that costs zero sats.

## 6. Converged state: registrations, ledger, blocks — one broadcast stream

**The gap.** Three kinds of pool-side state must reach every front: (a) `ark_delivery` registrations (SPEC §11.4) — region B must know them before building a distribution containing that `sp_id`; (b) ledger effects of settlement (`pending` deltas and the forced `SetPayoutDistribution` republish + grace-slot drop that SPEC §3.5 requires **on ledger apply**); (c) pool block notifications (SPEC §3.7's `NewBlockFound` broadcast; type-32 window service for other fronts' blocks). With one front these are all in-process. Before the 2026-10-14 amendments the spec had no stream for any of them in §5.2.

**One stream, not three (reviewed simplification).** `payout` is the sole publisher of all three, and fronts consume them the same way, so:

```
miner ──SetArkIdentity──▶ front A        (1) validate per §11.4
front A ──XADD spc:arkreg──▶ {sp_id, b_scan, b_spend, ark_id, b_ark,
                              mailbox_mode, M, D, epoch_fallback, sig}   (2)
payout ──XACK──▶ Postgres: upsert sp_identity, then ark_delivery        (3)
payout ──XADD spc:front:events──▶ { ArkRegApplied{sp_id, rev},
                                    LedgerApplied{block_hash, pending_deltas,
                                                  republish},
                                    BlockFound{block_hash, height, front_id,
                                               provisional_paids, high_water},
                                    TipHeartbeat{hash, height} }         (4)
every front ──XREAD──▶ applies to RAM caches; rebuilds distribution when
                       LedgerApplied.republish or a relevant ArkRegApplied lands (5)
```

- **`rev`, not wall clocks.** `payout` assigns a monotone revision per registration in commit order; fronts apply only newer `rev`s. Front clocks never order anything (§12) — a clock-ahead front must not pin an old registration over a rotated `b_ark`.
- **`b_scan`/`b_spend` ride `spc:arkreg`** so `payout` can upsert `sp_identity` before `ark_delivery` — the FK would otherwise race the per-minute `spc:stats`.
- **Forfeit race — removed, not priced (reviewed).** The honest loss window is not the agent's own connection (it re-registers and never mines before `Success`, §11.5); it is a *second* front building the winning distribution before the fan-out lands. Fix at settlement, not on the wire: when `payout` settles a snapshot that has no `dest_id` for an `sp_id` whose registration **was already committed in Postgres**, it books that id to `pending` — a pool-side failure, exactly like a failed Ark send (§11.1) — never to `forfeit`. This matches invariant §4.6(f) (forfeit only if *no* `ark_delivery` exists) and demotes `spc:arkreg` from correctness mechanism to freshness optimisation. Loss bound: none; cost bound: one block's wait.
- **In-flight `pending` across regions (O11-adjacent).** `owed = raw + pending`, and `pending` only moves at booking, `confirmation_depth` blocks later — so front B, finding a block 2 heights after front A paid `pending(id)` in A's unbooked coinbase, would pay it again. Single-site can track this locally; multi-region cannot. Fix: **`provisional_paids`** (found-time paid amounts per `sp_id`, announced on `BlockFound` (decided 2026-10-15; `LedgerApplied` reverses them on orphan) from the winning snapshot's entries — already public to every JDC in JD mode), and every front subtracts announced-but-unbooked paids from `owed`. Reconciled at booking per SPEC §4.6's mined-coinbase rule and its revenue-scaled claim (O11, resolved 2026-10-14).
- **Front startup / crash recovery (O-MR1, resolved).** Bootstrap RPC on `payout` (not a read-only PG role): returns a consistent snapshot `{ark_delivery@rev, pending_balance@rev, window tail, type-32 share blobs}` plus the `rev`/stream cursors to resume from — no attribution data (`slice_credit`, snapshot `entries`) leaves the primary, and there is no gap between the read and the subscription. A front is **not ready** (no channel opens) until bootstrap completes. Wire-level format of the RPC defined in `docs/WIRE-FORMATS.md` §3.3.
- **Redis over the WAN is a security boundary, and the trust boundary is the pool-wide attribution data.** TLS plus per-front ACLs: a front may XADD only `spc:slices`/`spc:blocks`/`spc:stats`/`spc:arkreg` and XREAD only `spc:slices`/`spc:front:events`; **never** `spc:ark:*` (§11.3's ACL already keeps `ark` to its two streams). The ACLs protect the float and Postgres: a compromised front cannot touch the float or read Postgres. It **can** read attribution data, by design — fronts learn `sp_id → (B_scan, B_spend)` and per-identity aggregates from `spc:slices` (§1; SPEC §5.2), which is the share→identity link that SPEC §5.1 scrubs. Attribution is therefore pool-wide-readable by every front.
  - **Deployment note.** If exposing pool-wide attribution to regional front hosts is unacceptable, run single-region (front co-located with `payout`). Multi-region inherently widens the attribution trust boundary.
- `RegisterEpochMailboxes` rides `spc:arkreg` the same way; its own spec already tolerates timing (MAILBOX.md pre-registration window).

## 7. Block-found races, snapshots, orphans

Two fronts finding blocks at the same height is a normal Bitcoin race (made likely by design, not freak — §2's peering keeps it rare), and §7.4 already contains it: `block_found` PK is `block_hash`; the loser writes `status='orphaned'` and no ledger rows; ledger idempotency is keyed `(block_hash, sp_id)`; the losing block's slices age out under `retain_windows`/scrub like any non-winning slice.

**Snapshots move into `spc:blocks` (reviewed simplification; deletes a failure class).** The found-block event becomes `{block_hash, height, front_id, entries}` where `entries` is the winning distribution's full entry list — which the finding front already holds in RAM for the jobs it issued. Then:

- settlement never depends on the 1200 s `snapshot_ttl_secs`, which previously expired under exactly the conditions that matter (payout down or partitioned for > 20 min) with Ark destinations unrecoverable after `a_send` erasure (§2.2);
- the `spc:snapshot:<fingerprint>` Redis keys are deleted from the design (pool-template and JD both ride the event; JD's published distributions remain available from the JDS itself for grace validation);
- the vouching invariant gets stronger: the payout list is durably published *with* the block, not merely referenced by it. A block event that cannot be XADDed sits in the front's outbox (§8a) and alarms at 60 s.

The fingerprint stays in the event as the jobs' audit handle, but nothing is *keyed* on it for correctness anymore.

## 8. Failure modes

**(a) Region loses central Redis/Postgres, keeps mining.** Streams live on the central server — there is no such thing as "Redis's buffer" on the partitioned side. So: shares, jobs, JD continue locally; sealed slices, block events, arkreg writes accumulate in a **bounded in-RAM outbox** (public accounting records only; never `a_send`, never tweaks). On heal, the outbox replays in order into the normal XADD path. Outbox exceeds `outbox_max` (sized for ~10 min of slices + a handful of block events): the front **fails closed** — stops accepting shares, disconnects miners so they fail over to the healthy region (§8c), and pages. This is simpler and more honest than "hours of buffer": a region that cannot publish accounting for > 10 minutes is not a region, it is a liability. The one nuance worth the bytes: a **won block during the partition pays miners on-chain regardless** (the coinbase *is* the payout in pool-template mode); what waits is the ledger entry and the Ark sends, and the block event is the last thing the outbox is allowed to drop. Chain-side, `payout`'s central node sees any pool coinbase on-chain and reconciles it against `block_found` — an unmatched pool coinbase is itself the missed-settlement page (O-MR3's metric: `spc_orphaned_settlement_total{reason=missing_block_event}`; orphan *rate*, not `> 0`, pages on `spc_blocks_total{status=orphaned}` since same-height self-races are now routine).

**(b) `payout` down.** Producers keep XADDing; Redis buffers centrally. **No `MAXLEN` on `spc:slices`/`spc:blocks`/`spc:arkreg`** — maxlen trims unread entries whether or not any reader has consumed them; trim only with `XTRIM MINID` behind the slowest live reader (the consumer group's ack **and**, for `spc:slices`/`spc:front:events`, every front's heartbeat cursor), per the discipline in SPEC §5.2 (600 s floor, 1M-entry backstop alarm); alert on Redis memory and pending-entry age, and fail closed (stop the world, page) at a configured retention budget. On recovery: commit-before-ack, `ledger_history` PK makes re-application a no-op (§7.4), abandoned consumer-group pending entries are reclaimed (`XAUTOCLAIM`) before new processing. Ark sends queue as `scheduled` rows; `due_height` is height-based, so the queue just flushes. Settlement latency grows; nothing is lost.

**(c) Whole region down.** Miners reconnect to the surviving region (failover list in `spc-agent [upstream]` — sticky primary, ordered fallbacks; out of scope here). What changes for them: new `a_send` (irrelevant — it rides the coinbase), same `sp_id` accounting (global at `payout`), one `SetArkIdentity` on the new connection exactly like any connect (the survivor already knows them if the registration ever committed anywhere). Stale-job shares from the dead region are rejected (§3.4), costing a few shares, as in any reconnect.

**(d) Split-brain.** Impossible by construction: only `payout` writes Postgres, only `ark` spends, both exist exactly once, centrally. Two fronts can never disagree about the ledger because neither can write it. The worst "brain split" available is two fronts mining different tips — §1/§2 show that is accounting-neutral.

## 9. Latency budget

| Flow | Budget | Path |
|---|---|---|
| miner → front share submit | **ms, latency-critical** | regional |
| bitcoind → front `NewTemplate`/`SetNewPrevHash` | **ms–s, latency-critical** | regional (local node) |
| front → miner job / `SetPayoutDistribution` push | **ms, latency-critical** | regional |
| `spc:slices` front → payout **and** → other fronts' window tail | seconds | WAN, buffered |
| `spc:blocks(+entries)` front → payout | seconds (outbox otherwise, §8a) | WAN |
| `spc:arkreg` front → payout → `spc:front:events` → fronts | seconds | WAN |
| ark orders / receipts | 6 blocks of slack | WAN |

Nothing latency-critical crosses a region boundary; everything that crosses regions has seconds-to-blocks of slack and a stream buffer in front of it. That is the entire justification for this topology — and the reason a remote-template or shared-`a_send` design was rejected without further analysis.

## 10. Spec deltas (2026-10-14)

Status: deltas **1, 2, 4, 5, 9, 10, 11 are applied** to SPEC.md/ARCHITECTURE.md (decision `2026-10-14-multi-region-amendments-adopted.md`). Deltas **3, 6, 7 and 8 are applied 2026-10-15** to SPEC.md (this decision).

1. **§2.2** — scope the repeat alarm per front: "MUST alarm if `ser_x(A_send)` repeats across heights **within its own sequence**"; cross-front repeats are detected retrospectively from persisted `a_send_x` (`GROUP BY a_send_x HAVING count(*) > 1`, §3 above). `front_id` label on `spc_a_send_repeat_total`.
2. **§3.5** — "`distribution_id: U64 strictly increasing **per `front`**; wire value = `front_id << 56 | local_seq`, `local_seq = max(last+1, unix_ms)`" (replaces "pool-wide").
3. **§4.3 / §5.1** — distribution snapshots record the high-water vector `{front_id → last slice stream-id}`; settlement and the type-32 window reconstruct exactly the slices at or below it; fork-ineligibility per §1's stale-fork rule. `slice_id` partitioned per delta 2's scheme.
4. **§5.2** — add `spc:arkreg` (front → payout) and `spc:front:events` (payout → all fronts; `ArkRegApplied` / `LedgerApplied` / `BlockFound` / `TipHeartbeat`); `spc:blocks` becomes `{block_hash, height, front_id, fingerprint, entries}`; **delete** `spc:snapshot:<fingerprint>`; trim discipline = `XTRIM MINID` behind the slowest live reader (group ack and front cursor heartbeats), never `MAXLEN` (SPEC §5.2).
5. **§5.3** — front RAM gains: live slice tail (§1), `ark_delivery` cache fed by `spc:front:events` (§6), provisional-paid deductions (§6), bounded outbox (§8a). Front readiness requires bootstrap-RPC completion (§6).
6. **§11.1 / §4.6** — settlement rule: committed registration with no `dest_id` in the settled snapshot books to `pending`, never `forfeit` (§6).
7. **§11.3** — `ark` records pool outputs by outpoint and re-checks the active chain at maturity (§5).
8. **§3.7** — type-32 window/`GetShares` service for other fronts' blocks is served from Postgres via the bootstrap RPC's read path (§6).
9. **§9 Deployment** — the §0 topology: one central primary, N regional fronts each with local bitcoind; all bitcoinds peered; found blocks submitted to local + central; `front_id` config under `[pool]`; **fronts claim `front_id` at start via Redis `SET NX` lease + heartbeat holding an instance UUID, refusing to start if held** (a duplicate `front_id` silently collides `slice_id`s — blue/green deploys take a second id, then release).
10. **§6.3** — `[pool] front_id = 0` default; `outbox_max`, `tip_divergence_secs`, Redis TLS/ACL endpoints.
11. **§9 monitoring** — every front metric gains `front_id`; new: outbox depth/age, per-stream lag and pending-entry age, tip divergence, self-orphan rate, registration lag (`rev` applied vs committed), `spc_orphaned_settlement_total`.

## 11. Open items

- **O-MR1 — resolved** (§6): bootstrap RPC on `payout`, no edge PG credentials, consistent `rev`-cursor handoff, readiness gate.
- **O-MR2 — resolved** (§6): one broadcast stream from the sole writer; no bespoke fan-out choice remains.
- **O-MR3 — resolved** (§8a): missing-block-event detection is the central chain reconciliation (`spc_orphaned_settlement_total`); the block event is the outbox's last-droppable record.
- **O-MR4 — closed.** A miner hashing on two fronts appears legitimately in both candidate same-height distributions; each candidate pays the pool-wide window and only one block confirms, so there is no double payment and no double-counted weight (an output in an orphaned coinbase is not a payment). No overlap alert, no cross-region distribution gossip. The genuinely dangerous adjacent case — two fronts both paying the same *in-flight pending* — is handled by `provisional_paids` (§6), with the claim defined by SPEC §4.6 (O11, resolved 2026-10-14).
- **O-MR5 — N > 2 regions.** Nothing in this note is binary-specific (`front_id` has 7 bits; the high-water vector grows linearly; broadcast fan-out is N XREADs). Called out only so the 2-region defaults (`front_id ∈ {0,1}`) are not baked into schemas.
- **O-MR6 — clock discipline.** NTP required on all pool hosts; nothing orders by front wall clock (ids are time-seeded but tie-broken by stream order; `ntime` validation tolerance is unchanged). One line for the ops runbook, no mechanism.

## Recommendation

Deploy as one central primary site (`payout` + `ark` on its own host + Postgres + Redis + central bitcoin-core) and two regional fronts, each with a local bitcoin-core for templates and all bitcoinds peered. The window is pool-wide: fronts tail `spc:slices` live and stamp every distribution with a high-water vector, so settlement and the type-32 audit see exactly the window the builder saw. `a_send` is per-front per-height; ids carry a leased 7-bit `front_id` tag and time-seeded sequences; found blocks carry their own payout entries, deleting the snapshot-TTL failure class; registrations and ledger effects reach fronts over one broadcast stream, with settlement routing pool-side registration lag to `pending` rather than `forfeit`. Nothing latency-critical crosses regions, only `payout` ever writes, and every cross-region flow is buffered, idempotent, and keyed by content — which is why this topology needs no consensus, no multi-master, and no shared secrets beyond what single-site already has.
