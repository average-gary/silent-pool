# silent-pool — Component Architecture

Status: design-phase map for implementation planning. The only code today is the
validated prototype trio (`spc_ark/`, `spc_ark_mailbox/`, `spc_ark_client/`,
see `RESULTS.md`); everything else is specified in `docs/SPEC.md` (§1, §5, §6,
§7.1, §11) and phased in `docs/ROADMAP.md`. Where this file and the spec
disagree, the spec wins; where the payout derivation is concerned,
`DESIGN.md` wins over both.

## 1. System context

```
                        ┌───────────────────────── MINER SIDE ─────────────────────────┐
                        │                                                              │
   SV1 ASICs ──SV1──▶   │   ┌────────────────────── spc-agent ─────────────────────┐   │
                        │   │ translator role (SV1→SV2)                            │   │
   SV2 miners ──────────┼──▶│ JDC role (optional, needs miner's own bitcoin-core)  │   │
                        │   │ payout verifier · scanner · SP-aware bark keystore   │   │
                        │   └───────┬──────────────────────────────┬───────────────┘   │
                        └───────────┼──────────────────────────────┼───────────────────┘
                          SV2 MP    │                JDP + 0x0003  │  TDP
                                    │                              │
┌─────────── POOL SIDE ─────────────┼──────────────────────────────┼───────────────────┐
│                                   │                              │                   │
│   ┌──────────── spc-pool ─────────┼──────────────────────────────┴──────────┐        │
│   │  role: front  (includes the   │  spc-jds module: JDP listener, 0x0003   │        │
│   │  role: payout                  │  publication/validation — JD always on)│        │
│   │  role: api                     │                                        │        │
│   │  role: ark  (own host, prod)   │                                        │        │
│   └──────┬───────────┬────────────┼─────┬──────────────────────────────────┘        │
│          │           │            │     │                                            │
│       Redis       Postgres     (TDP/IPC via bitcoin_core_sv2)                        │
│      streams      (payout only)                                                      │
└────────────────────────────────┼────────────────────────────────────────────────────┘
                                 │                              │
                    ┌────────────▼─────────┐        ┌───────────▼───────────┐
                    │  bitcoin-core (pool) │        │ third-party Ark server │
                    │  IPC or sv2-tp       │        │ (stock captaind; NOT   │
                    └──────────────────────┘        │  operated by the pool) │
                                                    └───────────▲───────────┘
                                                                │ arkoor / boards / mailbox
                                                     ┌──────────┴───────────┐
                                                     │ miner bark wallet /  │
                                                     │ spc-scan (any block  │
                                                     │ source, ~500 B/blk)  │
                                                     └──────────────────────┘
```

External systems the design depends on but does not build: bitcoin-core
(template provider; IPC with the `block_reserved_weight` fix or sv2-tp, SPEC
§7.1), a stock third-party `captaind` Ark server (SPEC §11.3, O17), Redis, and
Postgres. The pool never speaks SV1 and never runs an Ark server.

## 2. Component inventory

### 2.1 Binaries (planned; roadmap Phases 0–7)

| Binary | Forked from | Responsibility | Secrets held | Durable state access |
|---|---|---|---|---|
| `spc-pool --roles front` | `sv2-apps/pool-apps/pool` (incl. its bundled JDS) | SV2 MP listeners **and the JDP listener** (the `spc-jds` module: `0x0003` publication + validation, JD always on), template stream, job building, share validation, slice aggregation, block submit, distribution building + publishing, `a_send` generation | `a_send[H]` (RAM only, zeroised on height advance; SPEC §2.2); Noise authority key | Redis (write streams); Bootstrap RPC to `payout` at start (window rebuild; single-site: Postgres read) |
| `spc-pool --roles payout` | (Blitzpool pattern, re-implemented) | Slice persistence, confirmation-gated idempotent ledger apply, Ark send-order signing/scheduling, scrub job | Ark order-signing key (not the ark seed) | **Sole Postgres writer**; Redis streams |
| `spc-pool --roles api` | — | Read-only HTTP/JSON, public + `b_scan`-authenticated | none | Postgres read, Redis read |
| `spc-pool --roles ark` | bark wallet integration | Pool's bark wallet + Ark float; executes signed, capped `ArkSendOrder`s; boards matured pool outputs to refill float | the **`ark` seed** (bark wallet + pool output keychain `m/86'/coin'/1'`) — the pool's only hot spending key | Redis ACL: read `spc:ark:orders`, write `spc:ark:receipts` **only**; no Postgres |
| `spc-agent` | `sv2-apps/miner-apps/{translator, jd-client}` (+ warioishere/sv2-apps@90b3e3d for 0x0003 JDC) | Miner side, one binary/one seed: translator role (SV1→SV2), JDC role (own template), own-payout verifier (SPEC §3.6), both-tier scanner, SP-aware bark keystore, registration client (0x5350), refresh/offboard | `b_scan`, `b_ark`, mailbox key(s); **never** `b_spend` or the master seed (SPEC §11.5) | local wallet DB |
| `spc-scan` | new | Reference scanner lib + CLI; finds/exports SPc outputs from blocks or header+branch+coinbase triples; type-32 audit client | `b_scan` (watch-only use) | none (stateless) |

### 2.2 Library crates (planned workspace, SPEC §7.1)

| Crate | I/O | Responsibility | Absorbs |
|---|---|---|---|
| `spc-crypto` | none | BIP 352 address parse, `sp_id`, SPc ECDH, tier KDF (`t_onchain`/`t_ark`/`t_mb`), output/secret derivation, both carriers, mailbox blinding (static + epoch variants), `SendKey` zeroising type | `spc_ark/`, `spc_ark_mailbox/` (frozen vectors carried over) |
| `spc-accounting` | none (pure) | Slices, window selection, SLICE scores, pending ledger + tier routing (SPEC §11.1), weight-budget trim, autoscaler, distribution builder | Blitzpool `bp-pplns` **patterns** (clean-room, SPEC §7.6) |
| `spc-share-ext` | codec only | Extension type 32 (`0x0020`) message structs on current `stratum-core`, server + sampling verifier | `demand-open-source/share-accounting-ext` port (deltas documented) |
| `spc-ext` | codec only | Project extension `0x5350`: `SetArkIdentity`, `RegisterEpochMailboxes`, `ArkPayoutNotice` (SPEC §11.4) | new |
| `spc-scan` (lib) | optional bitcoin RPC | Carrier parsing, block/triple scanning, tweak export, audit client | part of `spc_ark_client/` scan logic |
| `vendor/channels-sv2` | — | patched `pool_tag: Option<Vec<u8>>` (SPEC §7.5) | vendored SRI fork |

### 2.3 Existing code (validated prototypes, to be absorbed)

| Crate | Proves | Lands in | Frozen vector |
|---|---|---|---|
| `spc_ark/` | SPc ECDH + domain-separated tier tweaks + bark address with pool-side mailbox blinding | `spc-crypto` | `src/lib.rs::test_vector` |
| `spc_ark_mailbox/` | rotating per-epoch mailboxes (variant A `3'/i'`, variant B `M + t_mb·G`), on-demand reads, authorization format | `spc-crypto` (+ agent scheduling logic) | `src/lib.rs::tests::test_vector` |
| `spc_ark_client/` | miner-side claim path: `MinerKeys`, `SpArkKeystore`, `claim_vtxos`, signing with `u` against a stock server | `spc-agent` (keystore + receive filter; RESULTS.md: ~150–300 line bark patch surface, spec O13) | regtest e2e (`tests/e2e.rs`) |

## 3. Crate dependency graph (build-time)

```
spc-crypto ──────────────────────────────────────────────┐
   ▲      ▲            ▲                ▲                │
   │      │            │                │                │
spc-scan  │      spc-share-ext    spc-ext (sig verify)   │
   │      │            ▲                ▲                │
   │      │            │                │                │
   │  spc-accounting   │                │                │
   │      ▲            │                │                │
   │      │            │                │                │
bins/spc-pool ─────────┴────────────────┘                │
   (incl. the spc-jds module: JDP + 0x0003)              │
bins/spc-agent ─────────(spc-crypto, spc-scan, spc-ext,
                          spc-share-ext verifier, bark libs)
```

Rules that keep the graph sane: `spc-crypto` and `spc-accounting` are pure and
I/O-free (proptestable; SPEC §8.1–8.2); codec crates depend on `stratum-core`
only; no binary reaches into another binary — all cross-process communication
goes through §5's explicit interfaces.

## 4. Pool-internal topology (one binary, four roles; JDS in `front`)

```
            SV2 MP (Noise)                     JDP + 0x0003
                 │                                  │
                 ▼                                  ▼
┌────────────────────────── front ───────────────────────────┐
│ channels: HashMap<ChannelId,(sp_id,B_scan,B_spend)>        │
│ a_send[H] (RAM, Zeroizing; erased at height advance)       │
│   ▲ shared by reference with the spc-jds module (in-proc)  │
│ spc-jds module: JDP listener · SetPayoutDistribution       │
│   publication (0x0003) · distribution_id TLV validation    │
│ ecdh_cache (per height) · open_slice · seen_shares         │
│ window_cache · budget autoscaler · current distribution    │
│ distribution builder (spc-accounting + spc-crypto)         │
└───┬───────────┬─────────────┬──────────────┬───────────────┘
    │ spc:slices │ spc:blocks            │ spc:stats (stream)
    │ (stream)   │ (stream)              │
    │            │ (entries in event)    │
    ▼            ▼                       ▼
┌─ Redis ─────────────────────────────────────────────────────────────────┐
│ spc:slices · spc:blocks (+entries) · spc:stats · spc:arkreg             │
│ spc:front:events · spc:ark:orders ─────┐   ┌──── spc:ark:receipts       │
└─────────────────────────────────────────┼───┼───────────────────────────┘
                                 ▼   │
┌──── payout ────────────────┐  ┌────┴──────── ark ────────────┐
│ sole Postgres writer       │  │ bark wallet + float          │
│ ledger apply at H_f + 6    │  │ verifies signed orders       │
│ ark_payout scheduling,     │  │ (order-signing key), caps    │
│ order signing, scrub       │  │ (max_order_sats, ark_id)     │
└───────┬────────────────────┘  └───────┬──────────────────────┘
        ▼                               ▼
┌─ Postgres ────────────────┐    third-party captaind (Ark gRPC)
│ sp_identity · ark_delivery │    send_arkoor_payment · board_psbt ·
│ ark_epoch_mailbox · slice  │    mailbox posts
│ slice_credit ·             │
│ distribution_snapshot ·    │    ┌──── api ──────────┐
│ block_found ·              │    │ read-only HTTP    │
│ ledger_history ·           │◀───┤ Postgres + Redis  │
│ ark_payout ·               │    └───────────────────┘
│ pending_balance            │
└────────────────────────────┘
```

Process-boundary rules (SPEC §1, as amended 2026-10-13): only `front` ever
holds `a_send` — the `spc-jds` module shares it by reference in-process; only
`payout` writes Postgres; only `ark` holds the `ark` seed and it sees no
attribution data — orders carry only what the Ark server will see anyway.
Dev/regtest MAY run all roles in one process; production splits `front`
(per-region, RAM-only state), one `payout`, N `api`, one `ark` on its own host
(SPEC §9).

## 5. Interface catalog

### 5.1 Wire protocols

| Interface | Endpoints | Transport | Content / notes |
|---|---|---|---|
| SV2 Mining Protocol | miners / `spc-agent` ↔ `front` | TCP + Noise | `user_identity` = SP address, fail-closed parse (SPEC §2.1, §3.1); shares; jobs. One connection = one `sp_id` |
| Ext `0x0001` | all SV2 clients ↔ `front` | in-band | extensions negotiation, immediately after `SetupConnection.Success` (SPEC §3.2) |
| Ext `0x0003` (PR #203 @ 47dbedd) | `spc-agent` JDC role ↔ `front` (`spc-jds` module) | in-band | `SetPayoutDistribution{distribution_id, pool_payout(weight_P), payouts[], dust_limits[], additional_outputs[0] = OP_RETURN "SPc"‖A_send}`; `distribution_id` TLV on `DeclareMiningJob`/`SetCustomMiningJob`; same-height-only grace (SPEC §3.5). Always served — no opt-out |
| Ext `0x0020` (32) | `front` ↔ auditing clients | in-band | share accounting: `NewBlockFound`, `GetWindow`, `GetShares`, `GetTransactionsInJob`, … (SPEC §3.7; deltas doc) |
| Ext `0x5350` ("SP") | `spc-agent` ↔ `front` | in-band | delivery capabilities (SPEC §11.4), see §5.4 below |
| TDP / IPC | `front` ↔ pool bitcoin-core | Cap'n Proto IPC or sv2-tp | templates; **must** carry the `block_reserved_weight = max(1168 + 4·size, 2000)` fix (SPEC §7.1) |
| TDP | `spc-agent` JDC role ↔ miner's bitcoin-core | sv2-tp | miner's own template source (JD mode) |
| HTTP/JSON | anyone / miners ↔ `api` | HTTP | see §5.3 |
| Ark server gRPC | `ark` role ↔ third-party `captaind` | gRPC | stock bark wallet ops: arkoor sends, `board_psbt`, status; nothing custom (RESULTS.md: server changes required: none) |
| Ark mailbox/gRPC | `spc-agent` ↔ `captaind` | gRPC | `ReadMailbox` per epoch (MAILBOX.md §3; never `SubscribeMailbox`), `GetVtxosByIds` fallback, attestations, refresh/spend |

### 5.2 Redis streams (SPEC §5.2)

| Key / stream | Producer → consumer | Content |
|---|---|---|
| `spc:slices` | front → payout | sealed `Slice` + per-identity aggregates (`Σd`, `Σd·f`) |
| `spc:blocks` | front → payout | `{block_hash, height, front_id, fingerprint, entries}` on accepted submit |
| `spc:stats` | front → payout/api | per-minute `{sp_id, accepted_d, rejected, hashrate_est}` — never per share |
| `spc:arkreg` | front → payout | `{sp_id, b_scan, b_spend, ark_id, b_ark, mailbox_mode, M, D, epoch_fallback, sig}` (registration) |
| `spc:front:events` | payout → all fronts | `ArkRegApplied{sp_id, rev}`, `LedgerApplied{block_hash, pending_deltas, provisional_paids, republish}`, `BlockFound{block_hash, height, front_id}`, `TipHeartbeat{hash, height}` |
| `spc:ark:orders` | payout → ark | signed `ArkSendOrder{order_id, ark_address, sats, not_before_height, sig}` — **no identity fields** |
| `spc:ark:receipts` | ark → payout | `ArkSendReceipt{order_id, status, vtxo_id, expiry_height}` |

At-least-once delivery via consumer groups; `ledger_history` PK makes apply
idempotent, so redelivery is harmless (SPEC §7.4). Fronts live-tail `spc:slices`
and `spc:front:events` with RAM-only cursors; trimming is `XTRIM MINID` behind
the slowest live reader, front cursor heartbeats included (SPEC §5.2). Payload
byte formats: [`WIRE-FORMATS.md`](WIRE-FORMATS.md).

### 5.3 Durable state and HTTP API

Postgres (written only by `payout`; SPEC §5.1): `sp_identity`,
`ark_delivery`, `ark_epoch_mailbox`, `slice`, `slice_credit` (the only
share→identity link; scrub target), `distribution_snapshot` (`entries` scrubbed
after settle + `scrub_after_blocks`), `block_found`, `ledger_history`
(idempotency; no scripts), `ark_payout` (send scheduler; `vtxo_id` scrubbed),
`pending_balance`. **No per-share rows anywhere.**

`front` process memory (SPEC §5.3): channel map, `ark_delivery` cache,
`a_send[H]`, `ecdh_cache`, `open_slice`, `seen_shares`, `window_cache`,
`budget`. Crash loss bounded to the open slice (accepted for v0.1,
`ponytail:`).

HTTP API (`api` role; SPEC §6.1): public pool-wide endpoints
(`/api/pool`, `/pool/window`, `/pool/blocks`, `/pool/liability`,
`/pool/capacity`, `/healthz`, `/metrics`); per-identity endpoints behind
`Authorization: SPc <addr> <nonce> <schnorr_sig(b_scan, …)>`
(`/api/sp/challenge`, `/api/sp/me`, `/api/sp/me/audit/:block_hash`). Signing
with `b_scan` keeps `b_spend` cold; per-identity data without the signature
would make the API an attribution oracle.

### 5.4 The `0x5350` delivery-capability extension (SPEC §11.4)

| Msg | Dir | Payload | Semantics |
|---|---|---|---|
| `SetArkIdentity` | C→S | `ark_id`, `b_ark_pub`, `mailbox_mode` (none/static/epoch_derived/epoch_registered), `mailbox_pubkey M`, `mailbox_dh D = b_ark·S_mb`, `epoch_fallback`, `sig` (BIP340 by `b_scan` over `hash_SPc/ArkIdentity(sp_id ‖ fields)`) | register/replace Ark delivery for this connection's `sp_id`; signature is mandatory because the Noise session authenticates the pool, not the miner |
| `SetArkIdentity.Success` / `.Error` | S→C | — / `error_code` | fail-closed: `bad-signature`, `invalid-point`, `ark-key-is-spend-key`, `wrong-ark-server`, `bad-mailbox-fields`, `unsupported-mailbox-mode` |
| `RegisterEpochMailboxes` | C→S | `SEQ0_64K[ser32(i) ‖ ser_P(M_i)]`, `sig` | variant-A epoch mailboxes (MAILBOX.md); pre-registration window ≈ 5.3 KB/day |
| `ArkPayoutNotice` | S→C | `block_hash, sats, vtxo_id, expiry_height` | convenience in mailbox modes; **the** delivery path in `none` mode; also at `/api/sp/me` |

### 5.5 Internal crate APIs (proposed per roadmap Phase 1–2; prototype-shaped)

`spc-crypto` (all pure; signatures exist in `spc_ark/src/lib.rs` today):
`parse_sp_address() → (B_scan, B_spend, network)` ·
`sp_id(B_scan, B_spend)` ·
`input_hash(H, A_send)` ·
`SharedSecret::pool_side / miner_side` ·
`tier_tweaks(ecdh, k) → {t_onchain, t_ark}` (+ `t_mb` for mailbox variant B) ·
`spc_output_pubkey / spc_output_secret` · `ark_user_pubkey / ark_user_secret` ·
`mailbox_blinding_hint(b_ark, S_mb) → D` ·
`pool_blinded_mailbox_id(M, D, t_ark, S_mb)` · `build_bark_address(...)` ·
carrier parse/emit: scriptSig `push32` after the BIP34 push;
`OP_RETURN "SPc"‖ser_x(A_send)` · `SendKey` (zeroising, `Debug` prints only
`{h, A_x}`).

`spc-accounting` (pure; SPEC §4): `seal_if_needed` / `seal` ·
`select_window(slices, D)` · SLICE scoring (`raw(id)`, SPEC §4.4) · pending
ledger + tier routing (SPEC §4.6, §11.1) · `trim_to_budget` + autoscaler
(SPEC §4.8) · `Distribution(H, T, a_send)` builder (SPEC §4.7; emits sats in
pool-template mode, weights in JD mode).

`spc-share-ext`: message structs + `serve(window_store)` + `verify(samples)`
(sampling verifier, `s` slices × `m` shares; SPEC §3.7).

## 6. Secrets and trust boundaries

| Secret | Held by | Protects | Compromise costs |
|---|---|---|---|
| `a_send[H]` | `front` RAM only (shared in-process with the `spc-jds` module), erased at height advance | one height's payee set (forward secrecy) | that height's payee set; **no funds** (it authorises nothing) |
| attribution data (`slice_credit`, snapshot `entries`, `ark_delivery`) | SPc-side Postgres/Redis, scrubbed per policy | receiver privacy | linkage of identities to amounts/tiers until scrub; never to scripts after scrub |
| `ark` seed | `ark` role, own host/secret store | the float + unswept pool revenue | float + surplus; **no SPc receiver privacy** (sees only server-visible order fields) |
| Ark order-signing key | `payout` | order authenticity | forged orders still capped by `max_order_sats` and pinned `ark_id` |
| Noise authority key | `front` | session authentication | standard SV2 exposure; disjoint from both trees above |
| miner `b_scan` | `spc-agent` (+ wallet) | scan capability, registration signatures | privacy, not funds |
| miner `b_ark` | `spc-agent` (hot) or external signer | unrefreshed Ark VTXOs | Ark-tier funds only; never a coinbase output |
| miner `b_spend` / master seed | cold signer only | coinbase outputs | never on the agent (SPEC §11.5) |

The headline property: **SPc receiver privacy and Ark-tier funds are protected
by disjoint secrets on disjoint hosts** (SPEC §1, §11.3).

## 7. Block lifecycle (the money path)

### 7.1 Pool-template mode

```
SetNewPrevHash(H)            NewTemplate (≈30s cadence)         shares
      │                            │                              │
      ▼                            ▼                              ▼
a_send[H] ← CSPRNG ──▶ Distribution(H, T, a_send)      validate, dedup,
(RAM; old key          (spc-accounting + spc-crypto)    open_slice.push
 zeroised)             P_i for kept ids; U_i + dest     seal → spc:slices
                       for every Ark-capable id         ──────────────▶ payout persists
                       coinbase prefix‖push32(A_send)‖extranonce‖suffix
                                  │
                       share meets target ──▶ block submit & XADD spc:blocks {hash, H, front_id, fingerprint, entries}
                                  ▼
H_f + 6   payout: orphan check → ONE Postgres tx (SPEC §7.4):
          block_found, ledger_history (rail per id), ark_payout
          (status='scheduled', due H_f+6), pending_balance
          → sign & emit ArkSendOrder per ark_due id → spc:ark:orders
                                  │
H_f + 6   ark: verify order → arkoor U_i from float → receipt
          (retry same order_id; after retry_blocks → failed_to_pending,
           credit pending, alarm — never forfeit)
                                  │
H_f + 100 pool output matures → ark boards only float_target − float_balance;
          remainder = operator surplus (SPEC §11.3)
```

### 7.2 JD mode deltas

`front`'s `spc-jds` module publishes `SetPayoutDistribution` (weights, not
sats) on height change, on rebalance timer, and forced on ledger apply; `A_send` rides
`additional_outputs[0]` as `OP_RETURN "SPc"‖ser_x(A_send)` (46 B / 184 WU of
real block space). The JDC computes amounts at **its** template revenue, so
settlement MUST read the mined coinbase, never the snapshot's sats (SPEC §4.6;
claim formula resolved 2026-10-14 as revenue-scaled, `docs/decisions/2026-10-14-o11-claim-formula.md`). Grace acceptance is same-height-only
(R4). Everything else — slices, ledger, Ark scheduling — is identical; the
tier decision changes **how** `owed(id)` is paid, never how it is computed
(SPEC §11).

### 7.3 Tier routing per settled block (SPEC §11.1)

```
owed(id) = raw(id) + pending(id)
  owed ≥ C and id ∈ kept        → coinbase output P_i        rail = coinbase
  owed ≥ C, trimmed             → pending'(id) = owed        rail = pending
  owed < C, ark registered      → ark_due'(id) = owed        rail = ark
  owed < C, unregistered        → forfeited (pool revenue)   rail = forfeit
```

`C` = `coinbase_threshold_sats` (default = P2TR dust, 330). `pending` arises
only from trimming and failed sends — neither is ever forfeited.

## 8. Miner-side flow (`spc-agent`)

1. **Connect**: `sp_address` config → upstream `user_identity`; negotiate
   `0x0001` (+ `0x0003` on both connections for JDC role; `0x5350` when Ark
   tier enabled; `0x0020` by default).
2. **Register before mining**: `SetArkIdentity` with `{B_ark, M, D, mode}`,
   signed by `b_scan`. No `Success` → no share submission (unregistered
   sub-dust earnings are forfeited, SPEC §11.5). `epoch_registered` mode adds
   `RegisterEpochMailboxes` over a pre-registered height window.
3. **Verify before hashing (SPEC §3.6)**: derive own `P` from the job's
   `A_send` and check it appears in the outputs/distribution
   (`require_own_payout = warn | refuse`; small miners are legitimately
   absent some blocks).
4. **Scan both tiers (SPEC §2.5)**: per pool block, from header + branch +
   coinbase (≈500 B): find own SPc output; derive `U_i`, insert into the
   SP-aware keystore (`u_i = b_ark + t_ark` when the signer is present); open
   mailbox epoch.
5. **Receive**: per mailbox mode — `static`: bark's normal sync; `epoch_*`:
   `ReadMailbox M_i` at `H_f + 6 + [0, 6, 36, 144, 1008]`, closing on arrival,
   on-chain payout, or exhausted retries; `none`: `ArkPayoutNotice` →
   `GetVtxosByIds` → `import_vtxos`.
6. **Maintain**: auto-refresh inside the free window (default 144 blocks
   before expiry); `--offboard-on-exit` offered at shutdown if unrefreshed
   VTXOs or scheduled payouts exist (28-day sweep risk, SPEC §11.5).

## 9. Scoping assessment (for implementation planning)

Abstractions that are clean and can be built/tested in isolation:

- **`spc-crypto`** — pure, no I/O, already validated by frozen vectors. The
  root of the dependency graph; everything keys off it. (Roadmap Phase 1.)
- **`spc-accounting`** — pure, proptest-driven (conservation, tier disjointness,
  trim bounds; SPEC §8.2). Depends only on `spc-crypto`. (Phase 2.)
- **`spc-scan`** — stateless consumer of `spc-crypto` + any block source.
  Doubles as the audit client. (Phase 1.)
- **`spc-share-ext` / `spc-ext`** — codec-only crates with documented wire
  deltas; isolated from pool logic. (Phases 6–7.)
- **`ark` role** — the narrowest interface in the system: one stream in
  (signed, capped, identity-free orders), one stream out (receipts), plus the
  Ark server. Independently deployable and testable against stock `captaind`
  regtest. (Phase 7.)

Deliberate coupling points to respect in the plan (don't "fix" them — they're
load-bearing):

- **`a_send` is owned by `front` and shared with the `spc-jds` module
  in-process** (2026-10-13 decision: the JDS is part of `front`, JD always
  on — no IPC channel, never Redis). Note this makes `a_send` per-`front`
  per height, not literally pool-wide, if `front` is ever deployed per
  region — safe for scanning (the key rides the coinbase carrier), and the key
  is RAM-only, so `front` still has no durable state. Multi-region deployment is designed
  in [`docs/MULTI-REGION.md`](MULTI-REGION.md) (2026-10-14).
- **Distribution building needs `a_send` while it's alive** (SPEC §2.2
  erasure rule, §4.7): Ark destinations (`U_i` + blinded mailbox) MUST be
  derived at build time and only the public points retained — never `ecdh_i`
  or the tweaks. This is why the builder runs in `front`, not in `payout`.
- **Found-block event failure model**: a front that cannot XADD the `spc:blocks` event (crash before publish, outbox overflow) results in the block booked `status='pending'` with alarms — a build MUST never fail over a lost event (vouching invariant). Settlement reads the event payload.
- **Settlement reads the mined coinbase** (SPEC §4.6): JD-mode `pending'` is
  derived from `paid(id)` on-chain; the `claim` formula is resolved
  (2026-10-14, revenue-scaled, `docs/decisions/2026-10-14-o11-claim-formula.md`).
- **The bark keystore patch** (O13, ~150–300 lines): the only piece of
  required code that lives in someone else's tree; upstream-patch-vs-fork
  decision must land before the agent's receive path (Phase 7).

Phase mapping (roadmap): 0 skeleton+pins → 1 crypto/scan → 2 accounting →
3 pool-template `spc-pool` → 4 JD (`spc-jds` module in `front` + JDC role) → 5 translator
hardening → 6 share-ext → 7 Ark tier (`ark` role, `0x5350`, agent wallet) →
8 e2e/ops/testnet. Phases 1–3 are the minimal end-to-end SPc demo;
Phase 7 adds the Ark rail.

## 10. Open items that affect component boundaries

Carried from SPEC §10, only those that change *who talks to whom*:

- **O11** — resolved 2026-10-14 (`docs/decisions/2026-10-14-o11-claim-formula.md`).
- **O13** — bark keystore: upstream patch vs fork (affects `spc-agent`'s
  dependency surface).
- **O14** — resolved 2026-10-14 (`docs/decisions/2026-10-14-mailbox-default-epoch.md`).
- **O12** — `t_onchain` tag choice vs plain-SPc scanner compatibility
  (affects only `spc-crypto` internals + vectors).
- **O19** — covenants (future soft fork) would collapse the Ark tier's
  interactivity; no v0.1 work, but the tier routing abstraction (§7.3) is
  where it would plug in.
