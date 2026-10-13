---
title: "Lessons Learned: who controls the SV2 coinbase, and why address rotation doesn't hide amounts"
type: lessons-learned
source: session
date: 2026-08-24
tags: [lessons-learned, bip352, silent-payments, coinbase, stratum-v2, job-declaration, jdp, sv2-extensions, amount-linkability, pool-tag, spec-citation]
lesson_count: 7
category: notes
confidence: high
summary: "The BIP 352 nonce protects the receiver, not the sender, so a public pool doesn't retire it; who controls the SV2 coinbase has no single answer (it varies by JD mode and by extension); you structurally cannot hide coinbase outputs from the miner who hashes them; and the ceiling on the whole SPc idea is amount correlation, which address rotation does not touch and which was already documented in this wiki."
---

# Lessons Learned: who controls the SV2 coinbase, and why address rotation doesn't hide amounts

> Extracted from session on 2026-08-24, reviewing a blog post drafting the coinbase silent-payments
> variant (SPc) and investigating `sv2-spec` PR #203. Seven lessons: two corrections to claims this
> wiki and the published explainer had made, one structural impossibility result, and the process
> lessons behind finding them.

**Two of these are corrections to my own published output, and both came from the user pushing back
on a claim rather than from a test failing.** That is the pattern worth noting up front: the test
suite for this construction is green and has been throughout, because none of these are arithmetic
errors. They are errors about *what a mechanism is for* and *what a leak actually leaks* — the class
of mistake that executable vectors cannot catch, and that only argument surfaces.

## Lesson 1: The BIP 352 nonce protects the receiver, so a public sender does not retire it

**Category**: correction
**Context**: The explainer's section on why silent payments need a nonce read: *"If the pool pays the
same miner twice using the same `a`, then `s` is the same… So the pool mixes in something that never
repeats."* The user challenged it, having read BIP 352's **"Using all inputs"** section, and inferred
that the nonce might be unnecessary for SPc "since there isn't a privacy concern for the sender
because the pool is very public."
**Symptom**: No failure. A published page that invited exactly the wrong conclusion.
**Root cause**: Two adjacent BIP sections do different jobs, and the page's framing conflated them.
**"Using all inputs"** (`bip-0352.mediawiki:97`) is about *which pubkeys feed the ECDH* — the sum of
input pubkeys rather than one — and its three rationales are scanning cost, light-client feasibility,
and **sender** privacy in CoinJoin (*"If Alice uses a random input to create the output for Bob, this
necessarily reveals to Bob which input Alice has control of"*). All three genuinely are void for a
pool: a coinbase has no inputs, and a pool is one known entity with no anonymity set to protect. The
user's read of that section was correct. But the **nonce** comes from the section immediately above
it, **"Preventing address reuse"** (`:88`), whose beneficiary is the *receiver*: *"If Alice were to
use a different UTXO from the same public key A for a subsequent payment to Bob, she would end up
deriving the same destinations `P_i`."* The `why_smallest_outpoint` footnote states the purpose
flatly — *"a deterministic nonce that ensures that a unique address is generated each time."* The
address at risk of reuse is the **miner's**, so "the pool is public" does not bear on it.
**Fix**: Rewrote the section to say whose privacy is at stake, and added that `input_hash` does a
*second* job — committing to `A` — which is a separate requirement from the nonce. Also recorded the
substantive answer to the underlying question: rotating `a_send` per block and mixing in the height
are **overlapping** sources of freshness and either alone suffices, so the height is redundant for
that purpose. It is kept because it is free (already in every coinbase) and unique **by consensus**
rather than contingent on the pool rotating correctly — a pool reusing one `a_send` across two blocks
(stale key state after a restart, failover to a spare with an old batch, an off-by-one batch index)
would silently reuse every payout address for every miner with no on-chain signal.
**Rule**: When a mechanism exists to prevent a harm, name **whose** harm it is before reasoning about
whether a party's threat model retires it. "This party has no privacy to protect" is only an argument
against mechanisms that protect *that* party.

## Lesson 2: The anti-grinding property comes from committing to `A`, not from the nonce — and it is easy to wire to the wrong cause

**Category**: gotcha
**Context**: Reviewing three successive drafts of the blog post's `input_hash` paragraph after asking
the author to add the `why_include_A` requirement.
**Symptom**: Each draft stated both facts and attributed them to the wrong mechanism. Draft 2: *"By
using height as the nonce… preventing the pool from grinding malicious a_send."* Draft 3: *"By adding
height into the commitment, it prevents the pool from grinding a malicious a_send."*
**Root cause**: The two ingredients of `input_hash = hash(nonce ‖ A)` are counter-intuitive in their
roles, and the nonce is the more salient one. The attack (BIP 352 footnote 3, `why_include_A`) needs
the attacker to know `input_hash′` *before* choosing `a′ = input_hash·a / input_hash′`. If the hash
covers only the nonce, the attacker picks any nonce, computes the hash, and solves directly — the
freely-choosable nonce is the attack's **lever, not its defense**. Committing to `A` breaks it
because the hash then depends on the very key being chosen, turning a direct solve into a fixed-point
search.
**Fix**: Supplied wording that separates the two explicitly — height → freshness, `A_send`
commitment → anti-grinding — and noted the pool-specific sharpening: a pool picks a fresh `a_send`
every block anyway, so it already has exactly the freedom the attack needs.
**Rule**: In a hash that commits to several things, state each ingredient's job separately. When one
ingredient is a freely-chosen value and another is a key, the freely-chosen one is usually the
attacker's degree of freedom rather than the protection.

## Lesson 3: "Who controls the SV2 coinbase" has no single answer — it varies by JD mode and by extension

**Category**: correction
**Context**: Asked whether a pool can place `A_send` in the coinbase under Job Declaration. Answered
too bluntly, then had to revise twice as the user pushed.
**Symptom**: First answer — "under JD the JDC builds the coinbase, so the pool cannot place
`A_send`" — was true of base JDP but presented as though it settled the question. The user objected:
*"there must be an exchange of what/who the JDC should be paying, otherwise the pool would not want
to accept the shares."*
**Root cause**: Three distinct layers, each with a different answer, and I had checked only one:

1. **Base JDP negotiates the pool's own payout, not its payees.**
   `AllocateMiningJobToken.Success.coinbase_tx_outputs` (§6.4.3) has the pool reserve the **first**
   output as the pool payout output; JDC MUST allocate sats into it; any further pool-added outputs
   MUST be **0-value**. So value flows to one pool output and per-miner distribution happens
   elsewhere. The user's intuition that something is negotiated was right; the direction is inverted
   from what SPc needs.
2. **PR #203 reverses it.** The proposed non-custodial-payouts extension (`0x0003`) has the pool
   publish `SetPayoutDistribution` with per-miner outputs and *relative weights*, absolute amounts
   computed once template revenue `T` is known. Under it the pool **does** dictate per-miner coinbase
   outputs, which invalidates the base-JDP answer entirely.
3. **The two JD modes differ on the scriptSig, and one of them cannot host a tag at all.**
   Full-Template (`DeclareMiningJob` §6.4.4, JDC → JDS) carries `coinbase_tx_prefix` +
   `coinbase_tx_suffix`, so the pool **sees** the whole scriptSig and can reject a declaration.
   Coinbase-only (`SetCustomMiningJob` §5.3.18, JDC → Pool) carries only `coinbase_prefix` (≤8 B,
   the BIP 34 height), `nSequence`, `coinbase_tx_outputs`, `locktime`, `merkle_path` — **no arbitrary
   scriptSig bytes**. The pool reconstructs the scriptSig as `coinbase_prefix ‖ extranonce`, so it can
   neither specify nor validate a tag. The pool-tag concept does not exist in that mode.

Neither mode has a pool → JDC field for scriptSig content, so the pool has **approval** authority in
Full-Template and **specification** authority in neither.
**Fix**: Concluded that the `OP_RETURN` in PR #203's `additional_outputs` is not a fallback but the
only carrier working across both JD modes with no new fields — and that it costs ~43 bytes of real
block space (~172 WU), so the "the bytes are pre-paid, it costs nothing" argument holds in the
pool-controlled-template case and **not** under JD.
**Rule**: Before answering "who controls X" in a layered protocol, enumerate the layers (base spec,
proposed extensions, per-mode message variants) and check the actual field list of the specific
message in each. A confident answer from one layer will be wrong at another.

## Lesson 4: You cannot hide coinbase outputs from the miner who hashes them — the suffix is unhashable-away

**Category**: discovery
**Context**: The user proposed modifying PR #203 to give the pool more authority over
`coinbase_tx_prefix`, to keep the scriptSig carrier and avoid disclosing payouts to peers.
**Root cause / result**: Authority reallocation cannot fix disclosure, and the reason is structural to
mining rather than a spec choice. Coinbase serialization order is
`version | in-count | prevout | scriptSig-len | scriptSig(height ‖ extranonce ‖ …) | sequence |
out-count | outputs | locktime`. The extranonce sits **inside** the scriptSig, so every byte after it
must be re-hashed on each roll, which means the miner must possess those bytes in the clear. SHA-256
is Merkle–Damgård, so the **prefix** could in principle be collapsed to a midstate — but the
**suffix** cannot, because it follows the varying bytes. And the outputs live in the suffix.

**The bytes you would most want to withhold are precisely the ones that structurally must be handed
over.** The only real dial is the *number of payout slots*, which PR #203 §9.2 already names as the
mitigation; at one slot there is no cross-miner disclosure and at N slots there is N-way disclosure.
That is a leak-versus-non-custodiality tradeoff inherent to paying many miners in a coinbase.
**Rule**: You cannot give a party a transaction to mine without giving them the transaction. Any
scheme that hides transaction contents from the miner constructing it is impossible for everything
serialized after the rolled bytes — check where your secret sits relative to the extranonce before
designing around it.

## Lesson 5: Assess a leak by what it adds beyond public data, not by what it contains

**Category**: correction
**Context**: I claimed PR #203's distribution "kills forward secrecy" for SPc, because
`SetPayoutDistribution` goes to every JDC and a retaining peer would hold the payout mapping without
needing `a_send`.
**Symptom**: Overstated, and I had to walk it back the next turn — after the user had already been
given it as a reason to reframe the blog post.
**Root cause**: I reasoned from "the distribution contains payout scripts" to "the distribution
contains the mapping," which does not follow. `SetPayoutDistribution` carries `(script, weight)`
pairs and **no identities**; the script → miner mapping lives in the pool's accounting DB, so erasing
that still destroys the identity link and JDCs never had it. Worse for my claim, the weights are
nearly redundant with public data: `amount[i] = floor(weight[i]·T/W)` means shares are recoverable
from the on-chain amounts. What a retaining JDC genuinely gains over a chain observer is narrower:
distributions for blocks **nobody won**, and **sub-dust miners** pruned from the coinbase and
therefore invisible on-chain.
**Rule**: When judging a disclosure, compute the delta against what is already public, and check
whether the "extra" is derivable from public values. A leak that restates public data is not a leak;
the real find is usually the residue that is *not* derivable — here, the unfound-block snapshots and
the sub-dust set.

## Lesson 6: Address rotation and amount hiding are orthogonal channels — and this wiki already said so

**Category**: discovery
**Context**: Asked for "any huge gotchas" in the SPc design, with the premise that "since mining is
interactive, we can mitigate any of the issues presented with engineering."
**Root cause**: Four of the five frictions raised *are* engineering-solvable (grace-window height
mismatch → scan a small window of candidate heights; per-height distribution refresh; the JD carrier;
pre-block payout verification via the existing encrypted session). But the largest limitation is not
in that class and is not addressed by SPc at all: **SPc rotates addresses and leaves amounts in
cleartext.** A hashrate provider with a stable share receives a proportionally stable share of every
payout, so an observer computes `amount_i / T` from public data and obtains a fingerprint that
persists across blocks **without any address ever repeating**. Hashrate does not move fast and pool
hashrate is heavy-tailed, so the largest providers — those with the most at stake — are the easiest to
track.

**This was already documented in this wiki** at
[[../../wiki/concepts/coinbase-amount-linkability|Coinbase Amount Linkability]] § *"The ceiling on
what amount-hiding can achieve"*, citing WabiSabi's honest bound: anonymity only *"apart from what is
already deducible given the public amounts visible on the Bitcoin blockchain."* I reached for it only
after being asked for gotchas, having spent a week on the address channel without checking whether
the amount channel was already recorded as the ceiling.
**Rule**: Address unlinkability and amount unlinkability are independent channels; closing one leaves
the other fully open, and for recurring proportional payouts the amount channel is the stronger of
the two. Before treating a limitation as newly found, grep the existing notes — a documented ceiling
on the general problem still binds a new special case.

## Lesson 7: Find the framing where a protocol change is useful without your proposal

**Category**: pattern
**Context**: Deciding how to get `A_send` into a JD-declared coinbase, given that no message lets the
pool specify scriptSig content. The user proposed a separate small extension for the pool tag,
decoupled from the payout distribution: *"we largely needed that for the multiple payees in a
coinbase, not for the pool tag piece."*
**Root cause / discovery**: The decomposition is correct — outputs-and-amounts and scriptSig-content
are unrelated concerns — but the stronger point is about *how to pitch it*. A pool-required coinbase
tag has **standalone justification with no reference to silent payments at all**: today a JD-declared
block carries whatever tag the JDC chose, so a pool running JD loses its coinbase attribution
entirely, and coinbase tags are how pool hashrate share is publicly measured. Pitched as "pools want
their tag in JD-declared blocks," the extension needs no privacy argument bought first, and SPc
becomes one *consumer* of the mechanism rather than its justification.
**Fix**: Recommended raising it as a comment on #203 first ("this is orthogonal, should it be
separate?") rather than writing a full spec doc, to get a cheap read on maintainer appetite while
`plebhash` is actively iterating.
**Rule**: When a proposal needs a protocol change, look for the framing in which the change is
valuable to people who do not care about your proposal. It converts "help my scheme" into a
standalone gap, and it decouples your idea's fate from the extension's.
