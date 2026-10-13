---
title: xpub Payout Identity — miner-supplied descriptors as pool identity
category: concept
created: 2026-07-29
confidence: high
tags: [xpub, output-descriptor, bip-380, bip-389, bip-44, gap-limit, ckpool, public-pool, datum, tides, pplns-jd, firmware-limits, field-width]
volatility: warm
updated: 2026-07-30
summary: "What mechanically changes when a miner's stratum username is a wildcard descriptor rather than an address: the ledger key must split from the payout script, rotation triggers are unresolved upstream, and every identity field in every codebase is too narrow."
verified: 2026-07-30
sources:
  - "raw/repos/2026-07-29-pool-identity-vs-payout-script-conflation-code-read.md"
  - "raw/repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility.md"
  - "raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md"
---

# xpub Payout Identity

The user's construction — *"do TIDES/PPLNS-JD but the user provides an xpub or similar"* — is
buildable. The derivation math is trivial. **The work is entirely in decoupling the ledger key
from the payout script, and in field widths.**

Direct answer on the "or similar" clause: **BIP 32 xpubs and BIP 380 wildcard descriptors work;
BIP 352 silent payments cannot** — a coinbase has no input private key, so there is no ECDH
shared secret to derive from. See [[coinbase-amount-linkability#BIP 352 silent payments cannot rescue this — a hard structural no|the impossibility proof]].

## The engineering size varies by an order of magnitude across schemes

| Scheme | Coupling | Work required |
|---|---|---|
| **PPLNS-JD / share-accounting-ext** | Ledger key is `(slice, share_index)` — **positional, zero identity fields** | Identity lives only in SV2 `user_identity`. An xpub changes **only the identity layer.** |
| **TIDES** | Keyed per *user*; but "a generation transaction that would pay all miners appropriately" is built at **work-issue** time | A derivation **per miner per template refresh**, not per payout |
| **ckpool** | `username[128]` is the hash key **and** becomes `txnbin[48]` via `address_to_txn()` | The username *is* the scriptPubKey. Maximum coupling. |
| **public-pool** | `address varchar(62)` is part of a composite **PRIMARY KEY** | A primary-key schema migration, not a column widen |
| **DATUM** | "designed with the assumption that pool usernames are generally Bitcoin addresses" | Plus the firmware ceilings below |

**PPLNS-JD is the outlier and the opportunity.** Its `Slice{number_of_shares, difficulty, fees,
root, job_id}` / `Share{nonce, ntime, version, extranonce, job_id, share_index, merkle_path}` /
`PHash` structures contain **not one identity field**; the ledger is verified positionally by
`merkle_path(share) + share_hash == slice.root`. That is already the decoupling every other
scheme has to retrofit. (Caveat: the PPLNS-JD paper at `dmnd.work` returns HTTP 404; this is from
`extension.md` in the spec repo.)

## The five things that mechanically change

1. **Split the ledger key from the payout script.** Persist `identity → descriptor` plus a
   **monotonic derivation index** — new mutable state that must survive restart. This is the same
   wall [[coinbase-address-rotation#Per-miner xpub usernames|per-miner rotation]] hits: a PPLNS
   balance table doing `ON CONFLICT (address) DO UPDATE` keys balances on address *globally*, so a
   miner whose address changes every block writes a new row every block and pending-credit
   carry-forward silently stops working.
2. **Decide the rotation trigger.** This is the actual hard question, and it is **unresolved
   upstream**. SV2 issue #697, verbatim: *"The tricky part here is to decide when to rotate. Doing
   it for every `SetNewPrevHash` would generate too many addresses, which is difficult for wallet
   software to keep track of."* Greg Maxwell's alternative (issue #1652) is blockheight-as-index,
   which is self-synchronizing — the height *is* the index, so no persistence is needed — but
   apoelstra notes it "might confuse wallets that don't have an 800000+ gap limit." BIP 44's stated
   gap limit is **20**.
3. **Derivation is pool-side only.** The miner never sees the coinbase before it is built, so the
   miner cannot supply the derived script itself. This is why a miner-supplied xpub does *not*
   reduce what the pool knows — see [[payout-attribution-privacy]].
4. **Field widths break everywhere.** A wildcard descriptor runs ~150 characters. ckpool: 127
   usable, and it splits usernames on `._` — both characters collide with descriptor syntax
   (`()[],'/*#`). public-pool: `varchar(62)`, which fits a P2TR address and nothing else. DATUM:
   191 chars, but **Avalon firmware truncates at 63 and Whatsminer buffer-overflows past 127**
   ("which may damage your miner"). Some firmware percent-encodes non-alphanumerics, mangling
   descriptor characters. A descriptor also fails public-pool's `IsBitcoinAddress` validator
   outright, so authorization rejects it before any of this matters.
5. **Cost is per-template, not per-payout** for TIDES and ckpool — both already construct per-user
   coinbases on every template refresh (`generate_userwbs()` / `__generate_userwb()` splice
   `user->txnbin` between shared `coinb2bin`/`coinb3bin`). sv2-apps' `XpubDerivator` **re-parses
   the descriptor string on every derivation** — a deliberate trade because miniscript's
   `Descriptor<DescriptorPublicKey>` uses an internal `RefCell` and is not `Send + Sync`. Fine at
   once-per-block; a parse storm at once-per-miner-per-template.

## The hard output-count ceiling is firmware, not consensus

Any per-miner-output scheme is bounded by what ASIC firmware will accept as a coinbase, not by
block weight. DATUM's `available_coinbase_outputs[512]` and its coinbase-type table cap at "huge
— max 16 kB … something like **380 to 530 outputs**," with per-vendor limits: nicehash ≈500 B,
antminer ≈730 B, antminer2 2250 B, whatsminer 6500 B.

Braidpool's retrospective is the empirical verdict: "In p2pool this UHPO set was placed directly
in the coinbase of every block, resulting in a large number of very small payments to hashers…
the large coinbase with small outputs competed for block space with fee-paying transactions."
Braidpool's own coinbase has **two** outputs. See [[ctv-coinbase-payout-tree]] for the
fanout-compression direction.

## Upstream status: wildcards deliberately deferred

SV2 PR **#1720** (merged 2025-07-09) shipped `coinbase_output_descriptors` using BIP 385
`addr()`/`raw()`, deprecating `output_script_type` dictionaries — UX preserved as "You just
generate an address and put `addr(<my address>)` as the descriptor." **Wildcards remain rejected
in merged code**: `config_helpers/coinbase_output/mod.rs` asserts "Miniscript: key with a wildcard
cannot be a DerivedDescriptorKey," with the comment *"no wildcards allowed (at least for now;
gmax thinks it would be cool if we would instantiate it with the blockheight or something, but
need to work out UX)."* Also rejected: hardened steps, multipath `<0;1>` ("this is not a wallet
with change"), and xprv.

Crucially, the scope of all of this is **the pool's own single fallback/solo output — not
per-miner.** Confirmed negative result: **no pool anywhere accepts an xpub, output descriptor, or
payment code as a miner's payout identity**, verified against source for ckpool, public-pool,
DATUM, the SV2 reference apps, and Ocean's docs.

## The V1 path is gated on Translator aggregation mode

V1 miners can in principle reach a descriptor-identity pool through the SV2 Translator, with the
descriptor as the `mining.authorize` username forwarded upstream. **Aggregation mode decides whether
per-device identity is even representable**, and it is a mode requirement rather than an effort
estimate ([[../../raw/notes/2026-07-30-ll-pplns-window-units-and-identity-boundaries|lesson 5,
2026-07-30]]):

- `TproxyMode::Aggregated` shares *"a single extended Sv2 channel"* across all downstream V1
  connections, distinguishing devices by extranonce_prefix while *"presenting them as a single entity
  to the upstream server"* (`translator/src/lib/utils.rs:180-215`); every channel id collapses to
  `AGGREGATED_CHANNEL_ID = u32::MAX` (`utils.rs:36`, `downstream_message_handler.rs:117-118`). One
  upstream channel carries one `user_identity`, and the pool parses payout identity from the
  channel-open `user_identity` — so per-device descriptors are **unrepresentable** in this mode no
  matter how the Translator is refactored.
- The only per-device identity path — the `TLV_FIELD_TYPE_USER_IDENTITY` per-share TLV — is emitted
  only in non-aggregated mode and is then **discarded by the pool**
  (`pool/src/lib/channel_manager/mining_message_handler.rs:919`: `if let Some(_user_identity) =
  user_identity { /* …in the future */ }`).

General form of the trap: before designing per-device semantics on top of a proxy, check whether the
proxy multiplexes devices onto one upstream session.

## What it buys, and what it does not

**Buys**: defeats the Romiti et al. attack, which turns on payout-address reuse (median 20 reuses
→ 92 % of BTC.com miners identified). Fresh-per-payout also defeats the *only* block-withholding
detection that has ever worked in production — Eligius 2014 caught its attacker by clustering two
payout addresses. See [[block-withholding]].

**Does not buy**: any reduction in what the pool learns. The pool still validates every share,
still sets every target, still measures hashrate to within a few percent, and now additionally
holds the descriptor that links *all* of a miner's rotated addresses together. An xpub username
is an on-chain privacy upgrade and a **pool-side privacy no-op**.

### Why the descriptor engine can't be borrowed for the stealth variant

**Added 2026-08-16.** A concrete instance of the no-op above. BIP 32 unhardened derivation — what a
wildcard descriptor performs — is `a_i = a_par + H(K_par ‖ i)`, so the parent secret regenerates every
child. For a *payout* key that is the point: the pool must be able to spend, and recovery from a seed
is a feature. For the sender-side ECDH key in the silent-payment coinbase variant it is fatal, because
that key's entire value is that it can be **destroyed** — one parent secret would retroactively unmask
every payout the pool ever made. Same derivation, opposite requirement, which is why the stealth
variant needs independent CSPRNG keys with an erasure schedule rather than a descriptor. The rotation
*plumbing* still ports; only the key source doesn't. See
[[../../raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier|Lessons: SV2 pool tag as A_send carrier]]
lesson 2 and [[coinbase-address-rotation]].

## Sources

- [[../../raw/repos/2026-07-29-pool-identity-vs-payout-script-conflation-code-read|Identity-vs-payout-script code read]]
- [[../../raw/repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|BIP 352 / BIP 380 / BIP 44 gap limit]]
- [[../../raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read|sv2-apps xpub coinbase rotation]]

## See also

- [[coinbase-address-rotation]] — the pool-side mechanism this generalizes
- [[payout-attribution-privacy]] — why this doesn't blind the pool
- [[coinbase-amount-linkability]] — the amount channel it doesn't close
- [[pplns-jd]] — the already-decoupled ledger
- [[tides]] — work-issue-time coinbase precomputation
- [[datum]] — firmware ceilings
- [[ctv-coinbase-payout-tree]] — output-count compression
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]]
- [[block-withholding|Block Withholding (BWH) and FAW]]
- [[sv2-share-accounting-ext|SV2 Share Accounting Extension]]
- [[../../raw/notes/2026-07-30-ll-pplns-window-units-and-identity-boundaries|Lessons: window units, share retention, identity boundaries]] — the Translator aggregation constraint above, and where pool responsibility for payout identity ends

