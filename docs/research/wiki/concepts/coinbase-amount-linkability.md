---
title: Coinbase Amount Linkability
category: concept
created: 2026-07-29
confidence: high
tags: [subset-sum, knapsack, coinjoin, wabisabi, amount-linkability, coinbase, denomination, output-splitting, negative-result]
volatility: cold
updated: 2026-07-29
summary: "Why payout amounts leak attribution independently of addresses, and why a coinbase is a strictly EASIER subset-sum instance than a CoinJoin — one input of publicly known value, no input shuffling available, and no ability to pad the total."
verified: 2026-07-29
sources:
  - "raw/papers/2026-07-29-maurer-2017-knapsack-coinjoin-arbitrary-values.md"
  - "raw/papers/2026-07-29-romiti-2019-mining-pool-payout-deanonymization.md"
  - "raw/repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility.md"
---

# Coinbase Amount Linkability

The user's premise — *"payout amounts/shares would still be attributable"* — is correct, and the
CoinJoin literature says exactly how much so. The formal statement (Maurer, Neudecker & Florian,
TrustCom 2017):

> Determining the input→output mapping in a transaction with arbitrary values "**is equal to
> solving the subset sum problem**."

And the naive case is not merely hard-but-solvable, it is *trivial*:

> For unequal-amount CoinJoins, "there is generally only **one** non-derived mapping" — the honest
> one. Interpretation: naive unequal-amount joining "does **not** yield any privacy benefits."

## Why a coinbase is the hardest case to protect

The coinbase is a strictly **easier** subset-sum instance than any CoinJoin. Three reasons, all
structural:

1. **One input, of publicly known value.** Subsidy + fees is computable by anyone from the block.
   There is no input-side ambiguity to exploit at all.
2. **Input shuffling is unavailable.** CoinJoin privacy comes from *both* directions of
   ambiguity — an input could belong to many outputs and vice versa. A coinbase has
   `vin.size() == 1` with a null prevout, mandated by `IsCoinBase()`. Half the ambiguity budget
   is structurally absent.
3. **Padding is impossible.** A CoinJoin coordinator can add decoy inputs and outputs to enlarge
   the search space. A pool **cannot inflate the total** — the coinbase output sum is
   consensus-bounded by subsidy + collected fees. Every satoshi of decoy must be taken from a
   real miner.

Add the amplifiers Atlas found breaking SharedCoin: **round numbers accelerate the attack** ("if
mixing participants use round numbers… we can further reduce the number of possible
subsets"), and **address reuse across rounds** is fatal. A pool paying `payout = share_fraction ×
reward` produces highly non-round, high-entropy amounts — good — but Romiti et al. showed reuse
is where real pools fail (median 20 reuses at BTC.com, 92 % identification).

## The ceiling on what amount-hiding can achieve

WabiSabi (Ficsór et al., eprint 2021/206) states the honest upper bound for any scheme with
cleartext amounts in the final transaction:

> "cleartext amounts appearing in the final transaction might still link individual inputs and
> outputs" — anonymity is only "**apart from what is already deducible given the public amounts
> visible on the Bitcoin blockchain**."

That clause is the ceiling for a blinded pool too. Even with perfect cryptographic unlinkability
in the credential layer, the on-chain amount multiset is public and the sum is fixed.

**Output splitting works, but requires a party who knows the truth.** Maurer et al.'s countermeasure
splits a participant's output into sub-amounts, but "requires knowledge of all sub-transactions" —
so the splitter must know the real mapping to split correctly. **In a pool, the splitter is the
pool.** The mechanism cannot make the pool blind to itself; it only protects against chain
observers. (And splitting has costs: coinbase output count is firmware-bounded to roughly
380–530 outputs, and Braidpool's retrospective on p2pool notes the "large coinbase with small
outputs competed for block space with fee-paying transactions.")

**Denomination ladders are the real defense, and they bound the leak rather than removing it.**
Wasabi's approach — powers-of-two-ish denominations, plus a filter requiring an amount to have
"occurred in the frequency table at least twice" — converts exact-amount leakage into
bucket-level leakage. This is the same structure as ecash keysets, which is not a coincidence:
Cashu's mining-share proposal set `amount = 2^(share difficulty − keyset minimum difficulty)`, a
denomination ladder over difficulty. It reduces resolution; it does not eliminate the channel,
and it forces rounding losses somewhere.

## BIP 352 silent payments cannot rescue this — a hard structural no

The obvious "just let miners publish one reusable address" answer is **impossible in a coinbase**,
not merely unimplemented. BIP 352 requires the sender to compute

```
a = a_1 + a_2 + ... + a_n     (sum of input private keys)
input_hash = hash_BIP0352/Inputs(outpoint_L || A)
"If a = 0, fail"
```

and instructs the receiver: *"If `A` is the point at infinity, skip the transaction."* At least one
input **MUST** come from the Inputs For Shared Secret Derivation list. A coinbase has exactly one
input with a null prevout — no private key exists for it, by consensus:

```cpp
bool IsCoinBase() const { return (vin.size() == 1 && vin[0].prevout.IsNull()); }
```

There is no `a`, so there is no ECDH shared secret, so there is no silent payment. Confirmed
negative results from the primary-source read: **zero occurrences of "coinbase" in BIP 352's 524
lines**, and no proposal anywhere for a coinbase-specific tweak substitute. See
[[xpub-payout-identity]] for what *does* work (BIP 32/380 descriptors) and what it costs.

### Correction to the section above — the impossibility argument does not hold

**Added 2026-08-24.** The reasoning is *"a coinbase has no input private key, so there is no `a`, so
there is no shared secret."* Sound about BIP 352 **as written**; unsound as an impossibility result,
because it assumes `a` must come from the inputs. It need not. Supplying a dedicated sender key
`a_send` deliberately — published in the coinbase rather than recovered from an input — closes the
construction, with the BIP 34 block height substituting for the outpoint nonce. That variant now
exists in executable form ([average-gary/sp-coinbase-vectors](https://github.com/average-gary/sp-coinbase-vectors),
runnable at <https://average-gary.github.io/sp-coinbase-vectors/>) and is tracked by
[[../../inventory/candidates/coinbase-native-stealth-payout|the coinbase-native stealth payout
candidate]]. The "zero occurrences of coinbase in BIP 352" finding stands, and is in fact the point:
the case is **unspecified, not impossible**.

**What survives is this article's actual thesis — silent payments rescue *address* linkability, never
*amount* linkability.** The two are independent channels. A hashrate provider with a stable share of
the pool receives a proportionally stable share of every payout, so an observer computes
`amount_i / T` from public data and obtains a fingerprint persisting across blocks **without any
address ever repeating**. Hashrate moves slowly and pool hashrate is heavy-tailed, so the largest
providers are the easiest to track. So § *The ceiling on what amount-hiding can achieve* binds the
stealth variant exactly as it binds every other scheme here: the WabiSabi bound is not evaded by
rotating keys. Fresh-per-block addresses remain worth having — address reuse is the channel Romiti et
al. exploited — but they are a partial defense and should not be described as more. See
[[../../raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability|Lessons: SV2 coinbase control
and amount linkability]] lesson 6.

## Sources

- [[../../raw/papers/2026-07-29-maurer-2017-knapsack-coinjoin-arbitrary-values|Maurer et al. (TrustCom 2017), WabiSabi, CoinJoin Sudoku]]
- [[../../raw/papers/2026-07-29-romiti-2019-mining-pool-payout-deanonymization|Romiti et al. (WEIS 2019)]]
- [[../../raw/repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|BIP 352 coinbase incompatibility]]

## See also

- [[payout-attribution-privacy]] — the sum constraint against pool vs. chain observer
- [[xpub-payout-identity]] — descriptors as the workable alternative
- [[blind-share-accounting]] — hiding the credential, not the amount
- [[ctv-coinbase-payout-tree]] — output-count scaling
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]]
