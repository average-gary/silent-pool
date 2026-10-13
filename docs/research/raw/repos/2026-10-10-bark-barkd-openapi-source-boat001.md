---
title: "bark 0.7.1 — barkd OpenAPI, wallet/arkoor/mailbox source, server config, BOAT-001 address format (sender-side read)"
url: https://gitlab.com/ark-bitcoin/bark
source: "https://gitlab.com/ark-bitcoin/bark/-/raw/master/bark-rest/openapi.json ; .../lib/src/address.rs ; .../lib/src/arkoor/mod.rs ; .../bark/src/arkoor.rs ; .../bark/src/config.rs ; .../lib/src/mailbox.rs ; .../server/src/config.rs ; .../server/src/arkoor.rs ; .../server/src/rpcserver/mailbox.rs ; .../server-rpc/protos/mailbox_server.proto ; .../CHANGELOG.md ; https://raw.githubusercontent.com/ark-protocol/boats/master/boat-0001.md ; https://crates.io/api/v1/crates?q=bark"
type: repos
category: repo
language: Rust
license: MIT
version: "0.7.1 (2026-09-11); master read 2026-10-10"
ingested: 2026-10-10
research_path: technical
volatility: hot
quality: 5
credibility: high
confidence: high
tags: [ark, bark, barkd, captaind, arkoor, mailbox, boat-001, ark-address, openapi, sender-api, vtxo-expiry, fanout, second-tech, mining-payouts]
summary: "Code-level answers for a pool that wants to pay miners over Ark: barkd exposes one single-destination send endpoint (no arkoor batch), arkoor txs take exactly one input VTXO and inherit its expiry, change is split into a small tree (server fan-out cap 4), the server binary captaind is MIT and self-hostable but addresses are server-bound, the mailbox does no recipient-existence check so never-connected recipients can be paid, and a bark ark1 address with mailbox delivery is 130 chars (tark1 131)."
---

# bark 0.7.1 — sender-side read of barkd, wallet, server, and BOAT-001

Primary-source read of `gitlab.com/ark-bitcoin/bark` at master (≈ 0.7.1+, 2026-10-10), the BOAT-001 address spec, and crates.io. Done to answer: how would a mining pool programmatically pay many small arkoor payouts, can it run the server, what are the offline-receive semantics, and how long is an Ark address.

## Repo shape

README: "Rust implementation of the Ark protocol on bitcoin, by Second: wallet, server and libraries." Components: **bark** (wallet), **captaind** (server), protocol libs; **barkd** = "daemon wallet exposing REST API over HTTP" (`bark-rest` crate); uniffi bindings in a separate `bark-ffi` repo; typed barkd clients in TypeScript and C#. Root dirs: `bark, bark-cli, bark-common, bark-json, bark-rest, bark-rest-client, bark-runtime, bip321, bitcoin-ext, cln-rpc, docs, lib, server, server-log, server-rpc, testing, wasm-testing`. **License MIT.**

crates.io at 0.7.1 (published 2026-09-11): `bark-wallet` ("Wallet library and CLI for the bitcoin Ark protocol built by Second"), `bark-cli`, `bark-rest` (= barkd), `bark-rest-client`, `bark-json`, `bark-server-rpc` (gRPC), `bark-common`, `bark-runtime`, `bark-bitcoin-ext`. Dead placeholders `bark-aspd`, `bark-server`, `bark-client` at 0.0.0 — older server names. **No `captaind` crate**; build from `./server` or download release binaries.

## barkd REST (bark-rest/openapi.json, `info.version: 0.7.1`)

- **The Ark send endpoint**: `POST /api/v1/wallet/send`, body `SendRequest { destination: string (required — "The destination can be an Ark address, a BOLT11-invoice, LNURL or a lightning address"), amount_sat: int64|null ("Optional for bolt11 invoices"; required for Ark addresses), comment: string|null (lightning-address only), retry_for_secs: int64|null (lightning only) }`. Response `SendResponse { message: string, payment_hash: string|null }`. 400 invalid destination / 500. Description: Ark-address payments "settle instantly via an out-of-round (arkoor) transaction".
- **No batch / multi-recipient Ark endpoint.** The only multi-send is `POST /api/v1/onchain/send-many` (on-chain). **No fee parameter** on the send request — arkoor fees are server-side.
- Other routes (62 total) relevant to a payer: `/api/v1/wallet/create` (`network` ∈ mainnet|signet|mutinynet|regtest, `ark_server`, `chain_source`, `mnemonic`, `birthday_height`, `gap_limit` ≤ 100000, `force`), `/wallet/addresses/next`, `/wallet/addresses/index/{index}`, `/wallet/ark-info`, `/wallet/balance`, `/wallet/vtxos[?all=true]`, `/wallet/vtxos/{id}`, `/wallet/import-vtxo`, `/wallet/sync`, `/wallet/sync/mailbox` → `MailboxSyncResponse{checkpoint}`, `/wallet/refresh/all`, `/wallet/refresh/counterparty` ("replaces out-of-round VTXOs under arkoor trust assumptions with trustless, in-round VTXOs"), `/wallet/refresh/delegated/vtxos`, `/wallet/offboard/{all,vtxos}`, `/notifications/wait` (long-poll), `/notifications/ws/ticket`, `/history`, `/message/{sign,verify}`.
- `ArkInfo`: `network, server_pubkey, mailbox_pubkey ("Pubkey for blinding unified mailbox IDs"), round_interval, nb_round_nonces, vtxo_lifetime (blocks; vtxo_expiry_delta deprecated), vtxo_exit_delta, htlc_expiry_delta, htlc_send_expiry_delta, max_vtxo_amount, offboard_feerate_sat_per_kvb, fees: FeeSchedule`. `FeeSchedule` keys: `board, offboard, refresh, lightning_receive, lightning_send` — **no arkoor-send fee field**. `RefreshFees { base_fee_sat, ppm_expiry_table[PpmExpiryFeeEntry] }`.
- `VtxoInfo` includes `expiry_height` ("After expiry, the server can reclaim the sats"), `arkoor_depth` ("Exit chain depth"), `is_arkoor`, `policy`.
- Auth: bearer token required for all REST calls since 0.1.0; can only be disabled via CLI flags since 0.6.2.

## Wallet sender API (Rust)

`bark_wallet::Wallet::send_arkoor_payment(&self, destination: &ark::Address, amount: Amount) -> anyhow::Result<()>` — "Makes an out-of-round payment to the given ark::Address. This does not require waiting for a round." Single destination. "Chains multiple payments if the wallet lacks a single VTXO covering the amount, potentially resulting in multiple VTXOs delivered to the recipient." No multi-destination arkoor API at wallet or REST level; the lib's `ArkoorBuilder` supports multiple outputs per input but is not surfaced.

Config (`bark/src/config.rs`): `change_vtxo_split_factor: u8` — "number of pieces to split arkoor and lightning-send change into, between 1 (no splitting) and 3 (the server's default arkoor fanout limit of 4, minus the payment output). Default value: 2". CHANGELOG 0.7.1: "Split arkoor change into two pieces so repeated payments build a tree of VTXOs. A single change output made every payment chain off the previous one." `vtxo_refresh_expiry_threshold` default 144 blocks (mainnet) / 12 (testnets). `daemon_manual_sync` disables background sync incl. mailbox subscription.

## Arkoor core (`lib/src/arkoor/mod.rs`)

- "Arkoor does not allow multiple inputs" — **one input VTXO per arkoor tx**.
- Outputs ≤ 255 but server-capped (`max_arkoor_fanout`, default 4).
- **"All output VTXOs inherit the input's expiry_height."**
- Must balance exactly (input = Σ outputs, zero on-chain fee, ephemeral anchors). Dust threshold 330 sat with a dust-isolation checkpoint tx. Checkpoint policy `A + S or S after expiry`.
- Builder states: Initial → UserGeneratedNonces → UserSigned → ServerCanCosign → ServerSigned; `ArkoorCosignRequest/Response`. Ownership attestation on cosign mandatory since 0.1.0.

## Server (`server/src/config.rs`, `server/src/arkoor.rs`, protos)

- `vtxo_lifetime` default `6*24*30` = 4320 blocks ≈ 30 d. `max_arkoor_amount` ("Unset means no limit. Zero disables arkoor sends."). `max_arkoor_fanout` default 4. `max_input_exit_depth` — exceeded → client must "refresh the VTXO in a round before making further OOR payments". `allow_expired_arkoor`. `max_read_mailbox_items`. RPC: `public_address`, `admin_address`, `integration_address`.
- ArkInfo proto also exposes `max_vtxo_exit_depth, max_offboard_inputs, max_vtxo_amount, min_board_amount, vtxo_lifetime, mailbox_pubkey, tos_link`.
- Server persists cosigned outputs as "unregistered" until the signed chain is uploaded (sender does this after cosign; recipient redundantly re-registers on receive).
- 0.7.1: "per-action amount limits cap boards, offboards, sends individually"; captaind requires Bitcoin Core v31 and refuses older nodes; `watchmand` split into a separate process (0.7.0, breaking).

## Address (`lib/src/address.rs`) and BOAT-001

```
struct Address { testnet: bool, ark_id: ArkId, policy: VtxoPolicy, delivery: Vec<VtxoDelivery> }
```
- bech32m; HRP `ark` / `tark`; version char `q` (v0, Arkade) or `p` (v1, bark).
- `ark_id` = first 4 bytes of SHA256(server pubkey) → **addresses are server-bound**; receivers reject VTXOs "committing to a foreign server". No cross-server arkoor.
- `VtxoPolicy::Pubkey(PubkeyVtxoPolicy { user_pubkey: PublicKey })` — **33-byte compressed key, not x-only**. Other variants (`ServerHtlcSend/Recv`) are rejected in addresses by both receiver (`check_arkoor_receive_policy`) and server (`PostArkoorMessage`). Any pubkey works → per-payout derived receiver keys are representable; bark itself derives per-index keys (`GET /wallet/addresses/index/{i}`, recovery `gap_limit` default 250 as of 0.7.1).
- `VtxoDelivery::ServerMailbox { blinded_id }` (type byte `0x01`, 1 + 33 B); `0x00` built-in is "reserved, currently unused" (removed in v0.1.0-beta.5 — "Only `VtxoDelivery::ServerMailbox` delivery is now supported"). CHANGELOG 0.7.0: "Refuse to send to an Ark address bark cannot deliver to."
- **Lengths** (bark test vectors): `ark1pwh9vsmezqqpharv69q4z8m6x364d5m5prnmcalcalq9pdmzw0y7mpveck4pcfhezqypczkrrj3lkx5ue4qrf4jc7ztpt9htdttmh2judhqnu7aue8p0y9mqkr4cf5` = **130 chars**; `tark1…` = **131 chars**. BOAT-001's example without delivery data is 77 chars.
- BOAT-001 (`github.com/ark-protocol/boats`, only `boat-0001.md`, undated draft with "TODO link to policy"): v0 = `arkade_version (1) ‖ server_key (32 x-only) ‖ p2tr_key (32)`; v1 = `ark_id (4) ‖ VarInt policy_len ‖ policy ‖ (delivery_type (1) ‖ data)*`; only documents delivery `0x00`. **The BOAT lags bark** (mailbox `0x01`, 33-byte pubkey).

## Mailbox (`lib/src/mailbox.rs`, `bark/src/mailbox.rs`, proto, server handler)

- Mailbox id is **per seed** (`seed.to_mailbox_keypair()`, plus a separate `to_recovery_mailbox_keypair()`); blinded "by adding to the mailbox public key point the Diffie-Hellman secret between the server's key and the VTXO key from the address" — so **minting a valid address requires the server's `mailbox_pubkey`** (fetched once via `GetArkInfo`).
- `MailboxType`: `arkoor-receive, round-participation-completed, ln-recv-pending, recovery-vtxo-id, ln-send-finished`.
- gRPC `MailboxService`: `PostArkoorMessage(blinded_id, repeated vtxos)`, `ReadMailbox(mailbox_id, authorization, checkpoint) → {messages, have_more}`, `SubscribeMailbox` (stream), `PostRecoveryVtxoIds`.
- Server handler: **"No recipient existence check"** — a sender can pay an address whose owner has never contacted the server. Requires VTXOs already cosigned by this server and `Pubkey` policy; `MAX_NB_MAILBOX_ARKOOR_VTXOS` cap; reads need a signed, expiring `MailboxAuthorization`. **No pruning/expiry logic found in the handler** (DB-layer retention not verified).
- Sender delivery (`post_arkoor_to_mailboxes`): skips non-`ServerMailbox` variants, any-success semantics; empty delivery → "no mailbox delivery mechanism configured on destination".
- `docs/mailbox.md` still says "most of this document describes planned features that are not available yet"; shipped per proto/code: arkoor receive, LN receive, recovery ids, round completion. Not shipped: BOLT-12 receive, hArk-refresh delivery.

## hArk / delegated refresh

`lib/src/tree/signed.rs` leaves leaf txs unsigned and uses an unlock hash/preimage so a round can complete with a delegate cosigner; results delivered via `round-participation-completed`. Exposed as `Wallet::refresh_vtxos_scheduled`, `POST /wallet/refresh/delegated/vtxos`, `SubmitRoundParticipation{scheduled_height}`. **The user must pre-submit the intent while online**; a never-connected recipient cannot be refreshed by the server alone (Pubkey policy needs the user's forfeit signature).

## CHANGELOG anchors (selected)

beta.5 mailbox-only delivery, "arkoor-depth removed" client-side (server still enforces exit depth) · beta.6 hArk tree module, dust VTXOs, max fan-out · beta.7 unified mailbox delivery, delegated maintenance/refresh APIs · beta.9 non-interactive round participation via mailbox · 0.1.0 auth token, ownership attestation · 0.4.0 (2026-07-20) resumable boards/offboards, barkd long-poll · 0.5.0 (08-04) seed-derived recovery mailbox, "Forward Lightning receives to Ark addresses", scheduled delegated refresh · 0.6.0 (08-05) exit-depth cap on LN change · 0.6.2 (08-25) change split, protocol floor v4 (bark ≤ 0.4.0 cannot connect), payjoin boards, constant-time auth · 0.7.0 (09-07) refuse undeliverable addresses, watchmand separate process, "Bark no longer drops Ark payments that arrive and expire while the receiver is offline" · 0.7.1 (09-11) gap limit 50→250, change tree, Core v31, per-action amount limits.

## Source criticism

Code and OpenAPI are unambiguous primaries. Gaps: Second's mainnet values for `max_arkoor_amount`, `allow_expired_arkoor`, mailbox retention, and any throughput limit are **not published** (an older local `bark 0.1.0-beta.1` binary cannot handshake: "Client max is: 1, server min is: 4"). Method note from the agent: WebSearch was unavailable; ~45 direct fetches against primaries.

## See also

- [[../articles/2026-10-10-second-docs-pricing-lifetime-server-tos|Second docs: pricing, lifetime, rounds, server self-hosting, ToS]]
- [[../articles/2026-10-10-arkpool-bitaxe-pool-ark-payouts|ArkPool — the live precedent]]
- [[../../../covenantless-ark/wiki/concepts/out-of-round-payments|covenantless-ark: Out-of-round payments]]
- [[../../../covenantless-ark/wiki/concepts/ark-addresses-and-delivery|covenantless-ark: Ark addresses and delivery]]
