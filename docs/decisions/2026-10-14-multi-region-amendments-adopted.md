# 2026-10-14 — Adopt the MULTI-REGION §10 deltas into SPEC and ARCHITECTURE

**Trigger.** `docs/MULTI-REGION.md` proposed amendments to SPEC/ARCHITECTURE (its §10) that conflicted with the spec as written — most importantly SPEC's Redis snapshot keyed by fingerprint with `snapshot_ttl_secs = 1200`, which expires exactly when `payout` is down or partitioned, leaving Ark destinations unrecoverable after `a_send` erasure (SPEC §2.2). The 2026-10-14 implementation-readiness review (`docs/reviews/2026-10-14-implementation-readiness-review.md`, B1, H1, M1, M5) required a controlling decision.

## Decision

Adopt MULTI-REGION §10 deltas **1, 2, 4, 5, 9, 10 and 11** into SPEC.md and ARCHITECTURE.md (numbering as in MULTI-REGION §10):

1. **Repeat alarm per front** (§2.2): `ser_x(A_send)` repeat is alarmed within a front's own sequence; cross-front repeats are detected retrospectively from persisted `a_send_x`.
2. **Per-front `distribution_id`** (§3.5): `id = front_id << 56 | local_seq`, `local_seq = max(last_local_seq + 1, unix_time_ms)`. The grace rule no longer uses `id − 1` arithmetic: the JDS accepts the immediately preceding id **from the same front on the same connection**, same height only; ids are never compared across fronts.
3. **High-water vector** (§4.3/§5.1): *applied 2026-10-15, see below.*
4. **Streams** (§3.3, §5.2, §7.4): the `spc:blocks` event carries `entries`; `spc:snapshot:<fingerprint>` and `snapshot_ttl_secs` are deleted; `spc:arkreg` and `spc:front:events` are added; `spc:dist:current` is intra-`front` state. Trim discipline: `XTRIM MINID` behind the slowest **live** reader — the payout group's ack and every front's heartbeat cursor (TTL 120 s) — never `MAXLEN`, 600 s floor, 1M-entry backstop alarm, re-bootstrap for a front behind the horizon. A found block whose event never arrives is caught by chain reconciliation and booked `pending` with an alarm.
5. **`front` RAM** (§5.3): live slice tail, `ark_delivery` cache fed by `spc:front:events`, `provisional_paids`, bounded outbox; readiness requires bootstrap-RPC completion.
9. **Topology** (§9): `front` is "one per region; RAM-only state — no durable state of its own" (replaces "stateless except RAM"); `front_id` lease via Redis `SET NX`.
10. **Config** (§6.3): `[pool] front_id`, `outbox_max`, `tip_divergence_secs`; `[storage] front_cursor_heartbeat_secs`, `front_cursor_ttl_secs` (replacing `snapshot_ttl_secs`).
11. **Monitoring** (§9): `front_id` label on `spc_a_send_repeat_total`; new outbox, stream-lag, tip-divergence, registration-lag and `spc_orphaned_settlement_total` metrics; `spc_snapshot_lookup_total` removed.

**Deferred from this decision, applied 2026-10-15:**
- Delta 3 - **applied 2026-10-15**: high-water vector (`high_water`) in `select_window`, builder output, `distribution_snapshot`, `spc:blocks`/`BlockFound`, settlement (SPEC §4.3, §4.7, §5.1, §5.2, §7.4).
- Delta 6 - **applied 2026-10-15**: committed registration without `dest_id` books to `pending`, never `forfeit` (SPEC §11.1, §4.6 (f)).
- Delta 7 - **applied 2026-10-15**: `ark` records pool outputs by `(txid, vout)` and re-checks the chain at maturity (SPEC §11.3, §11.9).
- Delta 8 - **applied 2026-10-15**: type-32 window served from bootstrap-RPC share blobs + live `spc:slices` tail (SPEC §3.7, §5.3).

## Why

- Deletes the snapshot-TTL failure class: settlement becomes idempotent from a durable event via the `ledger_history` PK.
- Found blocks are durably published *with* their payout list; Ark destinations no longer depend on a 1200 s key.
- Fixes the grace rule, which cannot work with time-seeded ids (the predecessor is never `id − 1`).
- Fixes the trim race: fronts tail streams with RAM-only cursors, so a group-ack-only trim could discard slices a slow front had not read.

## Consequences

- One central `payout`, N regional `front`s (MULTI-REGION topology); ids are per front and consumers compare them only on one connection.
- Front readiness requires the bootstrap RPC; a front behind the trim horizon MUST re-bootstrap. Byte formats: `docs/WIRE-FORMATS.md`.
- Attribution data (`slice_credit`-equivalents in `spc:slices`, `entries`) is readable by every front by design; single-region is the fallback if that is unacceptable (MULTI-REGION §6 deployment note).

## Edits applied

- `docs/SPEC.md`: §2.2 (alarm scope, entries not snapshot), §3.3 steps 4–5, §3.5 (publication bullet, `distribution_id`, grace window), §5.1 (`slice_id` comment), §5.2 (stream table, delivery semantics and trim discipline), §5.3, §6.3, §7.4, §8.3 (grace test), §9 (topology, monitoring), changelog entry 2026-10-14 (c).
- `docs/ARCHITECTURE.md`: §5.2 stream table and trim note, §9 coupling points, found-block failure model.
- `docs/MULTI-REGION.md`: header precedence note, §6 security-boundary wording (attribution is front-readable by design), §8b trim rule, §10 status line.
- `docs/ROADMAP.md`: Decision 2 consequence, Phase 3 found-block task, Phase 8 metrics.
