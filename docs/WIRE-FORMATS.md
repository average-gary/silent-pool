# Wire Formats

Byte-level formats BESPOKE to this project. Never restate upstream SV2 content — reference it. Where this file and `SPEC.md` disagree, the spec wins and this file is the bug.

## 1. Scope and conventions

- **SV2 framing / standard messages** per `sv2-spec @ d8bf5b8` (pins in `UPSTREAM_DEPS.md`).
- **Integer endianness**: little-endian unless otherwise noted in SV2.
- **Type vocabulary** (the SV2 codec's, as used in SPEC §11.4; stated once here):
  - `U8`, `U16`, `U32`, `U64` — little-endian unsigned integers; `I64` — little-endian two's-complement signed; `U256` — 32-byte array.
  - `B0nn` — fixed-length byte array of `nn` bytes, no length prefix (`B004`, `B016`, `B032`, `B033`, `B064`).
  - `B0_nn` — variable-length byte array, 1-byte length prefix, at most `nn` bytes (`B0_32`, `B0_255`).
  - `STR0_255` — `B0_255` holding UTF-8.
  - `SEQ0_64K[T]` — `U16` element count, then the elements.
- **Redis stream entries.** Each `XADD` carries exactly one field, `d`, whose value is the binary payload defined below. Stream names and producers/consumers are in SPEC §5.2.
- **Identifiers.** `sp_id` is the 32-byte `SPc/Id` hash (SPEC §0, §2.1) and `block_hash` is in internal byte order. Public keys are `ser_P` (33-byte compressed) unless a field says `_x` (x-only, 32 bytes).

## 2. Specified elsewhere (reference anchors)

| Format | Anchor | Notes |
|---|---|---|
| `OP_RETURN` carrier payload (JD mode) | SPEC §2.4.2 | `0x6a 0x23 0x53 0x50 0x63 <ser_x(A_send)>` (`OP_RETURN`, push of 35 B = `"SPc"` ‖ 32 B x-only, even-Y) |
| `0x5350` messages (`SetArkIdentity` / `RegisterEpochMailboxes` / `ArkPayoutNotice`) | SPEC §11.4 | Project-defined extension, identifier `0x5350` ("SP"), negotiated via `0x0001` |
| `SetPayoutDistribution`, `distribution_id` TLV `0x0003`/`0x01` | SPEC §3.5 | PR #203 text pinned at `47dbedd`; `distribution_id` is the per-front scheme of §3.5 |
| `MailboxAuthorization` 105-byte layout | `MAILBOX.md` §3 | `ser_P(M_i) ‖ le64(expiry_unix) ‖ sig64` |
| Type-32 share-accounting messages | `share-accounting-ext @ c661e93` + SPEC §3.7 deltas | Type 32 (`0x0020`), negotiated via `0x0001` |
| Postgres row schemas | SPEC §5.1 | Used by the bootstrap RPC (§3.5) |

## 3. New definitions (no upstream byte-level spec)

### 3.1 `spc:blocks` entry (front → payout)

```
U256   block_hash
U32    height
U8     front_id
B032   fingerprint            // SPEC §4.7; audit/dedup handle only
SEQ0_64K[ Entry ] entries     // the winning distribution's full entry list (SPEC §3.3 step 5)
```

`Entry` mirrors SPEC §4.7's `entries` tuple:

```
B032   sp_id
U8     rail                   // 0 coinbase, 1 ark, 2 pending, 3 forfeit (the rail the builder expected)
B0_32  p_x                    // ser_x(P_id); empty if the id was not kept
B0_255 dest_id                // SPEC §4.7 / §11.8 Ark destination (U_i policy key + delivery); empty if not Ark-capable
U64    sats                   // pay(id) for a kept id, owed(id) otherwise
```

`entries` are attribution data: they never leave the SPc side of the Redis ACL (SPEC §11.3) and are scrubbed per SPEC §5.1.

### 3.2 `spc:front:events` payloads (payout → all fronts)

Every payload starts with `U8 type` and `U64 rev` — `rev` is assigned by `payout` in commit order; fronts apply only newer revs (MULTI-REGION §6). Field sets follow SPEC §5.2.

**0x01 — `ArkRegApplied`**
```
U8   0x01
U64  rev
B032 sp_id
```

**0x02 — `LedgerApplied`**
```
U8   0x02
U64  rev
U256 block_hash
U8   republish                // 0|1: force a SetPayoutDistribution republish and drop the grace slot (SPEC §3.5)
SEQ0_64K[ { B032 sp_id, I64 pending_delta_sats } ]   pending_deltas
SEQ0_64K[ { B032 sp_id, U64 sats } ]                 provisional_paids
```

**0x03 — `BlockFound`**
```
U8   0x03
U64  rev
U256 block_hash
U32  height
U8   front_id
```

**0x04 — `TipHeartbeat`**
```
U8   0x04
U64  rev
U256 tip_hash
U32  height
U64  unix_ms
```

> **Open:** MULTI-REGION §6 describes `provisional_paids` as "announced on `BlockFound`", while SPEC §5.2 lists them in `LedgerApplied`. This file follows SPEC. Resolve before multi-front work (it changes whether the paid amounts are visible between found-time and booking).

### 3.3 `spc:arkreg` entry (front → payout)

Carries the validated `SetArkIdentity` of SPEC §11.4 plus the keys `payout` needs to upsert `sp_identity` first.

```
U8     kind                   // 0 SetArkIdentity, 1 RegisterEpochMailboxes
B032   sp_id                  // payout recomputes it from (b_scan, b_spend) and rejects a mismatch
B033   b_scan
B033   b_spend
B004   ark_id
B033   b_ark
U8     mailbox_mode           // 0 none, 1 static, 2 epoch_derived, 3 epoch_registered
B0_33  mailbox_pubkey         // M; empty iff none
B0_33  mailbox_dh             // D; empty iff none
U8     epoch_fallback         // 0 static, 1 hold
B064   sig                    // BIP340 by b_scan, SPEC §11.4
```

The layout above is `kind = 0` (`SetArkIdentity`). For `kind = 1` (`RegisterEpochMailboxes`) the entry is `U8 kind, B032 sp_id, SEQ0_64K[B037] entries, B064 sig` per SPEC §11.4.

### 3.4 `spc:ark:orders` / `spc:ark:receipts` (payout ↔ ark)

```
ArkSendOrder:
B016    order_id              // random, opaque
STR0_255 ark_address          // bark address string, delivery per SPEC §11.8
U64     sats
U32     not_before_height     // H_f + confirmation_depth
B064    sig                   // BIP340 over hash_SPc/ArkSendOrder(the preceding fields exactly as serialised above)

ArkSendReceipt:
B016    order_id
U8      status                // 0 sent, 1 failed
B0_64   vtxo_id               // empty if failed
U32     expiry_height         // 0 if failed
```

The signing key is the order-signing key of SPEC §11.3 (secp256k1, BIP340; `order_pubkey` is the 32-byte x-only key). No `sp_id`, `B_scan`, `B_spend`, `P_i` or block hash appears in either message (SPEC §11.3, schema-tested in §11.9).

### 3.5 Bootstrap RPC (front → payout)

Transport: length-prefixed (`U32 LE`) JSON over TCP to the `payout` role's internal listener, TLS with per-front credentials (the same trust level as the front's Redis ACL, MULTI-REGION §6). It returns attribution data, so it MUST NOT be exposed beyond the fronts. A front is not ready (no channel opens) until the RPC completes.

**Request**
```json
{
  "front_id": 1,
  "instance": "uuid-of-this-front-process",
  "cursors": { "spc:slices": "<last-seen stream id or null>", "spc:front:events": "<…>" }
}
```

**Response**
```json
{
  "rev": 12345,
  "pending_balance":   [ { "…": "rows per SPEC §5.1 pending_balance" } ],
  "ark_delivery":      [ { "…": "rows per SPEC §5.1 ark_delivery, current as of rev" } ],
  "window_tail":       [ { "…": "sealed slice aggregates for the live tail (SPEC §5.3 window_cache)" } ],
  "type32_blobs":      [ "<share blobs for the type-32 window service, base64>" ],
  "resume_cursors":    { "spc:slices": "<stream id>", "spc:front:events": "<stream id>" }
}
```

The response is a consistent snapshot as of `rev`; the front then subscribes from `resume_cursors`, so there is no gap between the read and the live tail. A front whose cursor has fallen behind the trim horizon (SPEC §5.2) repeats this call.

## 4. Where this file is linked from

- `docs/SPEC.md` §5.2 (stream table and delivery semantics).
- `docs/MULTI-REGION.md` §6 (bootstrap RPC and `spc:arkreg`).
