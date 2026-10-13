---
title: Payout Attribution Privacy — what a pool structurally knows
category: concept
created: 2026-07-29
confidence: high
tags: [attribution, privacy, threat-model, hashrate-inference, deanonymization, sum-constraint, stratum, sv2-noise]
volatility: warm
updated: 2026-07-29
summary: "The threat model for miner attribution. A pool's knowledge comes from share VALIDATION, not from payment — so blinding the payout rail addresses third-party observers only. Separates what a pool cannot avoid learning from what is merely conventional to log."
verified: 2026-07-29
sources:
  - "raw/papers/2026-07-29-recabarren-carbunar-hardening-stratum.md"
  - "raw/papers/2026-07-29-romiti-2019-mining-pool-payout-deanonymization.md"
  - "raw/repos/2026-07-29-mining-privacy-prior-art-survey.md"
  - "raw/repos/2026-07-29-pool-identity-vs-payout-script-conflation-code-read.md"
---

# Payout Attribution Privacy

The prerequisite for any "blinded payout" design: establishing **what a pool learns whether it
wants to or not**. The central finding reorders the whole problem.

> **The pool's knowledge comes from share *validation*, not from *payment*.** Any scheme that
> leaves the pool as share validator has already conceded per-connection hashrate before a
> single satoshi moves. Blinding the payment rail — Ocean's BOLT12, an ecash mint, a derived
> address — addresses the **third-party** adversary, not the pool.

This is why the question "how could a service blind itself to attribution?" splits into two
very different problems with different difficulty: **unlinking identity from payout** (tractable)
and **not knowing per-unit work volume** (no known solution — see
[[blind-share-accounting]]).

## What a pool structurally cannot avoid learning

These follow from the pool's function as validator and accountant. No transport encryption or
protocol revision touches them, because **the pool is the decrypting endpoint**.

1. **The exact content of every accepted share.** To credit a share the pool must recompute the
   header and check `H²(nonce‖F) < target` — requiring `job_id`, `nonce`, `ntime`, `version`,
   extranonce. *Validation is observation.*
2. **Which work-unit each share belongs to.** Shares are only verifiable against a specific
   coinbase/extranonce assignment; extranonce space is partitioned per channel precisely so
   submissions are distinguishable. Attribution to *some* unit of account is the definition of
   pooled accounting.
3. **Exact per-unit share count and timing.** SV2's `sequence_number` is a monotone per-channel
   counter; even without it, counting accepted shares *is* the payout computation.
4. **Per-unit assigned difficulty.** The pool sets it (`SetTarget` / `mining.set_difficulty`).
5. **Hashrate, to within a few percent.** `hashrate = difficulty × 2^32 / mean_inter_share_time`.
   Unavoidable: the pool assigns the difficulty and timestamps the arrivals. Recabarren &
   Carbunar measure **1.75–6.5 % payout-prediction error for an adversary strictly weaker than
   the pool** (see [[hashrate-inference-side-channels]]).
6. **The transport peer** — source IP/port, connection open/close, reconnects. SV2 Noise
   authenticates the *server* to the client, not the client anonymously.
7. **Session structure over time** — uptime, reconnect cadence, and the difficulty-ramp transient
   after each reconnect.
8. **nTime- and version-rolling behaviour**, since both are required submit fields and both are
   hardware/firmware-characteristic.

## What is merely conventional

Removable without breaking accounting — this is the actual attack surface a design can reduce:

| Leak | Why it's optional |
|---|---|
| `mining.authorize` username / SV2 `user_identity` | Nothing about validation requires a *persistent* or *meaningful* name. A per-session blinded token validates identically. (V1 also sends the password in cleartext, which pools ignore.) |
| Worker names (`account.worker1`) | Pure operator convenience; voluntarily reveals fleet structure. |
| SV2 `SetupConnection` `vendor` / `hardware_version` / `firmware` / `device_id` | Gratuitous fingerprints — `device_id` is by definition a stable unique hardware identifier. **SV2 is worse than V1 here**, which had no such fields. |
| `nominal_hash_rate` self-declaration | Convenience for target-setting; the pool measures hashrate empirically anyway. Removing it costs only VarDiff convergence speed. |
| Cross-session identifier persistence | `channel_id` is connection-scoped (fine); the username is not (not fine). Longitudinal linkage comes from the latter. |
| Payout-address stability | The dominant on-chain privacy variable — see the reuse figures below. |
| Coinbase markers/tags | Self-identification, not cryptographically secured. |

Notably, SV2's own spec authors already treat per-worker attribution as a privacy cost. Extension
`0x0002` §4.2: *"Mining farms should be aware that sharing per-worker data with pools could
reveal operational insights."* They made it **opt-in** for that reason — and base SV2 extended
channels aggregate work, so a proxy can front a whole farm as one channel. That is the single
existing lever for attribution reduction in shipped protocol, and it operates *below* the pool,
not against it.

## The sum constraint, stated precisely

The common worry — "an aggregate must equal the sum, so amounts reveal share weight" — is
**near-vacuous against the pool and sharp against a chain observer**. Getting this distinction
right is what keeps a design honest.

**Against the pool: zero incremental leak.** The pool *computed* the sum; it already knows every
addend. There is no scenario in which a pool learns share weights *from* the payout total.

**Against a chain observer: sharp — and it is normalization that does the damage, not the total.**
For payout outputs `{a₁…a_N}`:

- `aᵢ / Σaⱼ` is miner *i*'s **exact** relative share weight — definitionally the payout
  function's output. No knowledge of pool difficulty, window length, or absolute hashrate needed;
  the sum normalizes them away.
- `N` itself bounds the participant count, hence the anonymity-set size. AntPool's fixed
  101-output structure was precisely the *filter* Romiti et al. used to find payout transactions.
- So a blinded scheme can hide **who** but not the **distribution shape** — the multiset of share
  weights. The Gini coefficients of 0.938–0.945 that Romiti et al. computed *are* that shape,
  recovered from public data alone.

See [[coinbase-amount-linkability]] for why the coinbase is the worst case of this.

## Attribution is already achieved by third parties

Romiti et al. (WEIS 2019) is the empirical proof that this is not hypothetical. With **no pool
cooperation** — just coinbase reward addresses, coinbase tags, and multiple-input clustering:

| Pool | Individual miners identified | Median payout-address reuse |
|---|---|---|
| BTC.com | **92 %** | 20 |
| ViaBTC | **75 %** | 5 |
| AntPool | **30 %** | 2 |

The inverse correlation between reuse and identification is the empirical case for
per-payout address derivation. It is also the only defense visible in that data — and note the
one pool that rotates fastest is *also* the one with the strictest filter applied against it, so
30 % is a floor, not a measured privacy win.

**Sybil-splitting does not defeat the sum constraint and can worsen it.** Splitting into N
outputs preserves the sum, so any clustering that reunites the pieces recovers the full weight —
and consolidating split payouts into one spending transaction is the multiple-input clustering
heuristic's *ideal* input. At the connection level the pool re-correlates trivially via source
IP, reconnect co-timing, and hashrate signature, though **no paper measures connection-level
Sybil resistance** — treat it as unquantified rather than disproven.

## What SV2 Noise does and does not protect

**Does**: confidentiality and integrity against passive eavesdroppers and MITM on the
miner↔pool path — killing StraTap and share-hijacking attacks, with BIP340 Schnorr server
authentication against impersonation.

**Does not**:

1. **Any privacy from the pool.** The pool holds the session key. The spec's §4.1 privacy
   requirement is written against an *"adversary"*, and **the pool is not modeled as one
   anywhere in the spec**.
2. **Timing/volume metadata** — see [[hashrate-inference-side-channels]]; encryption is
   explicitly called "ineffective" against the inference channel.
3. **On-chain payout analysis** — entirely out of scope; Romiti et al. needed no network access.
4. **The `SetupConnection` fingerprints** — encrypted in flight, delivered to the pool in
   cleartext.

## Where identity is welded to payout in real code

The engineering size of the problem varies by an order of magnitude across schemes — see
[[../../raw/repos/2026-07-29-pool-identity-vs-payout-script-conflation-code-read|the code read]]:

| Codebase | Coupling | Consequence |
|---|---|---|
| **ckpool** | `username[128]` is the hash key **and** is converted to `txnbin[48]` via `address_to_txn()` | The username *is* the scriptPubKey. Maximum coupling. |
| **public-pool** | `address varchar(62)` is part of the composite **primary key** | Changing identity type is a PK schema migration. |
| **DATUM** | "designed with the assumption that pool usernames are generally Bitcoin addresses" | Plus firmware limits: Avalon truncates usernames at 63 chars, Whatsminer **buffer-overflows past 127**. |
| **TIDES** | keyed per *user*; but the coinbase is precomputed at **work-issue** time for every template | Derivation cost is per-miner-per-template, not per-payout. |
| **PPLNS-JD / share-accounting-ext** | ledger key is **(slice, share_index)** — positional, **zero identity fields** | Already decoupled. Identity lives only in SV2 `user_identity`. |

PPLNS-JD is the outlier and the opportunity: its share ledger contains no address, user, or
account field at all.

## Sources

- [[../../raw/papers/2026-07-29-recabarren-carbunar-hardening-stratum|Hardening Stratum (PETS 2017)]]
- [[../../raw/papers/2026-07-29-romiti-2019-mining-pool-payout-deanonymization|Romiti et al. (WEIS 2019)]]
- [[../../raw/repos/2026-07-29-mining-privacy-prior-art-survey|Mining privacy prior-art survey]]
- [[../../raw/repos/2026-07-29-pool-identity-vs-payout-script-conflation-code-read|Identity-vs-payout-script code read]]

## See also

- [[hashrate-inference-side-channels]] — the measured timing channel
- [[coinbase-amount-linkability]] — the amount half of the problem
- [[blind-share-accounting]] — cryptographic primitives, and why none fit cleanly
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]] — the synthesis
- [[ehash]] — the only design that claims this property
- [[pplns-jd]] — the positional ledger
- [[tides]] — per-user share log, coinbase precomputed at work-issue
- [[sv2-share-accounting-ext]]
- [[../decisions/attribution-retention-tradeoffs|Attribution Retention Tradeoffs]]
- [[block-withholding|Block Withholding (BWH) and FAW]]
- [[braidpool|Braidpool]]
- [[coinbase-address-rotation|Coinbase Address Rotation]]
- [[p2poolv2-accounting|p2poolv2 Accounting (deep-dive)]]
- [[radpool|Radpool]]
- [[self-blinding-architectures|Self-Blinding Architectures (cross-domain)]]
- [[xpub-payout-identity|xpub Payout Identity — miner-supplied descriptors as pool identity]]
- [[../theses/blinded-share-credit-commitment|Thesis: blinded share-credit commitment]] — the 2026-07-29 verdict round

