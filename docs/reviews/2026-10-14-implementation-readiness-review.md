# Implementation-Readiness Review — 2026-10-14

**Scope:** full repo audit for handoff to an implementation agent swarm: `DESIGN.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `docs/MULTI-REGION.md`, `docs/ROADMAP.md`, `MAILBOX.md`, `RESULTS.md`, `UPSTREAM_DEPS.md`, `docs/decisions/*`, `docs/research/*`, and the three prototype crates (`spc_ark`, `spc_ark_client`, `spc_ark_mailbox`).

**Method:** four parallel extraction/analysis passes (core docs; architecture/roadmap/deps; code↔doc consistency; research tree), followed by an independent validation pass that confirmed/refuted each headline claim against the working tree with file:line evidence. Findings below are marked **[validated]** (confirmed with verbatim quotes by the second pass) or **[single-source]** (one analyst; spot-check before acting).

**Repo hygiene note:** the reviewed state includes uncommitted work — modified `README.md`, `UPSTREAM_DEPS.md`, `docs/ROADMAP.md`, `docs/SPEC.md` and untracked `docs/ARCHITECTURE.md`, `docs/MULTI-REGION.md`, `docs/decisions/2026-10-13-jds-in-pool-binary-jd-always-on.md`. The working tree is self-consistent; commit before handoff. No `AGENTS.md`/steering files exist in the repo.

---

## Verdict

**Conditionally ready.** Phases 0–3 (crypto, accounting, pool-template mode) are implementable as-is: normative docs are internally consistent, the prototype validates the crypto/mailbox/claim math with frozen vectors, and dependencies are pinned. **The handoff is blocked for Phases 4+** by one unresolved spec fork (MULTI-REGION's proposed amendments), one undecided accounting formula (O11), and missing byte-level wire formats for the multi-region streams.

Headline counts: **3 blockers, 4 high, 6 medium**, plus minor staleness.

---

## Blockers (resolve before swarm handoff)

### B1. Found-block snapshot durability: two conflicting designs, no controlling decision — [validated]

- SPEC freezes the winning distribution as a Redis snapshot keyed by fingerprint with `snapshot_ttl_secs = 1200` (`docs/SPEC.md:268`, `:562`, `:655`), and books a block whose snapshot expired as `status='pending'` (`SPEC.md:709`). ARCHITECTURE repeats this (`docs/ARCHITECTURE.md:125`, `:274`, `:374`).
- MULTI-REGION deletes the mechanism: "the `spc:snapshot:<fingerprint>` Redis keys are deleted from the design" (`docs/MULTI-REGION.md:122`), moving the entries into the `spc:blocks` event payload (`:119`), explicitly to kill the failure class where the 1200 s TTL expires during a payout partition and Ark destinations become unrecoverable (`:121`).
- MULTI-REGION defers to SPEC by its own header ("Where this file and the spec disagree, **the spec wins** until the §10 amendments land", `:3`; §10 deltas are "proposed amendments — not applied here", `:151`). The amendments have **not** been applied to SPEC/ARCHITECTURE, and no decision record settles which design holds. This is the durability story for the entire payout pipeline, so implementers of `front` and `payout` would build opposite storage designs depending on which doc they open first.
- **Fix:** write a decision record adopting (or rejecting) MULTI-REGION §10 deltas 3–4, then apply them to SPEC §3.3/§5.2/§6.3 and ARCHITECTURE §5.2/§7.1. Note `ROADMAP.md:98` already reads as if the MR design won ("Block-found events carry the frozen distribution snapshot").

### B2. O11 — JD-mode booking formula undecided; ROADMAP doesn't flag it — [validated]

- `docs/SPEC.md:787` (O11): `pending'(id) = claim(id) − paid(id)` "needs a definition of `claim`" — revenue-scaled `owed(id)·(T_actual−F)/W` vs fixed-sat signed ledger `raw(id)·(T_actual−F)/W + pending(id)` "can go negative". Marked "Decide before Phase 4"; also flagged at `SPEC.md:427`, `ARCHITECTURE.md:300`, `:378`, `:394`, `MULTI-REGION.md:110`, `:170`.
- **But `ROADMAP.md` contains no mention of O11 or `claim(` anywhere** — Phase 4 (`:166-175`) and Phase 6 (`:189-`) proceed as if unblocked. A swarm executing the roadmap literally would hit the undefined formula mid-Phase-4.
- **Fix:** decide the formula (it changes ledger semantics: negative balances possible or not), record it in `docs/decisions/`, and add the decision as an explicit Phase-4 entry criterion in ROADMAP.

### B3. PR #203 stack: Phase 4 hard-depends on unmerged, unreviewed upstream PRs — [validated]

- The JD extension is pinned to an **unmerged spec PR** (`stratum-mining/sv2-spec` PR #203 text frozen at `47dbedd`, `UPSTREAM_DEPS.md:7`) plus three **open, CI-less fork PRs**: `warioishere/stratum@9be4023` (PR #1), `warioishere/sv2-apps@90b3e3d` (PR #3) which still lacks two required fixes living only in PRs #1/#2 of the same repo (`UPSTREAM_DEPS.md:9-12`). Mainline `sv2-apps` (both `main@56a916ac` and `v0.8.0@7f490743`) is explicitly **excluded** as broken (`:13`).
- SPEC itself flags the residual drift risk: O10 (`SPEC.md:786`) lists two unanswered review-thread items on PR #203.
- **Fix:** this is an accepted-risk item, but ROADMAP should state the contingency explicitly (rebase policy if upstream merges/changes; who owns the fork chain). At minimum, add a Phase-4 entry criterion: "fork pins re-verified against upstream HEAD."

---

## High severity

### H1. `distribution_id` grace rule breaks under MULTI-REGION's id scheme — [validated]

- SPEC: "`distribution_id`: U64 strictly increasing, pool-wide" (`SPEC.md:296`) with same-height grace defined as accepting `distribution_id − 1` (`SPEC.md:292`, `:308`).
- MULTI-REGION replaces the id: "`id = front_id << 56 | local_seq`" with `local_seq = max(last+1, unix_ms)` (`MULTI-REGION.md:73`, delta at `:154` "(replaces 'pool-wide')"). Because `local_seq` jumps by wall-clock gaps, **the predecessor is never `id − 1`**, and MR §10 does **not** amend the `−1` wording. An implementer following SPEC literally rejects every valid grace declaration; one following MR can't compute grace at all.
- **Fix:** when B1's decision lands, rewrite SPEC §3.5 R4 grace in terms of "predecessor in this publisher's sequence" per MR `:69`, or keep pool-wide ids.

### H2. MULTI-REGION's security claim conflicts with its own data flow — [validated]

- `MULTI-REGION.md:112` claims a compromised front "can neither read attribution data nor touch the float."
- But every front live-tails `spc:slices` (`:37`), which carries per-identity aggregates (`SPEC.md:557`, `ARCHITECTURE.md:180`), and "from the same stream's slice records each front also learns `sp_id` → `(B_scan, B_spend)`" (`MULTI-REGION.md:37`). That *is* the attribution data — the share→identity link SPEC §5.1/§6 treats as the scrub target. `:111` repeats the claim.
- **Fix:** either reword the security boundary (attribution is pool-wide-readable by fronts by design) or redesign the slice stream to strip identity bindings (which conflicts with §4.7's per-front distribution builder). Decision needed before multi-region work.

### H3. Byte-level wire formats missing for multi-region streams — [validated, tempered]

- `spc:ark:orders` / `spc:ark:receipts` exist as struct sketches (`SPEC.md:564-565`, `:858-859`, `ARCHITECTURE.md:185-186`) but no field-level encoding (endpoints, `ark_address` string vs bytes, `vtxo_id` format, sig algorithm/encoding).
- `spc:front:events` (`{ArkRegApplied, LedgerApplied, BlockFound, TipHeartbeat}`) has **no** struct definitions anywhere (`MULTI-REGION.md:98-103` is a diagram), nor does the bootstrap RPC (`:111`) fronts must complete before becoming ready (`:157`), nor `spc:arkreg`.
- **Fix:** add an appendix (or new doc) with field-level wire schemas for all Redis stream payloads and the bootstrap RPC before Phase 3 multi-front work or Phase 7 Ark work. Single-region phases can proceed without it.

### H4. Upstream pins: no verification mechanism — [single-source]

- Six commits are pinned across `UPSTREAM_DEPS.md` and `docs/decisions/2026-10-12-pr203-reference-implementation.md`, but nothing describes how a build verifies the checkout matches the pin, and no policy for re-pinning when upstream moves. A swarm will `git clone` whatever is convenient.
- **Fix:** add expected-commit assertions to build scripts / a `UPSTREAM_DEPS.md` "verify" column, and state the re-pin policy.

---

## Medium severity

### M1. Stale line: `front` "stateless except RAM" — [validated]
`SPEC.md:761` still says production fronts are "stateless except RAM", contradicting `SPEC.md:153` ("`front` is then not fully stateless" — per-height `a_send` in-process), `ARCHITECTURE.md:367-368`, and the 2026-10-13 JDS decision's own multi-region caveat. One-line fix; do it with B1's edit pass.

### M2. O14 (default mailbox mode) unresolved; docs point different ways — [validated as open]
v0.1 default is `static` per DESIGN.md, but `MAILBOX.md` recommends epoch-derived (variant B), which itself depends on O13's bark patch. The choice changes what Phase 7 builds first. Decide and record.

### M3. Ark order-signing key provenance unspecified — [single-source]
`SPEC.md:862-863` requires "a dedicated order-signing key (not the `ark` seed, not any SPc key)" and `SPEC.md:964` gives `order_key_path = "/run/secrets/spc/ark-order-key"`, but no derivation/import/generation procedure, format, or rotation policy. Half-page fix in SPEC §11.

### M4. Ark float edge cases unspecified — [single-source]
Reorg invalidating scheduled sends after `float` boarding, `float_target_sats = 0` (REQUIRED default) with no operator guidance, surplus policy, and `failed → pending_balance` recovery beyond `retry_blocks` (`SPEC.md:967-968`) are all absent. Mostly operator-policy, but the reorg→float-accounting interaction is protocol-level and should be pinned.

### M5. XTRIM-vs-front-cursor race — [validated]
`MULTI-REGION.md:131` trims `spc:slices` behind the **payout consumer group's** ack, but fronts tail the same stream with plain RAM-only `XREAD` cursors (`:37`). A slow/restarting front can permanently lose sealed slices it never read. Needs a trim rule that accounts for front cursors (or fronts get consumer groups too).

### M6. Acceptance criteria gaps in ROADMAP — [single-source]
Phases 3–8 have e2e goals but miss spec-mandated checks, e.g.: fail-closed distribution parsing (`SPEC §3.1`), `spc-scan` finding outputs via OP_RETURN carrier in JD mode (Phase 4), §11.9 conservation identity test (Phase 7 explicitly promises it but no harness note), schema test that `spc:ark:orders` carries no identity fields (`SPEC.md:950`). Add per-phase checklists keyed to SPEC sections.

---

## Validated as OK (no action)

- **JDS-in-pool-binary decision (2026-10-13) is fully synced.** No leftover `bins/spc-jds`, standalone-binary, or `[jd] enabled` references in working-tree SPEC/ROADMAP/README/UPSTREAM_DEPS; only M1's one stale line remains. [validated]
- **`spc_ark_client/test/_bitcoind_snapshot/` is NOT committed.** It's gitignored (`.gitignore:4`, with an explanatory comment), runtime-generated by `ark-testing`'s `TestContext` (`e2e.rs:12,25`; `run_e2e.sh:26` sets `TEST_DIRECTORY`). An earlier draft of this review suspected committed chain data — refuted. [validated]
- **`PAYOUT_DELAY = 6` is consistent everywhere** (SPEC.md:794, ROADMAP.md:206, `spc_ark_mailbox/src/lib.rs:361`); remaining "100-block" references are genuine coinbase maturity (float boarding), which is correct. RESULTS.md is not stale on this point. [validated]
- **Prototype↔docs fidelity is high.** Constants (`RETRY_OFFSETS`, `READ_AUTH_VALIDITY_SECS=60`, `MAX_READ_PAGES=10`, mailbox blinding, auth token format) match MAILBOX.md exactly; all bark/ark deps pinned to `c419159` / ark-lib 0.7.1 consistently across three crates; e2e is reproducible by an agent given `BARK_CHECKOUT` + `BITCOIND_EXEC`. [validated]
- **Research tree is mostly healthy.** All SPEC citations resolve; the changelog honestly records the 2026-10-13 supersessions (BIP0352-tag reuse → fresh `SPc/*` tags; 500k-sat band → dust threshold; 100-block → 6-block send schedule). [validated]
- **Precedence is declared, narrowly:** both ARCHITECTURE (`:6-7`) and MULTI-REGION (`:3`) say "the spec wins" on disagreement. The hole is that SPEC never acknowledges MULTI-REGION's *proposed amendments*, so B1/H1 sit in limbo rather than being contradictions per se. [validated]

## Low severity / hygiene

- **L1.** `docs/research/output/plan-xpub-miner-identity-spec-2026-07-29.md` (1,264 lines, the superseded xpub plan) has **no "superseded" banner**, and `docs/research/README.md:8` gives no status. It is still legitimately cited for its fail-closed finding (`SPEC.md:242`, `:671`; `ROADMAP.md:61` documents the rejection), so don't delete — but add a one-line banner to prevent an agent from implementing the old plan. [validated]
- **L2.** `README.md:5` ("design phase. No pool code yet") is technically true (no `spc-pool` exists) but sits oddly next to `RESULTS.md`'s validated prototype; consider one clarifying sentence so swarm agents don't think the prototype crates are off-limits reference material — ARCHITECTURE.md:82-84 explicitly absorbs them. [single-source]
- **L3.** Uncommitted working-tree state (see header). Commit before handoff so swarm agents get a clean, citable baseline. [validated]

---

## Recommended action order before handoff

1. **Decision record: MULTI-REGION §10 deltas** (adopt/reject each of the 5 amendments) → apply to SPEC/ARCHITECTURE. Resolves B1, H1, M1, M5.
2. **Decision record: O11 `claim()` formula** → add as ROADMAP Phase-4 entry criterion. Resolves B2.
3. **Decision record: O14 mailbox default** (and confirm O13 bark patch scope as Phase-7 entry criterion). Resolves M2.
4. **Wire-format appendix** for `spc:front:events`, `spc:arkreg`, `spc:ark:orders/receipts`, bootstrap RPC. Resolves H3 (and most of the "missing schema" class).
5. **SPEC §11 addendum**: order-signing-key provenance + float reorg accounting. Resolves M3, M4.
6. **ROADMAP**: add fork-pin re-verification criterion (B3), per-phase acceptance checklists keyed to SPEC (M6).
7. **Hygiene pass**: banner on the superseded xpub plan (L1), README clarification (L2), commit the working tree (L3), consider adding an `AGENTS.md` so the swarm has a sanctioned entry point (none exists today).

Items 1–3 are decisions only you can make; 4–7 are mechanical once decided. Estimated effort after decisions: ~1–2 days of doc editing.

---

## Resolutions (2026-10-15)

Final-review pass over the doc change set. All edits are uncommitted (L3). Decision records are in `docs/decisions/`.

- **B1** — `2026-10-14-multi-region-amendments-adopted.md`: found-block event carries `entries`, Redis snapshot/TTL deleted. SPEC §2.2, §3.3, §5.2, §5.3, §6.3, §7.4, §9; ARCHITECTURE §5.2/§9; ROADMAP Phase 3/8. MULTI-REGION §10 deltas 3, 6, 7, 8 remain unapplied (flagged in MULTI-REGION §10 and the decision record; SPEC wins).
- **B2** — `2026-10-14-o11-claim-formula.md`: revenue-scaled `claim(id) = floor(owed(id)·(T_actual−F)/W)`. SPEC §4.6, §10 O11; ROADMAP Phase 4 entry criterion; analysis in `docs/analysis/` (marked illustrative).
- **B3** — `2026-10-14-upstream-own-suite.md`: upstream forks and PR #203 are reference pins, not gates. UPSTREAM_DEPS framing line; ROADMAP R3 and Phase 4 Dependencies reworded.
- **H1** — `2026-10-14-multi-region-amendments-adopted.md`: per-front `distribution_id = front_id << 56 | local_seq`; grace rule is "previous id from the same front on the same connection, same height". SPEC §3.5, §8.3.
- **H2** — MULTI-REGION §6: security-boundary wording corrected (attribution is front-readable by design) plus a single-region deployment note.
- **H3** — `docs/WIRE-FORMATS.md` (new): `spc:blocks` entries, `spc:front:events`, `spc:arkreg`, `spc:ark:orders`/`receipts`, bootstrap RPC. Linked from SPEC §5.2, ARCHITECTURE §5.2, MULTI-REGION §6, README.
- **H4** — UPSTREAM_DEPS.md "Verification" section (per-entry-type checks and re-pin policy); ROADMAP Phase 4 acceptance item.
- **M1** — SPEC §9 ("RAM-only state — no durable state of its own"); SPEC §2.2 and ARCHITECTURE §9 reworded; no "stateless except RAM" remains.
- **M2** — `2026-10-14-mailbox-default-epoch.md`: default `epoch_derived` (variant B). SPEC §10 O14, §11.4, §11.5, §11.8, §11.10; MAILBOX.md note; DESIGN.md annotated.
- **M3** — `docs/research/output/ark-order-key-spec-recommendation.md` (input, with status banner): BIP340 key, file perms, `[ark] order_pubkey`, rotation. SPEC §11.3, §11.10; message format in WIRE-FORMATS §3.4.
- **M4** — SPEC §11.3 "Reorg and float edge cases": the 6-block rule is normative; deeper reorgs, float surplus and `float_target_sats` sizing are operator policy with defaults.
- **M5** — SPEC §5.2 trim discipline: `XTRIM MINID` behind the slowest live reader (group ack and front cursor heartbeats, TTL 120 s), 600 s floor, 1M backstop alarm, re-bootstrap. SPEC §6.3 `front_cursor_*`; MULTI-REGION §8b aligned.
- **M6** — ROADMAP Phases 3–8: "Acceptance (keyed to SPEC)" checklists.
- **L1** — superseded banner on `plan-xpub-miner-identity-spec-2026-07-29.md`; note in `docs/research/README.md`.
- **L2** — README status paragraph: prototype crates are reference material absorbed into `spc-crypto` / `spc-agent`; README document list extended.
- **L3** — **commit pending** (working tree intentionally left uncommitted).

Residual items for a human (all four resolved later on 2026-10-15, see the lines below): MULTI-REGION §10 deltas 3, 6, 7, 8 (not yet in SPEC); `provisional_paids` placement (`BlockFound` vs `LedgerApplied`, WIRE-FORMATS §3.2); the `hash_SPc/ArkSendOrder` domain tag (WIRE-FORMATS §3.4); `block_found.fingerprint NOT NULL REFERENCES distribution_snapshot` cannot represent the "event never arrived / fingerprint unresolved" case in SPEC §7.4.
- **Resolved (2026-10-15) - MULTI-REGION §10 deltas 3, 6, 7, 8:** applied to SPEC (§4.3/§4.7/§5.1/§5.2/§7.4 `high_water`; §11.1 + §4.6 (f) committed registration -> `pending`; §11.3 + §11.9 `ark` tracks pool outputs by outpoint; §3.7/§5.3 type-32 via bootstrap RPC) and WIRE-FORMATS.
- **Resolved (2026-10-15) - `provisional_paids` placement:** announced on `BlockFound`, not `LedgerApplied` (SPEC §5.2/§5.3, WIRE-FORMATS 0x02/0x03, ARCHITECTURE §5.2); the WIRE-FORMATS "Open" note is deleted.
- **Resolved (2026-10-15) - `hash_SPc/ArkSendOrder` domain tag:** `ArkSendOrder.sig` is BIP340 over `hash_SPc/ArkSendOrder(order_id || ark_address || sats || not_before_height)`, no chain prefix (SPEC §11.3, §11.10; WIRE-FORMATS §3.4).
- **Resolved (2026-10-15) - `block_found.fingerprint`:** nullable, with `CHECK (status <> 'confirmed' OR fingerprint IS NOT NULL)`; NULL while the found-block event is lost, set by operator reconstruction (SPEC §5.1, §7.4, §11.1).
