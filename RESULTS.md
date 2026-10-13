# Results — SPc + Bark one-identity payout

Date: 2026-10-12
Status: end-to-end validated on regtest against a stock bark Ark server.

This file records what has been proven and what building a real client
requires. See `DESIGN.md` (scheme + privacy), `MAILBOX.md` (rotating mailbox
layer). Code: `spc_ark/` (derivation), `spc_ark_mailbox/` (mailbox rotation),
`spc_ark_client/` (the regtest e2e + client module).

## Proven

Against a completely unmodified `captaind` (built from bark @ c419159),
bitcoind v31, and a throwaway Postgres:

- The stock server co-signs an arkoor payout to the tweaked pubkey
  `U = B_ark + t_ark·G` and delivers it to the miner's mailbox, unblinding the
  pool-built blinded id to exactly `M` (verified in the server's own `mailbox`
  table).
- A stock bark wallet picks the VTXO up and rejects it as `not owned by this
  wallet` — confirming delivery works and the wallet keystore is the only gap.
- The modified client derives `u = b_ark + t_ark`, reads mailbox `M`, claims
  the VTXO, proves ownership (server returns SPENDABLE on a `u`-signed
  VtxoStatusAttestation; a wrong key is rejected), and spends the VTXO back
  through the same stock server to a normal bark address.
- The pool's blinded id equals bark's own `BlindedMailboxIdentifier::new`.
- One BIP39 seed backs the bark wallet and all SP keys; the `m/350'/1'`
  derivation matches bark's own `mailbox_identifier()`.

Server changes required: **none.** Pool-wallet changes required: **none** (a
stock bark wallet pays the pool-built address as-is).

## What a real client needs

All in `spc_ark_client/src/lib.rs`:

1. `MinerKeys::from_bip39_seed` — derive `b_scan`, `b_spend`, `b_ark` at
   `m/352'/…` alongside bark's mailbox key.
2. `SpArkKeystore::scan` — for each pool block `(height, A_send)` and output
   index `k`, compute the shared secret → `t_ark` → store `U → u`. This is the
   hook bark's key lookups need.
3. `claim_vtxos` — the receive filter at `bark/src/mailbox.rs:384`
   (`find_signable_clause`) must also consult that keystore.
4. Signing with `u` — mailbox read, status query, and arkoor spend are bark's
   existing calls, just signed with `u`.
5. `mailbox_hint` — compute `D = b_ark·S_mb` for pool registration.

## Effort estimate for a clean bark patch

~150–300 lines: a pluggable "extra keys" source consulted by `pubkey_keypair`,
`find_vtxo_keypairs`, `get_vtxo_key`, and `find_signable_clause`. Receive,
spend, refresh, and exit then work unchanged. The one real design issue: bark
ties a VTXO key to a seed derivation index (`u32`), so tweaked keys need their
own key type + DB storage, with knock-on effects in recovery (re-scan pool
blocks on seed restore) and movement records.

## Remaining decisions / risks

- Mailbox variant A (`3'/i'`, independent keys, per-epoch registration) vs
  variant B (`M_i = M + hash(ecdh_i)·G`, no registration, related keys). See
  `MAILBOX.md`.
- Cross-layer linkability at the Ark server is mitigated by the rotating
  mailbox + on-demand reads, but reads from one IP/connection re-link epochs —
  needs connection hygiene (new conn / Tor isolation + jitter per read).
- The e2e simulated parsing `A_send` from a coinbase tag rather than mining a
  real tagged coinbase; that input feeds the derivation math, which `spc_ark`
  pins with its test vector.

## Reproduce

`./spc_ark_client/run_e2e.sh` (expects bitcoind v31 at
`$TMP/bitcoin-31.0/bin/bitcoind`; builds stock `captaind` from the local bark
checkout and runs the e2e test).
