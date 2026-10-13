> Copied from the research wiki (`bitcoin-mining-payout-schemas/output/plan-silent-pool-spec-2026-10-09.md`) on 2026-10-12. Cited wiki articles and raw sources are snapshotted under `docs/research/` (same relative layout); links were rewritten to point there.

---
title: "Spec: silent-pool — SPc pool technical specification"
type: plan
format: spec
generated: 2026-10-09
updated: 2026-10-12
status: current
supersedes: none
changelog:
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
  - "Hash tags: reuse BIP0352/Inputs and BIP0352/SharedSecret"
  - "A_send: x-only 32 B on the wire, even-Y normalisation"
  - "License: MIT OR Apache-2.0 (changed 2026-10-12 from AGPL-3.0-or-later — Blitzpool math is re-implemented from §4.6/§4.8, not ported; matches SRI, bark, extensions_sv2 and the spc_ark prototype)"
  - "a_send cadence: per block height in both modes"
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

**silent-pool** pays each miner of a found block to a **fresh, unlinkable P2TR output in the coinbase**, derived from the miner's BIP 352 silent-payment address by the **SPc** construction (coinbase-scoped silent payments). Accounting is **PPLNS-JD / SLICE**. The pool never holds miner funds; sub-dust and trimmed amounts are **pending credits**, carried forward and paid in a later coinbase.

| Term | Meaning |
|---|---|
| `B_scan`, `B_spend` | miner's BIP 352 scan/spend public keys, parsed from the `sp1q…`/`tsp1q…` address in `user_identity` |
| `sp_id` | `tagged_hash("SPc/Id", ser_P(B_scan) ‖ ser_P(B_spend))`, 32 B — the pool's internal ledger key. **Not** a consensus or wire value; never leaves the pool |
| `a_send`, `A_send` | pool's per-height ephemeral ECDH keypair; `A_send` is published in the coinbase, `a_send` is never persisted |
| `H` | block height being mined (the `SetNewPrevHash` height + 1 relationship is fixed by the template) |
| `P_i` | miner `i`'s payout output key for height `H` |
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
   │   spc-jdc    │◀────────────────▶ spc-jds        ├─ distribution builder → coinbase outputs                  │
   │ (sp_address) │──SV2 MP────────▶  (shares state) └─ Redis streams: spc:slices, spc:blocks, spc:stats         │
   └──────┬───────┘                                                 │                                            │
          └── TDP ─▶ miner's bitcoin-core (JD mode)                 ▼                                            │
                                                     spc-pool :: payout ──▶ Postgres (slice, snapshot, ledger)  │
                                                     spc-pool :: api    ──▶ HTTP (public + b_scan-authenticated) │
   miner wallet ◀── spc-scan (header + merkle branch + coinbase ≈ 500 B/block) ◀── any block source ─────────────┘
```

**Components**

| Binary / crate | Role | Forked from |
|---|---|---|
| `spc-pool` | one binary, `--roles front,payout,api` (Blitzpool pattern). `front`: SV2 listeners, template stream, job building, share validation, slice aggregation, block submit. `payout`: slice persistence, confirmation-gated ledger apply, scrub. `api`: read-only HTTP | `sv2-apps/pool-apps/pool` |
| `spc-jds` | Job Declarator Server implementing sv2-spec PR #203 (`0x0003`), publishing `SetPayoutDistribution` per height; shares Postgres/Redis with `spc-pool` (PR #203 §10: "Pool and JDS are assumed to share state") | `sv2-apps/pool-apps/jd-server` |
| `spc-translator` | SV1→SV2 proxy; `sp_address` config becomes the upstream `user_identity`; verifies its own payout appears in every job (§3.6) | `sv2-apps/miner-apps/translator` |
| `spc-jdc` | Job Declarator Client with `0x0003`; builds outputs per PR #203 §4; verifies own `P_i` in the distribution | `sv2-apps/miner-apps/jd-client` |
| `spc-crypto` | BIP 352 address parsing, `sp_id`, SPc derivation and scanning, zeroising key types | new (`secp256k1`, `bitcoin` crates) |
| `spc-scan` | library + CLI: find and export SPc payouts from blocks or header+branch+coinbase triples | new |
| `spc-accounting` | pure, I/O-free: slices, window, SLICE scores, pending ledger, trim, autoscaler, output builder | new; ledger/trim/autoscaler re-implemented from §4.6/§4.8 (Blitzpool patterns, no code) |
| `spc-share-ext` | extension type 32 messages on current `stratum-core` codec; server + sampling verifier | `demand-open-source/share-accounting-ext` |

**Miner-side consolidation (added 2026-10-12).** `spc-translator` and `spc-jdc` are listed separately because that is how SRI ships them; since this project writes the miner side itself, they SHOULD be **one binary, `spc-agent`**: SV1/SV2 downstream listener, optional JDC (own `bitcoind`, declares to the pool), payout verifier (§3.6), and the Ark-rail agent of §11.5. One process, one config, one seed.

**Process boundaries that matter.** Only `front` ever holds `a_send`. Only `payout` writes Postgres. `api` reads Postgres and Redis. `spc-jds` holds the per-height `a_send` for JD mode and MUST run co-located with (or be the same process as) `front` so one key per height exists pool-wide (§3.2).

## 2. SPc cryptographic protocol (normative)

All hashes are BIP 340 tagged hashes. `ser_P(X)` is 33-byte compressed; `ser_x(X)` is the 32-byte x-coordinate; `ser32(n)` is 4-byte big-endian; `G`, `n` are secp256k1's. Everything in this section is testable against the regenerated `sp-coinbase-vectors`.

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
- One `a_send` per height **pool-wide**: `front` and `spc-jds` MUST share it (same process, or `front` publishes `a_send` to `spc-jds` over a local authenticated channel — never Redis).
- A restart mid-height generates a new `a_send` for the same `H`. This is safe: outputs differ, both are scannable (§2.5). Reuse of one `a_send` across two heights is **not** a correctness failure (the height is in `input_hash`) but widens forward-secrecy exposure; `front` MUST alarm if `ser_x(A_send)` repeats across heights.

### 2.3 Output derivation (pool → miner `i`, height `H`)

```
input_hash = tagged_hash("BIP0352/Inputs", ser32(H) ‖ ser_P(A_send))      # ser_P(A_send) = 0x02 ‖ ser_x
ecdh_i     = input_hash · a_send · B_scan,i
t_i        = tagged_hash("BIP0352/SharedSecret", ser_P(ecdh_i) ‖ ser32(0)) # k = 0 always
P_i        = B_spend,i + t_i·G
output_i   = P2TR(ser_x(P_i)), amount = pay_i                              # scriptPubKey: OP_1 <32 B>
```

Design notes, each with its source:
- **Nonce = height.** Replaces `outpoint_L`, which is constant across all coinbases. Shared, non-circular, consensus-unique by BIP 34, and no gap limit applies because the receiver computes one candidate for the height in front of it rather than scanning forward ([08-14 L1–L2](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md)).
- **`input_hash` commits to `A_send`.** This, not the nonce, is what defeats the `why_include_A` grinding attack (BIP 352 footnote; [08-24 L2](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)). The pool chooses `a_send` freely every height, so it has exactly the freedom the attack needs — the commitment removes it.
- **Tag reuse (interview decision).** `BIP0352/SharedSecret` is applied to the same `(point, k)` shape as BIP 352, so wallet libraries reuse `create_output` unchanged. `BIP0352/Inputs` is applied to a 37-byte preimage (`4 + 33`) where BIP 352 uses 69 bytes (`36 + 33`); the length difference rules out cross-structure ambiguity. Reviewer pushback on the second is anticipated and the defence is recorded here. Security does not depend on the tags.
- **`k = 0` always.** Each miner has a distinct `B_scan`, so each is a BIP 352 group of size one. One output per `(sp_id, H)` is an invariant of the accounting (§4.6), so `k` never increments, `K_max` is irrelevant, and the BIP 352 `:319` dropped-output footgun has nothing to bite ([08-14 L5, L7](research/raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce.md)).
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
        input_hash = tagged_hash("BIP0352/Inputs", ser32(h) ‖ 0x02 ‖ A)
        ecdh = input_hash · b_scan · lift_x_even(A)
        t    = tagged_hash("BIP0352/SharedSecret", ser_P(ecdh) ‖ ser32(0))
        P    = B_spend + t·G
        if OP_1 <ser_x(P)> ∈ coinbase.vout: record (txid, vout, amount, t)
```

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

### 3.2 Extension negotiation

Clients that want JD payouts or auditing send `RequestExtensions{request_id: U16, requested_extensions: SEQ0_64K[U16]}` (ext `0x0001`) immediately after `SetupConnection.Success`. `spc-pool` advertises:

| ID | Extension | Mandatory for |
|---|---|---|
| `0x0001` | extensions negotiation | everyone (`spc-translator`, `spc-jdc`) |
| `0x0003` | Coinbase Transaction Payouts (PR #203) | `spc-jdc`, on **both** its JDP and MP connections |
| `0x0020` (32) | Share Accounting (§3.7) | optional; `spc-translator` and `spc-jdc` request it by default |

A client whose `RequestExtensions.Error` lists `0x0003` MUST NOT run JD mode (PR #203 §2).

### 3.3 Pool-template mode: job construction

Per `NewTemplate` (and per `SetNewPrevHash`) `front`:

1. Looks up `a_send[H]` (§2.2).
2. Asks `spc-accounting` for `Distribution(H, T = coinbase_tx_value_remaining, A_send)` (§4.7) → ordered outputs `[pool_output, P2TR(P_i)…]`.
3. Builds `coinbase_tx_prefix = version ‖ vin_count ‖ null_prevout ‖ scriptSig_len ‖ height_push ‖ push32(A_send)` and `coinbase_tx_suffix = sequence ‖ outputs ‖ locktime`, honouring `coinbase_output_max_additional_size` reserved from the TP (`spc-pool` sets it from the current weight budget, §4.8, Blitzpool's per-stream reservation).
4. Freezes the distribution as a **snapshot** in Redis keyed by `fingerprint = sha256(ser32(H) ‖ ser_x(A_send) ‖ Σ ser(output_i))` with TTL `snapshot_ttl_secs` (default 1200). The fingerprint rides every job so the winning share names its payout list exactly (Blitzpool `payouts_fingerprint`).
5. Sends `NewExtendedMiningJob` / `NewMiningJob`. Standard channels receive the merkle root only; they cannot verify payouts (§3.6).

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
- on **ledger apply** (booking at `confirmation_depth`, §7.4): force a republish regardless of `distribution_rebalance_bps` so pending credits reach `payouts` immediately, and drop the grace slot — `distribution_id − 1` is rejected after this republish (PR #203 §10: the grace window SHOULD NOT span a settlement event; Blitzpool's `invalidate_all_distributions()` + forced publish does exactly this at booking, not at found-time). Found-time invalidation is already satisfied by the `SetNewPrevHash` bullet plus the same-height rule: every id published before the pool's own block belongs to the previous height and is rejected.

```
SetPayoutDistribution {
  distribution_id:    U64   strictly increasing, pool-wide
  pool_payout:        TxOut{ script: pool P2TR, value: weight_P }           # weight, not sats
  payouts:            [ TxOut{ script: OP_1 ser_x(P_i), value: weight_i } ] # one per sp_id kept by §4.8
  dust_limits:        [ 330 … ]                                             # per payouts[i]; P2TR dust
  additional_outputs: [ TxOut{ script: OP_RETURN "SPc" ser_x(A_send), value: 0 } ]
}
```

`weight_P` = pool fee weight **plus** the weight of every identity not represented in `payouts` (under-threshold, trimmed), per PR #203 §3.1; `payout` books those as pending credits when the block settles (§4.6).

**Validation (`spc-jds` and `front`).** On `DeclareMiningJob` (Full-Template) or `SetCustomMiningJob` (Coinbase-only) carrying the `distribution_id` TLV (`0x0003`/`0x01`, 8 B): recompute `pay[i] = floor(weight_i · (T − F) / W)`, prune `< dust_limits[i]`, `pay_P = T − Σ pay[i] − F`, assert output order `pool_payout, payouts…, additional_outputs, JDC 0-value outputs, TP outputs`; mismatch → `invalid-payout-distribution`. Unknown/old id → `stale-payout-distribution`.

**Grace window (R4 in the roadmap).** `spc-jds` MAY accept `distribution_id − 1` **only if it was published at the same height `H`**. A declaration built for `H` and accepted at `H + 1` would pay outputs nobody can scan at `H + 1` (`input_hash` differs). Height-straddling declarations MUST be rejected with `stale-payout-distribution`. `spc-scan`'s `{H, H − 1}` tolerance is belt-and-braces for third-party pools, not a license to relax this rule.

**Shares under JD.** Carry `job_fees` from the declared job's template value into `ShareRec`; this is where `score_f` becomes non-trivial (§4.4).

**Leak, stated.** Every JDC sees `payouts` (one-time `P_i` scripts, no identity) and relative weights. Weights are derivable from on-chain amounts anyway; the residue a retaining JDC gains is distributions for blocks nobody won and the sub-dust set (PR #203 §9.2; [08-24 L5](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)). `max_payout_slots` (default 256) is the only dial.

### 3.6 Client-side payout verification (`spc-translator`, `spc-jdc`)

Both clients know `(b_scan or B_scan, B_spend)`, `H`, and `A_send` (from the job's `coinbase_tx_prefix` or the distribution's `OP_RETURN`). Before mining a job they MUST derive their own `P` (§2.3, receiver side) and check it appears among the coinbase outputs (extended channel) or `payouts` (JD). Result handling is `require_own_payout = warn | refuse` (default `warn`, because a small miner is legitimately dust-pruned or trimmed some blocks). This is the "pre-block payout verification via the existing encrypted session" the wiki lists as engineering-solvable ([08-24 L6](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md)), and it is also how `spc-translator` detects at startup that it is pointed at an SPc pool at all (no `push32` after the height ⇒ abort with a loud error).

### 3.7 Share Accounting extension (type 32 = `0x0020`)

Served by `front` from the slice store (§5), message set and semantics per `extension.md` @ c661e93 with these **documented deltas** (`docs/share-accounting-ext-deltas.md`):

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
| `coinbase_weight_budget` floor / ceiling | 50 000 / 400 000 WU | autoscaler bounds |
| `max_payout_slots` | 256 | hard cap on miner outputs (JD leak dial, firmware ceilings) |
| `retain_windows` | 16 blocks | slice retention for type-32 audits |
| `scrub_after_blocks` | 100 | delete `sp_id → P_i` snapshot entries this many blocks after settlement |

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
kept      = { id : owed(id) ≥ dust_limit_sats }
kept      = trim_to_budget(kept, weight_budget, max_payout_slots)      # §4.8
for id in kept:    pay(id) = owed(id); pending'(id) = 0
for id not kept:   pay(id) = 0;        pending'(id) = owed(id)
pay_P     = T − Σ pay(id)                              # fee + rounding + sats backing new pending credits
```

**Custody statement.** Sats behind pending credits land in `pay_P` and are owed by the pool until a later block pays them (PR #203 §3.1/§9.3 says the same of `weight_P`). This is the one place silent-pool is not non-custodial, bounded by `dust_limit × identities + trimmed set`. It is reported at `/api/pool/liability`.

**Invariants (proptested).** (a) `Σ pay + pay_P = T`. (b) one output per `(sp_id, H)` — `kept` is a set keyed by `sp_id`, so Blitzpool's missing duplicate-address merge ([finder-bonus code read](research/raw/repos/2026-07-27-blitzpool-finder-bonus-code-read.md) §8) cannot recur. (c) `Σ pending' − Σ pending = Σ_{not kept} raw − Σ_{kept} pending`. (d) idempotent apply: re-applying a settled block changes nothing (§5.1 `ledger_history` PK).

**Settlement reads the mined coinbase, never the snapshot's sats.** `payout` books from the block's actual coinbase: `T_actual = Σ coinbase outputs`; `paid(id)` = value of the output whose script is `OP_1 ser_x(P_id)` from `entries` (0 if absent). Pool-template mode: `T_actual = T` and `paid(id) = pay(id)` by construction, so nothing changes. JD mode: the JDC paid `floor(owed(id) · (T_actual − F) / W)` at **its** template revenue (PR #203 §4) and may have dust-pruned an id the snapshot kept, so the mined sats differ from `entries[].sats` on every JD block whose `T_actual ≠ T`; `pending'(id)` MUST be derived from `paid(id)`, never copied from the snapshot — which claim it is derived against is O11 (§10). Blitzpool books `claim(T_actual) − paid` per address from the mined coinbase (`bp-pplns-engine` `on_block_found`, `ActualCoinbase`) for the same reason; the §11.6 miner audit closes to zero only under this rule.

### 4.7 Distribution builder

```
Distribution(H, T, a_send):
    (raw, pending) ← §4.4–4.6 on the current window-in-progress   # the window ending at "now"
    for id in kept: P_id = derive(a_send, H, B_scan,id, B_spend,id)      # §2.3
    outputs = [ pool_output(pay_P) ] ++ [ P2TR(P_id, pay(id)) sorted by sp_id ] ++ carrier (JD only)
    fingerprint = sha256(ser32(H) ‖ ser_x(A_send) ‖ Σ ser(outputs))
    entries = [(sp_id, ser_x(P_id), pay(id))]       # snapshot payload; scrubbed later
```

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

CREATE TABLE slice (
  slice_id    BIGINT PRIMARY KEY,                -- monotone, assigned by front
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
  entries         JSONB NOT NULL,                -- [{sp_id, p_x, sats}] ; scrubbed to [] after scrub_after_blocks
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE block_found (
  block_hash   BYTEA PRIMARY KEY,
  height       INTEGER NOT NULL,
  fingerprint  BYTEA NOT NULL REFERENCES distribution_snapshot,
  status       TEXT NOT NULL CHECK (status IN ('pending','confirmed','orphaned')),
  found_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  settled_at   TIMESTAMPTZ
);

CREATE TABLE ledger_history (                    -- idempotency + accounting; no scripts
  block_hash   BYTEA REFERENCES block_found,
  sp_id        BYTEA REFERENCES sp_identity,
  paid_sats    BIGINT NOT NULL,
  pending_delta BIGINT NOT NULL,
  PRIMARY KEY (block_hash, sp_id)
);

CREATE TABLE pending_balance (
  sp_id      BYTEA PRIMARY KEY REFERENCES sp_identity,
  sats       BIGINT NOT NULL CHECK (sats >= 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Nothing here is a per-share row ([share accounting & durability](research/sibling/mining-pool-architecture/wiki/concepts/share-accounting-and-durability.md)). The attribution surface is exactly `slice_credit` + `distribution_snapshot.entries`; the **scrub job** deletes `slice_credit` rows for slices older than the oldest unsettled window + `retain_windows`, and empties `entries` for snapshots settled more than `scrub_after_blocks` ago. `ledger_history` keeps `(block_hash, sp_id, sats)` for idempotency and pending reconciliation but never a script, so a DB dump after scrub links identities to **amounts**, not to **outputs** — the residual leak is the amount channel, which exists on-chain anyway (§0).

### 5.2 Redis

| Key / stream | Producer → consumer | Content |
|---|---|---|
| `spc:slices` (stream) | front → payout | sealed `Slice` + its per-identity aggregates |
| `spc:blocks` (stream) | front → payout | `{block_hash, height, fingerprint}` on `SubmitSolution` accepted |
| `spc:stats` (stream) | front → payout/api | per-minute `{sp_id, accepted_d, rejected, hashrate_est}` — never per share |
| `spc:snapshot:<fingerprint>` | front → payout | `Distribution` entries, TTL `snapshot_ttl_secs` (1200) |
| `spc:dist:current` | jds → front | current `distribution_id`, `H` (JD validation) |

Consumer groups give at-least-once delivery; `ledger_history`'s PK makes apply idempotent, so redelivery is harmless (Blitzpool's exactly-once-by-idempotency).

### 5.3 `front` process memory

`channels: HashMap<ChannelId, (sp_id, B_scan, B_spend)>` · `a_send: Option<(u32 /*H*/, Zeroizing<[u8;32]>, [u8;32] /*A_x*/)>` · `open_slice: Slice` · `seen_shares: HashSet<[u8;32]>` (cleared on `SetNewPrevHash`) · `window_cache: Vec<SliceAgg>` (per-slice `Σd`, `Σd·f`, per-id maps; rebuilt from Postgres at start) · `budget: Autoscaler`.

Loss on crash: the open slice (≤ `max_slice_shares` shares, typically seconds of work) and `seen_shares` (mitigated §3.4). `ponytail:` accepted for v0.1; upgrade path is a periodic open-slice checkpoint to Redis.

## 6. Interfaces

### 6.1 HTTP API (`api` role, read-only, JSON)

Public endpoints expose **pool-wide** data only; anything per-identity requires proof of `b_scan`, otherwise the API is an attribution oracle for anyone holding a miner's public address.

| Method / path | Auth | Returns |
|---|---|---|
| `GET /api/pool` | — | hashrate estimate, connected channels, `N`, `D`, current `H`, `A_send_x`, mode flags, extensions advertised |
| `GET /api/pool/window` | — | current window: `t`, `Σd`, `Σd·f` per slice (no identities) |
| `GET /api/pool/blocks?limit=` | — | `block_found` rows (hash, height, status, output count, `pay_P`) |
| `GET /api/pool/liability` | — | `Σ pending_balance.sats` — the pool's outstanding pending-credit liability (§4.6) |
| `GET /api/pool/capacity` | — | weight budget, utilisation, slots used/max, next autoscale direction |
| `POST /api/sp/challenge` | — | `{nonce}`; 60 s TTL |
| `GET /api/sp/me` | `Authorization: SPc <addr> <nonce> <schnorr_sig(b_scan, "SPc/api" ‖ nonce)>` | pending balance, accepted difficulty last 24 h, last 50 `{block_hash, height, paid_sats}` (no scripts — the miner derives those with `spc-scan`) |
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
pool_payout_script = "tb1p…"    # P2TR; receives pay_P
fee_bps = 100

[template]
source = "ipc"                   # "ipc" (bitcoin-core Cap'n Proto) | "sv2-tp"
socket_path = "/run/bitcoin/node.sock"
extranonce_bytes = 16            # scriptSig budget: 100 − 4 − 33 − 16 = 47 B spare (§2.4.1)

[accounting]
window_n = 8
slice_fee_delta_sats = 0
max_slice_shares = 65000
dust_limit_sats = 330
max_payout_slots = 256
retain_windows = 16
scrub_after_blocks = 100
confirmation_depth = 3

[budget]
floor_wu = 50000
ceiling_wu = 400000
up_threshold = 0.85
down_threshold = 0.50
step_factor = 1.15
up_debounce = 3
down_debounce = 20
cooldown_secs = 300

[jd]
enabled = true
distribution_interval_secs = 60
distribution_rebalance_bps = 100
grace_same_height_only = true    # MUST stay true (§3.5)

[extensions]
share_accounting = true          # 0x0020
window_hot_blocks = 6

[storage]
postgres = "postgres://…"
redis = "redis://…"
snapshot_ttl_secs = 1200
```

```toml
# spc-translator.toml                     # spc-jdc.toml adds [tp] for the miner's own node
[upstream]
address = "pool.example:34254"
authority_public_key = "…"
sp_address = "tsp1q…"                      # becomes user_identity; validated at startup
require_own_payout = "warn"                # "warn" | "refuse"   (§3.6)
request_share_accounting = true

[downstream]
listen = "0.0.0.0:34255"                   # SV1 ASICs point here; their usernames are labels only
```

`spc-pool` MUST refuse to start if `pool_payout_script` is not P2TR (the reference scanner and the output-order rule assume a 34-byte script, and a non-P2TR pool output would be the only fingerprint distinguishing pool from miner outputs).

## 7. Implementation details

### 7.1 Workspace

```
silent-pool/                      MIT OR Apache-2.0
  Cargo.toml                      workspace; rust-toolchain pinned to sv2-apps'
  crates/spc-crypto               secp256k1, bitcoin, bech32, zeroize            (no I/O)
  crates/spc-scan                 lib + bin                                       (bitcoin RPC optional)
  crates/spc-accounting           pure; proptest dev-dep                         (no I/O)
  crates/spc-share-ext            binary-sv2 / codec-sv2 message structs + server/verifier
  bins/spc-pool                   fork of sv2-apps pool + roles
  bins/spc-jds                    fork of sv2-apps jd-server + 0x0003
  bins/spc-translator             fork of sv2-apps translator
  bins/spc-jdc                    fork of sv2-apps jd-client + 0x0003
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

Per template: `n_kept` scalar multiplications (`ecdh_i` — `B_scan,i` is fixed, `input_hash·a_send` is one scalar per height, so `ecdh_i = s_H · B_scan,i` is **one** point multiplication per miner per height, cached until `H` changes) plus one `P_i = B_spend,i + t_i·G`. At 256 slots that is ≤ 512 EC ops per height, not per template — the outputs' keys don't change within a height; only amounts do. Share path adds one `HashSet` insert and one `Vec::push`; slice sealing adds one merkle root over ≤ 65 000 leaves (~2 ms) off the share path.

### 7.4 Idempotency and exactly-once

Block settlement runs in one Postgres transaction: `INSERT block_found … ON CONFLICT DO NOTHING` → if inserted, `INSERT ledger_history` rows (PK rejects replays) → `UPDATE pending_balance` → `UPDATE block_found.status`. Orphan check (`confirmation_depth`) runs before the transaction; an orphaned block writes `status='orphaned'` and **no** ledger rows. A block whose snapshot was lost (TTL expired, Redis flush) is booked as `status='pending'` with `fingerprint` unresolved and alarms — a build MUST never fail over a lost snapshot (Blitzpool's vouching invariant), and the operator reconstructs from the on-chain coinbase + `slice_credit` if needed.

### 7.5 Pool-tag patch

`channels-sv2/src/server/jobs/factory.rs`: `pool_tag_string: Option<String>` → `pool_tag: Option<Vec<u8>>`; the `Sv2/<pool>/<miner>/` ASCII framing is dropped when `pool_tag.len() == 32` and the push is emitted as `OP_PUSHBYTES_32`. `fits_script_sig_budget` keeps its conservative 8-byte-height check; `ScriptSigSizeTooLarge` still fails at channel setup, not at block submission ([08-16 L5](research/raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier.md)). Upstreamable as a standalone change ("binary pool tags"), independent of SPc ([08-24 L7](research/raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability.md) — pitch the change so it stands without the proposal).

### 7.6 Blitzpool-derived design (no code ported)

The pending-credit ledger, keep-largest weight-budget trim, autoscaler constants, payout-list fingerprint, confirmation-gated idempotent apply and the vouching invariant are **patterns and parameters taken from Blitzpool's README and this wiki's code reads**, re-implemented here from §4.6 / §4.8 / §7.4 — not copied from `bp-pplns`, which is AGPL-3.0-or-later. Clean-room rule for implementers: write from the spec text and its proptests, do not read `crates/bp-pplns/src/distribution.rs` while doing so. Differences from Blitzpool are deliberate: the ledger key is `sp_id` not `address`; there is no finder bonus and therefore no debit side (`pending` is unsigned); withheld value lands in `pay_P` (`WithheldValue::ToPool` shape, §11.2). Blitzpool is cited as provenance (`7815884`) and as the `0x0003` JDS reading reference (`b48998f`, §7.1); no `NOTICE` obligation arises.

## 8. Testing strategy

### 8.1 Unit — `spc-crypto` (vectors regenerated for the chosen tags)

Regenerate `~/repos/sp-coinbase-vectors` with `BIP0352/*` tags and `ser_P(A_send) = 0x02‖x` in `input_hash`, then port all cases as `#[test]`s:
- positive: ordinary-tx split sanity; coinbase native (null prevout, height nonce); `N = 5` fan-out with a dropped output not affecting others; nonce distinctness across heights and across `A_send`.
- negative controls (each MUST fail where stated): vanilla BIP 352 eligibility gate rejects the coinbase; constant-null-outpoint nonce collides across blocks; `input_hash` without `A_send` admits the `:92` grind (and binding `A_send` kills it); odd-Y `A_send` without negation misses; group-linear key list `A_H = A_0 + H(A_0‖H)·G` yields every epoch from `a_0`.
- carriers: scriptSig layout round-trips byte-exact; `OP_RETURN "SPc"` round-trips; the rejected 1-of-2 multisig carrier stays as a **documentation test** that asserts `a_send` can sign the output (why it's rejected).
- 28 upstream BIP 352 vectors still pass through the shared `create_output` path (tag reuse is exercised, not assumed).

### 8.2 Property tests — `spc-accounting`

`proptest` over random windows (1–40 slices, 1–300 identities, random `d`, `f`, `T`, pending, budget): invariants §4.6 (a)–(d); `δ = 0` pool-template result equals an independent plain-PPLNS oracle to the sat; sorting by `sp_id` is deterministic across runs; trim never exceeds budget or `max_payout_slots`; §4.5 worked example as a fixed fixture.

### 8.3 Integration — SV2 surface (in-process, no bitcoind)

- channel open with `sp`, `tsp`, bare `bc1q`, descriptor, `sri/solo/…`, empty, 63-char-truncated, two addresses on one connection → exactly the §3.1 table; **assert no coinbase was built for a rejected channel**.
- `RequestExtensions` matrix: `{0x0001}`, `{0x0001, 0x0003}`, `{0x0001, 0x0020}`, unknown id → correct `.Success`/`.Error` lists; JDC with `0x0003` on only one connection refuses JD mode.
- share replay (same share twice, same share on two channels), stale job after restart → rejected.
- slice sealing on prev-hash change, fee band, `max_slice_shares`; merkle root matches an independent implementation.
- `SetPayoutDistribution` published on height change; validation rejects mutated amounts, reordered outputs, a missing `OP_RETURN`, and a **height-straddling** declaration under the grace window; accepts `id − 1` at the same height.
- type-32: `GetWindow` for a found block returns a window whose `Σ difficulty ≥ N × D`; `GetShares` merkle paths verify; a pool build compiled with `--features cheat-slice-difficulty` is caught by the verifier within 3 windows at `4 × 8` sampling (statistical test, seed-pinned).

### 8.4 End-to-end — regtest harness (`testing/regtest-harness`, CI)

bitcoin-core (IPC) + `spc-pool` + `spc-jds` + 2 × `spc-translator` + 1 × `spc-jdc` + CPU miners:
1. **Pool-template happy path**: three `tsp` identities mine; block found; coinbase has pool P2TR + 3 P2TR + no `OP_RETURN`; each `spc-scan find` recovers exactly its own output; each spends it with `b_spend + t`.
2. **JD happy path**: `spc-jdc` declares; block found; coinbase has `OP_RETURN "SPc"`; JDC's own `P` present; `spc-scan` recovers via carrier 2; type-32 `GetTransactionsInJob` returns the JDC's set.
3. **Dust and trim**: one identity below 330 sats → pending; next block pays `owed`; `/api/pool/liability` goes to 0.
4. **Orphan**: invalidate the found block → `status='orphaned'`, no ledger rows, pending unchanged.
5. **Restart mid-height**: kill `front`; new `A_send` for the same `H`; shares resume; next block still scannable.
6. **Fail-closed**: SV1 miner with a bare address → `spc-translator` refuses to start; a hand-rolled SV2 client with a bare address → `invalid-user-identity`, nothing mined for it.
7. **Light client**: `spc-scan light` on exported `header‖branch‖coinbase` triples finds the same outputs as `find`.
8. **Scrub**: after `scrub_after_blocks`, `distribution_snapshot.entries = []` and `slice_credit` rows beyond retention are gone; `ledger_history` still reconciles `pending_balance`.
9. **Full mempool**: fill the regtest mempool to ≈ 990 000 vB (Blitzpool `regtest_budget_autoscale.rs` `MEMPOOL_TARGET_VBYTES`; Core packs to `block_max_weight − block_reserved_weight`, so a part-full mempool never binds) before (a) the pool mines its own template with `max_payout_slots` outputs and (b) `spc-jdc` declares a block with `max_payout_slots` outputs; both blocks must pass the JDS node's `checkBlock` and be accepted. Fails with `bad-blk-weight` on `bitcoin_core_sv2` from upstream `sv2-apps` `main`/`v0.8.0` once `coinbase_output_max_additional_size > 208 B` (it reserves `max(4·size, 2000)` instead of TDP's `1168 + 4·size`); passes with the `block_reserved_weight` fix pinned in §7.1 or with `source = "sv2-tp"`. Steps 1–8 cannot surface this — it is the one class of bug the 2026-10-11 mainnet run found.

### 8.5 Adversarial (manual + CI where deterministic)

Grinding `a_send` to collide with a known earlier payment (must be a fixed-point search — test asserts the naive solve fails); JDC distribution shopping across `id−1/id` at the same height (accepted by design, bounded by `distribution_rebalance_bps`); 10 k channels with distinct valid addresses (memory, autoscaler, `max_payout_slots` trimming order).

## 9. Deployment

**Topology.** One `spc-pool` binary; dev/testnet runs `--roles front,payout,api` in one process; production splits `front` (one per region, stateless except RAM) from a single `payout` (sole Postgres writer) and N `api` replicas. `spc-jds` co-located with `front` (shared `a_send`, §2.2). Redis and Postgres are the only durable state; **no key material is durable anywhere** — the pool's own `pool_payout_script` is a watch-only P2TR address, and its spending key never touches these hosts.

**Migrations.** `sqlx migrate` at boot under `pg_advisory_lock`; every process may run them (Blitzpool pattern). Schema changes are additive until v1.0.

**Rollout.** regtest CI → testnet4 (`tsp`) soak ≥ 24 h with ≥ 1 found block independently recovered by a third party's `spc-scan` → publish `docs/spc-spec.md` to the Delving thread and the BIP mailing list → mainnet is out of scope for this document.

**Rollback.** Stopping `front` stops mining; no miner funds are at risk (payouts are already on-chain). Pending credits survive in Postgres and are paid when the pool resumes. Downgrading `spc-pool` across a schema version is unsupported; restore from the pre-migration dump.

**Monitoring (Prometheus).** `spc_a_send_age_seconds` (alarm > 1 h — a stuck height), `spc_a_send_repeat_total` (alarm > 0), `spc_open_slice_shares`, `spc_slices_sealed_total{reason}`, `spc_distribution_slots{kept,trimmed,dust}`, `spc_budget_wu`, `spc_budget_utilisation`, `spc_snapshot_lookup_total{hit,miss}` (miss on block found = page), `spc_blocks_total{status}`, `spc_pending_liability_sats`, `spc_channel_reject_total{reason}`, `spc_jd_declaration_reject_total{code}`, `spc_ext32_requests_total{msg}`.

**Operational privacy.** Logs never contain `user_identity`, `sp_id`, or `P_i` at `INFO`; `DEBUG` redacts to 8-hex-char prefixes. Postgres backups inherit the scrub policy; a backup older than `scrub_after_blocks` is itself attribution data and SHOULD be rotated on the same schedule.

## 10. Open items (carried from the roadmap, narrowed by this spec)

| # | Item | Status |
|---|---|---|
| O1 | BIP posture — amendment to 352 vs new informational BIP | **Open.** With `BIP0352/*` tags reused, the document reads as an *extension profile* of BIP 352 ("coinbase inputs"), which argues for an amendment PR first and a new BIP only if NACKed. |
| O2 | `A_send` serialization inside `input_hash`: `ser_P` (this spec) vs `ser_x` | Decided `ser_P` for maximal reuse of BIP 352's `input_hash` shape; vectors regenerate accordingly. |
| O3 | `{H, H−1}` scan tolerance | Decided for `spc-scan`; pool-side grace is same-height-only. Revisit if third-party SPc pools appear. |
| O4 | Labels (BIP 352 `B_m`) | Deferred to v0.2; one identity per address in v0.1. |
| O5 | Solo-pool profile (one payee) | Same carrier and derivation; a `[pool] mode = "solo"` flag is a ~20-line addition; not in v0.1. |
| O6 | Rail for pending credits below the coinbase threshold | **Resolved 2026-10-12 → §11 (Ark via a third-party bark server).** Lightning/BOLT12 stays a v0.2 alternative for miners without an Ark wallet. |
| O7 | Empirical mainnet scriptSig occupancy alongside AuxPoW tags | Unmeasured; needed before any mainnet claim about the 47 B spare. |
| O8 | Amount-linkability mitigation (randomised slot count, deferred payouts) | Research; see proposed inventory record in the roadmap. |
| O9 | `demand-share-accounting-ext` porting cost on current `stratum-core` | Unknown until Phase 6; message set is small. |
| O10 | PR #203 drift | Pinned at `47dbedd` (the PR's single commit, 2026-09-10; no spec movement as of 2026-10-12 — warioishere's 2026-10-11 comment pins warioishere/stratum#1 and sv2-apps#3 to the same commit). Residual text risk: TheBlueMatt's unanswered 2026-09-10 question on all-static payouts with only a subset paid (would relax the §7 'no partial payment of `additional_outputs`' MUST at L168) and warioishere's requested §9.3 sentence (prose only). `spc-jds`/`spc-jdc` isolate `0x0003` in one module each. |
| O11 | JD-mode booking when the JDC's template revenue ≠ the pool's (§4.6) | `pending'(id) = claim(id) − paid(id)` needs a definition of `claim`: revenue-scaled `owed(id) · (T_actual − F) / W` keeps `pending` unsigned but scales carried balances with the winning template; fixed-sat `raw(id) · (T_actual − F) / W + pending(id)` keeps balances in sats but can go negative (signed ledger, §7.6). Decide before Phase 4; either way §11.3 Ark sends pay `pending'` after booking, not `owed` before it. |

## 11. Sub-threshold Ark rail (added 2026-10-12)

> Earnings that do not fit the coinbase are paid over **Ark (bark)** from a pool-held wallet on a **third-party** Ark server. Grounding: [Ark (bark) as a Sub-Threshold Payout Rail](research/wiki/concepts/ark-sub-threshold-payout-rail.md), [Payout Threshold Economics](research/wiki/concepts/payout-threshold-economics.md), and the two bark code reads ([send side](research/raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md), [key side](research/raw/repos/2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md)). The rail changes **how** `owed(id)` is paid, never how §4 computes it — SLICE scoring, JD or pool-template, is untouched.

### 11.1 Policy (replaces the second half of §4.6)

```
owed(id) = raw(id) + pending(id)
if   owed ≥ T                      → coinbase candidate (subject to §4.8 trim); if trimmed → pending (next block)
elif dust_limit_sats ≤ owed < T    → Ark: one arkoor send of owed; pending'(id) = 0 on success, else owed stays pending
else                               → pending'(id) = owed
```

- `T` = `ark_threshold_sats`, default **500 000**, published at `/api/pool`. Rationale: Braiins' Lightning ceiling is 500 000; OCEAN's coinbase cut is 2^20; the binding constraint on the Ark side is what a miner can afford to lose to a 28-day sweep, which argues for keeping `T` low ([Payout Threshold Economics](research/wiki/concepts/payout-threshold-economics.md) §5).
- Trimmed-above-`T` miners are **not** diverted to Ark: large amounts are exactly what should not sit under arkoor trust. They wait one block (Blitzpool's "fairness, never validity").
- Miners with no registered Ark address (§11.4) accrue `pending` indefinitely — OCEAN's behaviour — and are paid in the coinbase once `owed ≥ T`.
- Cadence: **once per block found**, never per share or per template (Second: per-minute payments "build long exit chains … prohibitively expensive to exit on small balances").

### 11.2 Both template modes

| | Pool-template | Job Declaration (PR #203) |
|---|---|---|
| Where sub-threshold sats land | in `pay_P` (§4.6) | in `weight_P` — §3.1 of #203 explicitly allows "value credited on behalf of miners that are not represented in `payouts`" |
| Carrier for `A_send` | scriptSig (§2.4.1) | `OP_RETURN` (§2.4.2) |
| What the pool does after settlement | `payout` role sends arkoor to each sub-threshold id from the float | identical |

The coinbase leg is unchanged in both modes; the Ark leg is purely pool-side settlement. The JD column is the `WithheldValue::ToPool` weight composition from Blitzpool `bp-pplns` (`weights.rs`, origin/main), deliberately not its PPLNS `ToOtherMiners` composition — the one warioishere described on PR #203 (2026-09-11): `weight_P` is the fee only, withheld weight is spread over the published miners, who are then booked matching debits. Silent-pool cannot use `ToOtherMiners`: the Ark float (§11.3) is funded from `pay_P`, so the withheld sats MUST land in the pool output, and §4.6's `pending` is unsigned. Unlike Blitzpool's Team Mining (which pairs `ToPool` with no ledger), the withheld value is booked as pending (§4.6) and paid (§11.1). The JDC computes `pay_P = T − F − Σ pay[i]` without inspecting how `weight_P` was composed, so the two shapes are wire-identical. `distribution_snapshot.entries` gains `rail ∈ {coinbase, ark, pending}` per entry.

### 11.3 Pool side (`payout` role)

- **Server choice.** `[ark] server = "https://…"` MUST be a server **not operated by the pool**. Arkoor security is "at least one party (sender or Ark server) honest" (Second docs); a pool that is both removes the assumption ([critiques](research/raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md) §1). Default: Second's mainnet server (`ark_id` pinned in config). A self-hosted `captaind` MAY be used **only on regtest/signet for tests**.
- **Wallet.** Embedded `bark-wallet` (or `barkd` over REST) owned by `payout`; seed in the same secret store as Postgres credentials — this is the one hot wallet in the system and its blast radius is bounded by the float.
- **Float.** Board the matured pool output (`pay_P` after 100 confirmations) into VTXOs; target `float_sats ≥ ark_float_blocks × E[sub-threshold sats per block]` (default 3 blocks). Pay only from VTXOs with `expiry_height − tip ≥ ark_min_remaining_blocks` (default **3 024 ≈ 21 d**) so recipients inherit a near-full window; refresh float VTXOs below that (costs 0.2 % per Second's tiers — accepted). Keep `arkoor_depth` of float VTXOs under the server's `max_vtxo_exit_depth` by refreshing on depth, not only on expiry.
- **Settlement loop** (per confirmed block, idempotent via `ledger_history`): for each `(sp_id, owed)` on the Ark rail → `send(ark_address(sp_id), owed)`; on success record `ledger_history{rail:'ark', vtxo_id, expiry_height}` and emit `ArkPayoutNotice` (§11.4); on failure leave `pending` untouched and retry next block; alarm after `ark_retry_blocks` (default 6).
- **Fees.** 0 % on sends/receives at Second today; the pool absorbs refresh costs out of its own fee. Nothing is deducted from miners for the rail.

### 11.4 Identity transport — the SPc extension

**Derivation first (per `DESIGN.md`, §11.8).** The Ark destination is derived per block from the coinbase ECDH; the miner never sends a per-payout address. The extension carries the miner's **static** Ark material once; a literal address is the fallback for wallets that cannot hold derived keys.

`user_identity` remains the SP address and nothing else. Ark material travels in a project-defined SV2 extension negotiated via `0x0001` (identifier `0x5350` — "SP"; renumber if assigned):

| Msg | Dir | Fields | Semantics |
|---|---|---|---|
| `SetArkIdentity` | C→S | `b_ark_pub: B033` (optional; absent ⇒ `B_ark = B_spend`, cold — discouraged), `mailbox_pubkey: B033`, `mailbox_dh: B033` (`D = b_ark·S_m` for the pool's configured server) | Register once the static material the pool needs to derive and deliver per-block Ark payouts (§11.8). Pool validates the points and that `mailbox_dh` targets its configured server; `SetArkIdentity.Success` / `.Error{invalid-point, wrong-ark-server}`. |
| `SetArkPayout` | C→S | `ark_address: STR0_255` | **Fallback** for wallets that cannot hold derived keys: register/replace a literal Ark payout address. Pool validates: bech32m; HRP matches network; version `p` (bark v1); `ark_id` == configured server; policy `Pubkey`; ≥ 1 `ServerMailbox` delivery. |
| `SetArkPayout.Success` | S→C | — | Address accepted; applies to all future settlements for this `sp_id`. |
| `SetArkPayout.Error` | S→C | `error_code: STR0_255` | `invalid-ark-address`, `wrong-ark-server`, `unsupported-policy`, `no-delivery`. Fail closed — nothing is sent to a rejected address; earnings accrue as pending. |
| `ArkPayoutNotice` | S→C | `block_hash: U256, sats: U64, vtxo_id: B0_64, expiry_height: U32` | Sent after each successful arkoor send so the agent can reconcile without waiting for mailbox sync (§11.6). |

A miner MAY send `SetArkPayout` at any time (e.g., a fresh address every block); the pool uses the latest accepted one at settlement. No 32-byte cap, no overloading of `user_identity`, no `0x0002` dependency.

### 11.5 Miner side (`spc-agent`)

One binary (§1 note). Holds **`b_scan`** (to verify its own `P` in every job/distribution, §3.6), **`b_ark`** (§11.8) and a **bark wallet seed**; it MUST NOT hold `b_spend`. Consequence worth stating: compromise of the agent exposes privacy and the sub-threshold float only — never a coinbase output.

- **Single-seed recovery.** Derive the bark wallet seed from the miner's SP master seed (BIP 85-style child, e.g. `m/83696968'/…` or `tagged_hash("SPc/ArkSeed", master)`), so one backup recovers both rails; `B_ark` (§11.8) is a separate child under the same seed.
- **Register.** On connect: `SetArkIdentity { B_ark, M, D }` once (§11.4; all derived from the agent's seed, `b_ark` hot). Fallback for a non-derived wallet: `bark.new_address()` → `SetArkPayout`.
- **Receive.** For each block the pool found, compute `t_ark` from that coinbase's `A_send` and insert `u = b_ark + t_ark` into the external-key table (§11.8 item 2); subscribe to the mailbox (or the per-block mailbox `M_H`, §11.8 item 3); validate each VTXO (ARK #2) and that its policy key is now wallet-owned; record against the `ArkPayoutNotice`.
- **Refresh.** Auto-refresh at `vtxo_refresh_expiry_threshold` (bark default 144 blocks before expiry, inside the free window). Optional `ark_refresh_on_receive_above_sats`: refresh immediately (`refresh/counterparty`) for payouts above a size the miner cares about, shedding the sender+server trust early at 0.2–0.5 %.
- **Dormancy.** The agent runs while the miner mines; a miner that stops for > 28 days with unrefreshed VTXOs is swept (ToS: "no obligation to return"). The agent MUST warn at shutdown if unrefreshed VTXOs exist and SHOULD offer `--offboard-on-exit <address>` (cooperative on-chain payment of the whole Ark balance, one tx) for anyone going dark.

### 11.6 Audit across both rails

Per settled block, the agent asserts
`owed(id) [from the type-32 window] − paid_coinbase [own P found in the coinbase] − paid_ark [ArkPayoutNotice ∧ mailbox VTXO] − Δpending [pool-reported] == 0`
and alarms otherwise. Type-32 covers "what was I owed"; the two receipts cover "what was I paid". ArkPool's hash-chained receipts are the post-hoc half of this only.

### 11.7 Trust and privacy statement (normative text for README)

Ark payouts are **trustodial for up to 28 days**: safe if either the pool (sender) or the Ark server is honest; the pool deliberately uses a server it does not control so that this remains a 1-of-2 assumption. The Ark server sees the pool-wallet → miner-mailbox graph and every amount; Second publishes no privacy policy. The amount channel is open on both rails (§0). Swept VTXOs are unrecoverable.

### 11.8 Ark-tier key derivation — adopted per `DESIGN.md` (2026-10-12)

The Ark destination is derived from the **same per-block ECDH** as the coinbase output, domain-separated by tier (`DESIGN.md` at the repo root; prototype `spc_ark/`):

```
ecdh      = input_hash · a_send · B_scan          (= input_hash · b_scan · A_send on the miner side)
t_onchain = H(ecdh ‖ "spc" ‖ k)   →  P = B_spend + t_onchain·G     (coinbase tier, §2.3)
t_ark     = H(ecdh ‖ "ark" ‖ k)   →  U = B_ark   + t_ark·G         (Ark tier: the VTXO `Pubkey` policy)
```

- **`B_ark` is a separate key under the miner's seed, held hot** by `spc-agent` (DESIGN.md open question 2, resolved toward separation): VTXO keys sign refreshes, forfeits and arkoor cosigns, so they cannot be cold; `b_spend` stays cold and is never used on the Ark tier.
- **Never reuse `t` across tiers** — `P − U` would expose `B_ark − B_spend` (DESIGN.md's warning). The tags do this; `k` stays per-output within a tier as in BIP 352. Tag strings are placeholders pending the SPc spec's tagging convention (§2.3 uses `BIP0352/*`).
- The miner recomputes `ecdh` for every block the pool found (`A_send` is public in that block's coinbase), so it finds Ark payouts for blocks where it was below threshold with no notification required; `ArkPayoutNotice` (§11.4) is a convenience.

What the bark code read ([key-side read](research/raw/repos/2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md)) adds — DESIGN.md's open questions 2 and 4, answered against `c419159f`:

1. **Mailbox delivery needs two static points from the miner.** bark blinds the mailbox id as `Blinded = M + v·S_m` (`M` = recipient's per-seed mailbox pubkey, `v` = VTXO secret, `S_m` = server mailbox key) and the server unblinds to `M`. The pool lacks `v = b_ark + t_ark`, but by linearity needs only `D = b_ark·S_m`: `Blinded = M + D + t_ark·S_m`. So `SetArkIdentity` (§11.4) carries `B_ark`, `M`, `D` **once**; nothing per payout, nothing published. `BlindedMailboxIdentifier::from_pubkey(U)` — the prototype's current placeholder in `build_bark_address` — is not deliverable and is replaced by `Address::builder().delivery(ServerMailbox { blinded })` with the value above.
2. **The receiving wallet needs an external-key table.** Stock bark resolves keys only via `m/350'/0'/idx` and drops a VTXO to `U` as "not owned by this wallet"; `spc-agent`'s embedded wallet inserts `u = b_ark + t_ark` for each found block before matching. Signing is already abstracted over a plain `Keypair`, so nothing else changes.
3. **Linkage at the server is via the mailbox, not the key.** A fresh `U` per payout hides nothing from the Ark server while every delivery unblinds to one `M`. Optional hardening, available only because the agent is ours: derive a per-block mailbox `M_H = M + H(ecdh ‖ "mbx")·G` (secret `m + …`, hot) and read one mailbox per found block; the server then sees unrelated mailboxes and unrelated keys, leaving the timing/amount correlation DESIGN.md already lists as irreducible.

Supersedes the same-day evaluation that had rejected derivation: its objection (i) is void with a separate hot `B_ark`; (ii) is addressed by per-block mailboxes or consciously accepted; (iii) is a one-time registration over the authenticated SV2 session, not an identity bundle.

### 11.9 Tests (extend §8)

- Unit: `SetArkPayout` validation matrix (wrong HRP, wrong `ark_id`, Arkade `q` version, non-Pubkey policy, no delivery, 130/131-char valid) → exact error codes; nothing sent to a rejected address.
- Property: the coinbase identity §4.6 (a) `Σ_{kept} pay(id) + pay_P = T` is unchanged by the rail (Ark ids have no coinbase output; `T` here is block revenue, not §11.1's `ark_threshold_sats`); separately, per settled block `pay_P = fee_P + rounding + Σ_{ark} owed(id) + Δpending`, where `fee_P` is the pool's own fee share (§4.5 `pool` minus rounding) and `Δpending = Σ pending' − Σ pending` (§4.6 (c)). An id is never on two rails in one block.
- Integration (regtest, `captaind` as the *third-party* role): three agents; block found; above-`T` miner gets a coinbase output; two sub-threshold miners each receive one VTXO in their mailbox with `expiry_height − tip ≥ 3 024`; agents reconcile §11.6 to zero; kill the Ark server → sends fail → pending retained → retried after restart; agent without `SetArkPayout` accrues pending and is paid in the coinbase once ≥ `T`; `--offboard-on-exit` produces one on-chain tx.
- Negative: pool configured with its own `captaind` on mainnet → refuses to start (`ark.server_is_self = true` requires `network ∈ {regtest, signet}`).

### 11.10 Configuration additions

```toml
[ark]
server = "https://ark.second.tech"       # third-party; pinned ark_id below
ark_id = "…"                              # first 4 bytes of SHA256(server pubkey)
threshold_sats = 500000                   # T
float_blocks = 3
min_remaining_blocks = 3024               # pay only from VTXOs with ≥ 21 d left
retry_blocks = 6
server_is_self = false                    # true only on regtest/signet

# spc-agent.toml additions
[ark]
enabled = true
rotate_address_every_blocks = 1
refresh_on_receive_above_sats = 0         # 0 = never; else refresh/counterparty immediately
offboard_on_exit = ""                     # optional on-chain address
```

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
