> Copied from the research wiki (`bitcoin-mining-payout-schemas/output/plan-silent-pool-spec-2026-10-09.md`) on 2026-10-12. Cited wiki articles and raw sources are snapshotted under `docs/research/` (same relative layout); links were rewritten to point there.

---
title: "Spec: silent-pool — SPc pool technical specification"
type: plan
format: spec
generated: 2026-10-09
updated: 2026-10-15
status: current
supersedes: none
changelog:
  - "2026-10-15: four design decisions applied. D1: MULTI-REGION §10 deltas 3, 6, 7, 8 applied - `high_water` vector (§4.3 `select_window`, §4.7, §5.1 `distribution_snapshot.high_water`, §3.3/§5.2 `spc:blocks` and `BlockFound` payloads, §7.4 settlement reconstructs the window from it); §11.1 tier gains a committed-but-not-propagated registration -> `pending` branch and §4.6 (f) reads 'no `ark_delivery` row in Postgres at settlement'; §11.3 `ark` tracks pool outputs by `(txid, vout)` and re-checks the chain at maturity (+ §11.9 same-height test); §3.7/§5.3 type-32 served from bootstrap-RPC blobs + live `spc:slices` tail. D2: `provisional_paids` announced on `BlockFound`, not `LedgerApplied` (§5.2, §5.3). D3: `ArkSendOrder.sig` = BIP340 over `hash_SPc/ArkSendOrder(...)`, no chain prefix (§11.3 order verification, §11.10). D4: `block_found.fingerprint` nullable with `CHECK (status <> 'confirmed' OR fingerprint IS NOT NULL)` (§5.1, §7.4, §11.1)."
  - "2026-10-14 (c): multi-region amendments adopted (docs/decisions/2026-10-14-multi-region-amendments-adopted.md). §2.2 repeat alarm scoped per front; §3.3 the `spc:blocks` event carries `entries` (Redis snapshot and `snapshot_ttl_secs` deleted); §3.5 `distribution_id = front_id << 56 | local_seq`, `local_seq = max(last_local_seq + 1, unix_time_ms)`, and the grace rule rewritten as 'previous id from the same front on the same connection' (no `id − 1` arithmetic); §5.1 `slice_id` per front; §5.2 streams (+`spc:arkreg`, +`spc:front:events`, −`spc:snapshot:*`, `spc:dist:current` struck as intra-front) and trim discipline (XTRIM MINID behind the slowest live reader — payout group ack and front cursor heartbeats, TTL 120 s — 600 s floor, 1M-entry backstop alarm, re-bootstrap); §5.3 front RAM; §6.3 `front_id`, `outbox_max`, `tip_divergence_secs`, `front_cursor_*` (replacing `snapshot_ttl_secs`); §7.4 missing-event settlement failure model; §9 topology ('RAM-only state — no durable state of its own'), `front_id` lease, monitoring. MULTI-REGION §10 deltas 3, 6, 7, 8 were applied afterwards (2026-10-15 entry)."
  - "2026-10-14 (b): O11 resolved — JD-mode claim formula is revenue-scaled: `claim(id) = floor(owed(id) · (T_actual − F) / W)`; `pending'(id) = claim(id) − paid(id)` (docs/decisions/2026-10-14-o11-claim-formula.md). O14 resolved — default mailbox mode is `epoch_derived` (variant B); no server-side registration exists; caveats on related keys and network linkability recorded (docs/decisions/2026-10-14-mailbox-default-epoch.md). M3 resolved — order-signing key is secp256k1 BIP340 Schnorr, CSPRNG-generated, stored at `/run/secrets/spc/ark-order-key` with 0o600/0o700 perms; pubkey via `[ark] order_pubkey` config; operator-driven rotation (docs/research/output/ark-order-key-spec-recommendation.md)."
  - "2026-10-14 (a): §11.3 'Reorg and float edge cases' added (6-block rule normative; deeper reorgs and float surplus/sizing are operator policy with defaults); §11.4 states that no server-side registration endpoint exists; docs/WIRE-FORMATS.md added (byte-level formats of the new streams, `spc:arkreg` and the bootstrap RPC) and linked from §5.2."
  - "2026-10-13 (d): spc-jds is a module of the spc-pool binary's front role (matching the SRI pool's bundled JDS), not a separate binary; JD is always on and [jd] enabled is removed. a_send sharing becomes in-process; spc:dist:current becomes intra-front state. See docs/decisions/2026-10-13-jds-in-pool-binary-jd-always-on.md."
  - "2026-10-13 (c): payout flow per the operator decisions. One hot pool output per block, fresh P2TR from `pool_payout_descriptor` (xpub under the `ark` seed) indexed by block height; Ark-tier miners paid from the float at `confirmation_depth` = 6 (was 3; was coinbase maturity); at maturity `ark` boards only what the float needs, the rest is operator surplus with no prescribed handling. Treasury signer, watch-only `pool_payout_script` and `float_max_sats` removed. Unregistered miners below `C` are forfeited to the pool (published policy; `spc-agent` registers automatically and will not mine unregistered); `pending` now arises only from trimming and failed sends. Coinbase-as-board recorded as rejected (O17)."
  - "2026-10-13 (b): pool-side key separation (§11.3). The pool's bark wallet is a hot key in a new `ark` role on its own host, generated independently of every SPc-side key, holding a float capped at `float_max_sats` and topped up by an off-host treasury signer in round amounts decoupled from blocks. `ark` sees only signed, capped send orders carrying what the Ark server sees anyway — no `sp_id`, `P_i` or block data — so compromising it risks the float but never SPc receiver privacy, and compromising the SPc side risks attribution data but not the float beyond `max_daily_sats`. `pool_payout_script` returns to watch-only (§9)."
  - "2026-10-13: reconciled to DESIGN.md, which is authoritative for payouts wherever the two disagree. §2.3/§2.5/§11.8 adopt DESIGN's derivation (`SPc/Inputs`, `SPc/TierSpc`, `SPc/TierArk`; the tag is the domain separator; hash-to-scalar fails rather than reduces), superseding the `BIP0352/*` tag-reuse interview decision. §11.1 tier split is now coinbase-threshold (default = dust) → coinbase, below → Ark (replaces the 500 000-sat band). Ark sends happen at the found block's coinbase maturity, from a bounded float held by a separate `ark` role (see the next entry). §11.4 separates identity from delivery capabilities: `SetArkIdentity` requires a separate `B_ark`, carries a mailbox mode, and the literal-address `SetArkPayout` fallback is removed in favour of DESIGN's empty-delivery + out-of-band fetch. Pool now retains per-block Ark destinations until the Ark send (a_send is erased). Fixed: custody wording (§0), threshold/revenue `T` name clash, `k` usage, `spc-agent` in the workspace."
  - "2026-10-12 (d): license changed to MIT OR Apache-2.0; §7.6 rewritten — Blitzpool patterns re-implemented from the spec, no AGPL code ported, clean-room rule added."
  - "2026-10-12 (c): §11.8 reversed — Ark-tier key derivation ADOPTED per DESIGN.md (separate hot B_ark, domain-separated tweaks); §11.4 gains SetArkIdentity {B_ark, M, D}, SetArkPayout becomes the fallback; §11.5 agent holds b_ark and an external-key table. Code-read corrections recorded as implementation notes (mailbox needs M+D; stock bark drops derived keys)."
  - "2026-10-12 (b): PR #203 impact pass (workflow, 110 agents): pins by SHA (47dbedd; bitcoin_core_sv2 block_reserved_weight fix), §3.5 publication bullet re-timed to ledger apply, §4.6 settlement-reads-mined-coinbase rule + O11, §11.9 conservation identity corrected, §2.4.2 OP_RETURN 46 B/184 WU, §8.4 full-mempool step 9, §11.2 weight_P composition note."
  - "2026-10-12: added §11 Sub-threshold Ark rail (both template modes, SPc extension for the Ark address, spc-agent collapsing translator+JDC+payout agent, single-seed recovery, SP-derived-key appendix); resolved §10 O6; §1 note on spc-agent."
companion: output/plan-spc-pool-pplns-jd-silent-payments-2026-10-09.md
goal: "Technical specification for silent-pool: a Stratum-V2-only mining pool with BIP 352 silent-payment addresses as miner identity, PPLNS-JD/SLICE accounting, coinbase-direct payouts to one-time P2TR outputs (SPc), both pool-template and Job-Declaration modes, an SPc-enforcing Translator and JDC, and the share-accounting extension (type 32) served in v1."
decisions_from_interview:
  - "Both template modes in v1"
  - "Blitzpool-style role split (front/payout/api) over Redis streams + Postgres"
  - "SV2-only; SPc Translator + JDC shipped from this repo"
  - "Share-accounting extension (type 32) in v1, pool-defined where the spec is unfinished"
  - "Hash tags: reuse BIP0352/Inputs and BIP0352/SharedSecret — SUPERSEDED 2026-10-13 by DESIGN.md (`SPc/*` tags, §2.3); compatibility with a plain-SPc scanner is open item O12"
  - "A_send: x-only 32 B on the wire, even-Y normalisation"
  - "License: MIT OR Apache-2.0 (changed 2026-10-12 from AGPL-3.0-or-later — Blitzpool math is re-implemented from §4.6/§4.8, not ported; matches SRI, bark, extensions_sv2 and the spc_ark prototype)"
  - "a_send cadence: per block height in both modes"
authority: "DESIGN.md (repo root) is authoritative for the payout derivation, tier policy and Ark delivery. Where this spec disagrees with it, DESIGN.md wins and this spec is the bug."
sources:
  - output/plan-spc-pool-pplns-jd-silent-payments-2026-10-09.md
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
  - raw/repos/2026-07-27-blitzpool-server-rust-github.md
  - raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md
  - raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md
  - raw/repos/2026-07-14-demand-share-accounting-ext-github.md
  - ../mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md
external_pins:
  - "bitcoin/bips bip-0352.mediawiki v1.1.1 @ 60f5b33"
  - "stratum-mining/sv2-spec @ d8bf5b8; PR #203 extensions/0x0003-coinbase-transaction-payouts.md @ 47dbedd (single commit 2026-09-10; unchanged as of 2026-10-12)"
  - "demand-open-source/share-accounting-ext extension.md @ c661e93 (crate v0.0.13)"
  - "lorbax/pplns-with-job-declaration.tex (master)"
  - "warioishere/blitzpool-server-rust @ 7815884 (AGPL-3.0-or-later)"
---

# silent-pool — Technical Specification (v0.1 draft)

> Generated from [bitcoin-mining-payout-schemas](research/README.md). Companion to the [roadmap](ROADMAP.md), whose eight decisions this document makes concrete. Normative words (MUST/SHOULD/MAY) are RFC 2119.

## 0. Scope, terminology, threat model

**silent-pool** pays each miner of a found block on one of two tiers, both derived from the same per-block ECDH against the miner's BIP 352 silent-payment address (`DESIGN.md`): a **fresh, unlinkable P2TR output in the coinbase** (the **SPc** construction, coinbase-scoped silent payments) when the miner's balance clears the coinbase threshold, and otherwise a **fresh Ark VTXO key** paid out-of-round once the block's proceeds mature (§11). Accounting is **PPLNS-JD / SLICE**.

**Custody, stated once.** The coinbase tier is non-custodial: those sats are the miner's the moment the block is mined. Everything else is held by the pool for a bounded time: trimmed balances waiting for a later coinbase (§4.6), sub-threshold balances during the 6-block confirmation window before the Ark send (§11.3), and failed Ark sends awaiting retry. Sub-threshold earnings of a miner with no Ark delivery registered are not held: they are forfeited to the pool by published policy (§11.1). After the Ark send, Ark payouts are trustodial (sender-or-server honest) until the miner refreshes (§11.7). The total is reported at `/api/pool/liability`.

| Term | Meaning |
|---|---|
| `B_scan`, `B_spend` | miner's BIP 352 scan/spend public keys, parsed from the `sp1q…`/`tsp1q…` address in `user_identity` |
| **identity** | the SP address, and nothing else. It is the only thing that keys the miner's account |
| `sp_id` | `tagged_hash("SPc/Id", ser_P(B_scan) ‖ ser_P(B_spend))`, 32 B — the pool's internal ledger key. **Not** a consensus or wire value; never leaves the pool |
| **delivery capability** | optional, private, per-`sp_id` registration data the pool needs to *deliver* to a tier, never to identify the miner: today only the Ark registration `{B_ark, M, D, mailbox mode}` (§11.4). Never published; losing it changes where a miner can be paid, not who they are |
| `a_send`, `A_send` | pool's per-height ephemeral ECDH keypair; `A_send` is published in the coinbase, `a_send` is never persisted |
| `H` | block height being mined (the `SetNewPrevHash` height + 1 relationship is fixed by the template) |
| `ecdh_i` | the per-`(miner, H)` shared secret, `ser_P(input_hash · a_send · B_scan,i)` (§2.3); the single root of every per-block destination |
| `t_onchain`, `t_ark` | domain-separated tier tweaks from `ecdh_i` (§2.3) |
| `P_i` | miner `i`'s coinbase output key for height `H`, `B_spend + t_onchain·G` |
| `B_ark`, `U_i` | miner's separate Ark base key (`m/352'/0'/0'/2'/0`), and its per-block Ark VTXO key `B_ark + t_ark·G` (§11.8) |
| `C` | `coinbase_threshold_sats`: the balance a miner must reach to get a coinbase output (default = `dust_limit_sats`, §11.1). Not to be confused with `T`, the block revenue |
| carrier | where `A_send` sits in the coinbase: **scriptSig** (pool-template mode) or **`OP_RETURN`** output (JD mode) |
| slice | a run of shares on one prev-hash whose job fees lie within `[f_ref, f_ref + δ)` |
| window | the slices whose summed difficulty reaches `N × D` walking back from the block-finding share |

**Threat model (stated, not overclaimed).** SPc hides *which output pays which miner* from **third-party chain observers**. It does **not** hide anything from the pool, which validated the shares ([Payout Attribution Privacy](research/wiki/concepts/payout-attribution-privacy.md)), and it does **not** hide **amounts** — a miner with a stable hashrate share is fingerprintable by `amount_i / T` across blocks regardless of addresses ([Coinbase Amount Linkability](research/wiki/concepts/coinbase-amount-linkability.md)). Forward secrecy is bounded per height: compromise of one `a_send` reveals one block's payee set and nothing else; `a_send` authorises **no funds**.

Out of scope for v0.1: BIP 352 labels; a Lightning/BOLT12 rail for pending credits; finder bonuses; SV1 on the pool itself; mainnet.

## 1. System architecture

```
   SV1 ASICs ──SV1──▶ spc-translator ─┐                              ┌─ bitcoin-core (IPC / sv2-tp) ◀── TDP ──┐
                        (sp_address)  │  SV2 Mining Protocol          │                                        │
   SV2 miners ───────────────────────▶├──────────▶  spc-pool :: front ◀┘   (pool-template mode)                 │
                                      │             ├─ channel→sp_id map                                        │
   miner's own template               │             ├─ a_send[H]  (RAM only, zeroised on advance)               │
   ┌──────────────┐  JDP (0x0003)     │             ├─ open slice, seen_shares                                   │
   │   spc-jdc    │◀────────────────▶ front's spc-jds ├─ distribution builder → coinbase outputs                 │
   │ (sp_address) │──SV2 MP────────▶  module          └─ Redis streams: spc:slices, spc:blocks, spc:stats        │
   └──────┬───────┘                                                 │                                            │
          └── TDP ─▶ miner's bitcoin-core (JD mode)                 ▼                                            │
                                                     spc-pool :: payout ──▶ Postgres (slice, snapshot, ledger)  │
                                                     spc-pool :: api    ──▶ HTTP (public + b_scan-authenticated) │
   miner wallet ◀── spc-scan (header + merkle branch + coinbase ≈ 500 B/block) ◀── any block source ─────────────┘
```

**Components**

| Binary / crate | Role | Forked from |
|---|---|---|
| `spc-pool` | one binary, `--roles front,payout,api,ark` (Blitzpool pattern). `front`: SV2 listeners, template stream, job building, share validation, slice aggregation, block submit. `payout`: slice persistence, confirmation-gated ledger apply, Ark send orders, scrub. `api`: read-only HTTP. `ark`: the pool's bark wallet and Ark float; executes signed send orders and nothing else (§11.3) | `sv2-apps/pool-apps/pool` |
| `spc-jds` (**module of `spc-pool`'s `front` role**, not a separate binary) | Job Declarator Server implementing sv2-spec PR #203 (`0x0003`), publishing `SetPayoutDistribution` per height; JD is always on (decision 2026-10-13, matching the SRI pool's bundled JDS); PR #203 §10's "Pool and JDS are assumed to share state" is satisfied in-process | `sv2-apps` pool (JDS included) |
| `spc-agent` | the miner side, one binary: SV1/SV2 downstream listener and SV1→SV2 translation (the **translator role**, forked from `sv2-apps/miner-apps/translator`); optional Job Declarator Client with `0x0003` that builds outputs per PR #203 §4 (the **JDC role**, forked per roadmap Phase 4); own-payout verifier (§3.6); scanner and SP-aware Ark keystore (§11.5). `sp_address` config becomes the upstream `user_identity` | `sv2-apps/miner-apps/{translator,jd-client}` |
| `spc-crypto` | BIP 352 address parsing, `sp_id`, the SPc ECDH, the tier KDF (`t_onchain`, `t_ark`, `t_mb`), carriers, mailbox blinding, zeroising key types — every derivation in DESIGN.md / MAILBOX.md, used by pool and agent alike | new; absorbs the `spc_ark` and `spc_ark_mailbox` prototypes |
| `spc-scan` | library + CLI: find and export SPc payouts from blocks or header+branch+coinbase triples | new |
| `spc-accounting` | pure, I/O-free: slices, window, SLICE scores, pending ledger, trim, autoscaler, output builder | new; ledger/trim/autoscaler re-implemented from §4.6/§4.8 (Blitzpool patterns, no code) |
| `spc-share-ext` | extension type 32 messages on current `stratum-core` codec; server + sampling verifier | `demand-open-source/share-accounting-ext` |

**Miner-side consolidation (2026-10-12, made canonical 2026-10-13).** SRI ships the translator and JDC as separate binaries; this project ships them as **roles of one binary, `spc-agent`**: one process, one config, one seed. Where this spec says `spc-translator` or `spc-jdc` it means `spc-agent` running in that role.

**Process boundaries that matter.** Only `front` ever holds `a_send`; the `spc-jds` module is part of `front` (decision 2026-10-13), so JD-mode use of the key is an in-process share. Only `payout` writes Postgres. `api` reads Postgres and Redis. Only `ark` holds the `ark` seed (bark wallet and pool output keys), and `ark` sees no attribution data — only signed send orders carrying what the Ark server sees anyway (§11.3). SPc receiver privacy and Ark-tier funds are protected by disjoint secrets on disjoint hosts.

## 2. SPc cryptographic protocol (normative)

All hashes are BIP 340 tagged hashes. `ser_P(X)` is 33-byte compressed; `ser_x(X)` is the 32-byte x-coordinate; `ser32(n)` is 4-byte big-endian; `G`, `n` are secp256k1's. The derivation (§2.3) is DESIGN.md's, and its frozen vector is `spc_ark/src/lib.rs::test_vector`; the regenerated `sp-coinbase-vectors` cover carriers and negative controls.

### 2.1 Miner identity

```
addr          = user_identity with an optional ".worker" suffix stripped at the first '.'
(hrp, ver, B_scan ‖ B_spend) = bech32m_decode(addr)
REQUIRE hrp == "sp" (mainnet) | "tsp" (testnet4/signet/regtest), per pool network
REQUIRE ver == 0  (character 'q'); payload == 66 bytes; both keys valid compressed points
sp_id         = tagged_hash("SPc/Id", ser_P(B_scan) ‖ ser_P(B_spend))
```

Any failure → channel rejected (§3.1). A v0 address is 116 (`sp`) / 117 (`tsp`) characters, inside `STR0_255`.

### 2.2 Sending key schedule (pool side)

```
on SetNewPrevHash(prev_hash, H):
    a_raw  ← CSPRNG 32 bytes, reduce mod n, reject 0
    A_raw  = a_raw·G
    if has_odd_y(A_raw): a_send = n − a_raw else a_send = a_raw      # even-Y normalisation
    A_send = a_send·G                                                # ser_x(A_send) goes on the wire
    zeroise(a_send[H − 1])                                           # previous height's key is destroyed
```

- `a_send` MUST be generated from an OS CSPRNG. It MUST NOT be derived from any seed, xprv, descriptor, or earlier `a_send` — additive-tweak ladders (BIP 32 unhardened, `A_0 + H(·)·G`) let one secret regenerate every key and make erasure impossible ([08-16 L2](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md); vectors case 9).
- `a_send` MUST exist only in process memory (`Zeroizing<[u8; 32]>`); it MUST NOT be written to disk, logs, Redis, or Postgres. `ser_x(A_send)` MAY be stored (it is public on-chain anyway).
- One `a_send` per height per `front` process: `spc-jds` is a module of `front` (decision 2026-10-13), so the key is shared by reference and never crosses a process or machine boundary (never Redis). With one `front` per region (§9), each region has its own `a_send[H]`; that is safe — miners scan `A_send` from the coinbase carrier — and the key is RAM-only, so it adds no durable state (MULTI-REGION §3).
- A restart mid-height generates a new `a_send` for the same `H`. This is safe: outputs differ, both are scannable (§2.5). Reuse of one `a_send` across two heights is **not** a correctness failure (the height is in `input_hash`) but widens forward-secrecy exposure; `front` MUST alarm if `ser_x(A_send)` repeats across heights within its own sequence (cross-front repeats are detected retrospectively from the persisted `a_send_x`, MULTI-REGION §3).
- **Erasure vs deferred Ark payouts.** Ark sends happen `confirmation_depth` (6) blocks after the find, long after `a_send[H]` is erased at the next `SetNewPrevHash` (§11.3), so `front` MUST derive each Ark-capable id's Ark destination (`U_i` and its blinded mailbox id, §11.8) while `a_send[H]` is live, and put it in the distribution entries (§4.7), which travel in the `spc:blocks` event (§3.3). Only those public points are retained — never `ecdh_i`, `t_onchain` or `t_ark`: a stored tweak plus one leaked per-output secret (`u_i` or `p_i`) reveals the miner's base key (`b_ark` or `b_spend`).

### 2.3 Output derivation (pool → miner `i`, height `H`)

Normative per DESIGN.md "Derivation scheme". One shared secret per `(miner, H)`; every per-block destination on every tier is derived from it, domain-separated by hash tag alone (no extra tier bytes are hashed):

```
input_hash = hash_SPc/Inputs(ser32(H) ‖ ser_P(A_send))                 # ser_P(A_send) = 0x02 ‖ ser_x (even Y, §2.2)
ecdh_i     = ser_P(input_hash · a_send · B_scan,i)                      # pool side
           = ser_P(input_hash · b_scan,i · A_send)                      # miner side
t_onchain  = hash_SPc/TierSpc(ecdh_i ‖ ser32(k))      → P_i = B_spend,i + t_onchain·G    (coinbase tier)
t_ark      = hash_SPc/TierArk(ecdh_i ‖ ser32(k))      → U_i = B_ark,i   + t_ark·G        (Ark tier, §11.8)
output_i   = P2TR(ser_x(P_i)), amount = pay_i                                             # scriptPubKey: OP_1 <32 B>
```

- **Hash-to-scalar fails; it never reduces.** If `input_hash`, `t_onchain` or `t_ark` is 0 or `≥ n`, or any point result is infinity, derivation fails (BIP 352 semantics). The probability is negligible. Handling: `input_hash` is per-height, so on its failure `front` discards `a_send[H]` and draws a new one (§2.2); a tier tweak or point failure is per-id, so that id gets nothing on that tier at `H` and its `owed` is booked as pending. Never substitute a value.
- **`SPc/SharedSecret` is reserved** for a 32-byte digest of `ecdh_i` (`SharedSecret::digest`) and is **not** a tier-tweak input. `SPc/Mailbox` is the third tier tag (§11.8, MAILBOX.md variant B); `SPc/ArkIdentity` is the message tag for the `b_scan` signature on Ark registrations (§11.4).
- **Never reuse a scalar across tiers.** If `P = B_spend + t·G` and `U = B_ark + t·G` ever coexisted, `P − U` would expose `B_ark − B_spend`. The tags prevent it; do not shortcut them.

Design notes, each with its source:
- **Nonce = height.** Replaces `outpoint_L`, which is constant across all coinbases. Shared, non-circular, consensus-unique by BIP 34, and no gap limit applies because the receiver computes one candidate for the height in front of it rather than scanning forward ([08-14 L1–L2](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md)).
- **`input_hash` commits to `A_send`.** This, not the nonce, is what defeats the `why_include_A` grinding attack (BIP 352 footnote; [08-24 L2](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)). The pool chooses `a_send` freely every height, so it has exactly the freedom the attack needs — the commitment removes it.
- **Fresh `SPc/*` tags (DESIGN.md; supersedes the `BIP0352/*` reuse decision).** Domain separation is by tag, so no SPc value can collide with a BIP 352 value computed over the same bytes. The cost is that an unmodified BIP 352 library cannot compute `t_onchain`; whether `t_onchain` should instead use the upstream SPc/BIP 352 tag so a single-tier SPc scanner finds coinbase outputs is open item O12, and depends on the SPc BIP.
- **`k = 0` in v0.1, on every tier.** Each miner has a distinct `B_scan`, so each is a BIP 352 group of size one; the accounting gives one output per `(sp_id, H)` (§4.6) and puts an id on exactly one tier per block (§11.1), so `k` never increments, `K_max` is irrelevant, and the BIP 352 `:319` dropped-output footgun has nothing to bite ([08-14 L5, L7](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md)). `ser32(k)` stays in the preimage so a later version can pay several outputs per tier without a new tag.
- **No labels in v0.1.** A miner wanting several identities uses several addresses.

### 2.4 Carriers

**2.4.1 scriptSig (pool-template mode).** The coinbase input script MUST be exactly:

```
<BIP34 height push>  OP_PUSHBYTES_32 <ser_x(A_send)>  <extranonce push>
```

- Placed in `coinbase_tx_prefix` (SV2 §5.4.1.6: prefix ‖ extranonce_prefix ‖ extranonce ‖ suffix), so the key is **structurally unrollable** — the wire format, not discipline, keeps it out of the rolled bytes ([08-16 L5](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md)).
- Budget: `100 − 4 (height, 5 past 16,777,215) − 33 (push32) = 63 B` for the full extranonce; default allocation 16 B leaves ~4× headroom. No ASCII pool tag in v0.1 — `A_send` **is** the tag ("tag == entry H of pool X's published key" is unforgeable attribution). The 100 B / 400 WU are reserved by the Template Provider unconditionally, so marginal on-chain cost is 0.
- Implementation: `channels-sv2::JobFactory.pool_tag_string: Option<String>` becomes `pool_tag: Option<Vec<u8>>` in the vendored fork (the UTF-8 constraint is an SRI type, not protocol).

**2.4.2 `OP_RETURN` (JD mode).** The pool's `SetPayoutDistribution.additional_outputs[0]` MUST be a 0-value output with:

```
OP_RETURN  OP_PUSHBYTES_35  0x53 0x50 0x63 <ser_x(A_send)>        # "SPc" ‖ 32 B
```

Costs 46 B = 184 WU of real block space per block (8 value + 1 length + 37 script; not pre-paid — stated honestly; a `0x0003` JDC reserves exactly this TxOut size in `CoinbaseOutputConstraints`). Required because Coinbase-only JD carries no scriptSig bytes and Full-Template gives the pool approval but not specification authority over the scriptSig ([08-24 L3](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)). Pool-template mode MUST NOT emit the `OP_RETURN` (it would be a protocol fingerprint with no benefit).

### 2.5 Scanning (miner side; `spc-scan`)

```
for each block (header, coinbase, merkle_branch):
    REQUIRE merkle_branch proves coinbase at index 0 under header.merkle_root
    H  = parse_bip34_height(coinbase.vin[0].scriptSig)
    A  = scriptsig_push32_after_height(coinbase)            # carrier 1
         ?? op_return_SPc_payload(coinbase.vout)             # carrier 2
    if A is None: skip block (not SPc)
    for h in {H, H − 1}:                                     # tolerance, §3.5 R4
        input_hash = hash_SPc/Inputs(ser32(h) ‖ 0x02 ‖ A)
        ecdh = ser_P(input_hash · b_scan · lift_x_even(A))
        t    = hash_SPc/TierSpc(ecdh ‖ ser32(0))
        P    = B_spend + t·G
        if OP_1 <ser_x(P)> ∈ coinbase.vout: record (txid, vout, amount, t)
    if Ark tier enabled:                                     # DESIGN.md "Miner can scan both tiers"; h = H only —
        ecdh  = (as above, at h = H)                         #   the pool derives Ark destinations at the mined height
        t_ark = hash_SPc/TierArk(ecdh ‖ ser32(0))
        keystore.insert(U = B_ark + t_ark·G)                 # §11.5; u = b_ark + t_ark when the signer is present
        mailbox.open_epoch(H, ecdh)                          # §11.8 mailbox mode; closed at once if P was found above
```

The Ark tier is scanned for **every** block the pool found, including blocks where this miner got no coinbase output — that is how a sub-threshold miner learns which Ark keys to expect without any notification.

Light-client cost: header 80 B + branch (~12 × 32 B) + coinbase + 32 B ≈ 500 B/block, provable against the header ([08-14 L6](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md), in-session arithmetic, medium confidence). Spending: `d = b_spend + t (mod n)`, negated if `P` has odd Y, exactly as BIP 352 §Spending. `spc-scan --export-tweaks` emits `(outpoint, t)` pairs for wallets that can import tweaks (BIP 352 §"Backup and Recovery" caveat: outputs found by notification but not by derivation are unsafe — ours **are** derived, so this is safe).

### 2.6 What this section does not do

Hides amounts (no), hides the payee set from the pool (no), hides the coinbase contents from the miner hashing it (impossible: outputs follow the extranonce in serialization order, [08-24 L4](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)).

## 3. Stratum V2 surface

### 3.1 Channel open and identity (fail-closed)

On `OpenStandardMiningChannel` / `OpenExtendedMiningChannel`, `front` runs §2.1 on `user_identity` (`STR0_255`).

| Outcome | Response |
|---|---|
| valid SP address for the pool's network | `.Success`; `channel_id → sp_id` recorded; `sp_identity` upserted via `spc:stats` |
| anything else (bare bc1q…, descriptor, `sri/solo/…`, wrong HRP, bad checksum, empty) | `OpenMiningChannel.Error{ error_code: "invalid-user-identity" }` (`STR0_255`), connection kept open so the client can retry |

There is **no default payout mode and no donation fallback.** This is the direct consequence of the sv2-apps rotation branch, where an unrecognised identity became `PayoutMode::FullDonation` — the whole block to the pool, no log line ([xpub spec rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md)). An integration test asserts the error and asserts no coinbase was ever built for that channel.

One connection = one `sp_id`. Extended channels on the same connection MUST present the same address (differing addresses on one connection → `invalid-user-identity`), because SRI's payout slot is per connection and group jobs never rebuild the coinbase per channel ([xpub spec rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md)). Worker labels (`.worker`) are stats-only.

Channel open establishes **identity only**. Delivery capabilities (§0) are registered separately, over the authenticated session, by the SPc extension (§11.4). A channel with none is valid, but under the pool's published policy its earnings below `C` are **forfeited** (§11.1); only balances at or above `C` are paid, in the coinbase. The policy is advertised in the `0x5350` negotiation response and at `/api/pool`, and `spc-agent` does not start submitting shares until `SetArkIdentity.Success` (§11.5), so the default setup never forfeits.

### 3.2 Extension negotiation

Clients that want JD payouts or auditing send `RequestExtensions{request_id: U16, requested_extensions: SEQ0_64K[U16]}` (ext `0x0001`) immediately after `SetupConnection.Success`. `spc-pool` advertises:

| ID | Extension | Mandatory for |
|---|---|---|
| `0x0001` | extensions negotiation | everyone (`spc-translator`, `spc-jdc`) |
| `0x0003` | Coinbase Transaction Payouts (PR #203) | `spc-jdc`, on **both** its JDP and MP connections |
| `0x0020` (32) | Share Accounting (§3.7) | optional; `spc-translator` and `spc-jdc` request it by default |
| `0x5350` | SPc delivery capabilities (§11.4) | optional; `spc-agent` requests it when its Ark tier is enabled |

A client whose `RequestExtensions.Error` lists `0x0003` MUST NOT run JD mode (PR #203 §2).

### 3.3 Pool-template mode: job construction

Per `NewTemplate` (and per `SetNewPrevHash`) `front`:

1. Looks up `a_send[H]` (§2.2).
2. Asks `spc-accounting` for `Distribution(H, T = coinbase_tx_value_remaining, A_send)` (§4.7) → ordered outputs `[pool_output, P2TR(P_i)…]`.
3. Builds `coinbase_tx_prefix = version ‖ vin_count ‖ null_prevout ‖ scriptSig_len ‖ height_push ‖ push32(A_send)` and `coinbase_tx_suffix = sequence ‖ outputs ‖ locktime`, honouring `coinbase_output_max_additional_size` reserved from the TP (`spc-pool` sets it from the current weight budget, §4.8, Blitzpool's per-stream reservation).
4. Sends `NewExtendedMiningJob` / `NewMiningJob`. Standard channels receive the merkle root only; they cannot verify payouts (§3.6).
5. XADDs a `spc:blocks` event `{block_hash, height, front_id, fingerprint, high_water, entries}` where `high_water` is the window's high-water vector (§4.3) and `entries` is the winning distribution's full entry list, which the finding front holds in RAM for the jobs it issued. Settlement reads the event payload; no Redis snapshot key. The fingerprint (sha256 of distribution) serves for deduplication/idempotency.

Distribution recomputation is bounded: weights change with every share, but the output **keys** `P_i` are fixed per `(sp_id, H)`, so only amounts move. `front` rebuilds amounts on each template (every ~30 s) and never per share.

### 3.4 Share handling

Validate as SRI does (target, job, ntime, version mask), then:

```
key = sha256(channel_id ‖ job_id ‖ nonce ‖ ntime ‖ version ‖ extranonce)
REQUIRE key ∉ seen_shares                         # replay; identity-free nullifier (wiki: Nullifier vs Pseudonym)
rec = ShareRec{ sp_id, d: share_difficulty, job_fees, job_id, nonce, ntime, version, extranonce, share_index }
open_slice.push(rec)                              # §4.2 sealing rules may seal first
reply SubmitShares.Success (or ShareOk{ref_job_id, share_index} if 0x0020 is active)
```

`seen_shares` is per process and is lost on restart; `front` MUST reject shares whose `job_id` predates process start (jobs are re-issued on start), which bounds the replay window to zero at the cost of a few lost shares ([share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md)).

### 3.5 JD mode (PR #203, extension `0x0003`)

**Publication.** `spc-jds` sends `SetPayoutDistribution` as the first message after negotiation and again:
- **always** on `SetNewPrevHash` (new `H` ⇒ new `A_send`, new `P_i`), and
- on a timer (`distribution_interval_secs`, default 60) when weights moved by more than `distribution_rebalance_bps` (default 100 = 1 %), and
- on **ledger apply** (booking at `confirmation_depth`, §7.4): force a republish regardless of `distribution_rebalance_bps` so pending credits reach `payouts` immediately, and drop the grace slot — the immediately preceding id FROM THE SAME FRONT on that connection is rejected after this republish (PR #203 §10: the grace window SHOULD NOT span a settlement event; Blitzpool's `invalidate_all_distributions()` + forced publish does exactly this at booking, not at found-time). Found-time invalidation is already satisfied by the `SetNewPrevHash` bullet plus the same-height rule: every id published before the pool's own block belongs to the previous height and is rejected.

```
SetPayoutDistribution {
  distribution_id:    U64   strictly increasing, per front: `id = front_id << 56 | local_seq`, `local_seq = max(last_local_seq + 1, unix_time_ms)`
  pool_payout:        TxOut{ script: P2TR(pool_payout_descriptor @ H), value: weight_P }   # weight, not sats; fresh per height
  payouts:            [ TxOut{ script: OP_1 ser_x(P_i), value: weight_i } ] # one per sp_id kept by §4.8
  dust_limits:        [ 330 … ]                                             # per payouts[i]; P2TR dust
  additional_outputs: [ TxOut{ script: OP_RETURN "SPc" ser_x(A_send), value: 0 } ]
}
```

`weight_P` = pool fee weight **plus** the weight of every identity not represented in `payouts` (below `C`, trimmed), per PR #203 §3.1; `payout` books those per §11.1 (Ark tier or pending) when the block settles.

**Validation (`spc-jds` and `front`).** On `DeclareMiningJob` (Full-Template) or `SetCustomMiningJob` (Coinbase-only) carrying the `distribution_id` TLV (`0x0003`/`0x01`, 8 B): recompute `pay[i] = floor(weight_i · (T − F) / W)`, prune `< dust_limits[i]`, `pay_P = T − Σ pay[i] − F`, assert output order `pool_payout, payouts…, additional_outputs, JDC 0-value outputs, TP outputs`; mismatch → `invalid-payout-distribution`. Unknown/old id → `stale-payout-distribution`.

**Grace window (R4 in the roadmap).** `spc-jds` MAY accept the immediately preceding id FROM THE SAME FRONT on that connection **only if it was published at the same height `H` by the same front on the same connection**. A declaration built for `H` and accepted at `H + 1` would pay outputs nobody can scan at `H + 1` (`input_hash` differs). Height-straddling declarations MUST be rejected with `stale-payout-distribution`. `spc-scan`'s `{H, H − 1}` tolerance is belt-and-braces for third-party pools, not a license to relax this rule. Ids are not comparable across fronts.

**Shares under JD.** Carry `job_fees` from the declared job's template value into `ShareRec`; this is where `score_f` becomes non-trivial (§4.4).

**Leak, stated.** Every JDC sees `payouts` (one-time `P_i` scripts, no identity) and relative weights. Weights are derivable from on-chain amounts anyway; the residue a retaining JDC gains is distributions for blocks nobody won and the sub-dust set (PR #203 §9.2; [08-24 L5](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)). `max_payout_slots` (default 256) is the only dial.

### 3.6 Client-side payout verification (`spc-translator`, `spc-jdc`)

Both clients know `(b_scan or B_scan, B_spend)`, `H`, and `A_send` (from the job's `coinbase_tx_prefix` or the distribution's `OP_RETURN`). Before mining a job they MUST derive their own `P` (§2.3, receiver side) and check it appears among the coinbase outputs (extended channel) or `payouts` (JD). Result handling is `require_own_payout = warn | refuse` (default `warn`, because a small miner is legitimately below the coinbase threshold, dust-pruned or trimmed some blocks). This is the "pre-block payout verification via the existing encrypted session" the wiki lists as engineering-solvable ([08-24 L6](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)), and it is also how `spc-translator` detects at startup that it is pointed at an SPc pool at all (no `push32` after the height ⇒ abort with a loud error).

### 3.7 Share Accounting extension (type 32 = `0x0020`)

Served by `front` from the slice store (§5), message set and semantics per `extension.md` @ c661e93 with the **documented deltas** below (`docs/share-accounting-ext-deltas.md`). `front` serves type-32 for any pool block from share blobs obtained via the `payout` bootstrap RPC (WIRE-FORMATS §3.5 `type32_blobs`) plus the live `spc:slices` tail; there is no other cross-front share-blob path.

| Item | upstream | silent-pool |
|---|---|---|
| activation | `Activate`/`Activate.Success` (handshake TODO, sv2-spec #95) | negotiated via `0x0001`; `Activate` accepted as a no-op for compatibility |
| codec | demand fork of `roles_logic_sv2` | current `stratum-core` (`binary-sv2`, `codec-sv2`) |
| type names | `GetTransationsInJob`, `ProvideMissinTransactions` | spelled correctly; wire bytes unchanged |
| `Slice.job_id` | "last 4 bytes MUST be the same as the job's" | `job_id = (H << 32) | mining_job_id`, satisfies the rule |
| window older than `retain_windows` | unspecified | `GetWindow.Busy{retry_in_seconds: 0}` means *gone*; documented |

Server behaviour: `NewBlockFound{block_hash}` broadcast on every pool block; `GetWindow` → slices + phashes for the window (§4.3); `GetShares` → `Share{nonce, ntime, version, extranonce, job_id, reference_job_id, share_index, merkle_path}` from the slice's share blob; `GetTransactionsInJob` → pool's tx set (pool-template) or the JDC's declared set (JD). `GetWindow.Busy{retry_in_seconds}` for windows older than `window_hot_blocks` (default 6).

Client verifier (shipped in both clients and as `spc-scan audit`): sample `s` slices × `m` shares (defaults 4 × 8), check PoW, `merkle_path(share) + share_hash == slice.root`, `Σ d ≤ slice.difficulty`, `share.job_fees ≤ slice.fees + δ`. Statistical, not total — a cheating pool is caught over windows, not per window ([SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md)).

## 4. Accounting (normative; `spc-accounting`, pure functions)

### 4.1 Parameters

| Name | Default | Meaning |
|---|---|---|
| `N` | 8 | window target `= N × D`, `D` = network difficulty at the found block (TIDES and SLICE both use 8, [PPLNS-JD](research/wiki/concepts/pplns-jd.md)) |
| `δ` (`slice_fee_delta_sats`) | 0 in pool-template mode; 50 000 under JD | new slice when a share's job fees `≥ f_ref + δ` |
| `max_slice_shares` | 65 000 | hard seal (keeps `GetShares` payloads bounded) |
| `fee_bps` | 100 | pool fee, taken from both subsidy and fees |
| `dust_limit_sats` | 330 | P2TR dust |
| `coinbase_threshold_sats` (`C`) | = `dust_limit_sats` | minimum `owed` for a coinbase output; below it the id goes to the Ark tier, or is forfeited if it has no Ark registration (§11.1). MUST be `≥ dust_limit_sats`; raising it keeps the coinbase smaller at the cost of more Ark-tier payouts (custodial for the confirmation window, then trustodial) |
| `coinbase_weight_budget` floor / ceiling | 50 000 / 400 000 WU | autoscaler bounds |
| `max_payout_slots` | 256 | hard cap on miner outputs (JD leak dial, firmware ceilings) |
| `retain_windows` | 16 blocks | slice retention for type-32 audits |
| `scrub_after_blocks` | 100 | delete `sp_id → P_i` / `U_i` snapshot entries this many blocks after the block is fully settled — booked **and**, for Ark-tier entries, sent (§11.3) |

### 4.2 Slices

```
ShareRec { sp_id, d: u64, job_fees: u64, job_id: u64, nonce, ntime, version, extranonce: [u8;≤32], share_index: u32 }
Slice    { prev_hash, H, ref_job_id, ref_fees, shares: Vec<ShareRec>, difficulty: u128, root: [u8;32] }

seal_if_needed(open, incoming):
    if incoming.prev_hash != open.prev_hash        → seal (new H)
    or incoming.job_fees ≥ open.ref_fees + δ       → seal (fee band exhausted; lorbax: MMEF only grows within a round)
    or open.shares.len() == max_slice_shares       → seal
seal(open): open.root = merkle_root([share_leaf(s) for s in shares]); emit to spc:slices
share_leaf(s) = sha256d(ser(nonce ‖ ntime ‖ version ‖ extranonce ‖ job_id ‖ share_index))   # identity-free
```

Leaves contain no `sp_id`, so `GetShares` responses and retained share blobs carry no identity (the extension's types have zero identity fields — [SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md)). Identity lives only in the per-slice aggregates (§5.1 `slice_credit`), which are what get scrubbed.

### 4.3 Window

```
select_window(slices_desc_from_finding_slice, D):
    skip the slice containing the block-finding share            # excluded from reward (extension.md)
    take whole slices while Σ difficulty < N × D; include the slice that crosses the threshold
    t = slices taken
    high_water = {front_id: last_slice_stream_id} for each front whose slices are in the window    # settlement and the type-32 window reconstruct exactly the slices at or below the vector; fork-ineligibility per MULTI-REGION §1 stale-fork rule
```

Whole-slice granularity makes the window slightly larger than `N × D`; this matches `extension.md`'s definition and keeps `GetWindow` exact.

### 4.4 Scores and payout (lorbax formula)

For the found block with net subsidy `r_net = r − ⌊r·fee_bps/10⁴⌋` and net fees `F_net = F − ⌊F·fee_bps/10⁴⌋`, over window slices `S₁…S_t`:

```
D_win        = Σ_{s ∈ window} d_s
DF_S         = Σ_{s ∈ S} d_s · f_s                         (per slice)
raw(id)      = ⌊ r_net · Σ_{s∈id} d_s / D_win ⌋
             + Σ_S ⌊ (F_net / t) · Σ_{s∈id, s∈S} d_s·f_s / DF_S ⌋
```

Equivalently `payout(sᵢ) = r·score_d(sᵢ) + F_S·score_f(sᵢ)` with `F_S = F/t`, `score_d = dᵢ/D_win`, `score_f = dᵢfᵢ/DF_S` (lorbax; gap research). All arithmetic in `u128`; floors per term; remainders fall to the pool output. **Pool-template collapse**: with `δ = 0` every share in a slice has `f_s = f_ref`, so `score_f ≡ dᵢ/D_S` and the fee term becomes a difficulty-weighted split of `F_net` — plain PPLNS, same code path.

### 4.5 Worked example

`r = 312 500 000`, `F = 20 000 000`, `T = 332 500 000`, `fee_bps = 100` → `r_net = 309 375 000`, `F_net = 19 800 000`. Window: 2 slices, `F_S = 9 900 000`.

| slice | ref fees | Alice (d, f) | Bob (d, f) | Carol (d, f) | `DF_S` |
|---|---|---|---|---|---|
| S₁ | 15 M | 60, 15 M | 40, 15 M | — | 1 500 M |
| S₂ | 18 M | 50, 18 M | 30, 18 M | 20, 19 M (JD, picked richer txs) | 1 820 M |

`D_win = 200`.

| miner | subsidy term | S₁ fee term | S₂ fee term | `raw` |
|---|---|---|---|---|
| Alice | ⌊309 375 000·110/200⌋ = 170 156 250 | ⌊9.9 M·900/1500⌋ = 5 940 000 | ⌊9.9 M·900/1820⌋ = 4 895 604 | **180 991 854** |
| Bob | 108 281 250 | 3 960 000 | ⌊9.9 M·540/1820⌋ = 2 937 362 | **115 178 612** |
| Carol | 30 937 500 | 0 | ⌊9.9 M·380/1820⌋ = 2 067 032 | **33 004 532** |

`Σ raw = 329 174 998`; `pool = T − Σ = 3 325 002` (= 1 % fee 3 325 000 + 2 sats rounding). Carol's S₂ fee share (20.9 %) exceeds her S₂ difficulty share (20 %) because she declared higher-fee transactions — the SLICE property. This table is a fixture in `spc-accounting/tests/worked_example.rs`.

### 4.6 Pending ledger (Blitzpool `bp-pplns` pattern, re-implemented)

```
owed(id)  = raw(id) + pending(id)                      # pending ≥ 0 (no finder bonus ⇒ no debits; unsigned)
kept      = { id : owed(id) ≥ C }                      # C = coinbase_threshold_sats ≥ dust_limit_sats
kept      = trim_to_budget(kept, weight_budget, max_payout_slots)      # §4.8
for id in kept:    pay(id) = owed(id); pending'(id) = 0
for id not kept:   pay(id) = 0;        owed(id) routed by §11.1 to ark_due', pending' or forfeited
pay_P     = T − Σ pay(id)                              # fee + rounding + forfeited + sats backing every held' balance
```

`held' = pending' + ark_due'`: everything the pool owes a miner after this block that is not in the coinbase. `forfeited` is not owed to anyone; it is pool revenue (§11.1). §11.1 decides where each non-kept id lands; this section only decides who gets a coinbase output.

**Custody statement.** Sats behind `held'` land in `pay_P` and are owed by the pool until a later coinbase (pending) or the Ark send six blocks later (ark_due) pays them (PR #203 §3.1/§9.3 says the same of `weight_P`). This is where silent-pool is custodial (§0), bounded by roughly `trimmed set + Ark-tier sats of the blocks inside the confirmation window + failed sends`. It is reported at `/api/pool/liability`.

**Invariants (proptested).** (a) `Σ pay + pay_P = T`. (b) one output per `(sp_id, H)` — `kept` is a set keyed by `sp_id`, so Blitzpool's missing duplicate-address merge ([finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) §8) cannot recur. (c) `Σ held' + Σ forfeited − Σ pending = Σ_{not kept} raw − Σ_{kept} pending`. (d) idempotent apply: re-applying a settled block changes nothing (§5.1 `ledger_history` PK). (e) an id is on exactly one rail per block: `kept`, `ark_due'`, `pending'` and `forfeited` are disjoint for that block. (f) `forfeited(id) > 0` only if `id` has no `ark_delivery` row in Postgres at settlement and `owed(id) < C`.

**Settlement reads the mined coinbase, never the snapshot's sats.** `payout` books from the block's actual coinbase: `T_actual = Σ coinbase outputs`; `paid(id)` = value of the output whose script is `OP_1 ser_x(P_id)` from `entries` (0 if absent). Pool-template mode: `T_actual = T` and `paid(id) = pay(id)` by construction, so nothing changes. JD mode: the JDC paid `floor(owed(id) · (T_actual − F) / W)` at **its** template revenue (PR #203 §4) and may have dust-pruned an id the snapshot kept, so the mined sats differ from `entries[].sats` on every JD block whose `T_actual ≠ T`; `pending'(id)` MUST be derived from `paid(id)`, never copied from the snapshot. The claim is `claim(id) = floor(owed(id) · (T_actual − F) / W)` (revenue-scaled; resolved 2026-10-14, `docs/decisions/2026-10-14-o11-claim-formula.md`), so `pending'(id) = claim(id) − paid(id)` keeps the ledger unsigned and matches pool-template mode's scaling. Blitzpool books `claim(T_actual) − paid` per address from the mined coinbase (`bp-pplns-engine` `on_block_found`, `ActualCoinbase`) for the same reason; the §11.6 miner audit closes to zero only under this rule.

### 4.7 Distribution builder

```
Distribution(H, T, a_send):
    (raw, pending) ← §4.4–4.6 on the current window-in-progress   # the window ending at "now"
    for id in window:  ecdh_id = ecdh(a_send, H, B_scan,id)                 # §2.3; cached per (id, H), §7.3
    for id in kept:    P_id = B_spend,id + t_onchain(ecdh_id)·G
    for id in ark_capable:                                                 # registered Ark delivery, §11.4
                       U_id = B_ark,id + t_ark(ecdh_id)·G; dest_id = ark_destination(U_id, ecdh_id)   # §11.8
    outputs = [ P2TR(pool_payout_descriptor @ H, pay_P) ] ++ [ P2TR(P_id, pay(id)) sorted by sp_id ] ++ carrier (JD only)
    fingerprint = sha256(ser32(H) ‖ ser_x(A_send) ‖ Σ ser(outputs))
    high_water = {front_id: last_slice_stream_id for each front whose slices are in the window}
    entries = [(sp_id, rail, ser_x(P_id) | dest_id, pay(id) | owed(id))]    # snapshot payload; scrubbed later
```

`dest_id` (the `U_i` policy key plus its delivery: blinded mailbox id, or empty) is computed here because this is the last moment `a_send[H]` exists (§2.2). Fingerprint covers the coinbase outputs only; Ark destinations ride along in `entries`. `dest_id` is computed for every Ark-capable id, kept or not, because which rail an id lands on is decided at settlement from the mined coinbase (§4.6) — under JD the JDC may dust-prune an id the snapshot kept.

In JD mode the same function emits `weights` instead of sats (`weight_id = owed(id)`, `weight_P = fee_weight + Σ_{not kept} owed`) and lets the JDC compute amounts per PR #203 §4 — the two modes share everything except the final scaling.

### 4.8 Weight budget and autoscaler (ported from Blitzpool)

P2TR output = 43 B = 172 WU. `trim_to_budget` sorts `kept` by `owed` descending, keeps outputs while `172 × n + overhead ≤ budget` and `n ≤ max_payout_slots`, trims the rest to pending — so an under-sized budget costs **fairness** (a small miner waits a block), never **validity** (the coinbase never exceeds the TP reservation). Autoscaler per template: `utilisation = used/budget`; step `×1.15` when `≥ 0.85` for `up_debounce` (3) consecutive templates; step `÷1.15` when `≤ 0.50` for `down_debounce` (20); `cooldown_secs` 300; clamp to floor/ceiling; on change send new `CoinbaseOutputConstraints` to the TP (pool-template) or a new `SetPayoutDistribution` (JD). Capacity alert fires one step before trimming would start.

## 5. Data model

### 5.1 Postgres (written only by `payout`; migrations via `sqlx`, advisory-locked at boot)

```sql
CREATE TABLE sp_identity (
  sp_id       BYTEA PRIMARY KEY,                 -- 32 B, §2.1
  b_scan      BYTEA NOT NULL CHECK (octet_length(b_scan) = 33),
  b_spend     BYTEA NOT NULL CHECK (octet_length(b_spend) = 33),
  network     SMALLINT NOT NULL,                 -- 0 mainnet, 1 testnet4, 2 signet, 3 regtest
  first_seen  TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_seen   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ark_delivery (                      -- a delivery capability (§0), NOT identity; at most one per sp_id
  sp_id        BYTEA PRIMARY KEY REFERENCES sp_identity,
  ark_id       BYTEA NOT NULL,                   -- the configured server it was registered against
  b_ark        BYTEA NOT NULL CHECK (octet_length(b_ark) = 33),        -- MUST differ from sp_identity.b_spend
  mailbox_mode TEXT NOT NULL CHECK (mailbox_mode IN ('none','static','epoch_derived','epoch_registered')),
  mailbox_m    BYTEA,                            -- M; NULL iff mailbox_mode = 'none'
  mailbox_d    BYTEA,                            -- D = b_ark·S_mb; NULL iff mailbox_mode = 'none'
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
  -- CHECK ((mailbox_mode = 'none') = (mailbox_m IS NULL AND mailbox_d IS NULL))
);

CREATE TABLE ark_epoch_mailbox (                 -- MAILBOX.md variant A only
  sp_id      BYTEA REFERENCES ark_delivery ON DELETE CASCADE,
  height     INTEGER NOT NULL,                   -- epoch i
  mailbox_mi BYTEA NOT NULL,                     -- M_i
  PRIMARY KEY (sp_id, height)
);

CREATE TABLE slice (
  slice_id    BIGINT PRIMARY KEY,                -- `front_id << 56 | local_seq` (as distribution_id, §3.5): monotone per front, unique globally
  height      INTEGER NOT NULL,
  prev_hash   BYTEA NOT NULL,
  ref_job_id  BIGINT NOT NULL,
  ref_fees    BIGINT NOT NULL,
  n_shares    INTEGER NOT NULL,
  difficulty  NUMERIC(40,0) NOT NULL,            -- u128
  root        BYTEA NOT NULL,
  shares      BYTEA NOT NULL,                    -- identity-free compact blob for GetShares (§4.2)
  sealed_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX slice_height_idx ON slice (height DESC, slice_id DESC);

CREATE TABLE slice_credit (                      -- the only share→identity link; scrub target
  slice_id    BIGINT REFERENCES slice ON DELETE CASCADE,
  sp_id       BYTEA REFERENCES sp_identity,
  diff_sum    NUMERIC(40,0) NOT NULL,            -- Σ d
  diff_fee_sum NUMERIC(60,0) NOT NULL,           -- Σ d·f
  PRIMARY KEY (slice_id, sp_id)
);

CREATE TABLE distribution_snapshot (
  fingerprint     BYTEA PRIMARY KEY,
  height          INTEGER NOT NULL,
  a_send_x        BYTEA NOT NULL,                -- public; never the secret
  distribution_id BIGINT,                        -- JD only
  entries         JSONB NOT NULL,                -- [{sp_id, rail, p_x | ark_dest, sats}] (§4.7); scrubbed to [] per §5.1 note
  high_water      JSONB NOT NULL,                -- {front_id: last_slice_stream_id} (§4.3)
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE block_found (
  block_hash   BYTEA PRIMARY KEY,
  height       INTEGER NOT NULL,
  fingerprint  BYTEA REFERENCES distribution_snapshot, -- NULL only while the found-block event is lost (§7.4)
  status       TEXT NOT NULL CHECK (status IN ('pending','confirmed','orphaned')),
  found_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  settled_at   TIMESTAMPTZ,
  CHECK (status <> 'confirmed' OR fingerprint IS NOT NULL)
);

CREATE TABLE ledger_history (                    -- idempotency + accounting; no scripts
  block_hash   BYTEA REFERENCES block_found,
  sp_id        BYTEA REFERENCES sp_identity,
  rail         TEXT NOT NULL CHECK (rail IN ('coinbase','ark','pending','forfeit')),   -- exactly one per block, §4.6 (e)
  paid_sats    BIGINT NOT NULL,                  -- coinbase sats mined to P_id; 0 on the other rails
  ark_due_sats BIGINT NOT NULL DEFAULT 0,        -- scheduled for the Ark send at H_f + confirmation_depth, §11.3
  forfeited_sats BIGINT NOT NULL DEFAULT 0,      -- unregistered and below C, §11.1; pool revenue
  pending_delta BIGINT NOT NULL,
  PRIMARY KEY (block_hash, sp_id)
);

CREATE TABLE ark_payout (                        -- one per Ark-rail ledger row; drives the send at H_f + confirmation_depth
  block_hash    BYTEA,
  sp_id         BYTEA,
  sats          BIGINT NOT NULL,
  due_height    INTEGER NOT NULL,                -- found height + confirmation_depth (6)
  status        TEXT NOT NULL CHECK (status IN ('scheduled','sent','failed_to_pending')),
  vtxo_id       BYTEA,                           -- set on 'sent'; scrub target
  expiry_height INTEGER,
  PRIMARY KEY (block_hash, sp_id),
  FOREIGN KEY (block_hash, sp_id) REFERENCES ledger_history
);

CREATE TABLE pending_balance (
  sp_id      BYTEA PRIMARY KEY REFERENCES sp_identity,
  sats       BIGINT NOT NULL CHECK (sats >= 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Nothing here is a per-share row ([share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md)). The attribution surface is exactly `slice_credit` + `distribution_snapshot.entries` + `ark_payout.vtxo_id`; the **scrub job** deletes `slice_credit` rows for slices older than the oldest unsettled window + `retain_windows`, and empties `entries` (and nulls `vtxo_id`) for blocks fully settled — booked, and every `ark_payout` row out of `scheduled` — more than `scrub_after_blocks` ago. Ark-tier entries live until their send completes (normally `H_f + 6`, longer on retries): the Ark destination cannot be re-derived once `a_send` is gone (§2.2). `ark_delivery` is not scrubbed; it is live configuration, deleted when the miner withdraws it. `ledger_history` keeps `(block_hash, sp_id, rail, sats)` for idempotency and pending reconciliation but never a script or key, so a DB dump after scrub links identities to **amounts and tiers**, not to **outputs or VTXOs** — the residual leak is the amount channel, which exists on-chain anyway (§0).

### 5.2 Redis

| Key / stream | Producer → consumer | Content |
|---|---|---|
| `spc:slices` (stream) | front → payout | sealed `Slice` + its per-identity aggregates |
| `spc:blocks` (stream) | front → payout | `{block_hash, height, front_id, fingerprint, high_water, entries}` on `SubmitSolution` accepted |
| `spc:stats` (stream) | front → payout/api | per-minute `{sp_id, accepted_d, rejected, hashrate_est}` — never per share |
| `spc:arkreg` (stream) | front → payout | `{sp_id, b_scan, b_spend, ark_id, b_ark, mailbox_mode, M, D, epoch_fallback, sig}` (registration) |
| `spc:front:events` (stream) | payout → all fronts | `ArkRegApplied{sp_id, rev}`, `LedgerApplied{block_hash, pending_deltas, republish}`, `BlockFound{block_hash, height, front_id, provisional_paids, high_water}`, `TipHeartbeat{hash, height}` |
| ~~`spc:dist:current`~~ | intra-`front` state since 2026-10-13 | current `distribution_id`, `H` (JD validation); was a stream when the JDS was a separate process |
| `spc:ark:orders` (stream) | payout → ark | signed `ArkSendOrder` (§11.3) — no identity fields. Redis ACL: `ark` may read only this stream and write only `spc:ark:receipts` |
| `spc:ark:receipts` (stream) | ark → payout | `ArkSendReceipt` keyed by opaque `order_id` |

Consumer groups give at-least-once delivery; `ledger_history`'s PK makes apply idempotent, so redelivery is harmless (Blitzpool's exactly-once-by-idempotency).

**Delivery semantics.** `payout` consumes `spc:slices`, `spc:blocks`, `spc:arkreg` via consumer group (at-least-once; idempotent apply via `ledger_history` PK). Fronts live-tail `spc:slices` and `spc:front:events` via plain XREAD with RAM-only cursors re-primed at start. Trim discipline: `XTRIM MINID` behind the slowest LIVE reader — the payout consumer group's ack position AND every front's advertised cursor (each front heartbeats its cursor per stream to `spc:cursor:{stream}:{front_id}:{instance}` every `front_cursor_heartbeat_secs`, TTL `front_cursor_ttl_secs` = 120 s; a front whose key has expired is no longer live and stops holding the trim back); never `MAXLEN`. Absolute floor: never trim entries younger than 600 s. Backstop: if XLEN exceeds 1M entries, alarm. A front whose cursor fell behind the trim horizon MUST re-bootstrap via the payout bootstrap RPC.

Byte-level formats of the stream payloads above, the `spc:arkreg` entry and the front bootstrap RPC are defined in [`WIRE-FORMATS.md`](WIRE-FORMATS.md); multi-region rationale in [`MULTI-REGION.md`](MULTI-REGION.md) §6.

### 5.3 `front` process memory

`channels: HashMap<ChannelId, (sp_id, B_scan, B_spend)>` · `ark_delivery: HashMap<sp_id, ArkDelivery>` (loaded from §5.1, updated by `spc:front:events`) · `a_send: Option<(u32 /*H*/, Zeroizing<[u8;32]>, [u8;32] /*A_x*/)>` · `ecdh_cache: HashMap<sp_id, Zeroizing<[u8;33]>>` (cleared with `a_send` on height advance) · `open_slice: Slice` · `seen_shares: HashSet<[u8;32]>` (cleared on `SetNewPrevHash`) · `window_cache: Vec<SliceAgg>` (live slice tail; rebuilt at start through the bootstrap RPC, which in single-site reads Postgres directly) · `type32_blobs: HashMap<slice_id, ShareBlob>` (obtained via bootstrap RPC + live `spc:slices` tail) · `budget: Autoscaler` · `provisional_paids: HashMap<sp_id, u64>` (announced paid amounts from `BlockFound` events; reversed by `LedgerApplied` on orphan) · `outbox: Vec<RedisEvent>` (bounded; MULTI-REGION §8a). Front readiness requires bootstrap-RPC completion.

Loss on crash: the open slice (≤ `max_slice_shares` shares, typically seconds of work) and `seen_shares` (mitigated §3.4). `ponytail:` accepted for v0.1; upgrade path is a periodic open-slice checkpoint to Redis.

## 6. Interfaces

### 6.1 HTTP API (`api` role, read-only, JSON)

Public endpoints expose **pool-wide** data only; anything per-identity requires proof of `b_scan`, otherwise the API is an attribution oracle for anyone holding a miner's public address.

| Method / path | Auth | Returns |
|---|---|---|
| `GET /api/pool` | — | hashrate estimate, connected channels, `N`, `D`, current `H`, `A_send_x`, `coinbase_threshold_sats`, configured Ark `ark_id`, the forfeiture policy (§11.1: unregistered below `C` is forfeited), mode flags, extensions advertised |
| `GET /api/pool/window` | — | current window: `t`, `Σd`, `Σd·f` per slice (no identities) |
| `GET /api/pool/blocks?limit=` | — | `block_found` rows (hash, height, status, output count, `pay_P`) |
| `GET /api/pool/liability` | — | `Σ pending_balance.sats + Σ ark_payout.sats WHERE status = 'scheduled'` — everything the pool holds for miners (§0, §4.6) |
| `GET /api/pool/capacity` | — | weight budget, utilisation, slots used/max, next autoscale direction |
| `POST /api/sp/challenge` | — | `{nonce}`; 60 s TTL |
| `GET /api/sp/me` | `Authorization: SPc <addr> <nonce> <schnorr_sig(b_scan, "SPc/api" ‖ nonce)>` | pending balance, scheduled and sent Ark payouts `{block_hash, sats, due_height, status, vtxo_id}` (`vtxo_id` is the delivery path in `none` mailbox mode, §11.8), accepted difficulty last 24 h, last 50 `{block_hash, height, rail, paid_sats, forfeited_sats}` (no scripts or keys — the miner derives those with `spc-scan`) |
| `GET /api/sp/me/audit/:block_hash` | same | the type-32 window for that block as JSON (same data the extension serves) |
| `GET /healthz`, `GET /metrics` | — | liveness; Prometheus |

Signing with `b_scan` (not `b_spend`) keeps the spend key cold, mirroring BIP 352's own key split.

### 6.2 `spc-scan` CLI

```
spc-scan find    --address tsp1q… --scan-key <hex b_scan> --from <height> [--to <height>] --rpc <url>|--blocks <dir>
spc-scan light   --address … --scan-key … --proofs <file of header‖branch‖coinbase>      # ≈500 B/block input
spc-scan export-tweaks --format {sparrow,json}                                           # (outpoint, t) pairs
spc-scan audit   --pool <host:port> --address … --block <hash> [--slices 4 --shares 8]   # type-32 verifier
spc-scan vectors                                                                          # run the SPc vectors
```

### 6.3 Configuration (TOML; one file per binary)

```toml
# spc-pool.toml
[pool]
network = "testnet4"            # fixes HRP ("tsp") and TP params
roles = ["front", "payout", "api"]
listen = "0.0.0.0:34254"
authority_public_key = "…"      # Noise static key (SRI convention)
pool_payout_descriptor = "tr(tpub…/0/*)"   # xpub of the ark seed's account m/86'/coin'/1' (§11.3); index = block height
fee_bps = 100
front_id = 0                    # 0..127, unique per front (§3.5, §9); single-site = 0
outbox_max = 4096               # front: bounded in-RAM outbox of unpublished Redis events (~10 min of slices + block events); full ⇒ fail closed (MULTI-REGION §8a)
tip_divergence_secs = 30        # front: alarm when its tip differs from the central TipHeartbeat for this long

[template]
source = "ipc"                   # "ipc" (bitcoin-core Cap'n Proto) | "sv2-tp"
socket_path = "/run/bitcoin/node.sock"
extranonce_bytes = 16            # scriptSig budget: 100 − 4 − 33 − 16 = 47 B spare (§2.4.1)

[accounting]
window_n = 8
slice_fee_delta_sats = 0
max_slice_shares = 65000
dust_limit_sats = 330
coinbase_threshold_sats = 330    # C, §11.1; MUST be ≥ dust_limit_sats
max_payout_slots = 256
retain_windows = 16
scrub_after_blocks = 100
confirmation_depth = 6

[budget]
floor_wu = 50000
ceiling_wu = 400000
up_threshold = 0.85
down_threshold = 0.50
step_factor = 1.15
up_debounce = 3
down_debounce = 20
cooldown_secs = 300

[jd]                                # JD is always on (decision 2026-10-13); no enabled flag
distribution_interval_secs = 60
distribution_rebalance_bps = 100
grace_same_height_only = true    # MUST stay true (§3.5)

[extensions]
share_accounting = true          # 0x0020
window_hot_blocks = 6

[storage]
postgres = "postgres://…"
redis = "redis://…"                # TLS + per-front ACLs when fronts are remote (MULTI-REGION §6)
front_cursor_heartbeat_secs = 30
front_cursor_ttl_secs = 120
```

```toml
# spc-agent.toml                           # JDC role adds [jd] and [tp] for the miner's own node
[upstream]
address = "pool.example:34254"
authority_public_key = "…"
sp_address = "tsp1q…"                      # becomes user_identity; validated at startup
require_own_payout = "warn"                # "warn" | "refuse"   (§3.6)
request_share_accounting = true

[downstream]
listen = "0.0.0.0:34255"                   # SV1 ASICs point here; their usernames are labels only
```

`spc-pool` MUST refuse to start unless `pool_payout_descriptor` is a single-key `tr(xpub/…/*)` with exactly one unhardened wildcard, no hardened step after the xpub, and no multipath (the xpub spec's three assertions, §3.1 of `docs/research/output/plan-xpub-miner-identity-spec-2026-07-29.md`), and it contains no private key. The reference scanner and the output-order rule assume a 34-byte P2TR script, and a non-P2TR pool output would be the only fingerprint distinguishing pool from miner outputs.

## 7. Implementation details

### 7.1 Workspace

```
silent-pool/                      MIT OR Apache-2.0
  Cargo.toml                      workspace; rust-toolchain pinned to sv2-apps'
  crates/spc-crypto               secp256k1, bitcoin, bech32, zeroize; SPc ECDH, tier KDF, mailbox blinding (no I/O)
                                  — promoted from spc_ark/ + spc_ark_mailbox/, frozen vectors carried over
  crates/spc-scan                 lib + bin                                       (bitcoin RPC optional)
  crates/spc-accounting           pure; proptest dev-dep                         (no I/O)
  crates/spc-share-ext            binary-sv2 / codec-sv2 message structs + server/verifier
  crates/spc-ext                  0x5350 delivery-capability messages (§11.4)
  bins/spc-pool                   fork of sv2-apps pool (JDS included) + roles front/payout/api/ark;
                                  the spc-jds module (in front) implements PR #203 0x0003 — JD always on
  bins/spc-agent                  translator role (fork of sv2-apps translator) + JDC role (fork per roadmap
                                  Phase 4, 0x0003) + verifier + SP-aware bark keystore (§11.5)
  vendor/channels-sv2             patched: pool_tag: Option<Vec<u8>>
  docs/spc-spec.md                §2 of this document, standalone, for BIP review
  docs/share-accounting-ext-deltas.md
  UPSTREAM_DEPS.md                pinned commits: stratum-core, sv2-apps, sv2-spec PR #203, share-accounting-ext, bips
  testing/regtest-harness
```

**`UPSTREAM_DEPS.md` pins (added 2026-10-12).** Pin SHAs, never branches. `sv2-spec` PR #203 @ `47dbedd` — its single commit (2026-09-10) and the commit warioishere/stratum#1 and warioishere/sv2-apps#3 implement. `bitcoin_core_sv2` MUST be pinned at a commit containing the shared `block_reserved_weight(size) = max(1168 + 4·size, 2000)` helper in `unix_capnp/mod.rs` (it patches both `v30x` and `v31x` `bootstrap_template_ipc_client_from_coinbase_output_constraints`; warioishere/sv2-apps#2 @ `ec35ac8`, carried as `4e526de` in #3's branch, or its upstream merge) in **both** `spc-pool` (`[template] source = "ipc"`) and `spc-agent`/`spc-jdc` (`[tp]`). Upstream `sv2-apps` `main` (as of 2026-10-12) and tag `v0.8.0` still reserve `max(4·size, 2000)` — none of the 1168 WU of header, tx count, coinbase fixed fields, 100 B scriptSig and witness commitment once `coinbase_output_max_additional_size > 208 B` — and are excluded; a `0x0003` JDC that reserves the exact output size got `bad-blk-weight` on mainnet with 42 outputs and a full mempool. `source = "sv2-tp"` needs no pin (it reserves `1168 + 400 + 4·size`). If `spc-jdc` is forked from warioishere/sv2-apps#3 (roadmap Phase 4), pin `warioishere/sv2-apps @ 90b3e3d` and `warioishere/stratum @ 9be4023` (`extensions_sv2` 0.3.1, MIT OR Apache-2.0); #3's `stratum-apps/Cargo.toml` pins `stratum-core` by branch, so re-pin by SHA. `warioishere/blitzpool-server-rust @ b48998f` (release 2.4.1) is the reference `0x0003` JDS for reading, not porting; the frontmatter pin `7815884` stays as the provenance of the `bp-pplns` patterns §7.6 re-implements.

### 7.2 Key types and zeroisation

`struct SendKey { h: u32, a: Zeroizing<[u8; 32]>, a_x: [u8; 32] }` with `Drop` zeroising; constructed only in `front::template::on_set_new_prev_hash`. `derive()` takes `&SendKey` and never copies `a` out. `tracing` fields are allow-listed; `SendKey` implements `Debug` as `SendKey{h, A_x}` only. CI greps for `a_send` in any `serde` derive or SQL string.

### 7.3 Hot path cost

Per template: `n_kept` scalar multiplications (`ecdh_i` — `B_scan,i` is fixed, `input_hash·a_send` is one scalar per height, so `ecdh_i = s_H · B_scan,i` is **one** point multiplication per miner per height, cached until `H` changes) plus one `P_i = B_spend,i + t_onchain·G`, and for Ark-capable ids one `U_i = B_ark,i + t_ark·G` and one `t_ark·S_mb` for the blinded mailbox id (§11.8). At 256 slots that is ≤ ~1 000 EC ops per height, not per template — the keys don't change within a height; only amounts do. Share path adds one `HashSet` insert and one `Vec::push`; slice sealing adds one merkle root over ≤ 65 000 leaves (~2 ms) off the share path.

### 7.4 Idempotency and exactly-once

Block settlement runs in one Postgres transaction. `payout` first reconstructs the paid window from `distribution_snapshot.high_water` (§4.3, MULTI-REGION §1): exactly the slices at or below the vector, never a re-derivation from the current contents of Postgres. Then: `INSERT block_found … ON CONFLICT DO NOTHING` → if inserted, `INSERT ledger_history` rows (PK rejects replays) → `INSERT ark_payout` rows (`status='scheduled'`, `due_height = height + confirmation_depth`, so normally due at once) for the Ark-rail ids → `UPDATE pending_balance` → `UPDATE block_found.status`. The Ark send is a second, separately idempotent step keyed by the same `(block_hash, sp_id)` and driven by the `ark` role's receipts (§11.3). Orphan check (`confirmation_depth`) runs before the transaction; an orphaned block writes `status='orphaned'` and **no** ledger rows. If a front cannot deliver a found block's event (crashed before XADD, outbox overflow), `payout`'s chain reconciliation sees the pool coinbase on-chain with no `block_found` row (`spc_orphaned_settlement_total{reason=missing_block_event}`, MULTI-REGION §8a), books it `status='pending'` with `fingerprint` NULL (§5.1: the column is nullable while the event is lost; `CHECK` forbids `confirmed` with NULL) and alarms — a build MUST never fail over a lost event (Blitzpool's vouching invariant), and the operator reconstructs from the on-chain coinbase + `slice_credit` if needed; that reconstruction sets `fingerprint`, after which the block may be confirmed and booked.

### 7.5 Pool-tag patch

`channels-sv2/src/server/jobs/factory.rs`: `pool_tag_string: Option<String>` → `pool_tag: Option<Vec<u8>>`; the `Sv2/<pool>/<miner>/` ASCII framing is dropped when `pool_tag.len() == 32` and the push is emitted as `OP_PUSHBYTES_32`. `fits_script_sig_budget` keeps its conservative 8-byte-height check; `ScriptSigSizeTooLarge` still fails at channel setup, not at block submission ([08-16 L5](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md)). Upstreamable as a standalone change ("binary pool tags"), independent of SPc ([08-24 L7](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) — pitch the change so it stands without the proposal).

### 7.6 Blitzpool-derived design (no code ported)

The pending-credit ledger, keep-largest weight-budget trim, autoscaler constants, payout-list fingerprint, confirmation-gated idempotent apply and the vouching invariant are **patterns and parameters taken from Blitzpool's README and this wiki's code reads**, re-implemented here from §4.6 / §4.8 / §7.4 — not copied from `bp-pplns`, which is AGPL-3.0-or-later. Clean-room rule for implementers: write from the spec text and its proptests, do not read `crates/bp-pplns/src/distribution.rs` while doing so. Differences from Blitzpool are deliberate: the ledger key is `sp_id` not `address`; there is no finder bonus and therefore no debit side (`pending` is unsigned); withheld value lands in `pay_P` (`WithheldValue::ToPool` shape, §11.2). Blitzpool is cited as provenance (`7815884`) and as the `0x0003` JDS reading reference (`b48998f`, §7.1); no `NOTICE` obligation arises.

## 8. Testing strategy

### 8.1 Unit — `spc-crypto` (vectors regenerated for the chosen tags)

The derivation vectors are DESIGN.md's: port `spc_ark/src/lib.rs::test_vector` (`input_hash`, `ecdh`, `t_onchain`, `t_ark`, `P`, `U`, `D`, blinded mailbox id, `ark1` address) and `spc_ark_mailbox/src/lib.rs::tests::test_vector` (`m_i`, `M_i`, variant-B `M_i`, authorization bytes) unchanged, plus `pool_blinding_matches_bark` and `pool_blinding_unblinds_to_epoch_mailbox`. Regenerate `~/repos/sp-coinbase-vectors` with the `SPc/*` tags and `ser_P(A_send) = 0x02‖x` in `input_hash`, then port all cases as `#[test]`s:
- positive: ordinary-tx split sanity; coinbase native (null prevout, height nonce); `N = 5` fan-out with a dropped output not affecting others; nonce distinctness across heights and across `A_send`.
- negative controls (each MUST fail where stated): vanilla BIP 352 eligibility gate rejects the coinbase; constant-null-outpoint nonce collides across blocks; `input_hash` without `A_send` admits the `:92` grind (and binding `A_send` kills it); odd-Y `A_send` without negation misses; group-linear key list `A_H = A_0 + H(A_0‖H)·G` yields every epoch from `a_0`.
- carriers: scriptSig layout round-trips byte-exact; `OP_RETURN "SPc"` round-trips; the rejected 1-of-2 multisig carrier stays as a **documentation test** that asserts `a_send` can sign the output (why it's rejected).
- tiers: `t_onchain ≠ t_ark` for the same `(ecdh, k)`; a digest of 0 or `≥ n` fails rather than reducing (forced via a test-only hash hook); `B_ark = B_spend` is rejected at registration (§11.4).

### 8.2 Property tests — `spc-accounting`

`proptest` over random windows (1–40 slices, 1–300 identities, random `d`, `f`, `T`, pending, budget): invariants §4.6 (a)–(e); `δ = 0` pool-template result equals an independent plain-PPLNS oracle to the sat; sorting by `sp_id` is deterministic across runs; trim never exceeds budget or `max_payout_slots`; §4.5 worked example as a fixed fixture.

### 8.3 Integration — SV2 surface (in-process, no bitcoind)

- channel open with `sp`, `tsp`, bare `bc1q`, descriptor, `sri/solo/…`, empty, 63-char-truncated, two addresses on one connection → exactly the §3.1 table; **assert no coinbase was built for a rejected channel**.
- `RequestExtensions` matrix: `{0x0001}`, `{0x0001, 0x0003}`, `{0x0001, 0x0020}`, unknown id → correct `.Success`/`.Error` lists; JDC with `0x0003` on only one connection refuses JD mode.
- share replay (same share twice, same share on two channels), stale job after restart → rejected.
- slice sealing on prev-hash change, fee band, `max_slice_shares`; merkle root matches an independent implementation.
- `SetPayoutDistribution` published on height change; validation rejects mutated amounts, reordered outputs, a missing `OP_RETURN`, and a **height-straddling** declaration under the grace window; accepts the previous id from the same front on the same connection at the same height.
- type-32: `GetWindow` for a found block returns a window whose `Σ difficulty ≥ N × D`; `GetShares` merkle paths verify; a pool build compiled with `--features cheat-slice-difficulty` is caught by the verifier within 3 windows at `4 × 8` sampling (statistical test, seed-pinned).

### 8.4 End-to-end — regtest harness (`testing/regtest-harness`, CI)

bitcoin-core (IPC) + `spc-pool` (JDS module included) + 2 × `spc-translator` + 1 × `spc-jdc` + CPU miners:
1. **Pool-template happy path**: three `tsp` identities mine; block found; coinbase has pool P2TR + 3 P2TR + no `OP_RETURN`; each `spc-scan find` recovers exactly its own output; each spends it with `b_spend + t`.
2. **JD happy path**: `spc-jdc` declares; block found; coinbase has `OP_RETURN "SPc"`; JDC's own `P` present; `spc-scan` recovers via carrier 2; type-32 `GetTransactionsInJob` returns the JDC's set.
3. **Threshold, trim and delivery**: one Ark-registered identity below `C` → `ark_payout` scheduled and paid at `H_f + 6` from the float (§11.9 covers the Ark leg); one identity below `C` with no Ark registration → `rail='forfeit'`, its sats in `pay_P`, nothing pending; `/api/pool/liability` returns to 0 after the send.
4. **Orphan**: invalidate the found block → `status='orphaned'`, no ledger rows, pending unchanged.
5. **Restart mid-height**: kill `front`; new `A_send` for the same `H`; shares resume; next block still scannable.
6. **Fail-closed**: SV1 miner with a bare address → `spc-translator` refuses to start; a hand-rolled SV2 client with a bare address → `invalid-user-identity`, nothing mined for it.
7. **Light client**: `spc-scan light` on exported `header‖branch‖coinbase` triples finds the same outputs as `find`.
8. **Scrub**: after `scrub_after_blocks`, `distribution_snapshot.entries = []` and `slice_credit` rows beyond retention are gone; a block with a still-`scheduled` Ark payout keeps its entries until the send; `ledger_history` still reconciles `pending_balance`.
9. **Full mempool**: fill the regtest mempool to ≈ 990 000 vB (Blitzpool `regtest_budget_autoscale.rs` `MEMPOOL_TARGET_VBYTES`; Core packs to `block_max_weight − block_reserved_weight`, so a part-full mempool never binds) before (a) the pool mines its own template with `max_payout_slots` outputs and (b) `spc-jdc` declares a block with `max_payout_slots` outputs; both blocks must pass the JDS node's `checkBlock` and be accepted. Fails with `bad-blk-weight` on `bitcoin_core_sv2` from upstream `sv2-apps` `main`/`v0.8.0` once `coinbase_output_max_additional_size > 208 B` (it reserves `max(4·size, 2000)` instead of TDP's `1168 + 4·size`); passes with the `block_reserved_weight` fix pinned in §7.1 or with `source = "sv2-tp"`. Steps 1–8 cannot surface this — it is the one class of bug the 2026-10-11 mainnet run found.

### 8.5 Adversarial (manual + CI where deterministic)

Grinding `a_send` to collide with a known earlier payment (must be a fixed-point search — test asserts the naive solve fails); JDC distribution shopping across the grace id and the current id at the same height (accepted by design, bounded by `distribution_rebalance_bps`); 10 k channels with distinct valid addresses (memory, autoscaler, `max_payout_slots` trimming order).

## 9. Deployment

**Topology.** One `spc-pool` binary; dev/testnet runs `--roles front,payout,api` in one process, and MAY add `ark` to it on regtest/signet only; production splits `front` (one per region; RAM-only state — no durable state of its own) from a single `payout` (sole Postgres writer), N `api` replicas, and a single `ark` on its own host. Each `front` claims its `front_id` at start with a Redis `SET NX` lease (heartbeat-refreshed, holding an instance UUID) and refuses to start if the lease is held — a duplicate `front_id` would silently collide `distribution_id`s and `slice_id`s (MULTI-REGION §10). The `spc-jds` module is part of `front` — JD is always on, and `a_send` is shared in-process (§2.2). Redis and Postgres are the only durable state of the SPc side, and **no spending key lives on it**: it holds only the xpub of `pool_payout_descriptor`. The pool has exactly one hot seed, the `ark` seed (bark wallet + pool output keychain), held only by the `ark` role on its own host with its own secret store and no access to the attribution data (§11.3). Its blast radius is the float plus whatever pool revenue the operator leaves under it; moving surplus elsewhere is operator policy and out of scope.

**Migrations.** `sqlx migrate` at boot under `pg_advisory_lock`; every process may run them (Blitzpool pattern). Schema changes are additive until v1.0.

**Rollout.** regtest CI → testnet4 (`tsp`) soak ≥ 24 h with ≥ 1 found block independently recovered by a third party's `spc-scan` → publish `docs/spc-spec.md` to the Delving thread and the BIP mailing list → mainnet is out of scope for this document.

**Rollback.** Stopping `front` stops mining. Coinbase-tier payouts are already on-chain and not at risk; pending balances and scheduled Ark payouts survive in Postgres and are paid when the pool resumes (the Ark send is idempotent per `(block_hash, sp_id)`, §7.4). Downgrading `spc-pool` across a schema version is unsupported; restore from the pre-migration dump.

**Monitoring (Prometheus).** `spc_a_send_age_seconds` (alarm > 1 h — a stuck height), `spc_a_send_repeat_total{front_id}` (alarm > 0), `spc_open_slice_shares`, `spc_slices_sealed_total{reason}`, `spc_distribution_slots{kept,trimmed,dust}`, `spc_budget_wu`, `spc_budget_utilisation`, `spc_blocks_total{status}`, `spc_pending_liability_sats`, `spc_ark_due_sats`, `spc_ark_payouts_total{status}`, `spc_ark_overdue_total` (scheduled past `due_height + ark_retry_blocks` = page), `spc_ark_float_sats` (below `float_target_sats` = refill needed; `ark` boards at the next matured pool output), `spc_ark_surplus_sats` (matured pool revenue not boarded, for operator tooling), `spc_forfeited_sats_total`, `spc_ark_orders_rejected_total{reason}` (any `bad-signature` = page), `spc_channel_reject_total{reason}`, `spc_jd_declaration_reject_total{code}`, `spc_ext32_requests_total{msg}`, `spc_outbox_depth`, `spc_outbox_age_seconds`, `spc_stream_lag_seconds{stream}`, `spc_tip_divergence_seconds`, `spc_registration_lag_revs`, `spc_orphaned_settlement_total{reason}`.

**Operational privacy.** Logs never contain `user_identity`, `sp_id`, `P_i`, `U_i`, mailbox ids or VTXO ids at `INFO`; `DEBUG` redacts to 8-hex-char prefixes. Postgres backups inherit the scrub policy; a backup older than `scrub_after_blocks` is itself attribution data and SHOULD be rotated on the same schedule.

## 10. Open items (carried from the roadmap, narrowed by this spec)

| # | Item | Status |
|---|---|---|
| O1 | BIP posture — amendment to 352 vs new informational BIP | **Open.** With fresh `SPc/*` tags (DESIGN.md) the construction no longer shares BIP 352's hash domain, and it adds a second tier BIP 352 has no concept of; that argues for a new informational BIP referencing 352 (the roadmap's original recommendation). O12 can move this back toward an amendment for the coinbase tier alone. |
| O2 | `A_send` serialization inside `input_hash`: `ser_P` (this spec) vs `ser_x` | Decided `ser_P` for maximal reuse of BIP 352's `input_hash` shape; vectors regenerate accordingly. |
| O3 | `{H, H−1}` scan tolerance | Decided for `spc-scan`; pool-side grace is same-height-only. Revisit if third-party SPc pools appear. |
| O4 | Labels (BIP 352 `B_m`) | Deferred to v0.2; one identity per address in v0.1. |
| O5 | Solo-pool profile (one payee) | Same carrier and derivation; a `[pool] mode = "solo"` flag is a ~20-line addition; not in v0.1. |
| O6 | Rail for pending credits below the coinbase threshold | **Resolved 2026-10-12 → §11 (Ark via a third-party bark server).** Lightning/BOLT12 stays a v0.2 alternative for miners without an Ark wallet. |
| O7 | Empirical mainnet scriptSig occupancy alongside AuxPoW tags | Unmeasured; needed before any mainnet claim about the 47 B spare. |
| O8 | Amount-linkability mitigation (randomised slot count, deferred payouts) | Research; see proposed inventory record in the roadmap. |
| O9 | `demand-share-accounting-ext` porting cost on current `stratum-core` | Unknown until Phase 6; message set is small. |
| O10 | PR #203 drift | Pinned at `47dbedd` (the PR's single commit, 2026-09-10; no spec movement as of 2026-10-12 — warioishere's 2026-10-11 comment pins warioishere/stratum#1 and sv2-apps#3 to the same commit). Residual text risk: TheBlueMatt's unanswered 2026-09-10 question on all-static payouts with only a subset paid (would relax the §7 'no partial payment of `additional_outputs`' MUST at L168) and warioishere's requested §9.3 sentence (prose only). `spc-jds`/`spc-jdc` isolate `0x0003` in one module each. |
| O11 | JD-mode booking when the JDC's template revenue ≠ the pool's (§4.6) | **Resolved 2026-10-14** (`docs/decisions/2026-10-14-o11-claim-formula.md`): revenue-scaled — `claim(id) = floor(owed(id) · (T_actual − F) / W)`; `pending'(id) = claim(id) − paid(id)`. Unsigned ledger preserved; §11.3 Ark sends pay `ark_due'` as booked from the mined coinbase. |
| O12 | Plain-SPc compatibility for the coinbase tier (DESIGN.md open question) | **Open.** Should `t_onchain` use the upstream SPc / BIP 352 tag instead of `SPc/TierSpc`, so a single-tier SPc scanner also finds coinbase outputs? Depends on the SPc BIP, which isn't final. `t_ark` and `t_mb` keep `SPc/*` tags either way. |
| O13 | Bark integration for derived keys (DESIGN.md) | **Open.** An extra-keys source consulted by `pubkey_keypair`, `find_vtxo_keypairs`, `get_vtxo_key`, `find_signable_clause` (RESULTS.md: ~150–300 lines); upstream patch vs fork. Tweaked keys need their own key type and storage, and seed recovery must re-scan pool blocks. |
| O14 | Default mailbox mode (DESIGN.md per-block vs static; MAILBOX.md variants) | **Resolved 2026-10-14** (`docs/decisions/2026-10-14-mailbox-default-epoch.md`): default = `epoch_derived` (variant B). No per-epoch registration window; no fallback-to-static. Variant A (`epoch_registered`) remains an option for independent per-epoch keys. |
| O15 | Wallet UX across tiers (DESIGN.md) | **Open.** One balance spanning pending, scheduled Ark payouts, immature coinbase outputs and VTXOs; `GET /api/sp/me` (§6.1) supplies the pool-side half. |
| O16 | Light-client scanning of both tiers (DESIGN.md) | **Open.** The ≈500 B/block proof (§2.5) covers the coinbase tier and the pool-block list; the Ark tier then needs only mailbox reads, but a light client still needs a trusted source of pool-block heights. |
| O17 | Coinbase output as the board itself (`board_funding_address` in the coinbase) | **Rejected (2026-10-13).** The pool does not run the Ark server, so the design MUST work with a stock third-party `captaind`, and stock mainnet `captaind` forces `require_board_funding_tx`, whose cosign check rejects a coinbase's null-prevout input. Recipients would also have no unilateral exit until coinbase maturity. The pool output is paid normally and boarded when mature (§11.3). |
| O18 | Prototype timing | **Resolved 2026-10-13.** `spc_ark_mailbox` now schedules reads from `PAYOUT_DELAY = 6` (was `COINBASE_MATURITY = 100`); tests updated and passing. |
| O19 | Covenants (CTV / BIP 119, or an equivalent such as `OP_TEMPLATEHASH`) | **Future; needs a soft fork.** Would greatly improve the Ark story by **removing interactivity**: today every board, arkoor and refresh needs the third-party Ark server online to cosign (bark is covenant-less Ark), which is why sub-threshold payouts come from a hot float and why the coinbase can't be the board (O17). With a covenant, the coinbase could commit the sub-threshold payouts to a pre-defined tree whose leaves are each miner's `U_i`, giving every recipient a unilateral exit with no cosigning party online and no pool float. More generally, **all** pool payouts could be compressed into one coinbase output committing to a payout tree: the §4.8 weight budget, trimming, `max_payout_slots` and the coinbase dust floor stop binding, and per-miner amounts stay off-chain until a leaf is expanded (helping R1 amount linkability). Costs to weigh then: coinbase maturity still applies to the tree root, and expanding a leaf costs one on-chain transaction per tree level. Track; no v0.1 work. |

## 11. Sub-threshold Ark tier (added 2026-10-12; reconciled to DESIGN.md 2026-10-13)

> Balances that do not clear the coinbase threshold are paid over **Ark (bark)** to a **per-block key derived from the same ECDH as the coinbase output** (DESIGN.md), from the pool's Ark float on a **third-party** Ark server, once the found block has 6 confirmations; the block's own pool output refills the float when it matures. Grounding: `DESIGN.md` (authoritative), `MAILBOX.md`, `RESULTS.md`; [Ark (bark) as a Sub-Threshold Payout Rail](research/wiki/concepts/ark-sub-threshold-payout-rail.md), [Payout Threshold Economics](research/wiki/concepts/payout-threshold-economics.md), and the two bark code reads ([send side](research/raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md), [key side](research/raw/repos/2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md)). The tier changes **how** `owed(id)` is paid, never how §4 computes it — SLICE scoring, JD or pool-template, is untouched.

### 11.1 Tier policy (completes §4.6)

DESIGN.md "The proposal": the tier is decided per block by whether the miner's balance clears the coinbase threshold.

```
owed(id) = raw(id) + pending(id)
if   owed ≥ C and id ∈ kept (§4.8)        → coinbase output of owed                       rail = coinbase
elif owed ≥ C (trimmed)                   → pending'(id) = owed  (next block's coinbase)  rail = pending
elif ark_delivery(id) registered, dest known (§11.4)   → ark_due'(id) = owed  (sent at H_f + 6, §11.3)  rail = ark
elif ark_delivery(id) committed in Postgres, dest not yet propagated to this front → pending'(id) = owed  rail = pending
else                                      → forfeited(id) = owed (kept by the pool)       rail = forfeit
```

- `C` = `coinbase_threshold_sats` (§4.1), default **`dust_limit_sats`** (330) — DESIGN.md's "coinbase dust threshold". An operator MAY raise it (never below dust) to bound coinbase size; the cost is more value on the custodial Ark tier. The binding constraint on raising it is what a miner can afford to lose to a 28-day sweep ([Payout Threshold Economics](research/wiki/concepts/payout-threshold-economics.md) §5), so keep it low. Published at `/api/pool`.
- Under JD the branch is taken from the **mined** coinbase (§4.6 settlement rule): an id the JDC dust-pruned is "below `C`" for this block whatever the snapshot said.
- Trimmed miners above `C` are **not** diverted to Ark: large balances are exactly what should not sit under arkoor trust. They wait one block (Blitzpool's "fairness, never validity").
- **Unregistered below `C` is forfeited to the pool.** Ark delivery is part of the instructions for mining here; `spc-agent` registers automatically on connect (§11.5), so only a miner running other software, or misconfiguring it, reaches this branch. Their sub-threshold earnings for that block go to `pay_P` and are booked as `forfeited` — not carried as pending, so the pool holds no liability for operators who don't follow the instructions. This is a published policy, not a silent fallback: it is stated at `/api/pool`, reported per block at `/api/sp/me`, and signalled at channel open (§3.1). It is distinct from the sv2-apps `FullDonation` defect (§3.1), which silently took 100 % of the block for *every* miner on an unparsed identity; here identity is still validated fail-closed, and only the earnings of an unregistered miner below `C` are affected. Above `C` an unregistered miner is paid in the coinbase as usual.
- `pending` arises from the pool's side: trimming (§4.8), failed Ark sends (§11.3), and a registration committed in Postgres whose `dest_id` had not reached the building front (MULTI-REGION §6). None of these is ever forfeited; forfeit applies only when no `ark_delivery` row exists in Postgres at settlement.
- Booking requires `block_found.fingerprint IS NOT NULL` (§5.1, §7.4); a block with NULL fingerprint contributes nothing to `owed()` until resolved.
- Cadence: **one Ark send per id per found block**, never per share or per template (Second: per-minute payments "build long exit chains … prohibitively expensive to exit on small balances").

### 11.2 Both template modes

| | Pool-template | Job Declaration (PR #203) |
|---|---|---|
| Where Ark-tier and forfeited sats land | in `pay_P` (§4.6) — the single hot pool output | in `weight_P` — §3.1 of #203 explicitly allows "value credited on behalf of miners that are not represented in `payouts`"; `pool_payout` is the same per-height hot output |
| Carrier for `A_send` | scriptSig (§2.4.1) | `OP_RETURN` (§2.4.2) |
| Ark destination | derived at distribution build, stored in the snapshot (§4.7) | identical |
| What the pool does after booking (`H_f + 6`) | `payout` emits a send order per `ark_due` id; the `ark` role pays it from the float (§11.3) | identical |

The coinbase leg is unchanged in both modes; the Ark leg is purely pool-side settlement. The JD column is the `WithheldValue::ToPool` weight composition from Blitzpool `bp-pplns` (`weights.rs`, origin/main), deliberately not its PPLNS `ToOtherMiners` composition — the one warioishere described on PR #203 (2026-09-11): `weight_P` is the fee only, withheld weight is spread over the published miners, who are then booked matching debits. Silent-pool cannot use `ToOtherMiners`: the pool output is what refills the Ark float (§11.3), so the withheld sats MUST land in it, and §4.6's `pending` is unsigned. Unlike Blitzpool's Team Mining (which pairs `ToPool` with no ledger), the withheld value is booked per §11.1 (`ark_due`, `pending` or `forfeited`). The JDC computes `pay_P = T − F − Σ pay[i]` without inspecting how `weight_P` was composed, so the two shapes are wire-identical.

### 11.3 Pool side: the `ark` role

**Flow per found block.**

```
H_f            coinbase = [ pool_output = P2TR(pool_payout_descriptor @ index H_f) , P2TR(P_i) … , carrier (JD) ]
                 pool_output = fee + rounding + Σ ark_due + Σ forfeited + Σ pending changes   (one hot output, §11.2)
H_f + 6        booked (confirmation_depth); `ark` pays every ark_due id from its float (arkoor to U_i)
H_f + 100      pool_output matures; `ark` boards only what the float needs; the rest is operator surplus
```

**Separation principle.** The pool has two secrets with two different jobs, and compromise of either MUST NOT compromise the other:

| | SPc side (`front`, `spc-jds`, `payout`, Postgres) | Ark side (`ark` role) |
|---|---|---|
| Secret | `a_send[H]` (RAM, per height); the attribution data (`slice_credit`, snapshot `entries`, `ark_delivery`) | the `ark` seed: the pool's bark wallet **and** the `pool_payout_descriptor` private key (hot, by necessity: it cosigns arkoors, refreshes and boards unattended) |
| Protects | **privacy of SPc receivers**: which coinbase output pays which miner | **pool funds**: the float and the pool outputs |
| Compromise costs | one height's payee set, or the attribution DB until scrub; **no funds** (the SPc side holds only the descriptor's xpub) | the float plus whatever pool revenue the operator has not moved out; **no SPc receiver privacy** |

Concretely:

- **Independent key.** The `ark` seed MUST be generated from its own CSPRNG draw. It MUST NOT share a seed, xprv or derivation tree with the Noise `authority_public_key` or anything on the SPc side, and it MUST NOT be stored in the same secret store as Postgres/Redis credentials. Under it, bark's own wallet uses its usual paths, and the pool output keychain is a **separate account** (`m/86'/coin'/1'`, so it never collides with bark's BIP86 account 0): `pool_payout_descriptor = tr(<account xpub>/0/*)`.
- **Fresh P2TR pool output per height, indexed by the height.** `front` and `spc-jds` hold only the account **xpub** and derive `pool_output(H) = descriptor.at_derivation_index(H)` — a pure function, no coordination with `ark`, no state, and coinbase building never waits on `ark` or the Ark server. Every height gets a fresh key-path P2TR that looks like the SPc outputs beside it. The descriptor is a pool constant, so the xpub spec's three assertions (wildcard, no hardened step, not multipath) are checked once at startup. A compromised SPc side can link the pool's own outputs to each other; it learns nothing about miners and can spend nothing.
- **`ark` finds its outputs without a gap limit.** Indices are sparse (only heights the pool wins are ever paid), so `ark` MUST NOT rely on a gap-limit wallet scan. `ark` records matured pool outputs by `(txid, vout)` outpoint, never by height alone, and re-checks the active chain at maturity (H_f + 100) before boarding — two pool blocks at the same height share the script but not the txid; boarding is idempotent per outpoint. For each new block it derives the script for that height and checks the coinbase for it — one derivation per block.
- **Separate process, least knowledge.** The Ark wallet lives only in the `ark` role (`spc-pool --roles ark`, its own host in production). `ark` has **no** credentials for the pool's Postgres or the identity-bearing Redis keys. Its entire input is a send-order stream, and each order carries only what the **Ark server will see anyway**:
  ```
  ArkSendOrder   { order_id: [u8;16] (random, opaque), ark_address (policy U_i, delivery per §11.8),
                   sats, not_before_height, sig }                  # spc:ark:orders, payout → ark
  ArkSendReceipt { order_id, status: sent | failed, vtxo_id, expiry_height }   # spc:ark:receipts, ark → payout
  ```
  No `sp_id`, `B_scan`, `B_spend`, `P_i`, block hash or snapshot data crosses into `ark`, so a compromised `ark` host learns exactly the Ark server's view (destination keys, amounts, timing) and nothing about who was paid in any coinbase. `payout` alone maps `order_id → (block_hash, sp_id)`.
- **Orders are authenticated and capped.** `payout` signs each order with a dedicated order-signing key — secp256k1 BIP340 Schnorr, CSPRNG-generated (independent of the `ark` seed and all SPc keys; resolved 2026-10-14, `docs/research/output/ark-order-key-spec-recommendation.md`), stored hex-encoded 32-byte secret at `order_key_path` (default `/run/secrets/spc/ark-order-key`) with `0o600` file / `0o700` dir perms (bark `fs_perms` convention), x-only pubkey distributed to the `ark` role via config (`[ark] order_pubkey`), validated at startup; the signed message is defined in `WIRE-FORMATS.md` §3.4. Rotation is operator-driven: generate new keypair on `payout` host, update `order_pubkey` on `ark` host, restart `ark`; in-flight orders on rotation follow the existing retry/`failed_to_pending` logic (orders are short-lived, max `retry_blocks` = 6). **Order verification (`ark`)**: (1) parse per WIRE-FORMATS §3.4; (2) reject `sats == 0` or `sats > max_order_sats`; (3) hold (send nothing) while `not_before_height > tip`, per the send rule below; (4) reject if `ark_id` parsed from `ark_address` != configured `ark_id`; (5) reject duplicate `order_id` within the retry window; (6) verify BIP340 `sig` over `hash_SPc/ArkSendOrder(...)` against configured `order_pubkey`; any rejection increments `spc_ark_orders_rejected_total{reason}` and sends nothing. Rationale: `ark_address` binds the server (and thus network); `order_id` + `not_before_height` prevent replay; no identity or block-hash field is bound (privacy, §11.3).

**Server choice.** `[ark] server = "https://…"` MUST be a server **not operated by the pool**. Arkoor security is "at least one party (sender or Ark server) honest" (Second docs); a pool that is both removes the assumption ([critiques](research/raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md) §1). Default: Second's mainnet server (`ark_id` pinned in config). A self-hosted `captaind` MAY be used **only on regtest/signet for tests**. RESULTS.md: a stock server and a stock pool-side bark wallet pay derived destinations unmodified.

**Paying (`H_f + 6`).** Ark-tier payouts for a block found at `H_f` are sent at `due_height = H_f + confirmation_depth` (6), from the float — the pool's own working capital, so miners do not wait for coinbase maturity. Pay only from VTXOs with `expiry_height − tip ≥ min_remaining_blocks` (default **3 024 ≈ 21 d**) so recipients inherit a near-full window; keep `arkoor_depth` under the server's `max_vtxo_exit_depth` by refreshing on depth, not only on expiry. A reorg deeper than `confirmation_depth` after the send is the pool's loss: arkoors are not reversible, which is the reason to wait 6 blocks rather than pay at find time.

**Reorg and float edge cases.**

_Normative (MUST):_
- `ArkSendOrder.not_before_height = H_f + confirmation_depth` (6), and `ark` MUST NOT send before its own chain tip reaches it.
- `payout` MUST re-verify that `H_f` is still on the active chain immediately before emitting each order (the send loop below; `ark` never sees the block hash).
- A reorg that removes `H_f` before depth 6 cancels the `ark_payout` row and reverses its ledger entries (send loop); the block no longer exists, so nothing is owed, pending or forfeited.

_Operator policy (defaults given; deployments MAY change them):_
- Reorgs deeper than 6 that touch already-sent orders are the pool's loss (above). Default: alarm on any invalidation deeper than `confirmation_depth` that affects an `ark_payout` row with `status='sent'`, then manual review.
- Where surplus above `float_target_sats` goes (cold storage, another setup, left in place) is operator-configured; the spec prescribes nothing beyond "never board more than the float needs" (Refilling, below).
- `float_target_sats` sizing default: cover the Ark-tier payouts expected over roughly 144 blocks of the pool's own finds (one maturity period plus margin).

**Refilling (`H_f + 100`).** When a pool output matures, `ark` boards **only what the float needs**: it builds a PSBT spending the pool output (key path) to `board_funding_address(..)` for `float_target_sats − float_balance` (if positive), with the remainder to an on-chain address `ark` controls, and registers it with `board_psbt` (`bark/src/board.rs:153-240`: the board output may sit at any index alongside other outputs). Everything not boarded is **operator surplus**. This spec does not define what the operator does with it — sweep to cold storage, pay it out over Ark to another setup, or leave it — only that `ark` never boards more than the float needs. Size `float_target_sats` to cover Ark-tier payouts for the blocks the pool expects to find in one maturity period (≈ 1 block's worth for a 1 % pool, ≈ 10 for a 10 % pool), plus margin.

**Send loop.** `payout`, per `ark_payout` row reaching `due_height` (idempotent per `(block_hash, sp_id)`, §7.4): check block `H_f` is still on the active chain (if reorged out, cancel the row and reverse its ledger entries); build the address from the stored destination (§4.7, §11.8); emit a signed `ArkSendOrder`. `ark` sends and returns a receipt. On `sent`, `payout` sets `status='sent'`, records `vtxo_id`, `expiry_height`, and emits `ArkPayoutNotice` (§11.4); on `failed` or no receipt (Ark server down, float short), re-order next block (same `order_id`, so `ark` dedupes); after `retry_blocks` (default 6) set `status='failed_to_pending'`, credit `pending_balance`, and alarm. A failed send is the pool's failure, so it becomes pending, never forfeit.

**Why not make the coinbase output the board itself.** The pool does not run the Ark server, so everything here MUST work against a stock third-party `captaind`, and a stock mainnet server rejects a coinbase-funded board: mainnet forces `require_board_funding_tx = true` (`server/src/config.rs:587`), and the cosign path then requires every funding input's txid to be a known transaction (`server/src/lib.rs` board cosign, "invalid funding tx: unknown input tx"), which a coinbase's null prevout never is. Even with a patched server, the VTXO's exit transaction cannot confirm before coinbase maturity, so every arkoor recipient would have no unilateral exit for ~100 blocks, and the board's expiry clock would start before the funds are spendable. Recorded as O17. A covenant (O19) is what would actually remove this interactivity.

**Fees.** 0 % on sends/receives at Second today; the pool absorbs refresh and boarding costs out of its own fee. Nothing is deducted from miners for the tier.

### 11.4 Delivery capabilities — the SPc extension

**Identity vs delivery capability (§0).** `user_identity` is the SP address and nothing else; it alone keys the account. What the pool additionally needs to *deliver* on the Ark tier is a **delivery capability**: static, private, per-`sp_id` registration data, sent once per Ark server over SV2 and never published. "Registration" here is pool↔miner bookkeeping only — **there is no server-side registration endpoint**; the Ark server only unblinds. The per-block destination is always **derived** (§2.3, §11.8); the miner never sends a per-payout address, and there is **no literal-address registration** — a static reused Ark address is the ArkPool model DESIGN.md calls strictly worse, so silent-pool does not offer it.

Project-defined SV2 extension negotiated via `0x0001` (identifier `0x5350` — "SP"; renumber if assigned):

| Msg | Dir | Fields | Semantics |
|---|---|---|---|
| `SetArkIdentity` | C→S | `ark_id: B004`, `b_ark_pub: B033` (**required**), `mailbox_mode: U8` (0 `none`, 1 `static`, 2 `epoch_derived`, 3 `epoch_registered`), `mailbox_pubkey: B0_33` (`M`; empty iff `none`), `mailbox_dh: B0_33` (`D = b_ark·S_mb`; empty iff `none`), `epoch_fallback: U8` (0 `static`, 1 `hold`; `epoch_registered` only), `sig: B064` | Register or replace the Ark delivery capability for this connection's `sp_id`. `sig` is BIP340 by **`b_scan`** over `hash_SPc/ArkIdentity(sp_id ‖ all preceding fields)` — the Noise session authenticates the pool, not the miner, and anyone can open a channel with a public SP address, so without this signature anyone could redirect a miner's Ark payouts. Applies to distributions built after acceptance (the destination is derived at build time, §4.7). |
| `SetArkIdentity.Success` | S→C | — | Stored in `ark_delivery` (§5.1). |
| `SetArkIdentity.Error` | S→C | `error_code: STR0_255` | `bad-signature`, `invalid-point`, `ark-key-is-spend-key` (`B_ark = B_spend`; DESIGN.md requires separate keys), `wrong-ark-server` (`ark_id` ≠ the pool's configured server; `D` is server-specific), `bad-mailbox-fields`, `unsupported-mailbox-mode`. Fail closed: nothing is derived for a rejected registration; earnings accrue as pending. |
| `RegisterEpochMailboxes` | C→S | `entries: SEQ0_64K[B037]` (`ser32(i) ‖ ser_P(M_i)`, MAILBOX.md `EpochRegistration`), `sig: B064` (by `b_scan`, as above) | `epoch_registered` mode only: supply `M_i` for found or upcoming heights (MAILBOX.md "Registration timing": after the find, within ~100 blocks, or a pre-registration window ≈ 5.3 KB/day). |
| `ArkPayoutNotice` | S→C | `block_hash: U256, sats: U64, vtxo_id: B0_64, expiry_height: U32` | Sent after each successful send. A convenience in the mailbox modes (the agent finds payouts by scanning, §2.5); **the delivery path** in `none` mode (§11.8). Also served at `GET /api/sp/me`. |

`D` cannot be checked by the pool (it would need a DLEQ proof); a wrong `D` only dead-letters the registering miner's own payouts, and the `b_scan` signature makes that the only party who can submit it. No 32-byte cap, no overloading of `user_identity`, no `0x0002` dependency.

### 11.5 Miner side (`spc-agent`)

One binary (§1). Holds **`b_scan`** (to verify its own `P` in every job/distribution, §3.6, to scan both tiers, §2.5, and to sign registrations, §11.4), the mailbox key(s) for its mode, and **`b_ark`** unless running watch-only; it MUST NOT hold `b_spend` or the master seed. Compromise of the agent therefore exposes privacy and unrefreshed Ark-tier VTXOs only — never a coinbase output.

- **Keys (DESIGN.md / MAILBOX.md paths, one master seed).** `b_spend` `m/352'/0'/0'/0'/0` (cold), `b_scan` `m/352'/0'/0'/1'/0`, `b_ark` `m/352'/0'/0'/2'/0`, per-epoch `m_i` `m/352'/0'/0'/3'/i'` (`epoch_registered`; the agent may hold only the `…/3'` account xpriv). The agent's bark wallet — whose `m/350'/1'` key is the static mailbox `M` — runs on a **child seed** of the master (BIP 85-style), not on the master itself, so that holding the bark wallet does not mean holding `b_spend`. One backup (the master) recovers both tiers. The `spc_ark_client` prototype derives everything from one seed for test convenience; production MUST split as above.
- **Hot/cold posture (DESIGN.md).** Scanning and recognising `U_i` watch-only need only `b_scan` and `B_ark`; signing for a VTXO (status attestation, refresh, spend, exit) needs `u_i = b_ark + t_ark`. `b_ark` MAY be held hot by the agent (default: refresh is unattended) or by an external signer, which MUST then come online inside the refresh window. `b_ark` is never `b_spend`, and `b_spend` is never used on the Ark tier.
- **Register (mandatory).** On connect, once per Ark server: `SetArkIdentity` (§11.4) with `D = b_ark·S_mb` computed by whoever holds `b_ark`. In `epoch_registered` mode, also `RegisterEpochMailboxes` per MAILBOX.md — with sends at `H_f + 6`, after-the-find registration has only ~6 blocks, so pre-register a height window instead. (Default mode is `epoch_derived`, which needs no per-epoch registration; resolved 2026-10-14.) The agent MUST NOT submit shares until `SetArkIdentity.Success`: unregistered earnings below `C` are forfeited (§11.1), so a registration failure is a hard stop, not a warning.
- **Receive.** For each pool block found while scanning (§2.5): insert `U_i → u_i` (or `U_i` watch-only) into the SP-aware keystore (§11.8 item 2), then fetch per mailbox mode — `static`: bark's normal sync of `M`; `epoch_*`: MAILBOX.md §3 on-demand reads of `M_i` at `H_f + 6 + [0, 6, 36, 144, 1008]`, closing the epoch when the VTXO for `U_i` arrives or an SPc output for that block is found; `none`: `ArkPayoutNotice` / API `vtxo_id` → `GetVtxosByIds` → `import_vtxos`. Validate each VTXO (ARK #2) and record it against the notice.
- **Refresh.** Auto-refresh at `vtxo_refresh_expiry_threshold` (bark default 144 blocks before expiry, inside the free window). Optional `refresh_on_receive_above_sats`: refresh immediately (`refresh/counterparty`) for payouts above a size the miner cares about, shedding the sender+server trust early at 0.2–0.5 %.
- **Dormancy.** The agent runs while the miner mines; a miner that stops for > 28 days with unrefreshed VTXOs is swept (ToS: "no obligation to return"). The agent MUST warn at shutdown if unrefreshed VTXOs exist or Ark payouts are still scheduled, and SHOULD offer `--offboard-on-exit <address>` (cooperative on-chain payment of the whole Ark balance, one tx) for anyone going dark.

### 11.6 Audit across both tiers

At booking (`confirmation_depth` = 6), the agent asserts
`owed(id) [from the type-32 window] − paid_coinbase [own P found in the coinbase] − ark_due [pool-reported] − forfeited [pool-reported; MUST be 0 for a registered agent] − Δpending [pool-reported] == 0`;
when the Ark send lands (normally the same block) it asserts the VTXO received for `U_i` carries exactly `ark_due`. It alarms otherwise. Type-32 covers "what was I owed"; the coinbase output and the VTXO cover "what was I paid". ArkPool's hash-chained receipts are the post-hoc half of this only.

### 11.7 Trust and privacy statement (normative text for README)

Following DESIGN.md "Privacy analysis, by observer":

- **Custody.** Ark-tier balances are custodial for the 6-block confirmation window, then **trustodial for up to 28 days**: safe if either the pool (sender) or the Ark server is honest. The pool deliberately uses a server it does not control so that this remains a 1-of-2 assumption. Swept VTXOs are unrecoverable. Earnings below the coinbase threshold of a miner who has not registered Ark delivery are forfeited to the pool.
- **Pool compromise is split.** The pool's hot Ark seed is a separate key on a separate host that never sees who was paid in a coinbase; compromising it risks the pool's funds (float and unswept pool revenue), never SPc receiver privacy. Compromising the mining side risks attribution data and nothing spendable (it holds only an xpub, §11.3).
- **The pool** sees everything on both tiers, as it must to do the accounting; one identity makes this no worse. Mitigation is operational (scrub, §5.1).
- **On-chain observers** see coinbase-tier outputs only; the Ark tier is invisible on-chain, and derived Ark keys leak nothing. **Never publish a bundle** of SP address and Ark material (e.g. a BIP 321 `?sp=…&ark=…` URI): it would be the correlation point the design avoids.
- **The Ark server** sees every Ark-tier destination key, amount and timing, and can parse public coinbases. Bursts of pool-originated arkoors ~6 blocks after each pool block are fingerprintable, and amounts map to the pool's known payout distribution; the pool's boards, ~100 blocks after its blocks, link its coinbase outputs to its Ark wallet (the pool's own outputs, not miners'). In `static` mailbox mode it links all of a miner's Ark payouts through `M`; the `epoch_*` modes reduce that to one block, with the network-level residue MAILBOX.md describes (new connection / Tor isolation and jitter per read). If a coinbase-tier miner later boards that output into Ark, the server links it to their Ark activity. Second publishes no privacy policy.
- **Irreducible costs**: amount fingerprinting on the coinbase tier (§0); tier transitions are observable to anyone watching both layers; the Ark server as cross-layer observer. Soft mitigations only: timing jitter, amount rounding, boarding coinbase outputs via paths that do not advertise "fresh coinbase payout".

### 11.8 Ark-tier derivation and delivery (DESIGN.md, authoritative)

The Ark destination comes from the **same per-block ECDH** as the coinbase output, domain-separated by tag (§2.3 is normative; prototype `spc_ark/`):

```
t_ark = hash_SPc/TierArk(ecdh_i ‖ ser32(k))   →  U_i = B_ark + t_ark·G        (the VTXO Pubkey policy key; k = 0)
t_mb  = hash_SPc/Mailbox(ecdh_i)               →  epoch_derived mailbox tweak  (MAILBOX.md variant B)
```

Both fail on a digest of 0 or `≥ n` (§2.3). The miner recomputes `ecdh_i` for every block the pool found, so it finds Ark payouts for blocks where it was below threshold with no notification.

**Delivery, by mailbox mode.** bark blinds a mailbox id as `blinded = M + u·S_mb`; the server unblinds `blinded − s_mb·U = M`. The pool lacks `u = b_ark + t_ark` but, by linearity, needs only `D = b_ark·S_mb` (DESIGN.md "Recommended approach" 1; tested in `pool_blinding_matches_bark`):

| `mailbox_mode` | Pool computes per payout | Wallet reads | Server can link a miner's payouts? |
|---|---|---|---|
| `static` (DESIGN.md) | `blinded = M + D + t_ark·S_mb` | `M` (bark's normal sync) | yes, all of them, via `M` |
| `epoch_derived` (v0.1 default; resolved 2026-10-14, MAILBOX.md variant B) | `M_i = M + t_mb·G`; `blinded = M_i + D + t_ark·S_mb` | `M_i` with `m_i = m + t_mb`, on demand | no (only within one block); `m_i` are related keys — privacy-only exposure (see caveat below) |
| `epoch_registered` (MAILBOX.md variant A) | registered `M_i`; `blinded = M_i + D + t_ark·S_mb`; if `M_i` missing at send: `static` or hold, per `epoch_fallback` | `M_i` from `m/352'/0'/0'/3'/i'`, on demand | no; independent per-epoch keys |
| `none` (DESIGN.md "Fallback") | address with an **empty delivery list** (valid per `check_delivery`) | nothing; fetch by `vtxo_id` and `import_vtxos` | not by mailbox |

The address is `ark::Address::builder().server_pubkey(..).pubkey_policy(U_i).delivery(ServerMailbox { blinded })` (`spc_ark::build_bark_address`); `BlindedMailboxIdentifier::from_pubkey(U)` alone is a dead letter (DESIGN.md (b)). Default is `epoch_derived` (resolved 2026-10-14, `docs/decisions/2026-10-14-mailbox-default-epoch.md`). **Caveats on `epoch_derived`:** (a) mailbox keys are additive tweaks of static `m` — anyone holding BOTH static `m` and `b_scan` can derive all `m_i`; this is privacy-only exposure (VTXO spending needs `u_i = b_ark + t_ark_i`, which stays independent); (b) reads of `M_i`, `M_j` from one IP/connection are linkable at the network level — use a new gRPC connection per epoch, Tor stream isolation where possible (MAILBOX.md §4).

**The receiving wallet needs an SP-aware keystore** (DESIGN.md "Recommended approach" 2). Stock bark resolves keys only via `m/350'/0'/idx` and drops a VTXO to `U_i` as "not owned by this wallet" (`find_signable_clause`, and `import_vtxos` → `KeyNotFound`) — confirmed against a stock server in RESULTS.md. `spc-agent`'s embedded wallet consults an extra-keys source holding `U_i → u_i` for each scanned pool block; signing is already abstracted over a plain `Keypair`. Patch vs fork is O13.

### 11.9 Tests (extend §8)

- Unit: `SetArkIdentity` matrix — bad `b_scan` signature, signature over another `sp_id`, `B_ark = B_spend`, invalid points, wrong `ark_id`, unknown mode, `none` with `M`/`D` present, `static` without them → exact error codes; nothing derived for a rejected registration. DESIGN/MAILBOX frozen vectors per §8.1.
- Property: §4.6 (a)–(f) hold with the tier enabled (Ark-tier ids have no coinbase output); separately, per settled block `pay_P = fee_P + rounding + Σ ark_due' + Σ forfeited + Δpending`, where `fee_P` is the pool's own fee share (§4.5 `pool` minus rounding) and `Δpending = Σ pending' − Σ pending`. Raising `C` moves value from coinbase to `ark_due'`/`forfeited` and never changes `Σ owed`.
- Integration (regtest, `captaind` as the *third-party* role, `confirmation_depth = 6`): five agents; block found at `H_f`; the above-`C` miner gets a coinbase output; three registered below-`C` miners in `static`, `epoch_derived` and `none` modes get **nothing before `H_f + 6`**, then exactly one VTXO each from the float — in `M`, in `M_i` (static `M` stays empty), and via notice + `import_vtxos` respectively — with a full expiry window; an unregistered below-`C` miner (hand-rolled client) is booked `forfeit`, its sats in `pay_P`, nothing pending; agents reconcile §11.6 to zero; a `SetArkIdentity` for another miner's `sp_id` without its `b_scan` signature is rejected; `spc-agent` against a pool that rejects its registration submits no shares; kill the Ark server → sends fail → retried → `failed_to_pending` after `retry_blocks` (never forfeit); reorg of `H_f` before `due_height` cancels its Ark payouts; two competing same-height pool blocks → ark boards exactly one outpoint; at `H_f + 100` the pool output is found by its height-indexed script and only `float_target_sats − float_balance` is boarded, the remainder left on-chain under `ark`; with the float already at target nothing is boarded; `--offboard-on-exit` produces one on-chain tx.
- Negative: pool configured with its own `captaind` on mainnet → refuses to start (`ark.server_is_self = true` requires `network ∈ {regtest, signet}`); `pool_payout_descriptor` with an xprv, a hardened step, multipath, no wildcard, or a non-`tr()` wrapper → refuses to start.
- Separation (§11.3): `ark` with only its own config and Redis ACL can execute orders but cannot read any Postgres table or identity-bearing Redis key (connection attempts fail); a captured `spc:ark:orders` stream contains no `sp_id`, `B_scan`, `B_spend`, `P_i` or block hash (schema test); unsigned, wrong-`ark_id` and over-`max_order_sats` orders are rejected and nothing is sent; the SPc-side processes hold no private key for `pool_payout_descriptor`; `ark` refuses to start if its seed file is readable from the SPc-side secret store path; mainnet config with `ark` in the same process as `front`/`payout` refuses to start.

### 11.10 Configuration additions

```toml
# spc-pool.toml additions (coinbase_threshold_sats lives in [accounting], §6.3)
[ark]
enabled = true
server = "https://ark.second.tech"       # third-party; pinned ark_id below
ark_id = "…"                              # first 4 bytes of SHA256(server pubkey)
float_target_sats = 0                     # REQUIRED: board up to this at each matured pool output; ≈ Ark-tier sats of the blocks expected in 100
max_order_sats = 0                        # 0 = coinbase_threshold_sats (an Ark payout is always < C)
max_daily_sats = 0                        # optional extra cap; 0 = off
order_pubkey = "…"                        # ark side: BIP340 x-only pubkey (64 hex chars) of the order-signing key (CSPRNG, independent of ark seed and all SPc keys), verifies payout's order signatures; REQUIRED
order_key_path = "/run/secrets/spc/ark-order-key"   # payout side: order-signing key (secp256k1 BIP340, CSPRNG-generated,
                                          # independent of ark seed and all SPc keys); file 0o600, dir 0o700 (bark fs_perms)
seed_path = "/run/secrets/ark/seed"       # ark's own secret store; MUST NOT be shared with SPc-side secrets
pool_output_account = "m/86'/1'/1'"       # pool output keychain under the ark seed (coin type per network); its xpub is [pool] pool_payout_descriptor
min_remaining_blocks = 3024               # pay only from VTXOs with ≥ 21 d left
retry_blocks = 6
server_is_self = false                    # true only on regtest/signet

# spc-agent.toml additions
[ark]
enabled = true
mailbox_mode = "epoch_derived"            # "epoch_derived" (default) | "static" | "epoch_registered" | "none"
epoch_fallback = "static"                 # epoch_registered only: "static" | "hold"
signer = "local"                          # "local" (b_ark hot) | "external" (watch-only U, §11.5)
refresh_on_receive_above_sats = 0         # 0 = never; else refresh/counterparty immediately
offboard_on_exit = ""                     # optional on-chain address
```

`ark` refuses to start while a REQUIRED value is 0, or if `pool_output_account`'s xpub does not match `[pool] pool_payout_descriptor`. With `max_daily_sats` set, an order over it is not dropped: it waits for the next day's allowance and alarms, so a legitimate burst costs latency, not funds.

## Sources

- [Roadmap: SPc pool](ROADMAP.md) — the eight decisions this spec implements
- [Ark (bark) as a Sub-Threshold Payout Rail](research/wiki/concepts/ark-sub-threshold-payout-rail.md), [Payout Threshold Economics](research/wiki/concepts/payout-threshold-economics.md) — §11 grounding
- [bark send-side read](research/raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md), [bark key-side read](research/raw/repos/2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md) — §11.3–11.5, Appendix 11.8
- [Inventory: coinbase-native stealth payout](research/inventory/candidates/coinbase-native-stealth-payout.md) — construction, carrier table, SV2 budget, close-out condition 1 (this spec's §2 is the "spec diff")
- [LL 2026-08-14](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md) — height nonce, `k = 0`, light client, sender-side split
- [LL 2026-08-16](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md) — no BIP 32 for `a_send`; `String` tag type; budget; rotation as a knob; wire-format-enforced invariants
- [LL 2026-08-24](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) — JD carriers, `OP_RETURN`, unhidable suffix, amount ceiling, standalone pitch for the tag patch
- [delvingbitcoin t/2833](research/raw/articles/2026-10-09-delving-silent-payments-coinbase-thread.md) — Seldenthuis's budget correction; Anzus's scanning-profile question; optout's BIP-posture question
- [PPLNS-JD / SLICE](research/wiki/concepts/pplns-jd.md), [SV2 Share Accounting Extension](research/wiki/concepts/sv2-share-accounting-ext.md) — `N = 8 × D`, window definition, type-32 messages, identity-free types
- [xpub Payout Identity](research/wiki/concepts/xpub-payout-identity.md), [Coinbase Address Rotation](research/wiki/concepts/coinbase-address-rotation.md), [Spec rev. 5](research/output/plan-xpub-miner-identity-spec-2026-07-29.md) — ledger-key split, fail-closed, per-connection payout slot
- [Coinbase Amount Linkability](research/wiki/concepts/coinbase-amount-linkability.md), [Payout Attribution Privacy](research/wiki/concepts/payout-attribution-privacy.md), [Block Withholding](research/wiki/concepts/block-withholding.md) — threat model boundaries
- [Blitzpool raw](research/raw/repos/2026-07-27-blitzpool-server-rust-github.md), [finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) — ledger, trim, autoscaler, roles, snapshot fingerprint, vouching invariant, duplicate-merge landmine
- [share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md) — no per-share rows; replay set non-persistence
- External pins: `bip-0352.mediawiki` v1.1.1 (§Versions, §Spending, `:88–:99`, `:299`, `:319`); `sv2-spec` @ d8bf5b8 (`05-Mining-Protocol.md` `user_identity`/`error_code` = `STR0_255`; `extensions/0x0001-extensions-negotiation.md`); PR #203 `0x0003-coinbase-transaction-payouts.md` @ 47dbedd (unchanged since 2026-09-10, verified 2026-10-12); `share-accounting-ext/extension.md` @ c661e93; `lorbax/pplns-with-job-declaration.tex`; `bip-0323.mediawiki` (context only)
