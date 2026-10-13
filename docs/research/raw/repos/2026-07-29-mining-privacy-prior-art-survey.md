---
title: "Attribution privacy in existing mining designs — hashpool, Braidpool, Radpool, p2poolv2, SV2 ext 0x0002, NUT PR #293"
source: https://github.com/vnprc/hashpool/blob/master/docs/poisson-proof-consolidation-plan.md
supporting_sources:
  - https://github.com/cashubtc/nuts/pull/293
  - https://github.com/braidpool/braidpool/blob/main/docs/braidpool_spec.md
  - https://github.com/pool2win/p2pool-v2
  - https://github.com/pool2win/radpool-design
  - https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0002-worker-specific-hashrate-tracking.md
  - https://github.com/stratum-mining/sv2-spec/blob/main/06-Job-Declaration-Protocol.md
  - https://delvingbitcoin.org/t/ecash-tides-using-cashu-and-stratum-v2/870
type: repos
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [hashpool, ehash, cashu, nut-293, braidpool, radpool, p2poolv2, sv2-extension-0x0002, datum, attribution, privacy, negative-result, primary]
summary: "Code- and spec-level audit of the attribution question across every design in this topic. The headline: **no mining system anywhere is blind to share attribution**, and the one that claims to be says so itself in its own docs."
---

# What each existing mining design still leaks about attribution

Code- and spec-level audit of the attribution question across every design in this topic. The
headline: **no mining system anywhere is blind to share attribution**, and the one that claims
to be says so itself in its own docs.

| System | Share→miner link visible to operator? | What the operator/network still learns | Privacy claimed? |
|---|---|---|---|
| **hashpool / eHash** | **Weakest link of any design — but not zero** | Mint sees per-share `header_hash`, exact difficulty (via `amount = 2^(diff − keyset_min_diff)`), timestamp, Noise/TCP peer, and the full swap graph (input `Y` values + output amounts + timing) | Yes, explicitly: "accountless", "the pool cannot link mining activity to identities" |
| **Ecash TIDES** (dbtc #870) | Yes, deliberately | Full share log + blinded sigs **published** at block-found for Proof-of-Liabilities | Yes, goal #3 |
| **Braidpool** | Yes, maximally | `BraidpoolMetadata` — committed in PoW, gossiped to all peers, retained forever — contains `payout_address` **and `miner IP`** | **None.** Zero occurrences of "privacy" in the spec |
| **p2pool / p2poolv2** | Yes, maximally | `ShareCommitment.miner_bitcoin_address` committed into the coinbase and gossiped; store keeps a `UserIndex` (btcaddress → user_id) | None; transparency is the design |
| **Radpool** | Yes, and **fanned out to N operators** | MSP knows username + hashrate + payout pubkey; "Verifiable Share Ownership" *requires* broadcasting `username` + `sequence_no` with every `mining.submit` syndicate-wide | None |
| **DATUM (OCEAN)** | Yes | Template contents to be blinded; attribution untouched. Still requires "the unique identifier provided by the pool" in every submitted template | Template privacy only |
| **SV2 Job Declaration** | Yes | Coinbase-only mode preserves "the privacy around the miner's mempool" — nothing about who submitted | Template/mempool privacy only |
| **BMAX / p2share** | Yes, by construction | Deterministic winner selection `q = sha256(...) mod S_k` **requires** a public share register | None |

## hashpool — the project's own adversarial analysis

`docs/poisson-proof-consolidation-plan.md` contains the only rigorous privacy analysis found in
any mining project. Key admission:

> "Cashu's blinding prevents linking a proof to the user who holds it. It does **not** prevent
> temporal correlation or batch fingerprinting by the mint. Since the hashpool mint issues all
> proofs, it knows when every proof was created and can observe when they are consumed."

Observables on every swap: exact input proof `Y` values, output amounts, timestamps, and
creation timestamps of each input. Consolidation strategies ranked worst→best: fixed interval →
fixed threshold (`consolidation_time ≈ threshold / shares_per_hour`) → uniform random →
**Poisson** (memoryless, `interval = -ln(U) × mean`) + random subset selection.

**The decisive limitation, stated by the project itself:**

> "the hashpool faucet wallet is the only entity swapping `hash`-denomination proofs at this
> mint. The mint knows every proof it ever issued in this unit, so all consolidation operations
> are **trivially linkable to the same wallet regardless of timing strategy**."

Described as "forward-looking" — i.e. aspirational, not achieved.

`docs/SETTLEMENT_DESIGN.md` (2026-05-13) is worse for attribution: the on-chain payout path
requires the miner to hand the mint a **cleartext `payout_address`** in an accumulating melt
quote, and the pool queries the mint for "accumulated quote balances per payout address" to
build the coinbase. **Attribution is fully restored for anyone opting into on-chain payout.**

(Note: `EHASH_PROTOCOL.md` has moved to `docs/archive/`; the widely-linked
`master/EHASH_PROTOCOL.md` URL 404s.)

## NUT-XX Mining Share Payment Method (cashubtc/nuts PR #293) — opened, then closed

Author vnprc, opened 2025-10-10, **closed 2026-03-09** by collaborator `thesimplekid` —
rationale: incomplete/inactive; experimental payment methods should first prove adoption via CDK
custom units. **There is no accepted spec.**

The most precise primary statement of the difficulty→denomination mapping anywhere:

> `amount` "**MUST** equal `2^(share difficulty - keyset minimum difficulty)`"

where share difficulty = "number of leading zero bits in the 256-bit `header_hash`."

The mint is handed `header_hash` (32-byte hex) as the payment identifier and "**MUST** treat
`header_hash` values as unique payment identifiers and **MUST** reject duplicate shares" —
i.e. **the mint necessarily retains a per-share fingerprint.** Unit format
`"HASH-POOL1-EPOCH42"` scopes tokens to pool+epoch, partitioning the anonymity set by epoch.
`min_amount`/`max_amount` encode a difficulty window (0–255 leading-zero bits); over-target
shares "MUST be accepted but paid out only up to the reward implied by `max_amount`."

Only privacy text in the whole spec: wallets "**SHOULD** generate a fresh NUT-20 key pair per
mining-share quote to avoid linking multiple quotes."

## Ecash TIDES (delvingbitcoin #870) — the privacy-vs-multi-redemption tradeoff

35 posts, 2024-05-15 → 2025-01-08. Design is intentionally publish-everything: "When a block is
found, a full list of shares and associated blinded signatures is published by the pool."

**Post #32 (vnprc, 2025-01-07) is the landmark**, stating the core impossibility:

> "these tokens are inherently unlinkable to the underlying collateral… You can do multiple
> redemptions by linking the tokens to the mining share, but **in the process you destroy the
> privacy properties of ecash**."

Introduces epoch/keyset rotation as an "arrow of time," and a `keyset_id` coinbase commitment
which "prevents miners from submitting shares to multiple pools." Goal #2 is "a new FOSS
self-hostable KYC-free bitcoin onramp." Also floats coinbase-output markets as "a fundamentally
more private manner than coinjoin services because coinbase outputs have no on-chain transaction
history."

Calle (#8, #13) endorses miner-chosen amounts over "let the mint decide the values of blind
signatures after the fact" — confirming the amount is disclosed at submission time.
1440000bytes raises the custodial/regulatory objection (resembles "a custodial mixer that
provides a token on deposit," NUT-16 KYC risk). **mcelrath (#34)** counters from Braidpool that
shares need full-block data to validate, so use Deterministic Block Templates where "the data
within is entirely public" — an **explicit rejection of privacy in favor of verifiability**.

## Braidpool — payout address *and miner IP* committed in the PoW

`BraidpoolMetadata`, hashed into an `OP_RETURN` coinbase output and thus **committed in the
PoW**, contains `payout_address` ("P2TR address for this miner's payout"), `comm_pubkey`, and
**`miner IP` ("IP address of this miner")**. Every bead is gossiped to every peer and retained:
"within a braid we keep *all* beads with a valid PoW." Attribution is permanent and global.

`grep -ci privacy docs/general_considerations.md` → **0**. The word does not appear. The threat
model is censorship resistance. The only "private" usages concern private OTC contracts for
share derivatives — financial privacy between counterparties, not attribution privacy.

Also the empirical verdict on many-output coinbases: "In p2pool this UHPO set was placed
directly in the coinbase of every block, resulting in a large number of very small payments to
hashers… the large coinbase with small outputs competed for block space with fee-paying
transactions." Braidpool's coinbase has **two** outputs.

## p2poolv2 — attribution published to everyone

`ShareCommitment` ("created by miner and embedded in the bitcoin coinbase") has
`pub miner_bitcoin_address: Address` — "Bitcoin address identifying the miner mining the share"
— and `hash()` commits its `script_pubkey` into the PoW-committed structure. Store schema:
`Share` CF keyed on `n_time ‖ user_id ‖ seq`; `SimplePplnsShare{user_id, difficulty, btcaddress,
workername, n_time, job_id, extranonce2}`; a `User` registry and a **`UserIndex` mapping
btcaddress → user_id**. `grep privacy` → **0 results**.

Publishing all shares means attribution is available to *everyone*, not just the pool —
**strictly worse than a centralized pool on this axis**, in exchange for removing custody.

## Radpool — attribution multiplied, not reduced

Thread by `jungly` 2024-11-16; design repo last updated 2024-11-26 (likely dormant).
`grep -i privacy|anonym|KYC` across the LaTeX paper → **zero hits**.

"Verifiable Share Ownership" makes attribution *mandatory and replicated*: `jobID` = hash(miner
`username` ‖ `sequence_no` ‖ MSP pubkey), and "To prove the ownership of `mining.submit`
messages the MSP provides the `username`, `sequence_no` with each broadcast" to the entire
syndicate. Registration: miner submits payout pubkey `m_pk` plus username/password; syndicate
runs a per-miner DKG. Nonce optimization broadcasts `<MSP id, Miner username, Sequence number,
R>` syndicate-wide. **FROST is threshold custody, never privacy** — same for the DLCs.

mcelrath vs. jungly (Nov 27 – Dec 10 2024): "If you don't have consensus on who has which
shares, you can't possibly pay everyone accordingly" — the tension with attribution privacy,
stated directly.

## SV2 — the only attribution extension moves the wrong way

`06-Job-Declaration-Protocol.md`, verbatim: "Pool + JDS operating under Coinbase-only mode do
not require to ever know which transactions are included in the miner template, **preserving
the privacy around the miner's mempool**." Scope is the mempool, full stop. §6.3.3's comparison
table enumerates exactly three privacy-relevant properties — knowledge of fee revenue,
knowledge of txdata, ability to broadcast — and **no row for share attribution**.

`02-Design-Goals.md` has the only privacy line in the design goals: "Optional telemetry data…
without sacrificing the privacy of miners who wish to remain private." Telemetry only.

**Extension `0x0002` "Worker-Specific Hashrate Tracking"** adds a `user_identity` TLV (≤32
bytes, UTF-8 worker name) that "MUST" be appended to **every** `SubmitSharesExtended`, so
"pools can track worker-specific hashrate." The ecosystem's only attribution-related extension
**increases** attribution.

Its §4.2 is nonetheless the ecosystem's clearest statement that attribution is a privacy cost:
"Mining farms should be aware that sharing per-worker data with pools could reveal operational
insights. This could potentially compromise the privacy of the mining farm's operations." The
authors made it **opt-in** for that reason — and note that base SV2 extended channels aggregate
work and do **not** carry `user_identity`, so a proxy can front an entire farm as one channel.

The spec's one ZK reference is for template integrity, not privacy: "Zero-Knowledge-Proof based
protocol extensions, where JDC proves that the fee revenue on the coinbase belongs to a valid
template."

## Negative results — the gap, confirmed

- **No BIP** addresses mining payout or share-attribution privacy.
- **No SV2 extension** proposes payout privacy (only `0x0001` negotiation and `0x0002`, which
  increases attribution).
- **No ZK or MPC design for share accounting exists.** The only ZK mentions target a different
  problem (fee-revenue integrity; Luke Dashjr's withholding-detection suggestion). FROST in
  Braidpool/Radpool is threshold signing.
- delvingbitcoin queries for `pool+cannot+link`, `xpub+payout+mining`, `p2pool+privacy`,
  `MPC+mining+pool`, `hashrate+market+privacy` returned **zero topics**.

## Earliest and adjacent sketches (secondary)

- **delvingbitcoin #110 Fedimint/Fedipool** (EthnTuttle, 2023-09-29) — proposes nsec/npub-derived
  pool accounts so payouts work "without revealing miner identities to the custodial
  federation." Earliest attribution-minimizing sketch found; never specced. MattCorallo
  dismisses the need.
- **delvingbitcoin #1753 Scaling Noncustodial Mining Payouts with CTV** (vnprc, 2025-06-04) —
  CTV coinbase fanout, up to 319 outputs from 179 bytes. Never discusses attribution; vnprc notes
  "coinbase outputs, unlike coinjoin outputs, have no transaction history."
- **#2093 p2share** (VzxPLnHqr, 2025-11-07) and **#2165 BMAX** (2025-12-16) — newest sharechain
  material, public by construction. VzxPLnHqr: "due to the custodial nature of ecash, I have not
  investigated hashpool in relation to BMAX at all." *Caveat: an automated summary claimed the
  p2share thread discussed Mimblewimble-style sharechain privacy; a direct re-fetch of page 2
  found no such discussion (participants debated share burning and grinding). Treat as
  unverified — likely a related-topics sidebar bleed from dbtc #2081.*
