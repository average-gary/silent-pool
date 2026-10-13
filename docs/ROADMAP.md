> Copied from the research wiki (`bitcoin-mining-payout-schemas/output/plan-spc-pool-pplns-jd-silent-payments-2026-10-09.md`) on 2026-10-12. Cited wiki articles and raw sources are snapshotted under `docs/research/` (same relative layout); links were rewritten to point there.

---
title: "Plan: SPc Pool — a net-new PPLNS-JD (SLICE) pool paying silent-payment addresses in the coinbase"
type: plan
format: roadmap
generated: 2026-10-09
updated: 2026-10-12
status: current
changelog:
  - "2026-10-13: aligned with SPEC/DESIGN as revised 2026-10-13. Sub-threshold earnings now paid over Ark (new Phase 7; old Phase 7 is Phase 8): one hot P2TR pool output per block, height-indexed; registered sub-threshold miners paid from the pool's Ark float at 6 confirmations; matured pool output boarded only to refill the float; unregistered miners below the coinbase threshold forfeit to the pool. Hash tags settled as `SPc/*` (DESIGN.md); scriptSig carrier is x-only push32 (63 B budget); `spc-translator` + `spc-jdc` are roles of one `spc-agent`; confirmation depth 6. Must work with a stock third-party Ark server."
  - "2026-10-12 (b): license MIT OR Apache-2.0; Decision 5 / Phase 0 / R13 updated — Blitzpool math re-implemented, not ported."
  - "2026-10-12: PR #203 impact pass — Phase 4 spc-jdc becomes a fork of warioishere/sv2-apps@90b3e3d (+ stratum@9be4023); pins by SHA; R3 reworded; OP_RETURN cost 46 B/184 WU; watch record keyed on headRefOid."
goal: "Net-new mining pool: PPLNS-JD / SLICE accounting, Blitzpool's coinbase-direct architecture as the reference, BIP 352 silent-payment address as the SV2 user_identity, A_send carried in the coinbase (SPc). Both pool-template and Job-Declaration modes in v1, SPc-enforcing Translator and JDC shipped with it, share-accounting extension (type 32) served in v1."
sources:
  - inventory/candidates/coinbase-native-stealth-payout.md
  - raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md
  - raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md
  - raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md
  - raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md
  - wiki/concepts/pplns-jd.md
  - wiki/concepts/sv2-share-accounting-ext.md
  - wiki/concepts/xpub-payout-identity.md
  - wiki/concepts/coinbase-address-rotation.md
  - wiki/concepts/coinbase-amount-linkability.md
  - wiki/concepts/payout-attribution-privacy.md
  - wiki/concepts/block-withholding.md
  - wiki/concepts/lottery-pplns.md
  - raw/repos/2026-07-27-blitzpool-server-rust-github.md
  - raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md
  - raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md
  - raw/repos/2026-07-14-demand-share-accounting-ext-github.md
  - output/plan-xpub-miner-identity-spec-2026-07-29.md
  - ../mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md
gap_research:
  - "lorbax/pplns-with-job-declaration .tex (raw GitHub) — the PPLNS-JD reward formula"
  - "stratum-mining/sv2-spec PR #203 — extension 0x0003 Coinbase Transaction Payouts, OPEN, updated 2026-09-15"
  - "bitcoin/bips bip-0323.mediawiki @ 60f5b33 — 24 nVersion bits as extra nonce"
  - "bitcoin/bips bip-0352.mediawiki §Versions — address length 116/117 chars"
  - "local clones: sv2-apps translator/jd-client user_identity flow; share-accounting-ext src/"
---

# Plan: SPc Pool — PPLNS-JD accounting, Blitzpool architecture, silent-payment identity

> Generated from [bitcoin-mining-payout-schemas](research/README.md) (18 articles/sources consulted, 4 gap lookups, 4 interview decisions). Format: roadmap.

## Executive Summary

Build a **new Stratum-V2-only pool** whose miner identity is a BIP 352 silent-payment address in `user_identity`, whose accounting is **PPLNS-JD / SLICE** (slices by prev-hash and fee band, window `N = 8 × D`, `payout = r·score_d + F_S·score_f`), and whose payouts are derived per block from one ECDH against that address (DESIGN.md): a **fresh, unlinkable P2TR output in the coinbase** by the SPc construction (`input_hash = H(height ‖ A_send)`, ephemeral `a_send`, erased after use) for balances at or above the coinbase threshold, and a **fresh Ark VTXO key** paid from the pool's Ark float on a third-party Ark server for balances below it (spec §11). Blitzpool supplies the architectural shape — coinbase-direct payout, signed pending ledger for trimmed sats, weight-budget trim so a found block is always valid, confirmation-gated ledger apply, `front`/`payout`/`api` roles over Redis streams on Postgres — but **not** its ledger key (address) or its PPLNS math (plain 4×D window, no slices). Per the interview, v1 ships **both** template modes (pool-template with `A_send` in the scriptSig pool tag; Job Declaration via sv2-spec PR #203 with `A_send` in an `OP_RETURN` `additional_output`), an **SPc-enforcing Translator and JDC** (roles of one `spc-agent` binary) so SV1 firmware field widths never matter, and the **share-accounting extension (type 32)** served from day one so miners can audit the window.

The one unifying trick that keeps this tractable: in pool-template mode every share in a slice carries the same job fees, so the SLICE fee term collapses to the difficulty term and **one accounting engine serves both modes**. The ledger key is `sp_id = H(B_scan ‖ B_spend)`, never an address — which is exactly the ledger-key/payout-script split the xpub spec found unavoidable, and it is what makes Blitzpool's `ON CONFLICT (address)` wall and the sv2-apps 100 %-to-pool defect both unrepresentable here.

## Architecture Decisions

### Decision 1: Identity is the SP address; the ledger key is `sp_id`, never a script

**Context**: [xpub Payout Identity](research/wiki/concepts/xpub-payout-identity.md) establishes that "the work is entirely in decoupling the ledger key from the payout script"; Blitzpool keys `pplns_balance ON CONFLICT (address)` globally, so a per-block address breaks pending-credit carry-forward ([rotation code read](research/raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md)). PPLNS-JD's ledger is positional `(slice, share_index)` with **zero identity fields** ([SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md)), so identity lives only at the channel.
**Options considered**:
- A. Username = address (ckpool/public-pool/Blitzpool convention) — rejected: address reuse is the leak SPc exists to remove (optout's ~2 % Ocean observation, [t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md)).
- B. Username = wildcard descriptor with height index ([xpub spec](research/output/plan-xpub-miner-identity-spec-2026-07-29.md)) — rejected: pool retains a descriptor linking all of a miner's addresses; bare-seed recovery needs `deriveaddresses` at height ~900k.
- C. **Username = `sp1q…` / `tsp1q…` address** (116/117 chars, fits `STR0_255`). `sp_id = tagged_hash("SPc/Id", B_scan ‖ B_spend)` keys ledger, slices' share records, pending credits and stats.
**Decision**: C. **Fail-closed parsing**: `OpenStandardMiningChannel`/`OpenExtendedMiningChannel` with a `user_identity` that is not a valid BIP 352 address for the pool's network → `OpenMiningChannel.Error` (`invalid-user-identity`). No default, no donation fallback. This is the direct lesson of the sv2-apps rotation branch, where an unrecognised identity silently became `PayoutMode::FullDonation` ([spec revision 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md)).
**Consequences**: one connection = one SP identity (per-device identity in extended channels is unrepresentable in SRI today; the per-share `USER_IDENTITY` TLV is unimplemented). Worker names, if wanted, ride a `.worker` suffix stripped before parsing — stats only, never payout. BIP 352 **labels are out of scope for v1** (one `B_spend` per identity).

### Decision 2: Accounting is PPLNS-JD/SLICE in both modes; pool-template is the degenerate case

**Context**: The lorbax paper (gap research) gives: slices begin on prev-hash change or when a job's fees reach `f + δ`; within slice `S`, `score_f(sᵢ) = dᵢfᵢ / Σⱼ dⱼfⱼ`; `score_d(sᵢ) = dᵢ / Σ_window d`; `payout(sᵢ) = r·score_d(sᵢ) + F_S·score_f(sᵢ)` with `F_S = F / t` (`t` slices in window); `Σ payout = r + F`. [PPLNS-JD](research/wiki/concepts/pplns-jd.md): DMND and TIDES both run `N = 8 × D`. Blitzpool runs a plain 4×D window with no slices.
**Options considered**: plain PPLNS in pool-template mode and SLICE only under JD (two engines); SLICE everywhere.
**Decision**: SLICE everywhere, `N = 8 × D` (configurable), `δ` configurable. In pool-template mode all shares in a slice share the pool's job so `fᵢ` is constant and `score_f ≡ score_d` — the fee term costs nothing and no second code path exists. Last (partial) slice excluded from reward, per `extension.md`.
**Consequences**: the slice store (merkle root per slice, share list) is **required** for correctness, not just for the audit extension — so Decision 7 is nearly free. One proptest pins conservation: `Σ pay + pay_P + pending_delta == T` for random windows in both modes.

### Decision 3: `A_send` carrier differs by mode; the scanner accepts both

**Context**: Pool-template mode: the pool owns `coinbase_tx_prefix`; the pool tag is "pre-paid" (TP reserves 100 B / 400 WU unconditionally) and extranonce-disjoint by wire format ([08-16 lessons](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) L5). Budget is `100 − 4 (height) − 33 (push32, x-only) = 63 B` for the full extranonce (Seldenthuis's `push33` figure of 62 B, [t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md), less the one byte x-only saves; spec §2.4.1). JD mode: Coinbase-only carries **no** scriptSig bytes, Full-Template gives the pool approval but not specification authority; PR #203's `additional_outputs` "MAY be 0 (e.g. `OP_RETURN`)" is the only pool→coinbase data channel in both JD modes ([08-24 lessons](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) L3).
**Options considered**:
- A. scriptSig everywhere — impossible under JD Coinbase-only.
- B. `OP_RETURN` everywhere — one scanning rule, but 46 B / 184 WU of real block space per block in pool-template mode where 0 B is available.
- C. **scriptSig in pool-template, `OP_RETURN` under JD; scanner checks scriptSig push first, then `OP_RETURN <"SPc"> <A_send>`.**
**Decision**: C. Tag layout in pool-template mode: `height ‖ push32(ser_x(A_send)) ‖ extranonce` (x-only, even-Y; spec §2.2/§2.4.1) with the SRI `pool_tag_string` **changed from `String` to `Vec<u8>`** so the x-only/compressed key needs no text encoding (recovers ~20 B vs bech32; the UTF-8 constraint is an SRI type, not a protocol one — L4). Under JD the pool's `SetPayoutDistribution.additional_outputs[0] = OP_RETURN 0x53 0x50 0x63 <ser_x(A_send)>` (32 B x-only, even-Y; spec §2.4.2). The `SPc` magic is the wallet "scanning profile" Anzus asked for.
**Consequences**: two carriers = two rules in a ~20-line parser. Record the JD cost honestly: 46 B/block (184 WU) is not pre-paid. BIP 323 (24 nVersion bits as extranonce) would let the pool shrink the scriptSig extranonce to ~8 B and is worth tracking, not depending on.

### Decision 4: `a_send` is per-height CSPRNG, RAM-only, erased at coinbase build; never BIP 32

**Context**: [08-16 L2](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md): BIP 32 unhardened derivation **is** the group-linear defect (vectors case 9) — a parent secret regenerates every child, so the key can't be destroyed. [08-14 L8](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md): key separation costs 33 transmitted bytes; the on-chain carrier pays them. [08-16 L3](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md): with the height bound into `input_hash`, rotation cadence is a forward-secrecy knob, not a correctness requirement. [08-24 L1](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md): keep the height anyway — a pool that accidentally reuses `a_send` across heights (restart, failover) would silently reuse every miner's address with no on-chain signal.
**Decision**: one fresh `a_send` **per block height** (generated on `SetNewPrevHash`), held only in the `front` process's memory, zeroised when the height advances. Under JD the same per-height key feeds every `SetPayoutDistribution` published at that height (the `P_i` are height-fixed, so weight-only updates don't change outputs). In pool-template mode a per-template key is permitted (finer forward secrecy) but per-height is the default — simpler, and it keeps both modes on one key schedule.
**Consequences**: After the coinbase is built the pool keeps `(sp_id → P_i, sats)` in the frozen distribution snapshot — that mapping, not `a_send`, is the retained attribution, and its retention is a **policy knob** (scrub after `ledger_apply + N` blocks, per the OP's "scrub share accounting logs"). Restart during a height → new `a_send` for the same height is fine (different outputs, still scannable). Threat model stated plainly: defends against **chain observers only**; the pool knows who it paid ([Payout Attribution Privacy](research/wiki/concepts/payout-attribution-privacy.md)).

### Decision 5: Non-custodial-to-the-sat via Blitzpool's signed pending ledger + weight-budget trim, keyed by `sp_id`

**Context**: [Blitzpool](research/raw/repos/2026-07-27-blitzpool-server-rust-github.md): trimmed/sub-dust sats → pending credit, bonus recipient picks matching debit, balances sum ≈ 0; greedy keep-largest trim against a weight budget so the coinbase never exceeds the reservation core was told; autoscaler 50 000 WU floor, ±15 % steps; confirmation-gated (depth 3) orphan-safe apply, idempotent via history `UNIQUE`. Known landmine: PPLNS ledger apply has **no duplicate-address merge** ([finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) §8). Taproot dust 330 sats; firmware/DATUM output ceilings bite before `K_max`.
**Decision**: adopt the pattern, **re-implement** the math from the spec (§4.6/§4.8) — `bp-pplns/src/distribution.rs` is AGPL and is not ported; silent-pool is MIT OR Apache-2.0 (decided 2026-10-12). Ledger rows key on `sp_id`, so one miner never produces two outputs in one block (one `P_i` per `(sp_id, height)` — the finder-bonus two-rows-one-address hazard from [Lottery-PPLNS](research/wiki/concepts/lottery-pplns.md) cannot arise; no finder bonus in v1 anyway). Under JD, PR #203 gives the same semantics natively: `dust_limits[i]` prune, `weight_P` absorbs pending/under-threshold miners, `pay_P` absorbs rounding — the pool's `payout` role books the pruned amounts per spec §11.1. **Amended 2026-10-13:** sub-threshold balances are no longer carried as pending until they reach dust; a registered miner below the coinbase threshold `C` (default = dust) is paid over Ark from the float at 6 confirmations, an unregistered one forfeits to the pool, and `pending` remains only for trimming and failed Ark sends.
**Consequences**: Postgres tables `sp_identity`, `slice`, `distribution_snapshot`, `ledger_history`, `pending_balance` (plus `ark_delivery`, `ark_epoch_mailbox`, `ark_payout` for the Ark tier, spec §5.1). No per-share rows — slices are the aggregation unit ([share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md): nobody in production writes one row per share).

### Decision 6: Blitzpool-style role split from day one (interview choice)

**Context**: user chose `front`/`payout`/`api` roles over Redis streams + Postgres over a single binary. Blitzpool's `front` holds stratum listeners, TDP streams, job building, block submit; `payout` applies the ledger after confirmations; exactly-once over Redis streams.
**Decision**: three roles in **one binary** selected by `--roles` (Blitzpool's pattern), plus a fourth, `ark`, added 2026-10-13 for the Ark tier: it alone holds the pool's hot Ark seed, on its own host, and sees only signed send orders with no identity data (spec §11.3). `front` is the only process that ever holds `a_send`. Share → slice aggregation happens in `front` RAM; a slice is published to the `slices` Redis stream when sealed (prev-hash change or `fees ≥ f+δ` or `max_shares`), and `payout` persists it. Block-found events carry the frozen distribution snapshot keyed by `(height, template fingerprint)`.
**Consequences**: a `front` restart loses the open (unsealed) slice — bounded loss of at most one slice of shares. Acceptable for v1; flag with a `ponytail:` note (upgrade: periodic open-slice checkpoint). `seen_shares` replay set also lives in `front`; SRI loses it on restart too — mitigate by rejecting shares for jobs issued before process start.

### Decision 7: Serve the share-accounting extension (type 32) in v1, pool-defined where the spec is unfinished (interview choice)

**Context**: `demand-share-accounting-ext` v0.0.13: activation handshake TODO (sv2-spec #95), typo'd type names baked in, built on a **demand fork** of the SRI codec stack ([article](research/wiki/concepts/sv2-share-accounting-ext.md)). Window: first slice where cumulative difficulty back from the block ≥ `N × window_size`; last slice excluded. Messages `GetWindow`/`GetShares`/`GetTransactionsInJob`/…
**Decision**: implement the server side on the current `stratum-core` codec (port the message structs; keep the type-32 identifier and message numbering; use the 0x0001 extensions-negotiation handshake since #95 is unresolved). The slice store from Decision 2 is the data source. Ship a miner-side verifier in the SPc Translator/JDC (sampling slices, PoW check, merkle inclusion, `Σ d ≤ slice.difficulty`, `fees ≤ slice.fees + δ`).
**Consequences**: this is our own dialect until DMND/spec settle; document the deltas in `docs/share-accounting-ext-deltas.md`. In pool-template mode `GetTransactionsInJob` answers with the pool's own job tx set.

### Decision 8: SV2-only, with an SPc Translator and SPc JDC shipped from this repo (interview choice)

**Context**: SRI translator sets **one** upstream `user_identity` from TOML (`PayoutMode::try_from(user_identity)`, `miner-apps/translator/src/lib/config.rs:177`); SV1 downstream usernames do not propagate. Avalon truncates at 63, Whatsminer overflows past 127 ([xpub identity](research/wiki/concepts/xpub-payout-identity.md) §4) — irrelevant once the SP address lives in the proxy config. JDC has `user_identity` and `coinbase_reward_script` as separate config fields; under PR #203 the latter is unused (outputs come from `SetPayoutDistribution`).
**Decision**: fork `sv2-apps` `translator` and `jd-client` as the translator and JDC **roles of one `spc-agent` binary** (named `spc-translator` / `spc-jdc` below; spec §1): config field `sp_address` (validated at startup against the network's HRP, `sp`/`tsp`), set as the upstream `user_identity`; refuse to start otherwise. `spc-jdc` negotiates `0x0003` on both JDP and MP connections, computes outputs per §4 with 128-bit arithmetic, appends the `distribution_id` TLV, applies §5 constraint transitions, and **MUST NOT** declare across a height boundary under the grace window (see Risk R4).
**Consequences**: SV1 ASICs work via `spc-translator`; the pool itself never speaks SV1. The firmware-field-width problem is closed by construction.

## Implementation Phases

### Phase 0: SPc spec freeze + license + repo skeleton (≈ 1 week)
**Goal**: the construction is written down before any pool code depends on it; closes inventory record `coinbase-native-stealth-payout` close-out condition 1.
**Tasks**:
- [ ] `docs/spc-spec.md`: diff against BIP 352 Sender/Receiver sections. Hash tags are settled by DESIGN.md (`SPc/Inputs`, `SPc/TierSpc`, `SPc/TierArk`, `SPc/Mailbox`; spec §2.3) and `A_send` is x-only + even-Y on both carriers (spec §2.2). Still to write down: `input_hash = tagged_hash(ser32(height) ‖ ser(A_send))`; `k = 0` always (one output per `(sp_id, height)`); both carriers and the scanner precedence; `H` and `H−1` scan tolerance.
- [ ] Decide amendment-vs-new-BIP posture (optout's question) — recommend **new informational BIP referencing 352**, since the eligibility gate at `bip-0352:193` is explicitly violated.
- [x] License decision: **MIT OR Apache-2.0** (2026-10-12) — matches every dependency; Blitzpool math re-implemented, not ported.
- [ ] Cargo workspace (spec §7.1): `spc-crypto` (absorbing the `spc_ark` / `spc_ark_mailbox` prototypes), `spc-scan`, `spc-accounting`, `spc-share-ext`, `spc-ext`, `spc-pool`, `spc-jds`, `spc-agent`. Pin `stratum-core`/`sv2-apps` commits in `UPSTREAM_DEPS.md` (Blitzpool's practice).
- [ ] Testnet choice: testnet4 or signet (`tsp` HRP); regtest harness via `sv2-tp` or bitcoin-core IPC.
**Dependencies**: none.
**Validation**: spec reviewed by one SP implementer; `sp-coinbase-vectors` cases 1–12 re-run green against the frozen tags.
**Wiki grounding**: [inventory record](research/inventory/candidates/coinbase-native-stealth-payout.md) § Close-out Condition; [t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md).

### Phase 1: `spc-crypto` + `spc-scan` (≈ 1 week)
**Goal**: derivation and scanning as a library with the published vectors as its test suite.
**Tasks**:
- [ ] `spc-crypto`: `parse_sp_address() → (B_scan, B_spend, network)`; `sp_id()`; `ecdh(a_send, height, B_scan)` and the tier tweaks `t_onchain`, `t_ark`, `t_mb` (spec §2.3); `derive_output(..) → P`; `scan(b_scan, B_spend, A_send, height, outputs) → Option<(vout, t_0)>`; `spend_key(b_spend, t_0)`. `secp256k1` crate; zeroise `a_send` on drop.
- [ ] `spc-scan`: coinbase parser for both carriers (scriptSig `push32` after the BIP 34 push; `OP_RETURN "SPc" <key>`), `scan_block(header, coinbase, merkle_branch)` for the ~500 B/block light-client path; CLI `spc-scan --address tsp1… --from-height …`.
- [ ] Port the 11 vector cases from `~/repos/sp-coinbase-vectors/coinbase_sp_test_vectors.json` as `#[test]`s, including the negative controls (constant-outpoint collision; `input_hash` without `A_send` → `:92` grind succeeds; group-linear list).
**Dependencies**: Phase 0 tags.
**Validation**: vectors green; a scripted regtest block with 5 SPc outputs is found by 5 distinct `b_scan`s and spent by their `b_spend`s.
**Wiki grounding**: [08-14 lessons](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md) L5–L7; vectors repo in the inventory record.

### Phase 2: `spc-accounting` — SLICE engine + signed ledger + distribution builder (≈ 2 weeks)
**Goal**: pure, deterministic accounting with no I/O.
**Tasks**:
- [ ] Types: `Slice{prev_hash, ref_job_id, ref_fees, shares: Vec<ShareRec{sp_id, d, job_fees, nonce, ntime, version, extranonce, job_id, share_index}>, root}`; `Window`; `Distribution{height, A_send, entries: Vec<(sp_id, P_i, sats)>, pool_sats, pending_deltas}`.
- [ ] Slice sealing rule: new prev-hash, or incoming share `fees ≥ ref_fees + δ`, or `max_shares` (cap `GetShares` payloads).
- [ ] Window selection: walk back from the block's slice until `Σ difficulty ≥ N × D`; exclude the finding slice.
- [ ] Reward: `payout(sᵢ) = r·score_d + F_S·score_f`, `F_S = F/t`; aggregate per `sp_id`; 128-bit intermediates.
- [ ] Pending ledger and tier routing (spec §4.6, §11.1): below-`C` → `ark_due` (registered) or `forfeited` (unregistered); weight-trimmed → `pending_credit[sp_id]`; conservation enforced; carry-forward adds pending to next distribution's weight (PR #203: fold into `weight_P` for JD, pay directly in pool-template mode when ≥ dust).
- [ ] Weight-budget trim: keep-largest under `coinbase_weight_budget`; autoscaler (floor 50 000 WU, thresholds 0.85/0.50, ×1.15) ported as a tiny state machine.
- [ ] Output builder: for each kept `sp_id`: `P_i = derive_output(a_send, height, …)` → P2TR output; pool output first; carrier output (`OP_RETURN`) when mode = JD.
**Dependencies**: Phase 1.
**Validation**: proptests — conservation `Σ pay + pay_P = T` and spec §4.6 (a)–(f); pool-template windows produce identical results to a plain difficulty-weighted PPLNS; one `(sp_id, height)` never yields two outputs; trim never exceeds budget.
**Wiki grounding**: [PPLNS-JD](research/wiki/concepts/pplns-jd.md); lorbax formula (gap research); [Blitzpool README](research/raw/repos/2026-07-27-blitzpool-server-rust-github.md) § PPLNS specifics / autoscaler / trim; [finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) §8 (the merge landmine this design avoids).

### Phase 3: `spc-pool`, pool-template mode (≈ 4 weeks)
**Goal**: a miner pointing `spc-translator` at the pool gets paid to a fresh P2TR output in the coinbase of a regtest block and finds it with `spc-scan`.
**Tasks**:
- [ ] Fork `sv2-apps/pool-apps/pool`. Channel open: parse `user_identity` → fail-closed per Decision 1; attach `sp_id` to the channel.
- [ ] Template path: on `SetNewPrevHash` generate `a_send` (CSPRNG, zeroise on advance); on each `NewTemplate` build the distribution for `(height, a_send, T)` and the coinbase `prefix ‖ push32(ser_x(A_send)) ‖ extranonce ‖ suffix`, with the pool output at `pool_payout_descriptor @ height` (spec §11.3). Patch `pool_tag_string: String → Vec<u8>` in the vendored `channels-sv2` `JobFactory` (or carry the push in `coinbase_tx_prefix` directly).
- [ ] Share path: validate (existing SRI), dedup (`seen_shares`), append `ShareRec` to the open slice; vardiff unchanged.
- [ ] Snapshot freeze: on each template, write the `Distribution` to Redis keyed by payout-list fingerprint (Blitzpool `payouts_fingerprint` pattern) with TTL; on block found, the fingerprint rides the event to `payout`.
- [ ] Redis streams: `slices`, `blocks_found`, `shares_stats` (aggregated per `sp_id` per minute, never per share).
- [ ] `payout` role: persist slices; on `blocks_found`, wait `confirmation_depth` (6), discard orphans, apply ledger idempotently (`ledger_history UNIQUE(block_hash, sp_id)`), book pending deltas, then **scrub** the `sp_id → P_i` mapping after `scrub_after_blocks` (policy, default 100).
- [ ] `api` role: `/api/sp/:sp_id/…` requires a signed challenge from `b_scan` (otherwise the API is an attribution oracle); public endpoints are pool-wide only.
- [ ] Postgres migrations at boot (sqlx, advisory-locked).
**Dependencies**: Phases 1–2.
**Validation**: regtest e2e: 3 translators with distinct `tsp` addresses → pool → `generatetoaddress` race → block with 3 miner outputs + pool output; each `spc-scan` finds exactly its own; orphan test discards; restart mid-height produces a new `A_send` and still-scannable outputs; invalid `user_identity` is rejected at channel open (never defaulted).
**Wiki grounding**: [08-16 lessons](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) L4–L6 (budget, `Vec<u8>`, prefix placement); [coinbase address rotation](research/wiki/concepts/coinbase-address-rotation.md) (take the `SharedLock` plumbing, not `XpubDerivator`); [xpub spec rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md) (fail-closed; per-connection payout slot).

### Phase 4: Job Declaration mode — `spc-jds` + pool-side PR #203 + `spc-jdc` (≈ 4 weeks)
**Goal**: a miner running `spc-jdc` with its own template gets the same payout outputs, with `A_send` in `OP_RETURN`.
**Tasks**:
- [ ] `spc-jds` (fork `jd-server`): negotiate `0x0003`; on `SetNewPrevHash` compute per-height `a_send`, publish `SetPayoutDistribution{distribution_id, pool_payout(weight_P), payouts[i] = P2TR(P_i) with amount = weight[i], dust_limits[i] = 330, additional_outputs = [OP_RETURN "SPc" A_send (0 sats)]}`; re-publish on a timer when weights move materially (§3.1.1), **always** on height change.
- [ ] Settlement invalidation (§10): on block found, invalidate all other `distribution_id`s and publish fresh.
- [ ] Validation (§7.1): recompute `pay[i]` from `(distribution, T)` and compare to the declared coinbase; `invalid-payout-distribution` / `stale-payout-distribution` error codes. **Grace window: never across a height change** (R4).
- [ ] Pool MP side: accept the `distribution_id` TLV on `SetCustomMiningJob`; shares under JD jobs carry the JDC's job fees into the slice (`score_f` now non-trivial).
- [ ] `spc-jdc`: fork `warioishere/sv2-apps` @ `90b3e3d` (PR #3, `coinbase_transaction_payouts = true`; stacked on #1's coinbase-only chain-tip fix and #2's `block_reserved_weight` fix; needs `warioishere/stratum` @ `9be4023` for `extensions_sv2`/`parsers_sv2::DistributionId` — its `Cargo.toml` pins that by branch, re-pin by SHA; MIT OR Apache-2.0). It already does `0x0003` on both connections, empty `AllocateMiningJobToken.Success.coinbase_tx_outputs`, deferred `CoinbaseOutputConstraints`, §4 outputs in u128, the TLV, and verbatim `additional_outputs` (0-value `OP_RETURN` unit-tested). Deltas: `sp_address` config → upstream `user_identity`; §3.6 own-payout check from the distribution's `OP_RETURN`; **refuse** JD mode (spec §3.2) instead of the fork's base-protocol continuation when the JDS declines or is silent 10 s; in Full-Template mode rebuild the stored future-template job whenever the applied distribution changes (hook `on_payout_distribution`/`apply_pending_payout_distribution`) and at `SetNewPrevHash`, instead of re-sending the stored `DeclareMiningJob` with its old `distribution_id` (`template_message_handler.rs:489-507`). The first declaration of a height still races the pool's `SetPayoutDistribution`; R4 remains the JDS's rule to enforce and `stale-payout-distribution` on a non-latest id is already non-fatal in the fork.
- [ ] Both JD modes (Full-Template and Coinbase-only) exercised.
**Dependencies**: Phase 3 (shares/slices/ledger); PR #203 text @ `47dbedd` (its single commit, 2026-09-10; the commit warioishere/stratum#1 and sv2-apps#3 implement) pinned in `docs/`; `warioishere/stratum @ 9be4023` and `warioishere/sv2-apps @ 90b3e3d` pinned in `UPSTREAM_DEPS.md` until merged upstream (spec §7.1).
**Validation**: regtest: JDC-built block pays per distribution; a JDC that mutates amounts is rejected; a declaration straddling a height change is rejected (not grace-accepted); `spc-scan` finds outputs via the `OP_RETURN` carrier; type-32 `GetTransactionsInJob` returns the JDC's tx set.
**Wiki grounding**: [08-24 lessons](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) L3–L5, L7; [PPLNS-JD § Non-custodial payouts under JD](research/wiki/concepts/pplns-jd.md); PR #203 §§3–10 (gap research).

### Phase 5: `spc-translator` hardening + miner UX (≈ 2 weeks)
**Goal**: any SV1 ASIC mines to an SP address through its owner's proxy; nothing about firmware limits reaches the pool.
**Tasks**:
- [ ] Fork `translator`: `sp_address` config (validated HRP/network at start); upstream `user_identity = sp_address`; downstream SV1 `username` accepted verbatim, logged as worker label only.
- [ ] Startup self-check: open channel, confirm the pool echoes the SPc capability (an `extension 0x0003` or a pool-advertised SPc flag); abort loudly otherwise — don't let a user mine to a non-SPc pool thinking they're private.
- [ ] Wallet guidance doc: which SP wallets can **spend** found outputs (needs `b_spend` + `t_0` import), and that none **detect** them yet (Anzus) — `spc-scan` fills the gap.
**Dependencies**: Phase 3.
**Validation**: a stock SV1 CPU miner (`cpuminer`) with a 63-char-truncated username → translator → pool → payout to the configured `tsp` address.
**Wiki grounding**: [xpub identity](research/wiki/concepts/xpub-payout-identity.md) § field widths; sv2-apps translator config (gap research).

### Phase 6: Share-accounting extension (type 32) server + client (≈ 3 weeks)
**Goal**: a miner can audit any window the pool paid on.
**Tasks**:
- [ ] `spc-share-ext`: port `demand-share-accounting-ext` message structs to current `stratum-core` codec; negotiate via `0x0001`; `NewBlockFound`, `GetWindow(.Success/.Busy)`, `GetShares`, `GetTransactionsInJob`, `IdentifyTransaction`, `ProvideMissingTransactions`, `NewTxs`, `ErrorMessage`.
- [ ] Server in `spc-pool`: windows served from the `slice` table; `GetWindow.Busy` with backoff for windows older than `K` blocks; share blobs retained for `retain_windows` (default 16 blocks ≈ 2 windows).
- [ ] Client verifier in `spc-translator`/`spc-jdc`: sample `s` slices × `m` shares; PoW; merkle inclusion; `Σ d ≤ slice.difficulty`; `fees ≤ slice.fees + δ`; report.
- [ ] `docs/share-accounting-ext-deltas.md`: every divergence from `extension.md` v0.0.13.
**Dependencies**: Phases 2–4.
**Validation**: a pool build that under-reports one slice's difficulty is caught by the verifier in ≤ 3 windows with `s = 4, m = 8`; a pool that mis-states `fees` under JD is caught.
**Wiki grounding**: [SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md); `~/repos/share-accounting-ext/extension.md`.

### Phase 7: Ark tier — `ark` role, SPc extension, SP-aware agent wallet (≈ 4 weeks, estimate)
**Goal**: a sub-threshold miner running `spc-agent` is paid over Ark at 6 confirmations from the pool's float, against a stock third-party `captaind`, and the agent claims and refreshes the VTXO.
**Tasks**:
- [ ] `ark` role (spec §11.3): own seed and secret store; pool output keychain `m/86'/coin'/1'`, xpub exported as `pool_payout_descriptor`; per-block discovery of the height-indexed pool output; float sends from signed `ArkSendOrder`s; at maturity board only `float_target_sats − float_balance` via `board_funding_address` + `board_psbt`, rest left as operator surplus.
- [ ] `payout` side: `ark_payout` scheduling at booking, order signing, receipts, `failed_to_pending` after `retry_blocks`, forfeiture booking.
- [ ] `spc-ext` (`0x5350`): `SetArkIdentity` (signed by `b_scan`), `RegisterEpochMailboxes`, `ArkPayoutNotice` (spec §11.4).
- [ ] `spc-agent`: mandatory registration before submitting shares; SP-aware bark keystore (extra-keys source for `pubkey_keypair` / `find_vtxo_keypairs` / `get_vtxo_key` / `find_signable_clause`, RESULTS.md ~150–300 lines; patch vs fork is spec O13); mailbox reads per mode (MAILBOX.md, `PAYOUT_DELAY = 6`); refresh and offboard-on-exit.
**Dependencies**: Phases 1–3; `spc_ark`, `spc_ark_mailbox`, `spc_ark_client` prototypes (frozen vectors).
**Validation**: spec §11.9 integration test on regtest with stock `captaind` in the third-party role.

### Phase 8: Regtest e2e, ops, and public testnet (≈ 2 weeks)
**Tasks**:
- [ ] `spc-regtest-harness`: bitcoin-core (IPC/TDP) + `spc-pool` + `spc-jds` + `spc-translator` + `spc-jdc` + 3 synthetic miners; CI job.
- [ ] Metrics: slices/min, open-slice shares, distribution entries, trimmed sats, `a_send` age, snapshot hit/miss on block found (a build must never fail over a lost snapshot — Blitzpool's vouching invariant).
- [ ] Capacity alert before anyone is trimmed (Blitzpool pattern).
- [ ] Testnet deployment with `tsp` addresses; publish `spc-scan` so external SP implementers can verify blocks.
**Validation**: 24 h testnet soak; at least one found block whose outputs are independently recovered by a third party running `spc-scan`.

**Total**: ≈ 23 weeks single-developer, ≈ 12–14 with two. Phases 1–3 (≈ 7 weeks) are the minimum that demonstrates SPc end-to-end on regtest; Phase 7 adds the Ark tier; everything else is the interview's scope additions.

## Risks & Mitigations

| # | Risk | Source | Mitigation |
|---|------|--------|------------|
| R1 | **Amount linkability**: a stable hashrate share is a stable `amount_i / T` across blocks, so miners are fingerprintable without any address repeating — the ceiling on what SPc buys | [Coinbase Amount Linkability](research/wiki/concepts/coinbase-amount-linkability.md); [08-24 L6](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) | State it in the spec and README. v2 candidates: randomised slot count, deferred/batched pending payouts — out of v1 scope. |
| R2 | scriptSig budget is 63 B for the full extranonce, 62 B past height 16,777,215; merged-mining tags would contend | Seldenthuis, [t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md); [08-16 L4](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) | Allocate 16 B extranonce (4–8× headroom); `fits_script_sig_budget` fails loudly at channel setup; track BIP 323. |
| R3 | PR #203 is unmerged (text frozen at `47dbedd` since 2026-09-10, no maintainer freeze; one SHA-pinned JDC exists — warioishere/stratum `9be4023` + sv2-apps `90b3e3d`, run on mainnet against Blitzpool's JDS per the author, F = 0 and height-agnostic grace, so none of our §2.4.2/§3.5 deltas were exercised); JD mode may need rework if the open 'paid subset of static payouts' question relaxes the §7 MUST | gap research; PR #203 comment 6109310390 | Pin the spec text in-repo by SHA; isolate `0x0003` handling behind one module; expect one rebase. |
| R4 | **Grace-window height mismatch**: a declaration built under distribution `k−1` at height `H` accepted at `H+1` pays outputs derived for `H`; scanners at `H+1` miss them | [08-24 L6](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) | JDS never grace-accepts across a prev-hash change; `spc-scan` tries `H` and `H−1` as belt-and-braces. |
| R5 | JD §9.2 leak: every JDC learns payout scripts + weights of top miners, plus unfound-block snapshots and the sub-dust set | PR #203 §9.2; [08-24 L5](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) | Scripts are one-time `P_i` (no identity); weights are already derivable from chain amounts; cap payout slots; document residue. |
| R6 | `a_send` reused across heights after restart/failover → forward-secrecy blast radius grows (correctness is saved by the height nonce) | [08-24 L1](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md); [08-16 L3](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) | `a_send` is never persisted; generated fresh per process per height; alert if the carrier key repeats across two heights. |
| R7 | Fresh per-block addresses defeat Eligius-style two-address withholding detection | [Block Withholding](research/wiki/concepts/block-withholding.md) | Detection moves pool-side on `sp_id` (share-vs-block ratio per identity); the pool has this data anyway. |
| R8 | Fail-open identity parsing (unknown username → 100 % to pool, silent) | [xpub spec rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md) | Decision 1: reject at channel open; integration test asserts the error, never a default. |
| R9 | Wallets cannot detect SPc outputs today; spending requires importing `b_spend + t_0` | Anzus, [t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md) | Ship `spc-scan` with a `--export-tweaks` mode; publish the scanning profile in the spec for wallet authors. |
| R10 | `demand-share-accounting-ext` is pre-1.0 on a forked codec; port cost unknown | [SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md) | Budgeted 3 weeks; message set is small; keep type 32 and numbering; log deltas. |
| R11 | `front` restart loses the open slice and `seen_shares` | [share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md) | Accept bounded loss (`ponytail:` note); reject shares for pre-restart jobs; v2: checkpoint open slice. |
| R12 | 330-sat P2TR dust prunes small miners every block; pending credits accumulate without a payout rail | Blitzpool README; OP's dust note | **Resolved 2026-10-13**: Ark tier (spec §11, Phase 7) pays registered sub-threshold miners from the float at 6 confirmations; unregistered sub-threshold earnings forfeit to the pool by published policy. |
| R14 | Ark tier depends on a third-party Ark server the pool does not run; anything needing server changes is out (e.g. coinbase-as-board, spec O17) | spec §11.3 | Design only against stock `captaind` behaviour, pinned by commit; regtest e2e with stock `captaind`. |
| R15 | Hot `ark` seed holds the float and unswept pool revenue | spec §11.3 | Own host and secret store; sees no attribution data; boards only what the float needs; surplus handling is operator policy. |
| R13 | Blitzpool is AGPL-3.0; porting its math into a non-AGPL repo is a license problem | Blitzpool raw | **Resolved 2026-10-12**: silent-pool is MIT OR Apache-2.0; ledger/trim/autoscaler re-implemented from spec §4.6/§4.8 under a clean-room rule (spec §7.6); Blitzpool cited as provenance only. |
## Open Questions

- **BIP posture**: amendment to BIP 352 or a new BIP that references it? (optout). The construction violates `bip-0352:193`'s eligibility gate, which argues for a new informational BIP. Decide in Phase 0.
- **Hash tags**: settled 2026-10-13 as fresh `SPc/*` (DESIGN.md). Remaining question (spec O12): should `t_onchain` use the upstream SPc/BIP 352 tag so a plain SPc scanner finds coinbase outputs.
- **Address version byte** (`bip-0352:152-176`): can v0 `sp1q` addresses be used unchanged, or does the pool need to require a new version signalling "I accept coinbase-derived outputs"? Unchanged is simpler and is what the thread assumes.
- **Solo pools** (optout): SPc for a solo pool is one output per block, same carrier; is a one-recipient variant worth specifying?
- **Per-height vs per-template `a_send`** in pool-template mode: per-height matches JD and is simpler; per-template is finer forward secrecy. Default per-height; revisit if a privacy reviewer objects.
- **Scan tolerance**: `H` and `H−1` only, or a wider window for pools with long grace handling?
- **Labels**: BIP 352 labels for one miner running several identities under one `b_scan` — excluded from v1; needed?
- **Empirical scriptSig occupancy** on mainnet alongside merge-mining commitments — still unmeasured (inventory record).

## Suggested inventory follow-ups

Three durable items fall out of this plan; sample records (not created yet):

- `inventory/candidates/spc-spec-bip-posture.md` — kind: question, p1, next action: "Phase 0 — decide amendment vs new BIP; write `docs/spc-spec.md`." Closes `coinbase-native-stealth-payout` condition 1.
- `inventory/candidates/sv2-spec-pr-203-watch.md` — kind: watch, p2, next action: "Re-read PR #203 diff when `headRefOid` moves off `47dbedd` (`updatedAt` also fires on comments — it did on 2026-10-11 with zero text change); Phase 4 depends on §3–§10 text at `47dbedd`; also watch warioishere/stratum#1 and warioishere/sv2-apps#1–#3 for upstream PRs and the unanswered L168 / §9.3 thread items."
- `inventory/candidates/spc-amount-linkability-mitigation.md` — kind: question, p2, next action: "Quantify fingerprint persistence for a top-10 miner over 100 blocks on simulated windows; evaluate randomised slot count." (R1)

Say the word and I'll create them.

## Sources Consulted

- [Inventory: coinbase-native stealth payout](research/inventory/candidates/coinbase-native-stealth-payout.md) — construction, carrier table, SV2 budget check, close-out conditions
- [LL 2026-08-14: the blocker is the sender pubkey](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md) — height nonce, `k = 0` fan-out, light-client cost, sender-side key split
- [LL 2026-08-16: `A_send` in the SV2 pool tag](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) — no BIP 32 for `a_send`; `String` tag type; exact budget; rotation as a knob
- [LL 2026-08-24: who controls the SV2 coinbase](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) — JD carriers, `OP_RETURN`, unhidable suffix, amount ceiling
- [delvingbitcoin t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md) — public write-up and external replies (ingested this session)
- [PPLNS-JD / SLICE](research/wiki/concepts/pplns-jd.md) — `N = 8 × D`, JD payout control, PR #203 summary
- [SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md) — type-32 message set, window definition, verification model
- [xpub Payout Identity](research/wiki/concepts/xpub-payout-identity.md) — ledger-key split, field widths, per-connection identity limits
- [Coinbase Address Rotation](research/wiki/concepts/coinbase-address-rotation.md) — rotation plumbing, persistence lessons
- [Coinbase Amount Linkability](research/wiki/concepts/coinbase-amount-linkability.md) — R1
- [Payout Attribution Privacy](research/wiki/concepts/payout-attribution-privacy.md) — threat-model boundary
- [Block Withholding](research/wiki/concepts/block-withholding.md) — R7
- [Lottery-PPLNS](research/wiki/concepts/lottery-pplns.md) — two-outputs-one-address hazard (avoided)
- [Blitzpool raw](research/raw/repos/2026-07-27-blitzpool-server-rust-github.md) + [finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) — ledger, trim, autoscaler, roles, snapshot fingerprint, merge landmine
- [sv2-apps rotation code read](research/raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md) — `ON CONFLICT (address)` wall
- [Spec: xpub miner identity, rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md) — fail-closed requirement, per-connection payout slot
- [share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md) — no per-share rows; replay set non-persistence
- Gap research: lorbax `pplns-with-job-declaration.tex`; sv2-spec PR #203 diff (`extensions/0x0003-coinbase-transaction-payouts.md`); `bip-0323.mediawiki`; `bip-0352.mediawiki` §Versions; local `sv2-apps` translator/JDC config; local `share-accounting-ext/src`
