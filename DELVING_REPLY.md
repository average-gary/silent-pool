# Draft reply for delvingbitcoin.org/t/silent-payments-coinbase/2833

Post-ready markdown. Written as marathon-gary, in the thread's voice. Code and
longer write-ups live in the companion repo (link at the bottom).

---

Following up on my own thread with some progress on the receive side, and one
concrete finding that changes how I'd think about the sub-dust tier. All of this
is prototyped and validated against a **stock, unmodified** bark Ark server on
regtest.

## One identity, two rails — derived, not bundled

A question that kept coming up (here and in thinking about Ocean-style BOLT12
fallbacks): what does a small miner who can't get a coinbase output actually get
paid with? My answer now: **the same `sp1q...` address, full stop.** You don't
hand the pool a separate Ark address or a BOLT12 offer. The Ark-side destination
is *derived* from the same per-block ECDH that SPc already computes for the
coinbase.

Recall SPc's shared secret, with `A_send` in the pool tag and the height as
nonce:

    ecdh = H(block_height ‖ A_send) · a_send · B_scan     (pool)
         = H(block_height ‖ A_send) · b_scan · A_send     (miner)

Domain-separate two tweaks out of that one secret (BIP340 tagged hashes,
`ser_P`/`ser_32` conventions from BIP 352):

    t_onchain = H_SPc/TierSpc(ser_P(ecdh) ‖ ser_32(k))  →  coinbase output P = B_spend + t_onchain·G
    t_ark     = H_SPc/TierArk(ser_P(ecdh) ‖ ser_32(k))  →  Ark user pubkey  U = B_ark   + t_ark·G

So per block the pool finds, a miner is either **above threshold** and gets an
SPc output in the coinbase (trustless, spendable at +100), or **below
threshold** and accrues off-chain until the pool boards its matured proceeds and
arkoors to `U`. One registered identity drives both. **Never reuse the same `t`
across the two** — `P = B_spend + t·G` and `U = B_ark + t·G` coexisting would
leak `B_ark − B_spend`. Domain separation handles it, but it's the obvious
footgun.

## The part that surprised me: the Ark server needs no changes

I assumed paying a derived Ark key would require server cooperation. It doesn't.
A pool can construct a complete, payable bark `ark1` address from just the
public `U`, because bark's blinded mailbox id is derived from the VTXO pubkey.
The one catch is the mailbox *blinding*: the server unblinds
`blinded − ECDH(s_mb, U)`, which normally needs the secret `u` the pool doesn't
have. Fix is a one-time **blinding hint** the miner registers per Ark server:

    register  D = b_ark · S_mb        (once per server)
    pool computes  blinded = M + D + t_ark·S_mb   →  server unblinds to exactly M

Validated on regtest: stock `captaind` co-signed the arkoor to `U`, delivered it
to mailbox `M`, the modified miner client derived `u = b_ark + t_ark`, claimed
the VTXO, proved ownership (server answered SPENDABLE on a `u`-signed
attestation), and spent it back through the same server. A stock bark wallet
rejects the VTXO (`not owned by this wallet`), so the keystore is the only gap.
**Pool wallet: unchanged. Server: unchanged.**

## Privacy, honestly

- **Pool:** sees everything both tiers regardless (it does the accounting). One
  identifier changes nothing here.
- **On-chain observer:** SPc outputs unlinkable per BIP 352; the arkoor tier is
  invisible on-chain. Derived Ark keys leak nothing — *unless* you publish a
  bundle (`?sp=...&ark=...`), which you should never do.
- **Ark server:** with a static `M` it links all your arkoor payouts. The fix is
  rotating `M_i` per epoch (epoch = the pool's block height) with **on-demand
  one-shot reads** instead of held subscriptions — first read at `i+100`, ≤5
  reads/epoch. Two variants: independent per-epoch keys `m/352'/0'/0'/3'/i'`
  (needs per-epoch registration), or `M_i = M + H(ecdh_i)·G` (no registration,
  but related keys). Residual leak: reads from one IP re-link epochs — needs Tor
  stream isolation + jitter.

## What a client actually needs (the real cost)

~150–300 lines in bark: a pluggable "extra keys" source consulted by
`pubkey_keypair` / `find_vtxo_keypairs` / `get_vtxo_key` / `find_signable_clause`,
so the wallet recognizes and signs for tweaked keys. The one design issue: bark
ties every VTXO key to a seed index (`u32`), so tweaked keys need their own key
type + DB storage, and recovery has to re-scan pool blocks. Receive, spend,
refresh, and exit all work unchanged after that.

Code, test vectors, and the longer design notes:
**https://github.com/average-gary/silent-pool** — derivation (`spc_ark`),
mailbox rotation (`spc_ark_mailbox`), and the regtest e2e (`spc_ark_client`).

Critique very welcome — especially on the blinding-hint handshake and whether
the mailbox-rotation privacy is worth the per-epoch registration.
