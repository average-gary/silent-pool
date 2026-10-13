# Rotating Mailbox Ids with On-Demand Reads

Status: design note + prototype (draft)
Date: 2026-10-12
Extends: `DESIGN.md`, "Mailbox delivery / wallet discovery". That section's
static-`M` flow stays valid. This note replaces the static `M` with a
per-epoch `M_i` to fix the privacy cost it names: the Ark server links all of
a miner's payouts to one mailbox.
Prototype: `spc_ark_mailbox/` (depends on `spc_ark`). The frozen vector is
`spc_ark_mailbox/src/lib.rs::tests::test_vector`.

Bark references are to commit `c419159` / `ark-lib` 0.7.1.

## Goals

1. The Ark server can't link a miner's payouts from different pool blocks by
   mailbox id.
2. The wallet doesn't hold a long-lived `SubscribeMailbox` stream per mailbox
   (cost would grow with the number of mailboxes, and captaind relies on the
   reverse proxy to cap open streams, `server/src/rpcserver/mailbox.rs:214-216`).
   The pool's block-find drives the schedule, and the wallet makes one
   authorized `ReadMailbox` call per epoch.

## Epoch

**Epoch `i` = the height of a block the pool found.** The Ark payout
`U_i = B_ark + t_ark_i*G` already comes from that block's SPc ECDH
(`input_hash` commits to `block_height`), so the mailbox follows the same key.

Implications:

- One mailbox per pool block the miner is owed from. Every output index `k`
  for that miner in block `i` goes to the same `M_i`. The server can link
  those to each other, but not to any other epoch.
- Heights are below `2^31`, so `i` is a valid hardened BIP32 index. Larger
  values fail with `EpochOutOfRange`.
- Both sides already know the epochs: the miner finds pool blocks by scanning
  coinbases for the pool tag / `A_send`, as SPc scanning does anyway.

## 1. Key derivation

| key | path | note |
|---|---|---|
| `b_spend` | `m/352'/0'/0'/0'/0` | BIP352 |
| `b_scan`  | `m/352'/0'/0'/1'/0` | BIP352 |
| `b_ark`   | `m/352'/0'/0'/2'/0` | DESIGN.md |
| **`m_i`** | **`m/352'/0'/0'/3'/i'`** | **this note, `i` = pool block height** |
| `m` (static) | `m/350'/1'` | bark `WalletSeed::to_mailbox_keypair`, used for fallback and variant B |

`M_i = MailboxIdentifier::from_pubkey(m_i * G)`.

The final index is hardened, so `M_i` for different epochs are independent
keys. Leaking one `m_i` says nothing about another, and nobody holding an
xpub can enumerate them. Every past `m_i` can be re-derived from the seed, so
re-scan needs no extra state. For hot/cold separation, the online reader can
hold only the account xpriv at `m/352'/0'/0'/3'`. That key can read mailboxes
but can't spend.

## 2. Registration and blinding

bark's blinding is `blinded = M + ECDH(u, S_mb) = M + u*S_mb`
(`BlindedMailboxIdentifier::new`, `lib/src/mailbox.rs:197-207`). With
`u_i = b_ark + t_ark_i`:

    u_i * S_mb = b_ark*S_mb + t_ark_i*S_mb = D + t_ark_i*S_mb

`D` doesn't depend on the epoch. Only `t_ark_i` rotates, and the pool already
knows it. So the per-epoch data is just `M_i`.

**Once per Ark server** (unchanged from DESIGN.md):

    S_mb   = ArkInfo::mailbox_pubkey
    D      = b_ark * S_mb                          (33 bytes)

**Per epoch** (`EpochRegistration`, 37 bytes):

    ser_32(i) || ser_P(M_i)

Send it over the same private, authenticated pool-account channel as `D`.
The pool must authenticate the account owner. Otherwise anyone could
register a bogus `M_i` and send that miner's payout to a dead letter.

**Pool, per payout** (`pool_blinded_epoch_id`). This needs one EC
multiplication and registered public data only:

    blinded_i = M_i + D + t_ark_i * S_mb

The address is a normal `ark1` address: policy `U_i`, delivery
`ServerMailbox { blinded_i }`. The address format and the server don't change.

**Server**: `from_blinded(blinded_i, U_i, s_mb) = blinded_i - s_mb*U_i`
`= M_i + u_i*S_mb - s_mb*u_i*G = M_i`.

Test evidence (`pool_blinding_unblinds_to_epoch_mailbox`): for epochs
800000, 800001 and 802016, the pool's `blinded_i` equals bark's
`BlindedMailboxIdentifier::new(M_i, S_mb, u_i)`.
`ark::mailbox::MailboxIdentifier::from_blinded` with a stand-in server
mailbox key recovers `M_i`, never the static `M`.
`on_demand_read_end_to_end` posts through a mock captaind that unblinds the
same way, and checks that payouts for two epochs land in two separate
mailboxes.

### Registration timing

The final index is hardened, so the pool can't derive `M_i` itself. The miner
has two ways to supply it:

- **After the find (default).** The wallet sees pool block `i` while scanning,
  then registers `M_i`. The pool can't board block `i`'s proceeds before
  coinbase maturity, so the miner has about 100 blocks (about 16 h) to do
  this.
- **Pre-registration window.** Register `M_h` for every height in an upcoming
  range. This costs about 144 × 37 B ≈ 5.3 KB per day of coverage. Heights
  the pool never finds are simply unused.

If the pool has no `M_i` when it pays epoch `i`, it falls back to the static
`M` (that payout is then linkable, as in DESIGN.md) or holds the payout,
depending on the miner's preference. Variant B removes this problem.

### Variant B: no per-epoch registration

    t_mb_i = hash_SPc/Mailbox(ser_P(ecdh_i))   (fail if 0 or >= n)
    M_i    = M + t_mb_i * G                     (pool, from the static M)
    m_i    = m + t_mb_i                         (wallet)

The pool already computes `ecdh_i`, and `(M, D)` is already registered, so
nothing new is sent. The server can't link `M_i` to `M` or to `M_j` without
`ecdh_i`, which takes `b_scan` or `a_send`. The tag `SPc/Mailbox` keeps
`t_mb_i` separate from `t_onchain` and `t_ark`. BIP340 commits to the signing
pubkey, so signing with tweaked keys is safe. Tested in
`variant_b_derived_mailbox`.

Trade-off: the `m_i` keys are related. Anyone holding `m` and `b_scan` (both
hot keys) can derive every `m_i`, and a leaked `m_i` together with `ecdh_i`
reveals `m`. Mailbox keys only grant read access and VTXOs still need `u_i`,
so the exposure is privacy only. **Recommendation:** prefer variant B in
practice because it needs no registration window or fallback. Keep the
hardened `3'/i'` scheme where independent per-epoch keys matter.

Rejected: `M_i = U_i`. Authorizing reads would then need a signature from
`u_i`, which is a spending key that should stay cold.

## 3. On-demand read

**Trigger.** The wallet sees pool block `i` while scanning the chain. The
payout can arrive no earlier than `i + 100`. The wallet reads `M_i` at
`i + 100 + RETRY_OFFSETS[r]` blocks, with `RETRY_OFFSETS = [0, 6, 36, 144,
1008]`, until the epoch closes. If the pool provides a "paid epoch `i`"
notification (API or webhook), the wallet reads once on that notification
instead of following the schedule.

**Flow** (`read_epoch_mailbox`):

1. Re-derive `m_i` from the seed (or use variant B's `m + t_mb_i`).
2. Build an authorization valid for 60 s
   (`MailboxAuthorization::new(&m_i, now + 60s)`).
3. Send `MailboxRequest { mailbox_id: ser_P(M_i), authorization, checkpoint }`
   to `ReadMailbox`. Keep requesting pages with the same authorization while
   `have_more` is set, up to 10 pages (bark's `MAX_MAILBOX_REQUEST_BURST`).
4. Collect the arkoor VTXOs and store the highest checkpoint seen for epoch
   `i`. Bark's normal VTXO validation and the SP-aware keystore
   (`u_i = b_ark + t_ark_i`, DESIGN.md step 2) handle the rest.

**Authorization format.** This matches bark exactly
(`lib/src/mailbox.rs:250-299`):

    msg  = sha256(b"Ark VTXO mailbox authorization: " || le64(expiry_unix))   // plain SHA256, 32-byte prefix
    sig  = BIP340 Schnorr(m_i, msg)                                           // bark uses random aux
    wire = ser_P(M_i) || le64(expiry_unix) || sig64                           // 105 bytes, ProtocolEncoding

The message doesn't include the id. The BIP340 challenge commits to the
x-only `M_i` instead. The server checks `auth.mailbox() == mailbox_id`,
`!is_expired()` (5 s leeway), and `verify()`
(`server/src/rpcserver/mailbox.rs:182-212`). Anyone holding the token can
read until it expires, so keep expiries short.
`epoch_authorization_verifies` checks that the token verifies, survives the
wire encoding, matches our re-implementation of the message, fails when the
id is swapped for `M_{i+1}`, and counts as expired once past its expiry.

**Where the RPC plugs in.** The `MailboxReader` trait. A production
implementation wraps bark's tonic client
`srv.mailbox_client.read_mailbox(protos::mailbox_server::MailboxRequest { mailbox_id, authorization, checkpoint })`
(the same call as `Wallet::sync_mailbox`, `bark/src/mailbox.rs:309-342`) and
maps `MailboxMessages` back. It never calls `SubscribeMailbox`.

## 4. Catch-up and recovery

**State per epoch:** `open` or `closed`, the number of reads done, and the
last checkpoint. An epoch closes when the expected VTXO for `U_i` arrives,
when an SPc output for that block is found on chain (that block paid
on-chain, so there's nothing to read), or when the last retry is used up.

**Startup** (`catch_up_epochs`): scan coinbases in `(last_synced, tip]` for
pool blocks. This scan is already needed for SPc. Read every epoch that is
mature (`e + 100 <= tip`) and not closed. Epochs still open from earlier
runs continue on their retry schedule (`read_due`).

**Recovery from seed:** use the same rule from the wallet's birthday height.
Everything older than the horizon has already been posted, so one read per
epoch is enough (plus pages).

**Bounds.** Let `B` be the number of pool blocks in the range, and let `f` be
the pool's hashrate share, so the pool finds about `144·f` blocks per day.

- Steady state: at most `|RETRY_OFFSETS| = 5` reads per pool block, minus
  blocks that paid on-chain. Typically 1 read when the pool pays promptly or
  sends a notification.
- Recovery: at most `B` reads, each a single indexed DB query
  (`get_mailbox_messages(id, cp, limit)`).
- Example: a 1% pool means about 1.4 reads/day, or about 525 per year of
  recovery. A 10% pool means about 14 reads/day, or about 5,300 per year.
- For comparison, subscribing to every open mailbox would mean
  `f·1108` concurrent streams (about 111 for a 10% pool, one per epoch still
  inside the retry horizon). The static `M` uses 1 stream.

## Privacy and cost

| observer | static `M` (DESIGN.md) | rotating `M_i` |
|---|---|---|
| Ark server, by mailbox id | links all payouts | links only outputs within one pool block |
| Ark server, by read auth | one key signs every read | a fresh unrelated key per epoch |
| Ark server, by network | n/a (one stream) | **residual:** reads of `M_i`, `M_j` from the same IP or connection can be linked |
| pool | sees everything | unchanged |
| on-chain | nothing | nothing |

Residual leaks and their mitigations:

- **Network.** Use a new gRPC connection for each epoch read, with Tor
  stream isolation if possible. Add random jitter after each trigger. During
  catch-up, don't read many `M_i` back-to-back on one connection.
- **Timing.** Reads cluster around `i + 100 + offsets`. Every miner owed from
  block `i` reads in the same window, so the anonymity set is the set of
  miners paid in that epoch. Jitter makes the set larger.
- The amount and pool-burst fingerprinting described in DESIGN.md is
  unchanged.

Cost: one cheap authorized DB read per epoch instead of a stream that stays
open. Load on the server scales with pool blocks, not with how long wallets
stay online, and the reverse proxy's stream budget is untouched. Variant A
adds 37 bytes of registration per epoch. Variant B adds nothing.

## Relation to the static-`M` flow

Only the mailbox point changes, from `M` to `M_i`. The blinding formula, the
`D` hint, the `ark1` address format, the server's unblinding, and the
authorization format are all the same. The static flow keeps working as the
fallback when the pool has no `M_i`. A regtest check of the static flow
(blinded post → `from_blinded` → delivery) carries over directly. The extra
regtest step for this note: pay `blinded_i`, then call `ReadMailbox` for
`M_i` with an `m_i` authorization and check that the static `M` is empty.
Stock bark won't poll `M_i`. This needs the SP-aware wallet that DESIGN.md
step 2 already requires.

## Frozen test vector

Fixture: same as `spc_ark` `test_vector`: seed = `0x42`×32,
`a_send` = `0x07`×32, epoch `i` = block height = 800000, `k` = 0, server
pubkey = (`0x09`×32)·G, server mailbox pubkey `S_mb` = (`0x0a`×32)·G. The
authorization uses expiry = 1800000000 and BIP340 with no aux randomness.

    m_i                  = fa83f7461a305cccaadafa3d3c2efe115543d8b39a2b83d09ac2ae5eaaa308e4   (m/352'/0'/0'/3'/800000')
    M_i                  = 03f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc3
    registration         = 000c350003f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc3
    t_ark                = 5f6d66827770b92b1d7250175da5f90862be9d19f73ed09a485251286158dd0c   (= spc_ark vector)
    U_ark                = 028774fd21598aa994083488788d42c87d344b116e2552d2236c3644bd47f53937 (= spc_ark vector)
    mailbox_hint D       = 02d864313e3db31b98210d966a3a02589696d63491921b7a119bd6943be353820a (= spc_ark vector)
    blinded_mailbox_id_i = 0360539d99a4da9b50c72bbc9bcc9b40d19bf26bf15969ec6039de8022e40063dc
    ark_address_i        = ark1pw4zjupfzqqpgwa8ay9vc42v5pq6gs7ydgty86dztz9hz25kjydkrv39agl6njdezqypkq5uanxjd4x6scu4mex7vndqdrxljd0c4j60vvquaaqpzusqx8hq6au2cp
    auth_message         = 9fb17cbdff11be6d831da0e9f72119f7c7291e50503e100be9c274367399c77b
    auth_bytes           = 03f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc3
                           00d2496b00000000
                           d25fe29d03bbefe95ae2cebfda734d1fc50aa0808f7d24d044b57bd783f4399dc973034683ca333d42d8011719c4b43eef2587b40a3303f775f25bbc6a2bc310
    variant_b_M_i        = 031ff5ec81ad81bcbf75e828fec51c334d97191d1f001bfebc85cea1989b1b23a2

`auth_message` was checked independently with Python's
`sha256(prefix + struct.pack('<q', 1800000000))`. `auth_bytes` decodes with
`ark::mailbox::MailboxAuthorization::deserialize`, and `verify()` returns
true on the result.
