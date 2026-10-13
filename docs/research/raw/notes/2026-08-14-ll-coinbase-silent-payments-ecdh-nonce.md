---
title: "Lessons Learned: coinbase silent payments — the blocker is the sender pubkey, not the outpoint"
type: notes
source: session
date: 2026-08-14
tags: [lessons-learned, bip352, silent-payments, coinbase, bip34, ecdh, stealth-address, gap-limit, extranonce, forward-secrecy, coinbase-address-rotation, light-client, negative-result, sender-side-key-split, out-of-band-notification, witness-commitment]
lesson_count: 8
category: notes
confidence: high
volatility: warm
verified: 2026-08-14
summary: "BIP 352's coinbase incompatibility decomposes into two independent failures, and the wiki had been treating them as one. The nonce half (constant null outpoint) is fixable and the coinbase is better supplied than an ordinary tx — BIP 34 mandates the block height. The shared-secret half (no input pubkey A) is the real structural blocker. Separating them dissolves the BIP 44 gap-limit collision that had been recorded as a blocker on block-height-as-index: a gap limit binds a BIP 32 child index, never an ECDH nonce. BIP 352 also fuses the sender's ECDH key with its signing key, and the split cannot be derived (a derivable second key is unlocked by the first; a non-derivable one is unusable by its owner) — so it costs transmitted bytes, and a pool's stratum session is the out-of-band channel that lets those bytes be free on-chain."
---

# Lessons Learned: coinbase silent payments — the blocker is the sender pubkey, not the outpoint

> Extracted from a 2026-08-14 Socratic seminar on BIP 352 (`bitcoin/bips@60f5b33`,
> `bip-0352.mediawiki` v1.1.1) exploring whether a coinbase can carry silent payments to hashers,
> with receiver privacy (not payer privacy) as the goal. Eight lessons. Three refine claims already
> in this wiki; two are negative results closing options the wiki had listed as unexplored; three are
> new design findings. Nothing was implemented or tested — all conclusions are primary-source reads
> of BIP 352 / BIP 34 / BIP 141 plus in-session derivation.
>
> Lesson 8 supersedes Lesson 5's variant recommendation: the sender-side ECDH/spend key split that
> BIP 352 fuses is what makes "the pool's fee key is well guarded" and "does ECDH every block"
> compatible again, and a pool's stratum session is the out-of-band channel that makes the split
> free on-chain.

## Lesson 1: A coinbase fails BIP 352 on the *shared secret*, not on the *nonce* — and the wiki had these fused

**Category**: correction
**Context**: Asking whether a coinbase can pay silent-payment addresses. The obvious first observation is that `outpoint_L` is the null outpoint — 32 zero bytes and `vout = 0xFFFFFFFF` — identical in every coinbase ever, so `input_hash = hash_BIP0352/Inputs(outpoint_L || A)` degenerates to a constant.
**Symptom**: That observation is true and *not the blocker*, so reasoning from it leads to the wrong repair. [[../repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|the 2026-07-29 note]] lists the outpoint constant alongside `a = 0` and "A is the point at infinity" as one undifferentiated wall of failure, and [[../../wiki/topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]] compresses it further to "no input private key `a`, so no ECDH shared secret."
**Root cause**: BIP 352 derivation needs two structurally independent things, and they fail for different reasons with different fixes. A shared secret needs a point whose discrete log the payer holds; a nonce needs a value that varies per transaction. The coinbase input supplies **no pubkey at all** — no prevout scriptPubKey, no witness, no scriptSig pubkey, nothing that `get_pubkey_from_input` (`bip-0352/reference.py:37-88`, every branch tests `vin.prevout`) can read. So the eligibility gate at `bip-0352.mediawiki:193` — *"The transaction has at least one input from the Inputs For Shared Secret Derivation list"* — fails **first and independently** of the outpoint issue.
**Fix**: Split the diagnosis. Confirmed the prior note's negative result still holds: `grep -ic coinbase bip-0352.mediawiki` returns **0** across its 524 lines.

| | Ordinary tx | Coinbase |
|---|---|---|
| Sender point `A` | free — input keys are published anyway | **absent** — structural, unfixable within the tx |
| Uniqueness nonce | `outpoint_L` | **free and consensus-mandated** — BIP 34 height |

**Rule**: State BIP 352's coinbase incompatibility as a *missing sender pubkey*, never as a *constant outpoint*. The nonce half is not merely fixable — the coinbase is **better** supplied than an ordinary transaction, because BIP 34 hands you a monotonic counter the whole network already agrees on. Fusing the two failures makes a solvable problem look structural and hides the one that actually is.

## Lesson 2: A gap limit binds a BIP 32 child index, never an ECDH nonce — block-height-as-index is only fatal for descriptors

**Category**: discovery
**Context**: This wiki records Greg Maxwell's block-height-as-index suggestion as blocked. [[../../wiki/topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]]: *"Greg Maxwell's blockheight-as-index is self-synchronizing but collides with BIP 44's gap limit of 20."* [[../repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|The 2026-07-29 note]] is blunter: *"a wallet would need an 800000+ gap limit."* [[../../wiki/concepts/coinbase-address-rotation|Coinbase Address Rotation]] repeats the cost: *"the derivation range becomes as large as the block height, making descriptor recovery scans correspondingly wide."*
**Symptom**: Nothing failed — this is a blocker recorded against the wrong construction, which suppresses a viable one. The gap-limit objection was being carried as a property of *height-as-index* when it is a property of *BIP 32 derivation*.
**Root cause**: The two schemes use the height in categorically different roles.

- **BIP 32 / BIP 380 wildcard descriptor**: height is a **child index into a forward-scanned chain**. Recovery means walking `i = 0, 1, 2, …` and stopping after 20 consecutive misses (BIP 44's *"currently set to 20. If the software hits 20 unused addresses in a row, it expects there are no used addresses beyond this point and stops searching"*). A pool that wins one block in 800,000 leaves 799,999-wide holes. Fatal, exactly as recorded.
- **ECDH / stealth derivation**: height is a **nonce hashed with a shared secret**, occupying the slot `input_hash` fills in `bip-0352.mediawiki:302`. The receiver never scans forward. Given block *H* in hand, they compute the one candidate `P = B_spend + hash(S || H || k)·G` for *that* height and compare against the block's outputs. Unmined heights are never enumerated, so **there is no gap to limit**. Skipped heights cost nothing.

**Fix**: Recorded the distinction and audited the height against the three properties a nonce actually needs — shared (✔ miner knows it at template time, scanner reads it from the block), non-circular (✔ fixed before the coinbase outputs are built), unique (✔ monotonic, consensus-enforced by `bip-0034.mediawiki:23`). Three for three, and denser than `outpoint_L` since it is sequential rather than merely distinct.

Also worth stating explicitly because it reliably confuses: **the nonce need not be secret or unpredictable.** `input_hash` is fully public in BIP 352 too. It is hashed *together with* the ECDH secret, so a publicly predictable height costs nothing.

**Rule**: Before importing a gap-limit objection, check which role the index plays. A **child index** in a derivation chain is gap-limit-bound and sparse usage is fatal; a **nonce** mixed into a shared secret is not bound at all, because the receiver computes the single candidate for the value in front of them instead of scanning forward. Greg Maxwell's height-as-index is fatal under BIP 32/BIP 380 and free under ECDH — same idea, opposite verdict, and this wiki had only the first half.

## Lesson 3: Outputs can never supply a derivation nonce — they are structurally circular, and outpoints are the only consensus-unique identifier available at derivation time

**Category**: pattern
**Context**: Working out whether transaction *outputs* alone could carry a stealth-payment scheme, with and without pre-shared pubkeys. Auditing every field of a transaction against the three nonce properties from Lesson 2.
**Symptom**: Repeated dead ends that all looked different and were the same failure.
**Root cause**: A nonce must be shared, non-circular, and unique. Only one field in a Bitcoin transaction scores three for three.

| Field | Shared | Non-circular | Unique |
|---|---|---|---|
| `outpoint` (input) | ✔ | ✔ | ✔ **by consensus** |
| input pubkeys | ✔ | ✔ | ✘ key reuse |
| output amounts | ✔ | ✔ | ✘ round numbers repeat |
| output scriptPubKeys | ✔ | ✘ | — |
| txid | ✔ | ✘ | — |
| version / locktime / nSequence | ✔ | ✔ | ✘ |

Two structural facts do all the work. **Outputs are circular by construction**: you cannot derive an output script from a value that commits to the output scripts, which eliminates the txid and the script set by causality rather than by spec choice. And **outpoint uniqueness is a consensus guarantee** — no outpoint is ever spent twice, which is what "no double spend" means — with no equivalent anywhere on the output side. Inputs *consume* identifiers that are already globally unique; outputs *create* identifiers that are not unique until the transaction confirms, which is strictly after derivation must happen.

This also settles the pre-shared-pubkey variant. Pre-sharing upgrades the *shared secret* only; it does nothing for the nonce. With a static `A` the fallbacks are (a) a counter, which needs cross-transaction state and desyncs — two sender devices both deriving index 3 collide on one address, a failure BIP 352 is immune to because outpoints differ — or (b) publishing fresh key material on-chain, which is no longer "pre-shared pubkeys alone." Under (a) the receiver reads *nothing* from the transaction, so the scheme collapses into watching a precomputed address list: the pre-share buys exactly what an xpub buys, which is where [[../../wiki/concepts/xpub-payout-identity|xpub Payout Identity]] already sits.

**Rule**: When sourcing a per-transaction derivation nonce, only the input side can supply one. Outputs are circular by construction and non-unique in the fields that aren't. Any "derive from the outputs" design must either import fresh on-chain key material or take on cross-transaction counter state — there is no third option, and the coinbase's BIP 34 height is a special case of the first (consensus supplies the varying value for free).

## Lesson 4: Never derive payout keys from the extranonce — it destroys extranonce rolling

**Category**: gotcha
**Context**: [[../repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|The 2026-07-29 note]] names three candidate tweak substitutes as unexplored: *"block height, prev-hash, extranonce."* Evaluating the third.
**Symptom**: Cryptographically the extranonce looks ideal — it is per-block-attempt unique data already sitting in the coinbase scriptSig, higher entropy than the height, and visible to any scanner reading the block.
**Root cause**: The failure is a mining-engineering one, not a cryptographic one. Miners **roll the extranonce as search space**. Changing it changes the coinbase, hence the merkle root, hence the header being hashed. If output keys were derived from the extranonce, every roll would invalidate every output in the coinbase and force a full template rebuild — putting EC scalar multiplications inside the mining loop and destroying a free search dimension.
**Fix**: Ruled the extranonce out. The height is safe because it is **fixed for the entire template**; a pubkey parked in the scriptSig is safe for the same reason. Prev-hash is also template-fixed and remains open, and is worth keeping as a mix-in for the reorg case in Lesson 5.
**Rule**: Any coinbase-derived key material must be constant across the whole mining template. Anything the miner rolls during proof-of-work search — extranonce first among them — must never be an input to output derivation, or each roll forces a template rebuild plus fresh elliptic-curve math per attempt. Cryptographic suitability and mining-loop suitability are separate audits, and the extranonce passes the first and fails the second.

## Lesson 5: `txOut[0]` as payer for `txOut[1..N]` costs zero bytes and forecloses forward secrecy

**Category**: discovery
**Context**: The session's strongest construction, and it is a **one-payer fan-out**, not a key-reuse trick: `txOut[0]` is the pool's own P2TR fee output — the output that receives the pool's fee — and it is the **payer of record for `txOut[1..N]`, where each `n` is a distinct paid miner**. One sender point `A` (its taproot output key), one nonce (the BIP 34 height), `N` recipients.

**The fan-out shape is a simplification, and it removes BIP 352's nastiest sender-side footgun.** Distinct miners hold distinct scan keys, so `bip-0352.mediawiki:304`'s grouping puts each miner in **its own group of size one, every output at `k = 0`**. There is no `k++` chain across recipients, which means `:319` — *"If an output `Pᵢ` with `i < k` is omitted, the receiver will not be able to find outputs `Pⱼ` where `i < j <= k`"* — has nothing to bite on. No contiguity requirement, no ordering requirement among `txOut[1..N]`, and `K_max` is irrelevant (see lesson 7). A dropped or dust-filtered miner output cannot silently destroy another miner's discoverability. That is a materially safer failure mode than general BIP 352 sending, and it is a property of paying *many distinct scan keys once* rather than *one scan key many times*.

Scanning stays trivial for the same reason: miner *i* computes one candidate `P_i = B_spend,i + hash(input_hash · a · B_scan,i || 0)·G` per block — a single scalar multiplication, or zero if `A` is static and `S_i = b_scan,i · A` is cached. Miner *i* learns nothing about miner *j*, since deriving `P_j` needs `b_scan,j`.
**Symptom**: It closes cleanly on the parts list at **zero marginal bytes**, which no other variant in the session did. The pool knows the discrete log because it is their own payout key; the key is readable straight from the scriptPubKey because P2TR publishes it (unlike P2WPKH/P2PKH/P2SH-P2WPKH, which commit to a hash and only reveal the key when *spent* — the asymmetry BIP 352 exploits). Everything that makes `txOut[0]`-as-`A` bad in an ordinary transaction is neutralized in a coinbase: one party builds it so index 0 is stable, no coordinator or payjoin can reorder it, no BIP 69 sorting applies, and there is no signature at all so `bip-0352.mediawiki:185`'s entire sighash analysis is vacuous — the merkle root commits to everything.
**Root cause**: The defect is custodial, and it lands squarely on the stated goal. **The pool must retain `a` forever** — it is the key to their own money. A payout key gets swept, consolidated, backed up in several places, handed to custodians, and is subpoenable. Anyone holding `a` plus a candidate silent-payment address can recompute `hash_BIP0352/SharedSecret(ser_P(input_hash·a·B_scan) || ser32(k))` and test it against the block's outputs. So a future compromise or legal compulsion of the **pool's treasury key retroactively deanonymizes every hasher payout in every block that key sat at index 0**, with no expiry.

**The fan-out amplifies this by `N`, and that is the part the key-reuse framing hid.** All `N` miner outputs in a block derive from the *same* `a` and the *same* height, so `a` is not a per-payment secret — it is a **single key that unlocks the block's entire payee set at once**. One recovered `a` plus the candidate silent-payment addresses (which a pool necessarily holds, or which an adversary can enumerate from any published set) yields every recipient in that block, not one. The zero-byte property and the blast radius come from the same fact: one sender point serving `N` recipients.

The coupling is inherent, not an implementation detail: the scanner reads the taproot *output* key, so ECDH must use that exact discrete log. You cannot ECDH with an internal key and tweak, cannot use a cold key, and cannot discard a key you need in order to spend.
**Operator counter-argument, and it lands on the theft branch**: *"if `a` is the payout key for the pool itself to collect its pool fees, then it will be well guarded."* Correct, and it retires the weakest form of my objection. A key guarding the pool's own revenue has the strongest possible incentive behind its hygiene — cold storage, HSMs, threshold custody — so "payout keys get handled sloppily, swept through exchange deposit flows, backed up carelessly" is a bad assumption and I should not have leaned on it. **Against extraction and theft, the fee-output variant is well defended by construction.**

Three things survive, and two are stronger than what they replace:

1. **Well-guarded is not erasable, and compulsion doesn't care how good the HSM is.** Forward secrecy means the key *cannot be produced later by anyone under any pressure*; guarding means it is hard to *steal*. This wiki already holds the dispositive case — Lavabit: a warrant for TLS keys covering 400,000 customers, the operator offered targeted code instead, and **the court rejected it**. Hence § Compulsion's rule that *"what is durable is (a) data never collected, which cannot be produced retroactively."* A well-guarded key is collected data.
2. **"Well guarded" and "does ECDH every block" are in direct tension — this is the stronger objection.** The construction needs `a` available at template-construction time every ~10 minutes in a latency- and availability-critical path. That is the opposite of cold storage. And if the treasury key is threshold-held (which is what well-guarded *means* at pool scale), then per-block ECDH becomes a threshold ceremony: `bip-0352.mediawiki:244` requires participants to compute `ECDH = a₁·B_scan + … + a_t·B_scan` collaboratively and notes a DLEQ proof may be needed, while `:38` states there is **no security proof for the collaborative setting** and recommends against it. So the guarding practices that make the premise true are the ones that break the mechanism. You can have a cold treasury key or a per-block ECDH key, not one key that is both.
3. **Good treasury hygiene and good privacy hygiene point opposite directions.** A well-guarded key is *long-lived* — rotation is operationally expensive, so pools don't. Long life means blast radius accumulates: every block that key sat at index 0, times `N` miners each. Frequent rotation would bound it, and frequent rotation is bad treasury practice.

**Fix**: Recommended deriving `A` from a **per-block ephemeral key published in the coinbase scriptSig** instead. It has no spending role, so the pool can **erase `a` the moment the block is found** — hashers still find their outputs from `b_scan·A`. Cost is 33 bytes of non-witness data (132 weight units, ~0.003% of a block) against a consensus scriptSig budget of 2–100 bytes, of which BIP 34's height push takes ~4 and typical extranonce plus pool tags take ~15–40. It fits, but it competes with merged-mining tags for a contested field. Middle path if the bytes matter: keep `A = txOut[0]` but make index 0 a one-time hot key, sweep to cold after the 100-block maturity, then erase — forward secrecy after a ~100-block window instead of never, at the cost of losing key-only retroactive payout audit.

Residual items recorded: even-Y negation is required since scanners read x-only and assume even Y (`bip-0352.mediawiki:299`); index 0 must be P2TR or the scheme silently fails for pools paying fees to P2WPKH/P2SH; the witness commitment does not contend for index 0 because BIP 141 takes the **highest**-index matching output (`bip-0141.mediawiki:78`), so this is a convention you control; and taproot dust at 330 sats excludes small shares regardless.
**Rule**: For ECDH in a payout construction, prefer a key with **no spending role**, so it can be erased once the derivation is done. Reusing a key that must be retained welds every recipient's privacy to the payer's long-term treasury-key custody — which is the same principle as this wiki's *"what is durable is data never collected"* ([[../../wiki/topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]] § Compulsion), applied to receiver privacy: an erasable ECDH key is the cryptographic form of not retaining. Note also the threat model this does *not* change — the pool computed the payouts and knows who got what, so the win is against third-party chain observers only, consistent with *"a pool's attribution knowledge derives from share validation, not from payment."* The `txOut[0]` variant is worse than that baseline in one specific way: the pool cannot *choose* to blind itself retroactively even if it wants to.

## Lesson 6: Scoping a stealth scheme to the coinbase makes light-client scanning cheap *and* trustless

**Category**: discovery
**Context**: Costing out what a hasher must download to detect payouts, versus BIP 352's general-transaction case.
**Symptom**: BIP 352's light-client story is its weakest part and the BIP says so. `bip-0352.mediawiki:468` puts the tweak data at **7–12 kB/block, 30–50 MB/month** at 2024 taproot adoption, and `:466` concedes the sourcing has no trustless story: *"It is still an open question as to how Bob can source the 33 bytes per transaction in a trustless manner."*
**Root cause**: The general case needs 33 bytes per *eligible transaction* per block, and nothing proves those bytes are the real input-key sum without the block. A coinbase-scoped scheme needs 33 bytes per *block* — and the coinbase's position in the merkle tree is fixed, so a short branch proves it against the header.
**Fix**: Estimated the per-block cost as block header (80 B) + merkle branch to the coinbase (~12 hashes at ~3,500 tx, ~384 B) + `A` (33 B) ≈ **~500 B/block, ~2 MB/month**, verifiable against the header. Roughly an order of magnitude below the general case, and it removes the trust assumption rather than relocating it.

**Confidence caveat**: the ~500 B / ~2 MB figures are in-session arithmetic from the components listed, not measured and not sourced. The 7–12 kB/block comparison figure is BIP 352's own (`:468`, citing analysis through July 2024) and is the sound half of the comparison. The claim that this *closes* `:466` is my reasoning, not a cited result — treat as medium confidence pending someone checking whether a merkle-branch-only client inherits BIP 158's security assumptions cleanly.
**Rule**: A protocol scoped to the coinbase gets a light-client story a general-transaction protocol cannot: the coinbase is transaction 0, so its merkle branch is short and its position needs no trust, making per-block key material self-proving against the header. When an on-chain protocol's hard part is trustless data sourcing, check whether restricting it to the coinbase dissolves the problem instead of optimizing it.

## Lesson 7: `K_max = 2323` caps outputs per scan key, not payees per block — the binding limit is block weight

**Category**: correction
**Context**: Checking whether BIP 352's group cap constrains how many hashers a pool could pay in one coinbase.
**Symptom**: [[../repos/2026-07-29-bip352-silent-payments-coinbase-incompatibility|The 2026-07-29 note]] records *"`K_max = 2323` caps scan-group size,"* which is accurate but reads, in a wiki that repeatedly discusses coinbase output-count ceilings (DATUM ~380–530, Avalon username truncation at 63, Whatsminer overflow past 127), as though it were a payee cap. It is not.
**Root cause**: `bip-0352.mediawiki:305` groups recipients **by `B_scan`** and caps each group at `K_max`. Distinct hashers have distinct scan keys, so each occupies its own group of size 1. The cap binds only when one recipient takes many outputs in one transaction. The number that *does* bind is block weight — and the BIP's own rationale for choosing 2323 is that it is the maximum P2TR output count in a 100 kvB transaction, so the two limits coincide numerically for unrelated reasons.
**Fix**: Restated the ceiling as weight-derived. A pool paying more than roughly 2,300 hashers per block needs batching regardless of BIP 352, which is the same conclusion the wiki already reaches from firmware and DATUM limits — those bite far earlier.
**Rule**: `K_max` is a per-scan-key output cap and is non-binding across distinct payees; quote it as such. When a privacy-protocol limit and a consensus limit share a number, check whether one was *derived from* the other before treating them as independent constraints.

## Lesson 8: BIP 352 fuses the sender's ECDH key with its signing key, and splitting them cannot be free — but a pool has the out-of-band channel that makes it free anyway

**Category**: discovery
**Context**: Asked directly why the *receiver* gets a scan/spend split and whether the same math works on the sender side. It does, and the reason the BIP doesn't offer it turns out to be the same reason the fee-output variant is stuck.

**The receiver's split works because `B_spend` enters additively as a public point.** In `P_k = B_spend + t_k·G` with `t_k = H(ecdh ‖ k)`, the online scanner computes candidate outputs from `B_spend` and `b_scan` alone, and the cold signer computes `b_spend + t_k` from `b_spend` and a handed-over `t_k` alone. Two disjoint secrets, neither sufficient, one output. `bip-0352.mediawiki:109` gives the motive: `b` would otherwise have to be online.

**The sender's key carries two roles and the BIP fuses them deliberately.** `a` is both the **ECDH participant** (`ecdh = input_hash·a·B_scan`) and the **signature authority** over the inputs. `:244` is explicit that these are one key — *"The sender uses the private key corresponding to the taproot output key (i.e. the tweaked private key)"* — and `:99`'s footnote gives the reason: the receiver must reconstruct `A` **from chain data alone**, and the only pubkeys a transaction exposes are its input pubkeys, which by definition *are* the spending keys. `A` therefore costs zero marginal bytes and light-client scanning is 33 B/tx (`:466`). The sender-side split is not unimplemented; it is structurally excluded by the discovery mechanism.

**The split math itself is sound.** Introduce a dedicated send-scan key `a_send`, `A_send = a_send·G`, for ECDH only, keeping `a_1…a_n` for signing only: sender computes `ecdh = input_hash·a_send·B_scan`, receiver computes `ecdh = input_hash·b_scan·A_send`. DH symmetry carries it. One thing must move with the key: `input_hash` must commit to `A_send` rather than `A`, because the `why_include_A` attack at `:92` — choosing `a′ = input_hash·a / input_hash′` to replay a shared secret and force address reuse — transposes verbatim to `a_send`, so the commitment has to bind whichever key is actually in the product.

**The security asymmetry is exactly parallel to the receiver's, and this is the point.** `a_send` computes `t_k`, but spending still requires `b_spend + t_k`. So compromise of the online sender key costs **privacy, never money** — whereas in the fee-output variant `a` *is* money. That is the whole reason to want the split.

**Symptom**: The natural next question — can `A_send` be *derived* from `A` so the split costs nothing? — has a clean negative answer, and it is worth stating as a dichotomy because it forecloses a whole family of "clever" designs.

**Root cause**: The receiver needs `A_send = f(A, public data)` evaluable in the group, and the sender needs `dlog(A_send)`. In the generic group model any group element efficiently computable from `A` and public points is a linear combination of them, so:

- **`f` group-linear** — `A_send = s·A + T` for public `s`, `T` — gives `a_send = s·a + t`. The sender knows it, and so does **anyone who ever learns `a`**. Forward secrecy and delegation both evaporate; the split is cosmetic.
- **`f` not group-linear** — e.g. hash-to-curve over `ser(A) ‖ h` — leaves *nobody* knowing the dlog, sender included. Unusable.

No third case. **`a_send` must be independent key material, and independent key material must be transmitted.** The 33 bytes are the price of the split, not an unoptimized implementation.

**This names the real asymmetry, and it is not cryptographic.** The receiver's split is free because it rides an **out-of-band channel** — the silent payment address, 66 bytes of pubkey material, paid once, off-chain. A general sender has no channel to a scanner it cannot even identify, so its key material must go on-chain, per transaction. Channel availability, not mathematics, is what gives the receiver a split and denies the sender one.

**Fix**: A pool is **not anonymous to its payees** — it holds a stratum/SV2 session with every one of them — so it has the channel a general sender lacks, and can take the sender-side split at **zero on-chain bytes**. Batch of ephemeral ECDH keys indexed by block height, pubkey list served over stratum or as a static file, each secret erased once its payout matures. Forward secret for past blocks, and no funds ever depend on those keys. Two constraints that must be recorded with it:

- **The list cannot be compressed to a single published seed.** `A_H = A_0 + H(A_0 ‖ H)·G` is precisely the group-linear case above, so `a_0` unlocks every epoch. Irreducibly 33 B per epoch; it merely need not be *on-chain*. Batch size is the retention knob — 1,000 blocks ≈ 33 kB and ≈ one week of *prospective* exposure, zero retrospective.
- **The pubkey list is a linkability trust surface, not a theft one.** A pool serving different lists to different miners partitions them, undetectably from chain data. Needs one public copy or a committed batch root.

**Two corrections to the on-chain variants this supersedes.** First, the 33-byte on-chain fallback is cheaper than the scriptSig row claimed: `bip-0141.mediawiki:74` allows *"39th byte onwards: Optional data with no consensus meaning"* on the witness-commitment output, which every segwit block already carries — so `A_send` costs exactly 33 B / 132 WU appended there, with **no new output and no contention** against BIP 34's height, the extranonce, and pool tags for the 2–100 byte scriptSig budget. Second, `:480-486` already sanctions skipping discovery entirely via an encrypted out-of-band notification carrying `H(ecdh ‖ k)`, the outpoint and the amount — but `:495` marks the trap: a key that was not *actually* derived per the protocol breaks seed restore, *"unsafe — no implementation should ever allow this."* Notification is an optimization layered over a scannable construction, never a substitute for one, so the scanning path must exist even where the channel is perfect.

**Rule**: When a stealth protocol gives one party a key split and not the other, check whether the obstruction is cryptographic or merely a missing out-of-band channel — the receiver's scan/spend split is free only because the address carries it, and any party that already has a session with its counterparty (a pool with its miners, a merchant with its customers) can claim the same split on the sender side at zero on-chain cost. Corollary, worth applying before designing any "free" key separation: a second key whose public half is derivable from the first is unlocked by the first, and one whose public half is not derivable is unusable by its own owner — so key separation always costs transmitted bytes, and the only real design freedom is *which channel* pays.
