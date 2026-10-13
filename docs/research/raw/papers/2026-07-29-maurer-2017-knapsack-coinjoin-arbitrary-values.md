---
title: "Anonymous CoinJoin Transactions with Arbitrary Values (the Knapsack paper) + WabiSabi + CoinJoin Sudoku"
authors: [Felix Konstantin Maurer, Till Neudecker, Martin Florian, István András Seres, Ádám Ficsór, Yuval Kogman, Lucas Ontivero, Kristov Atlas]
year: 2014-2021
venue: "IEEE TrustCom 2017 / IACR eprint 2021/206 / coinjoinsudoku.com advisory"
source: https://www.comsys.rwth-aachen.de/publication/2017/2017_maurer_anonymous-coin-join-transactions-with/2017_maurer_anonymous-coin-join-transactions-with.pdf
supporting_sources:
  - https://eprint.iacr.org/2021/206
  - https://www.coinjoinsudoku.com/advisory/
type: papers
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [knapsack, subset-sum, amount-linkability, coinjoin, wabisabi, kvac, denomination, output-splitting, coinjoin-sudoku, primary]
summary: "The formal and empirical basis for the claim that **payout amounts are the hardest leak**. Three sources spanning theory, a deployed system, and a real break."
---

# Amount-based attribution is subset-sum — Maurer et al. 2017, WabiSabi 2021, CoinJoin Sudoku 2014

The formal and empirical basis for the claim that **payout amounts are the hardest leak**.
Three sources spanning theory, a deployed system, and a real break.

## Maurer, Neudecker & Florian (TrustCom 2017) — the formal metric

Attribution is stated as an exact math problem: recovering who-paid-whom means finding "a set
of outputs with the same sum as a given set of inputs. **This is equal to solving the subset
sum problem** and a special case of the knapsack problem."

Metric: a **mapping** partitions inputs and outputs into sub-transactions with equal sums; a
**non-derived mapping** is one not obtainable by merging others. Link probabilities `p_II`,
`p_IO`, `p_OO` = fraction of mappings in which a pair co-occurs. Amounts were drawn from the
real blockchain value distribution up to March 2017.

**Headline negative result**: naive unequal-amount CoinJoin gives essentially zero privacy.
"For all number of sub-transactions with two inputs, there is generally only one non-derived
mapping" — the original composition is recovered unambiguously.

**The fix works, with two caveats.** Their output-splitting algorithm splits one output to
realize the difference between two merged sub-transactions' sums; "even for mixed CoinJoin
transactions with a low number of sub-transactions, the number of non-derived mappings is
reliably greater than one." Splitting cuts `p_IO = 1` pairs from ~30 to near zero (Figs. 8, 9).
Attack runtime is `O(2^n · m)`, ms → 100 s at ~10 inputs/sub-tx (Fig. 3).

Caveat 1: **input shuffling is also required**, because output splitting "only decreases the
linkability of input-output pairs and not the linkability of input-input pairs."

Caveat 2 — the one that lands on a coinbase: **"our output splitting algorithm requires
knowledge of all sub-transactions. Using it in a P2P network would likely leak information to
all participants."** The splitter must know the truth to split well. And privacy is partly
*computational*, not information-theoretic — "they only provide some form of computational
anonymity given a large-enough number of inputs and outputs."

## WabiSabi (eprint 2021/206) — hides amounts from the *coordinator*, not the chain

The historical rationale for equal outputs, stated plainly: "The use of standard denominations
in the resulting CoinJoin transaction obscures the relationship between individual inputs and
outputs." Privacy modeled as "k-anonymity with amounts and script types as quasi-identifiers."

Mechanism: user registers an input of amount `a_in` with k homomorphic (Pedersen) commitments
plus range proofs (`0 ≤ a < a_max`, `a_max = 2^51 − 1`) and a sum proof that sub-amounts equal
`a_in`; KVAC credentials carry serial numbers against double-spend and can be split/merged;
"bootstrap credentials" of amount 0 enable a first request. Balance `Δa` is proven, so value
is neither created nor destroyed. The coordinator "only learns their sum."

**The load-bearing caveat is a footnote, not the headline**: "Note that the cleartext amounts
appearing in the final transaction might still link individual inputs and outputs." The stated
design goal is explicitly bounded — the coordinator learns nothing "apart from what is already
deducible given the public amounts visible on the Bitcoin blockchain." **That phrase is the
honest ceiling on any coordinator-side scheme.**

Quantified failure of the fixed-denomination regime it replaces: Wasabi CoinJoins are "trivial
to detect" (">10 equal amount outputs are surely Wasabi"); a significant fraction of inputs
fall below the minimum denomination; all implementations produce "toxic change"; non-mixed
change "is almost always linkable to their corresponding inputs."

Residual leaks are timing/ordering: the coordinator can "linearize all requests by delaying
individual [ones] to recover the full set of labelled edges," and "always learns the requested
inputs and outputs, even if a round fails" (DoS → intersection attack).

## CoinJoin Sudoku (Atlas, 2014) — the theory, demonstrated on a live mixer

Against deployed SharedCoin: "The tool considers all of the possible ways to group inputs and
outputs, and eliminates the possibilities that include groups that do not add up."

**Quantified break**: on a sample transaction it grouped **69 % of inputs and 53 % of
outputs**. Cost: 30.75 hours on a single 2.3 GHz core — 2014 hardware. Detection was also
easy: from 20,000 transactions across 45 blocks, "2.6 % (529) fit the profile."

**Round numbers accelerate the attack.** The solver "processes a transaction by examining one
digit at a time in the inputs and outputs, working its way from right to left; this is faster
because transactions typically involve inputs and outputs with many zeros."

Its mitigation advice raises cost only: use "the maximum number of rounds permitted (10)…to
increase the amount of computation required," and privacy holds only "until user-friendly
analysis tools are released to the public."

## Wasabi's deployed denomination ladder (for reference)

`DenominationBuilder.cs` / `AmountDecomposer.cs`: `PowersOf(2) ∪ PowersOf(3) ∪ 2×PowersOf(3)
∪ PowersOf(10) ∪ 2×PowersOf(10) ∪ 5×PowersOf(10)` — 79 denominations from 5,000 sats to
1374.38953472 BTC. Three design details that transfer directly to a coinbase decomposition:

- denominations are filtered to those "occurred in the frequency table at least twice" — **a
  denomination only one participant could produce is a fingerprint, not a disguise**;
- `maxNumberOfOutputsAllowed = min(AvailableVsize / smallestScriptType, 10)`;
- changeless decompositions strictly preferred, and randomness injected deliberately: "We want
  to make sure our random selection is not between similar decompositions."

## Why this is strictly harder in a coinbase

A normal CoinJoin resists subset-sum because *both* the input partition and the output
partition are unknown — two degrees of freedom to inflate the mapping count.

**A coinbase has exactly one input, whose value is a public consensus constant** (subsidy +
fees, both independently computable). The input partition is fixed and trivial; input
shuffling — the half Maurer et al. found necessary for `p_II` — is unavailable. Every
ambiguity must come from the output side alone, against a publicly known exact total with no
fee slack. Padding cannot help: every satoshi added to a decoy output is subtracted from a
real one, because the pool cannot inflate the total.

## Negative results

- **No source proposes equal-value or padded coinbase outputs for payout privacy.** Searched
  delvingbitcoin, bitcoin-dev, Optech, general web.
- **No source analyzes subset-sum attribution against coinbase outputs specifically.** The
  knapsack literature assumes multi-input transactions.
- **No academic paper on mining-pool payout deanonymization by amount.** The clustering
  survey arXiv 2403.00523 explicitly excludes coinbase transactions.
- Confidential Transactions would solve it outright and **is not available on Bitcoin** — per
  the delvingbitcoin state-of-privacy thread, CT exists only on sidechains.
