# One Miner Identity Across Coinbase and Ark Payout Tiers

Status: design note (draft)
Date: 2026-10-12
Context: companion to the SPc ("Silent Payments coinbase") idea and ArkPool's
live custodial-Ark payout pool. Captures a finding from a design discussion:
a single miner identity can serve both non-custodial coinbase payouts (SPc)
and custodial Ark payouts, without publishing a linkable bundle.

## Background

Two pool payout models exist at opposite ends of the custody spectrum:

1. **SPc (Silent Payments coinbase).** Non-custodial. The pool pays miners
   directly in the coinbase transaction of blocks it finds. Because a coinbase
   has no spendable inputs, the standard BIP352 ECDH cannot run, so the pool
   uses `input_hash = hash(block_height || A_send)` as the nonce and publishes
   an ephemeral `A_send` (controls no funds) in the coinbase scriptSig pool
   tag. Each payout output is a P2TR output
   `P = B_spend + hash(input_hash * a_send * B_scan || k) * G`.
   The miner scans tx position 0 with `b_scan` and spends with
   `b_spend + tweak`. Constraint: coinbase outputs cannot go below the dust
   limit, so small miners are excluded.

2. **ArkPool (live, mainnet alpha).** Custodial. The miner's Stratum username
   *is* their `ark1...` Ark address. The pool credits work off-chain and pays
   hourly via `send_arkoor_payment` (arkoor / out-of-round). Payouts land
   directly as VTXOs in the miner's Bark wallet. Minimum payout 1 sat; no
   coinbase involvement; but the pool holds custody of funds during accrual
   and the static `ark1...` address is reused as a permanent account handle.

Key correction this note records: an Ark address **can** be a pool payout
target, contrary to an earlier assumption. It only works in the custodial /
arkoor-settled model (the pool is itself an Ark participant and the Ark server
co-signs at payout time), not in the non-custodial coinbase model SPc targets.

## The proposal

One miner identity — the SP address (`sp1q...`) — is the only string ever
registered with a pool. The payout tier is decided per block by whether the
miner's accrued balance clears the coinbase dust threshold:

- **Above threshold** -> SPc output directly in the coinbase. Trustless,
  non-custodial, spendable after the 100-block coinbase maturity.
- **Below threshold** -> once the block has 6 confirmations, the pool arkoors
  the miner from its own Ark float. The block's single pool output (a fresh
  P2TR under the pool's hot Ark key) carries those sats, and when it matures
  the pool boards only what the float needs to refill it. Custodial for the
  confirmation window; no on-chain footprint for the miner. A miner who has
  not registered Ark delivery forfeits earnings below the threshold to the
  pool (spec §11.1).

The crux: the Ark-side destination is **derived from the same per-block ECDH**
the pool already runs for SPc — not bundled with the SP address. See below.

## Derivation scheme

Notation follows BIP352: `ser_32(i)` is 4-byte big-endian, `ser_P(P)` is the
33-byte SEC1 compressed point, `hash_tag(m)` is the BIP340 tagged hash
`sha256(sha256(tag) || sha256(tag) || m)`. Keys (BIP32, from one seed):
`b_spend` at `m/352'/0'/0'/0'/0`, `b_scan` at `m/352'/0'/0'/1'/0`, and a
separate `b_ark` at `m/352'/0'/0'/2'/0` (one seed, cross-protocol key
separation; `B_ark != B_spend`).

Per block the pool finds, SPc computes a shared secret:

    input_hash = hash_SPc/Inputs(ser_32(block_height) || ser_P(A_send))
    ecdh       = ser_P(input_hash * a_send * B_scan)   (pool side)
               = ser_P(input_hash * b_scan * A_send)   (miner side)

Domain-separately derive both tiers' destinations from that one 33-byte
secret (the tag is the domain separator; no extra tier bytes are hashed):

    t_onchain = hash_SPc/TierSpc(ecdh || ser_32(k))  -> SPc output:  P = B_spend + t_onchain * G
    t_ark     = hash_SPc/TierArk(ecdh || ser_32(k))  -> Ark policy:  U = B_ark + t_ark * G

Every hash-to-scalar (`input_hash`, `t_onchain`, `t_ark`) **fails** if the
digest is 0 or `>= n` (BIP352 semantics: no mod-n reduction); a point at
infinity also fails. `SPc/SharedSecret` is reserved for a 32-byte digest of
`ecdh` and is not a tier-tweak input. Reference implementation and a frozen
test vector: `spc_ark/src/lib.rs` (`test_vector`).

Properties:

- **One identifier, one DH, two rails.** Only `sp1q...` is registered. The pool
  derives the Ark `user_pubkey` for deferred payouts from the same per-block
  shared secret it computed for the coinbase.
- **Miner can scan both tiers.** `A_send` is public in the pool tag, so the
  miner recomputes `ecdh` for every block the pool found — including blocks
  where they were below threshold — and scans for both the SPc output and the
  arkoor delivery with matching tweaked pubkey.
- **Fresh Ark pubkey per payout.** Strictly better than ArkPool's static
  reused address: each payout key is unlinkable to the others and to the SP
  address, for everyone except the pool — and the Ark server, if mailbox
  delivery is used (see below).
- **The pool can build a complete, payable Ark address** — but only with a
  small one-time registration hint for the mailbox blinding. An earlier draft
  claimed `BlindedMailboxIdentifier::from_pubkey(U)` suffices; it does not
  (the payment lands in a mailbox nobody polls). See "Mailbox delivery /
  wallet discovery".
- **Hot/cold separation survives.** Scanning (on-chain SPc outputs and arkoor
  deliveries) needs only `b_scan`; spending needs `b_spend + tweak` (or
  `b_ark + t_ark`) from a cold signer. Same posture as BIP352 proper.

**Warning — never reuse the same scalar `t` across tiers.** If
`P_onchain = B_spend + t*G` and `P_ark = B_ark + t*G` ever coexisted, their
difference exposes `B_ark - B_spend`. Domain-separate the hashes, as above.
The "reuse the ecdh" trick makes this shortcut tempting; do not take it.

## Mailbox delivery / wallet discovery

Findings from bark at commit `c419159` (paths relative to the bark repo).

**(a) A bark wallet polls one static mailbox, not per-pubkey mailboxes.**
`Wallet::mailbox_identifier()` is `MailboxIdentifier::from_pubkey` of a single
seed-derived key at `m/350'/1'` (`bark/src/mailbox.rs:127-130`,
`bark/src/lib.rs:480-486,718-721`). `sync_mailbox` and
`subscribe_mailbox_messages` read only that id, authorized by a signature from
the mailbox key (`bark/src/mailbox.rs:154-185,309-346`). Each address the wallet
issues blinds that same id per VTXO key:
`blinded = M + ECDH(vtxo_sk, S_mb)` (`Builder::mailbox` in
`lib/src/address.rs:492-507`; `BlindedMailboxIdentifier::new`,
`lib/src/mailbox.rs:197-207`; used by `peek_address`,
`bark/src/lib.rs:1148-1170`). `S_mb` is the server's *mailbox* pubkey
(`ArkInfo::mailbox_pubkey`), distinct from the server pubkey in the `ArkId`.

**(b) An unblinded `from_pubkey(U)` address does not reach the miner.** On
`PostArkoorMessage` the server unblinds with its mailbox key:
`M' = blinded - s_mb * U` (`server/src/rpcserver/mailbox.rs:130-145`,
`server/src/lib.rs:991-1002`, `lib/src/mailbox.rs:124-132`). With
`blinded = U` that is `(1 - s_mb) * U`, a mailbox nobody can authorize, so the
VTXO is a dead letter. Delivery also isn't enough on its own: on receipt the
wallet drops any VTXO whose user pubkey it can't sign for
(`find_signable_clause`, `bark/src/mailbox.rs:384`). It only knows keys on its
own `m/350'/0'/i` VTXO chain (`pubkey_keypair` / `find_vtxo_keypairs`,
`bark/src/lib.rs:1044-1115`). So a stock bark wallet rejects
`U = B_ark + t_ark*G` even if it arrives in the right mailbox.

**(c) Out-of-band path exists, with the same key limitation.**
`Wallet::import_vtxos` (`bark/src/import.rs:78-145`, exposed in
`bark-cli/src/bin/bark/dev.rs:229`) imports VTXOs obtained elsewhere (e.g. from
the pool's API, or fetched by id via `GetVtxosByIds`,
`server-rpc/protos/bark_server.proto:18`). It also requires the key to be
derivable from bark's seed (`find_vtxo_keypairs`, `import.rs:110`, else
`KeyNotFound`). An address with an empty delivery list is also valid
(`check_delivery`, `bark/src/arkoor.rs:154-162`), so the pool can pay without
using a mailbox at all.

**Recommended approach.**
1. *Blinding hint.* `ECDH(u, S_mb) = u*S_mb = b_ark*S_mb + t_ark*S_mb`. At
   registration the miner privately gives the pool, once per Ark server,
   `M` (its mailbox id) and `D = b_ark * S_mb`. Per payout the pool computes
   `blinded = M + D + t_ark * S_mb`, which equals bark's own blinding, so the
   server unblinds it to `M` (tested in `pool_blinding_matches_bark`). `D`
   gives the pool nothing new, since the pool already knows every `U`. This
   breaks "only `sp1q...` is registered" a little: `(M, D)` is extra, private,
   per-server registration data that is never published.
2. *Wallet key source.* The miner's wallet needs an SP-aware keystore: for
   each pool block it scans (`A_send` from the coinbase tag) it derives
   `t_ark`, and registers `u = b_ark + t_ark` (or `U` alone for watch-only),
   so that `find_signable_clause` / `import_vtxos` accept the VTXO. Stock bark
   can't do this; it needs a patch or a fork at the `pubkey_keypair` layer.
3. *Fallback.* Use an empty delivery list and have the miner fetch payouts
   from the pool out of band, then `import_vtxos` them (this still needs step 2).

Privacy cost: with a static `M`, the Ark server links all of a miner's pool
payouts (bark's `docs/mailbox.md` says so for all mailbox use). Using a
per-block mailbox id would avoid this, but the wallet would then have to poll
one mailbox per pool block.

## Privacy analysis, by observer

"Does combining the identifiers hurt privacy?" only has meaning per observer.

- **The pool.** Sees everything in both tiers, all amounts. A single
  identifier makes this no worse: it does the accounting either way.
  Correlation resistance against the pool is impossible — the SPc post
  concedes this. (Mitigation is operational: scrub share logs once paid.)
- **On-chain observer.** Sees SPc outputs only; the arkoor tier is invisible
  on-chain. A derived Ark address leaks nothing on-chain. A *published bundle*
  (e.g. a BIP321 `?sp=...&ark=...` URI) would be a correlation point — never
  post one publicly.
- **The Ark server.** Sees arkoor destination pubkeys, amounts, timing, and
  any boards. It can also parse public coinbases. This is the subtle one:
  bursts of pool-originated arkoors ~6 blocks after each pool block are
  fingerprintable, and amounts map to the pool's known payout distribution.
  If an SPc-tier miner later boards their on-chain payout into Ark, the server
  sees the funding UTXO, linking that SPc output to their Ark identity.
  With mailbox delivery, the server also unblinds every payout to the
  miner's static mailbox id, which links all of that miner's payouts.
- **Anyone the identifier is published to.** Sees whatever the identifier
  contains. Keep it to the bare `sp1q...`.

The single-identifier-with-derivation design is strictly more private than
ArkPool's status quo (static reused address visible to the Ark server) and no
worse than SPc alone on-chain. The costs that cannot be designed away:

1. **Amount fingerprinting on the SPc tier** (known SPc limitation, unchanged).
2. **Tier transitions are observable.** A miner vanishing from coinbase
   outputs while pool arkoor activity continues leaks tier status to anyone
   watching both layers.
3. **Ark server as cross-layer observer** (above). Soft mitigations only:
   payout timing jitter, amount padding/rounding, aggregating small payouts
   across epochs, and boarding SPc outputs via paths that do not advertise
   "fresh coinbase payout."

## Open questions

Resolved: hash tags and serialization (pinned in "Derivation scheme");
`B_ark` is a separate key at `m/352'/0'/0'/2'/0`; the pool builds the final
address, using the `(M, D)` blinding hint (see "Mailbox delivery / wallet
discovery").

- Compatibility with plain SPc: should `t_onchain` use the upstream SPc /
  BIP352 tag (`BIP0352/SharedSecret`-style) instead of `SPc/TierSpc`, so that
  a single-tier SPc scanner also finds the on-chain outputs? This depends on
  the SPc BIP, which isn't final yet.
- Bark integration for scanned keys (a keystore hook so that `pubkey_keypair`
  can return `b_ark + t_ark`); upstream patch vs. fork.
- Per-block vs. static mailbox id (trade-off between server linkability and
  polling cost). (resolved 2026-10-14, see `docs/decisions/2026-10-14-mailbox-default-epoch.md` — default is epoch-derived variant B)
- Wallet UX for tier migration: miner sees a single balance spanning pending
  off-chain accrual, immature coinbase outputs, and VTXOs.
- Light-client story for scanning both tiers without a full node.
- Covenants (CTV / BIP 119 or similar; needs a soft fork). These would
  greatly improve the Ark tier by removing interactivity: the coinbase could
  commit sub-threshold payouts to a tree of `U_i` leaves that each miner can
  exit unilaterally, with no Ark server cosigning and no pool float. All
  pool payouts could likewise be compressed into one coinbase output
  committing to a payout tree, which lifts the coinbase weight and dust limits
  and keeps per-miner amounts off-chain until a leaf is expanded. Spec O19.

## References

- SPc proposal: <https://delvingbitcoin.org/t/silent-payments-coinbase/2833>
- BIP352 Silent Payments: <https://bips.dev/352/>
- ArkPool: <https://arkpool.org/> and <https://arkpool.org/index.md>
- Bark / Second: <https://second.tech/>, <https://gitlab.com/ark-bitcoin/bark>
  (address encoding: `lib/src/address.rs`; policies: `lib/src/vtxo/policy/mod.rs`)
