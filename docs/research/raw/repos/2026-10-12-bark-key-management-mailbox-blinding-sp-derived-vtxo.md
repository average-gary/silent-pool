---
title: "bark key management, mailbox blinding formula, receive/import ownership gates, VtxoSigner — can a VTXO key be SP-derived? (code read @ c419159f)"
url: https://gitlab.com/ark-bitcoin/bark
source: "https://gitlab.com/ark-bitcoin/bark/-/raw/master/lib/src/vtxo/policy/mod.rs ; .../lib/src/address.rs ; .../lib/src/mailbox.rs ; .../server/src/rpcserver/mailbox.rs ; .../bark/src/lib.rs ; .../bark/src/mailbox.rs ; .../bark/src/import.rs ; .../bark/src/config.rs ; .../bark/src/vtxo/signing.rs ; .../lib/src/vtxo/policy/signing.rs ; .../bark-rest/openapi.json ; https://raw.githubusercontent.com/instagibbs/boats/master/02-vtxo.md ; .../05-arkoor.md ; https://community.second.tech/t/230 (Dog Pack) ; https://community.second.tech/t/154"
type: repos
category: repo
language: Rust
license: MIT
version: "bark master c419159f (2026-10-12); instagibbs/boats 6690b897 (2026-09-01)"
ingested: 2026-10-12
research_path: technical (question mode, sub-question 4 of a round whose other four paths were stopped at user request)
volatility: hot
quality: 5
credibility: high
confidence: high
tags: [ark, bark, vtxo-policy, mailbox, blinding, ecdh, key-derivation, bip32, gap-limit, import-vtxo, vtxo-signer, silent-payments, sp-derived-key, boats, code-read]
summary: "bark's Pubkey VTXO policy is a bare 33-byte key with no derivation constraint, and a third party can mint a valid mailbox address for any key because the blinding is linear (Blinded = M + v·S_m, DH point unhashed). But bark-wallet resolves keys only via db index → m/350'/0'/idx (gap limit 250), so an arkoor VTXO to a foreign key is dropped on receive ('not owned by this wallet') and rejected on import ('not derivable from this seed'); signing itself is trait-abstracted over a plain Keypair. An SP-derived VTXO key is therefore feasible only with a wallet-side external-key table — and it would need b_spend hot and still require the miner's mailbox pubkey plus a static ECDH point D = b_spend·S_m."
---

# bark key management and the SP-derived VTXO key question

Primary-source read of `gitlab.com/ark-bitcoin/bark` at `c419159f` (2026-10-12) plus `instagibbs/boats` (`6690b897`), done to answer: can a pool derive a miner's Ark receiver key from the miner's BIP 352 address (`P' = B_spend + H(ecdh‖1)·G`), pay an arkoor to it, and can the miner's bark wallet find and spend it?

## 1. Policy and address — the protocol side accepts any key

- `lib/src/vtxo/policy/mod.rs:252` — `pub struct PubkeyVtxoPolicy { pub user_pubkey: PublicKey }`; `taproot()` (`:272`) builds `combined_pk = musig::combine_keys([user_pubkey, server_pubkey])` with leaf `DelayedSignClause{ pubkey: user_pubkey, block_delta: exit_delta }`. `VtxoPolicy::new_pubkey(PublicKey)` (`:978`). **No derivation constraint anywhere in `lib`.**
- `lib/src/address.rs:156` — `Address { testnet, ark_id, policy: VtxoPolicy, delivery: Vec<VtxoDelivery> }`. Two ways to attach mailbox delivery:
  - `Builder::mailbox(server_mailbox_pubkey, mailbox: MailboxIdentifier, vtxo_key: &Keypair)` (`:491`) enforces `vtxo_key.public_key() == pol.user_pubkey()` — needs the **private** VTXO key.
  - `Builder::delivery(VtxoDelivery::ServerMailbox { blinded_id })` (`:484`) takes a precomputed `BlindedMailboxIdentifier` with **no key check** (doc example `:143` uses `BlindedMailboxIdentifier::from_pubkey(vtxo_pubkey)` directly).
  - `into_address()` requires only server pubkey, policy, ≥ 1 delivery ("missing delivery mechanism").
- `instagibbs/boats` ARK #2 `02-vtxo.md`: "`pubkey` is the general-purpose policy… Fields: `user_pubkey` (33 bytes). internal key: `musig(A, S)`; leaf: delayed-sign `(exit_delta, A)`." ARK #5 `05-arkoor.md`: delivery is "out of scope"; recipient "MUST fully validate received VTXOs… SHOULD refresh the VTXO in a round"; "there is no sender/recipient unlinkability inside one transfer."

## 2. Mailbox blinding — exact formula, and why a third party can mint it

`lib/src/mailbox.rs:197`:
```rust
pub fn new(mailbox_id: MailboxIdentifier, server_pubkey: PublicKey, vtxo_key: &Keypair) -> Result<BlindedMailboxIdentifier, _> {
    let dh = ecdh::shared_secret_point(&server_pubkey, &vtxo_key.secret_key());
    let dh_pk = point_to_pubkey(&dh);
    let ret = PublicKey::combine_keys(&[&mailbox_id.as_pubkey(), &dh_pk])...
```
i.e. **`Blinded = M + v·S_m`** — `M` recipient mailbox point, `v` VTXO secret, `S_m` server *mailbox* pubkey. The DH output is the **raw curve point** (`shared_secret_point`, not hashed), so blinding is **linear in `v`**. Unblind (`:124`): `combine_keys([blinded, −(s_m·V)])`.

Server `post_arkoor_message` (`server/src/rpcserver/mailbox.rs:97–175`): rejects non-`Pubkey` policies; requires all VTXOs share one `user_pubkey`; unblinds with the VTXO pubkey; checks stored VTXO ids belong to that pubkey; stores under `MailboxType::ArkoorReceive`; **unauthenticated, no check that the unblinded mailbox exists.**

Recipient: mailbox id fixed per seed — `MailboxIdentifier::from_pubkey(seed.to_mailbox_keypair().public_key())` (`bark/src/mailbox.rs:127`); `subscribe_mailbox_messages` is **per mailbox, not per VTXO key** (`:170`) — anything routed to `M` arrives.

**Consequence (agent's derivation, algebraically sound, not executed):** a pool lacking `v = b_spend + t` can still mint the address because `v·S_m = b_spend·S_m + t·S_m`. It knows `t` and `S_m`; it needs the static point **`D = b_spend·S_m`** once from the miner. Then `Blinded = M + D + t·S_m`, which the server unblinds correctly. No lib/server change. Caveats: `D` is per Ark server; computing it puts `b_spend` through an ECDH with a third-party-known point — outside BIP 352's threat model; byte-level equivalence of `point_to_pubkey` vs `combine_keys` arithmetic not tested.

## 3. Wallet key management — the single blocker

- Derivation (`bark/src/lib.rs:480–486, 692–726`): `BARK_PURPOSE_INDEX = 350`, `VTXO_KEYS_INDEX = 0`, `MAILBOX_KEY_INDEX = 1`, `RECOVERY_MAILBOX_KEY_INDEX = 2`; vtxo xpriv `m/350'/0'`; `derive_vtxo_keypair(idx)` = non-hardened child; mailbox key `m/350'/1'`.
- Lookup is **DB index → seed only**: `pubkey_keypair(pk)` (`:1109`) = `db.get_public_key_idx(pk)` → `derive_vtxo_keypair(index)`; `get_vtxo_key(vtxo)` (`:1123`) = `find_signable_clause` → `db.get_public_key_idx(&pubkey).context("VTXO key not found")`. `find_vtxo_keypairs(wanted, gap_limit)` (`:1044`) scans forward. `DEFAULT_VTXO_KEY_GAP_LIMIT = 250`, `MAX = 100_000` (`config.rs:60,67`). **No API to import an external private key or tweak**; `WalletSeed` exposes only the three derivation fns.
- Import (`bark/src/import.rs:1–9`): "The wallet owns a VTXO when it can derive the VTXO's user pubkey from its seed." → `ImportVtxoError::KeyNotFound { id, gap_limit }`; REST `POST /api/v1/wallet/import-vtxo` (`ImportVtxoRequest { vtxos, gap_limit ≤ 100000, skip_status_check, allow_partial }`) → 400 "a VTXO whose user pubkey is not derivable from this seed within the gap limit". **Import cannot take `P'`.**
- Receive gate (`bark/src/mailbox.rs:101–115, 351–396`): `check_arkoor_receive_policy` checks only `policy_type() == Pubkey` and server pubkey ("Ownership… is checked by the caller"); caller `process_raw_vtxos`: `if self.find_signable_clause(&vtxo).await.is_none() { error!("Refusing received arkoor VTXO {}: not owned by this wallet", …); invalid_vtxos.push(bytes); continue; }` — and the mailbox checkpoint advances past it.
- Signing is **not** a blocker: `lib/src/vtxo/policy/signing.rs:19` `trait VtxoSigner { async fn sign_keyspend(&self, vtxo, sighash) -> Option<schnorr::Signature>; async fn witness(&self, clause, control_block, sighash) -> Option<Witness>; }`; bark's impl (`bark/src/vtxo/signing.rs:14,42`) resolves `clause_keypair(clause) = pubkey_keypair(&clause.pubkey())` then `SECP.sign_schnorr_with_aux_rand(&sighash, &keypair, …)` over a plain `bitcoin::key::Keypair`. The coupling to derivation indices is entirely in lookup.

## 4. Prior art (thin)

- Delving t/2833 (SPc) — no Ark/off-chain rail mentioned; the extension is novel.
- BIP 352 labels `B_m = B_spend + hash_BIP0352/Label(ser256(b_scan) ‖ ser32(m))·G`, `m = 0` change; `k` increments per output to the same receiver (footnote: otherwise an observer could subtract outputs). No text on `k` for off-chain outputs.
- Dog Pack (community.second.tech t/230, 2026-09-29): additive-tweak `output_pubkey = forfeit_point[j] + offset_pubkey` with ring proofs — for in-round unlinkability, not stealth addresses.
- t/154 (2025-10-18), Erik (Second): wallets are seed-based via `Wallet::create` with a mnemonic — no foreign-key mechanism.
- Delving 1816 / 2203: stealth/SP notifications via Nostr — the out-of-band notification problem, not Ark-specific.
- Not found: Arkade address source; any SP-for-Lightning/statechain proposal.

## 5. Verdict carried into the wiki

**Feasible-with-wallet-changes; not worth it for this design.** Protocol side needs nothing; bark-wallet would need an external-VTXO-key table consulted by `pubkey_keypair` / `get_vtxo_key` / `find_signable_clause` plus a hook that derives `b_spend + t` on new block / unknown-pubkey mailbox VTXO. But (i) VTXO keys are **hot** by construction (refresh, forfeit, arkoor cosign) so an SP-derived key forces `b_spend` hot; (ii) the server unblinds every delivery to the per-seed mailbox `M`, so per-payout key freshness hides nothing from the only observer of off-chain VTXOs; (iii) the miner must still supply `M` and `D` — it is not a single identity. Minimal identity for the derived variant: SP address + mailbox pubkey `M` (`m/350'/1'`) + `D = b_spend·S_m` + server `ark_id`.

## Source criticism

All code is primary and pinned. The blinding-linearity consequence (§2) is the agent's algebra, consistent with the quoted code but **not executed**; one integration test would settle it. The "hot key" and "mailbox links everything" arguments are structural reads of the protocol, not vendor statements.

## See also

- [[2026-10-10-bark-barkd-openapi-source-boat001|bark 0.7.1 sender-side read]] — the companion read of send/arkoor/address
- [[../../wiki/concepts/ark-sub-threshold-payout-rail|Ark (bark) as a Sub-Threshold Payout Rail]]
- [[../../output/plan-silent-pool-spec-2026-10-09|Spec: silent-pool]] §11 and Appendix 11.8
