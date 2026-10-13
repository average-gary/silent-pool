---
title: Block Withholding (BWH) and FAW
category: concept
created: 2026-05-23
confidence: high
tags: [BWH, FAW, attack, Eyal, Schrijvers, Nash, attribution, detection, eligius, APoW]
volatility: warm
updated: 2026-07-29
summary: "Inter-pool attack. Pools infiltrate each other; infiltrators submit shares but withhold full block solutions to sabotage the victim's payout. Also: why attribution-based detection already fails under full identity, with Eligius 2014 as the one narrow exception."
verified: 2026-07-29
sources:
  - "raw/papers/2026-07-29-withholding-detection-does-not-need-attribution.md"
  - "raw/papers/2026-05-23-eyal-2015-miners-dilemma.md"
  - "raw/papers/2026-05-23-eyal-sirer-2014-selfish-mining.md"
  - "raw/papers/2026-05-23-kwon-2017-faw.md"
  - "raw/papers/2026-05-23-rosenfeld-2011-pool-reward-analysis.md"
  - "raw/papers/2026-05-23-schrijvers-2016-incentive-compatibility.md"
---

# Block Withholding (BWH) and Fork After Withholding (FAW)

Inter-pool attack. Pools infiltrate each other; infiltrators submit shares but withhold full block solutions to sabotage the victim's payout.

## Why it works

Under PPS or FPPS, pool pays out per share regardless of block-find. An attacker infiltrator:
- Submits shares (collects per-share payout from victim pool)
- Discards any full-block solution they find (denies victim the block reward)
- Gives attacker pool a relative hashrate advantage at victim pool's expense

Net effect: attacker drains victim's reward buffer.

## Eyal "Miner's Dilemma" (IEEE S&P 2015)

- *"No attacks"* is **never** a Nash equilibrium with multiple open pools.
- Two-pool (or identical) case: equilibrium is tragedy-of-the-commons; all pools attack, all earn less than baseline.
- **Profitable BWH**: attacker shares attack revenue with own loyal miners → positive-EV under realistic parameters.

## Schrijvers et al. (FC'16)

- Proves **proportional is not IC** (incentive compatible) — miners are incentivized to withhold blocks.
- **PPS is IC** but operator bears variance.
- **PPLNS is IC only under specific parameter regimes** (large-N relative to round length).
- Constructs a novel IC reward function preserving steady-payout property.

## FAW (Kwon et al. CCS 2017)

Fork After Withholding. Strict generalization of BWH.

**Mechanism**: Attacker with hashrate α joins target pool with infiltration τ. On finding an FPoW via infiltration, **withholds it**. If an external honest miner publishes a block, attacker **immediately submits the withheld FPoW** → fork. With probability `c` the attacker's branch wins.

**Profit (Theorem 5.1)**: Always ≥ pure BWH. Equal only when `c=0`.

**Worked numbers** (α = β = 0.2):
- c = 0 (BWH-equivalent): RER 1.14%
- c = 0.5: RER 1.74%
- c = 1: RER 3.75%

Multi-pool with F2Pool-class attacker (~20%): **+56.24%** more reward than BWH (4.63% vs 2.96% RER).

**Resolution of the miner's dilemma (Theorems 7.1-7.2)**: Unlike pure BWH (symmetric prisoners' dilemma), FAW becomes a **"pool size game"** — larger pool wins extra reward at unique Nash equilibrium. **Smaller pool can be net-loser while larger pool stays profitable.** This is the paper's headline novelty and explains why a Foundry-class pool could rationally launch FAW where pure BWH would zero out via mutual sabotage.

**Mitigation**: Two-phase PoW (requires hard fork), decentralized pools (P2Pool, p2poolv2), fork-rate detection. Authors: *"finding a cheap and efficient countermeasure remains an open problem."* **No deployed mitigation as of 2026.**

## Implication for modern schemes

Reward functions cannot be analyzed in isolation. Cross-pool attacks make the system-wide equilibrium worse than any single-pool design predicts.

**The 2024-2026 decentralized schemes sidestep some BWH dynamics** by changing the trust model:

- **eHash (hashpool)**: no internal ledger to manipulate; can't profitably infiltrate a pool that doesn't track per-miner accounts.
- **PPLNS-JD / SLICE**: miners declare their own jobs; infiltrating to withhold is structurally weaker because the attacker isn't choosing the template.
- **p2poolv2**: no operator → no buffer to drain.

These don't *eliminate* BWH (an attacker can still discard solutions found on a victim's mempool), but they remove the attack's monetary surface.

## Important nuance: profit vs incidence

The BWH/FAW/selfish-mining literature is **payout-scheme-agnostic on attacker profit** — formal models assume proportional reward per share and don't differentiate FPPS / PPLNS / TIDES / SLICE. Vulnerability is a property of any scheme that pays for shares without verifying full-PoW reveal.

**But the *incidence* of damage shifts by scheme** (an open observation, not a cited result):

- **FPPS / PPS**: pool operator absorbs the loss (operator pays for shares regardless of whether real blocks land).
- **PPLNS / TIDES / SLICE**: miners absorb the loss (pool only pays when blocks confirm; sabotage = no payout for that round).
- **eHash (hashpool)**: open question — issuance is per share, but redemption is per block.

So FAW *profit to attacker* is scheme-invariant. FAW *who-gets-hurt at the victim pool* is scheme-dependent.

## Does detection actually need attribution?

The reflexive objection to any [[payout-attribution-privacy|attribution-blind pool]] is that
per-miner identity is required to detect withholding. **The withholding literature refutes this in
its strong form: attribution-based detection already fails under full identity.**

Eyal 2015 §II-C, verbatim — note "**registered**":

> "Even if a pool detects that it is under a block withholding attack, it **might not be able to
> detect which of its registered miners are the perpetrators**."

The reason is variance, not anonymity. A small miner sends frequent partials but is expected to
find a full PoW rarely, so the pool "**cannot obtain statistically significant results**." A
miner with a yearly expected block has ~8 % chance of finding one in any month — so "a pool that
rejects miners based on this criterion would **reject the majority of its honest miners**."
§VIII-D: a pool "**cannot detect which of its miners is the attacker. Therefore a pool cannot
block or punish withholding miners.**" And registration provides no defense anyway: "An attacker
can use multiple small block withholding miners and **replace them frequently**."

Rosenfeld already conceded the same in 2011, with identity fully available: sabotage under PPLNS
"can create a significant loss, but since it is **difficult to detect**, it will likely not cause
desertion of the pool."

### The one narrow exception — and it is specifically address clustering

Eligius, May–June 2014: 300 BTC lost pre-detection, 200 BTC more blocked. Eyal is explicit this
worked only because of attacker sloppiness:

> "They have only used **two payout addresses** to collect their payouts, and so it was possible
> for the alert pool manager to **cluster** the attacking miners and obtain a statistically
> significant proof."

So the real objection is narrower than "attribution": **fresh-address-per-payout defeats the only
withholding detection that has ever actually worked in production.** That is a genuine cost of
[[xpub-payout-identity|xpub payout identity]] and of any per-payout rotation scheme — clustering
many individually insignificant per-worker signals into one significant signal is exactly what
rotation prevents.

### The scheme-by-scheme reframing (APoW, arXiv 2601.02496)

*Unrefereed preprint — provisional.* Frames exposure as a function of **payout scheme, not
identity**: "while valid solutions are publicly verifiable, **unsuccessful search effort leaves no
auditable trace**."

| Scheme | BWA resistance | Withholding profitable? |
|---|---|---|
| PPLNS, score-based | **Strong** | No |
| PPS | Weak | Yes |
| FPPS | **Very weak** | Yes |

"If rewards are independent of block discovery, a withholding miner can preserve their expected
income." Under PPLNS withholding costs the attacker its own revenue — **so a blind PPLNS pool
needs no attribution to be BWA-safe; the incentive does the work.** Estimated **25–35 % of current
hashrate** sits in PPS/FPPS and is therefore economically susceptible. APoW also demolishes both
textbook countermeasures: pop quizzes are detectable ("a miner receiving a quiz template can
detect that it does not correspond to the current tip"), and audits cost the pool more than they
save.

Anthony Towns (bitcoindev, 2024-07-23) supplies the reason this gap has political weight: without
KYC, statistical analysis "cannot distinguish attackers with multiple low-hashrate identities from
legitimate small miners," so withholding "economically advantages centralized, KYC-enforced pools
over open alternatives."

**Two couplings, this wiki's inference rather than any source's**: (1) FinCEN's custody trigger and
APoW's vulnerability ranking select the *same* set of pools — PPS/FPPS are simultaneously custodial
and withholding-vulnerable, while PPLNS coinbase-direct is neither. (2) The objection is backwards:
attribution-based detection is statistically hopeless, and incentive alignment under PPLNS makes
it unnecessary. What survives is **share-credit theft**, not withholding — see
[[hashrate-inference-side-channels#BiteCoin and the mining cookie — why identity currently guards share credit|the mining cookie]].

Note also that Rosenfeld's **oblivious shares** (§6.2.3) run the *opposite* direction from
self-blinding: they blind the **miner**, and require the pool to know *more*, not less.

## Sources

- [[../../raw/papers/2026-07-29-withholding-detection-does-not-need-attribution|Withholding detection vs. attribution — Eyal, Rosenfeld, APoW, Towns]]
- [[../../raw/papers/2026-05-23-eyal-2015-miners-dilemma|Eyal IEEE S&P 2015]] (BWH)
- [[../../raw/papers/2026-05-23-kwon-2017-faw|Kwon et al. FAW (CCS 2017)]] — primary
- [[../../raw/papers/2026-05-23-eyal-sirer-2014-selfish-mining|Eyal & Sirer 2014 — Majority is Not Enough]]
- [[../../raw/papers/2026-05-23-schrijvers-2016-incentive-compatibility|Schrijvers et al. FC'16]]
- [[../../raw/papers/2026-05-23-rosenfeld-2011-pool-reward-analysis|Rosenfeld 2011]]

## See also

- [[pool-hopping]]
- [[variance-and-risk-shifting]]
- [[lottery-pplns.md|Lottery-PPLNS (Finder-Bonus Hybrid)]]
- [[selfish-mining.md|Selfish Mining]]
- [[payout-attribution-privacy|Payout Attribution Privacy]] — the threat model this section serves
- [[xpub-payout-identity|xpub Payout Identity]] — what rotation costs on the Eligius axis
- [[../decisions/attribution-retention-tradeoffs|Attribution Retention Tradeoffs]]
- [[blind-share-accounting|Blind Share Accounting — cryptographic primitives for weight without identity]]
- [[coinbase-address-rotation|Coinbase Address Rotation]]

