---
title: "Block-withholding detection already fails under full identity — Eyal 2015, Rosenfeld 2011, APoW 2026, Towns 2024"
authors: [Ittay Eyal, Meni Rosenfeld, Anthony Towns]
year: 2011-2026
venue: "IEEE S&P 2015 / arXiv 1112.4980 / arXiv 2601.02496 / bitcoindev mailing list"
source: https://www.cs.cornell.edu/~ie53/publications/btcPoolsSP15.pdf
supporting_sources:
  - https://arxiv.org/abs/1112.4980
  - https://arxiv.org/abs/2601.02496
  - https://groups.google.com/g/bitcoindev/c/1tDke1a2e_Q
type: papers
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [block-withholding, BWH, detection, attribution, eligius, address-clustering, oblivious-shares, APoW, pplns, incentive-compatibility, anthony-towns, primary]
summary: "The reflexive objection to a self-blinding pool is that per-miner attribution is required to detect block withholding. **The withholding literature refutes this in its strong form**: attribution-based detection already fails under full identity. One narrow exception survives, and it is specifically about address clustering."
---

# The "you need attribution to detect withholding" objection, checked against the literature

The reflexive objection to a self-blinding pool is that per-miner attribution is required to
detect block withholding. **The withholding literature refutes this in its strong form**:
attribution-based detection already fails under full identity. One narrow exception survives,
and it is specifically about address clustering.

This note deliberately re-reads sources already in this wiki (`eyal-2015-miners-dilemma`,
`rosenfeld-2011-pool-reward-analysis`) through the attribution lens, and adds two new ones.

## Eyal, "The Miner's Dilemma" (IEEE S&P 2015) — detection fails *with* identity

Verbatim §II-C:

> "Even if a pool detects that it is under a block withholding attack, it **might not be able
> to detect which of its registered miners are the perpetrators**."

Note "**registered**" — identity is already assumed and still insufficient. The statistical
reason:

> "If the attacker has a small mining power, it will send frequent partial proofs of work, but
> the pool will only expect to see a full proof of work at very low frequency. Therefore, it
> **cannot obtain statistically significant results** that would indicate an attack."

Quantified: a miner whose expected full-PoW frequency is yearly has an ~8 % chance of finding
a block in any month, so "a pool that rejects miners based on this criterion would **reject
the majority of its honest miners**." Attribution is defeated by variance, not by anonymity.

§VIII-D: "a pool might detect that it is being attacked, but **cannot detect which of its
miners is the attacker. Therefore a pool cannot block or punish withholding miners**." And on
countermeasures: "there is no known silver bullet; all these techniques reduce the pool's
attractiveness and deter miners."

**Sybil-by-churn already defeats registration**: "An attacker can use multiple small block
withholding miners and **replace them frequently**." Registration provides no defense — so
anonymity removes none.

### The one genuine counter-datum: Eligius, May–June 2014

300 BTC lost pre-detection, 200 BTC more blocked. Eyal is explicit that this worked only
because of attacker sloppiness:

> "They have only used **two payout addresses** to collect their payouts, and so it was
> possible for the alert pool manager to **cluster** the attacking miners and obtain a
> statistically significant proof."

Detection succeeded via **payout-address clustering** — aggregating many individually
insignificant per-worker signals into one significant signal. This is the real objection, and
it is narrower than "attribution": **fresh-address-per-payout blinding defeats the only
withholding detection that has ever actually worked in production.**

## Rosenfeld (2011) — PPLNS makes withholding self-defeating, and he already called it undetectable

§6.2.1: under PPLNS "each participant (including the attacker) will lose a portion of his
rewards equal to the attacker's portion of the pool's hashrate" — payout `(1−f)(1−h/H)pB` per
share. Under PPS the operator absorbs it all, gaining `(f − h/H)pB`; "since f is typically
only a few percent, this can easily be negative. This way it is possible to cause
**bankruptcy of the pool**."

His concession, in 2011, with identity fully available: sabotage under PPLNS "can create a
significant loss, but since it is **difficult to detect**, it will likely not cause desertion
of the pool or any other long-term disruptions."

§6.2.2 "lie in wait" is the profitable variant (delay, then focus hashrate on the pool holding
an imminent block); optimal delay `T = ((m−1)/(2m−1))·T₀`, gain per share
`pB(1 + mh/4H₀)`. It is cross-pool and its detection depends on *timing* correlation, not
identity.

§6.2.3 **oblivious shares** — the pool holds a `SecretSeed`, publishes `ExtraHash`, miners
submit anything under `2²⁵⁶/2³²` without knowing which is a block: "They do not know if this
is a valid block because they don't know SecretSeed." Note this is the **inverse** of
self-blinding — it blinds the *miner*, and requires the pool to know *more*, not less.

Score-based systems exist specifically to "detect pool-hoppers and penalize them" — the other
attribution-dependent attack. PPLNS is already structurally hopping-immune, so that cost is
not incurred.

## APoW (arXiv 2601.02496v2, Jan 2026) — the scheme-by-scheme reframing

*Unrefereed preprint; treat conclusions as provisional.*

Frames the problem as structural, not informational: "BWAs exploit a structural limitation of
conventional proof-of-work schemes: while valid solutions are publicly verifiable,
**unsuccessful search effort leaves no auditable trace**"; "most existing pool protocols
**cannot attribute blame** or provide cryptographic evidence of withholding behavior."

Corroborates Eyal: "the absence of a block is not itself evidence of misbehavior …
distinguishing deliberate withholding from statistical fluctuation requires long observation
windows and provides, in the best case, **weak confidence guarantees**."

**The reframing that matters**: BWA exposure is a function of *payout scheme*, not of identity.

| Scheme | BWA resistance | Withholding profitable? |
|---|---|---|
| PPLNS, score-based | **Strong** | No |
| PPS | Weak | Yes |
| FPPS | **Very weak** | Yes |

Reason: "If rewards are independent of block discovery, a withholding miner can preserve their
expected income." Under PPLNS, withholding costs the attacker its own revenue — **so a blind
PPLNS pool needs no attribution to be BWA-safe; the incentive does the work.** Estimates
**25–35 % of current Bitcoin hashrate** sits in PPS/FPPS pools and is therefore economically
susceptible absent extra detection.

Demolishes both textbook countermeasures. Pop quizzes: "a miner receiving a quiz template can
detect that it does not correspond to the current tip of the chain … he may behave honestly,
avoiding detection." Audits: "the pool cannot dedicate much resources to pop-quizzes or audits
without substantial losses … they usually cannot collect cheating evidence before the pool
goes bankrupt" — and audit hashing doesn't contribute to pool hashrate, so "the miner can
rationally choose to ignore the challenge entirely."

On Rosenfeld's oblivious shares: requires a **hard fork**, adds block-broadcast latency versus
SV2 direct broadcast, and "inherently favors centralized pool architectures" because only the
operator can recognize blocks.

Concedes the Sybil exposure: decentralized/anonymous designs' countermeasures — "reputation
systems, payment delays, or centralized monitoring — either reintroduce trust assumptions or
remain vulnerable to **Sybil attacks and strategic churn**."

## Anthony Towns, "Mining pools, stratumv2 and oblivious shares" (bitcoindev, 2024-07-23)

Proposes a hard fork repurposing leading bits of `hashPrevBlock` as `nBitsShareShift` so the
*miner* cannot tell whether a share is a valid block, while the pool can — via information
asymmetry (pool withholds the merkle preimage), not blind signatures.

**The framing argument for why the attribution-privacy gap exists**: without KYC, statistical
withholding analysis "cannot distinguish attackers with multiple low-hashrate identities from
legitimate small miners," forcing pools into one of four buckets — heavy KYC, minimum hashrate
thresholds, full share validation, or vulnerability to withholding. His motivation is
explicitly that withholding "economically advantages centralized, KYC-enforced pools over open
alternatives."

Luke Dashjr (same day) suggests ZK proofs, conceding the tech is immature. Matt Corallo (from
2024-08-13) pushes back that market/payout solutions suffice. Thread ends unresolved.

## Synthesis (this wiki's reasoning, not any source's)

Two structural couplings, neither drawn by any source read:

1. **FinCEN's custody trigger (§5.4) and APoW's BWA-vulnerability ranking select the same set
   of pools.** PPS/FPPS are simultaneously custodial *and* withholding-vulnerable. PPLNS
   coinbase-direct is neither.
2. **The objection is backwards.** Attribution-based detection is statistically hopeless
   (sourced); incentive alignment under PPLNS makes it unnecessary (sourced). The inference
   that a blind PPLNS pool therefore loses nothing real is inference.

Also unsourced and flagged as the top open technical question: whether Recabarren &
Carbunar's mining-cookie share-theft defense survives being rebuilt on a *blinded* commitment
instead of a plaintext username. Their construction's security rests on the username being an
input to the PoW; whether a blinded variant retains the property is **unanalyzed by anyone**.
