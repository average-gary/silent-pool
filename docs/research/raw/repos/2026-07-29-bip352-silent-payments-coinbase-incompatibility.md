---
title: "BIP 352 Silent Payments — structural incompatibility with coinbase outputs"
source: https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki
supporting_sources:
  - https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki
  - https://github.com/bitcoin/bips/blob/master/bip-0389.mediawiki
  - https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h
  - https://gist.github.com/RubenSomsen/c43b79517e7cb701ebf77eec6dbb46b8
date: 2026-07-29
type: repos
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [bip352, silent-payments, coinbase, xpub, descriptor, bip-380, bip-389, gap-limit, primary, negative-result]
summary: "Primary-source determination for the question \"can a miner hand the pool a static payment code instead of an address, so that each block pays an unlinkable output?\" The answer is"
---

# BIP 352 Silent Payments cannot be used in a coinbase output

Primary-source determination for the question "can a miner hand the pool a static payment
code instead of an address, so that each block pays an unlinkable output?" The answer is
**no, structurally** — not "not yet implemented."

## The mechanism BIP 352 requires

Sender side ("Creating outputs"):

```
a = a_1 + ... + a_n              # sum of INPUT private keys
if a == 0: fail
A = a·G
input_hash = hash_BIP0352/Inputs(outpoint_L || A)
ecdh_shared_secret = input_hash · a · B_scan
```

`outpoint_L` is the lexicographically smallest outpoint (36 bytes: 32-byte txid LE ||
4-byte vout LE). Receiver side ("Scanning") computes `A = A_1 + ... + A_n` and the spec
says **"If `A` is the point at infinity, skip the transaction."**

"Selecting inputs" is normative: **"At least one input MUST be from the Inputs For Shared
Secret Derivation list"** — that list is exactly P2TR, P2WPKH, P2SH-P2WPKH, P2PKH.

## Why a coinbase can never satisfy it

Bitcoin Core, `src/primitives/transaction.h`:

```cpp
bool IsCoinBase() const { return (vin.size() == 1 && vin[0].prevout.IsNull()); }
```

Consensus *mandates* the null prevout. `consensus/tx_check.cpp` checks only
`scriptSig.size() ∈ [2,100]` on the coinbase branch (`bad-cb-length`); the
`bad-txns-prevout-null` rejection lives in the `else` branch. So a coinbase has:

- no prevout scriptPubKey → no input public key `A`
- no input private key → `a = 0` → sender-side **"fail"**
- receiver-side `A` = point at infinity → **"skip the transaction"**
- zero inputs from the required derivation list

The shared secret in BIP 352 is *bound to the input set* — which is also why the BIP
declares `SIGHASH_ANYONECANPAY` unsafe ("the inputs must not change once the sender has
signed"). A transaction with no spendable input set cannot produce one.

Ruben Somsen's original gist states the same constraint from the other end: **"the sender
must control one of the inputs in order to be fully private,"** and notes this already
excludes exchanges and custodians who don't control inputs.

**Consequence**: silent payments are usable only in a *second-stage transaction spending*
a coinbase output — never in the coinbase itself. That second stage cannot begin until
100-block maturity has elapsed, by which point the unmixed coinbase has been public for
~16 hours.

## Encoding sizes that break every pool identity field

Silent-payment address = bech32m over `ser_P(B_scan) || ser_P(B_m)` (66 bytes), HRP
`sp`/`tsp`, **minimum 117 characters**, recommended parser limit 1023. `K_max = 2323` caps
scan-group size. Derivation paths are `m/352'/coin'/account'/{1',0'}/0` with **mandatory
hardened derivation**.

## The descriptor alternative and its gap-limit collision

BIP 380 wildcards: "Optionally followed by a single `/*` or `/*h` final step to denote all
direct unhardened or hardened children"; "an output script will be produced for every
child key index." Descriptor charset includes `()[],'/*#` — every one of which collides
with stratum username parsing (ckpool splits on `._`; some miner firmware percent-encodes
non-alphanumerics).

BIP 389 multipath `/<NUM;NUM>`: one specifier per key expression, tuples equal length,
"derived in lockstep." Designed for receive/change, **not** per-payee fan-out.

BIP 44 "Address gap limit": **"currently set to 20. If the software hits 20 unused
addresses in a row, it expects there are no used addresses beyond this point and stops
searching."** This is exactly what breaks under Greg Maxwell's block-height-as-index
suggestion (see the sv2-apps rotation source) — a wallet would need an 800000+ gap limit.

## Negative results (searched, absent)

- **Zero occurrences of "coinbase" in the full 524-line BIP 352 text.**
- No proposal anywhere to extend BIP 352 with a coinbase-specific tweak substitute (block
  height, prev-hash, extranonce). delvingbitcoin search for "silent payments coinbase" and
  "coinbase address rotation" returns zero topics; Optech's silent-payments topic page has
  no mention of mining or coinbase. Genuinely unexplored, not explored-and-rejected.
- **No pool anywhere accepts an xpub, output descriptor, or payment code as a miner's
  payout identity.** Verified against source for ckpool, public-pool, DATUM gateway, SV2
  reference apps, and Ocean docs.
