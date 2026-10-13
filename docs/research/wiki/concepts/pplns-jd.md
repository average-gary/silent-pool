---
title: PPLNS-JD / SLICE
category: concept
created: 2026-05-23
updated: 2026-10-12
verified: 2026-07-15
confidence: medium
volatility: warm
tags: [pplns-jd, SLICE, dmnd, demand-pool, stratum-v2, JD]
summary: "PPLNS variant where shares are bound to SV2 Job-Declarator-declared jobs — the miner picks the block template; the pool just counts shares."
sources:
  - "raw/articles/2026-05-23-dmnd-demand-pool.md"
  - "raw/articles/2026-05-23-dmnd-slice-blog-pplns-jd.md"
  - "raw/repos/2026-05-23-stratum-v2-spec.md"
  - "raw/repos/2026-07-14-demand-share-accounting-ext-github.md"
---

# PPLNS-JD / SLICE

PPLNS variant where shares are bound to **SV2 Job-Declarator-declared jobs** — the miner picks the block template; the pool just counts shares.

Production deployment: **DMND / Demand Pool** as **SLICE**.

## Mechanism

- Miner runs a **Job Declarator Client (JDC)** alongside their mining stack.
- JDC builds a block template (selects transactions from the miner's own mempool).
- JDC sends `DeclareMiningJob` to the pool's **Job Declarator Server (JDS)**.
- Pool returns `DeclareMiningJob.Success` with a token authorizing payout to the miner if the block is found.
- Shares submitted under this job are scored PPLNS-style; if a share *is* a block, the coinbase pays per the JDS-allocated outputs.

This decouples **template construction** (miner) from **payout accounting** (pool) — the spec text in SV2-spec §6 is explicit:

> *"Pools…are only responsible for accounting shares and distributing rewards."*

## Why this changes payout fairness

Under FPPS, the pool decides which transactions go in the block. Under PPLNS-JD, the miner decides — so:
- **Miner captures the tx-fee revenue** they think appropriate (no FPPS averaging).
- **Pool can't censor** (it doesn't see the template content in coinbase-only mode).
- **Slice removes the FPPS subsidy** where miners pay (via fee) for the pool's transaction-filter policies.

## SLICE (DMND) operational claims

- Rejection rate: **0.0151%** (vs. 0.5–2% on Stratum V1).
- ~1–3 ms job-switch latency vs. ~200–300 ms on V1 — direct profitability lift.
- Zero hash-hijack incidents (binary encrypted framing eliminates SV1 plaintext attack).
- "Miners build the block. Miners keep the rBTC."

Operator: Guru Protocol Ltd (UK). Leadership: Alejandro De La Torre (ex-VP Poolin, CEO), Filippo Merli (long-time SRI contributor, CTO).

## Published parameters (DMND blog, March-April 2025)

**N is published** — closed by gap-research 2026-05-23.

- **N = 8 × Bitcoin network difficulty** — *same multiplier as TIDES*. Verbatim from DMND blog (2025-03-18, "Understanding SLICE (PPLNS+JD)"): *"DMND Pool uses the Bitcoin's network difficulty and multiplies it by 8 to create the look back window."*
- **Each share is paid across ~8 successive blocks** on average (8-block rolling window).
- **Share-difficulty target**: ~6 shares/minute per client.
- **Reject rate**: 0.0151% (vs 0.5–2% for typical SV1 pools).

The companion theoretical paper (lorbax/pplns-with-job-declaration) confirms the design rationale: *"N is not specified, but is suggested to be such that the sum at the denominator is a multiple [of] the bitcoin difficulty (for Ocean TIDES this multiple is 8)."* DMND's deployed N matches the TIDES-aligned suggestion.

**TIDES and SLICE converge on N = 8 × D.** Production consensus for "PPLNS done right with non-custodial coinbase payout."

## Still-open gaps

- **BTC-leg pool fee** — not published. The rBTC side-payout is explicit 0% pool cut: *"There is no revenue share. There is no pool wallet sitting in the middle."* The BTC subsidy/fee leg remains undisclosed.
- **Minimum payout / withdrawal threshold** — not published.
- KYB onboarding required (DMND changelog 2026-05-12) — implies fee terms may be **negotiated/contractual** rather than published.

## Comparison to TIDES

Both are PPLNS-derived non-custodial schemes with **identical N = 8 × D**:

| | TIDES (OCEAN) | SLICE (DMND) |
|---|---|---|
| Window N | **8 × D, scales with D** | **8 × D, scales with D** |
| Shares bound to | Pool template (or DATUM) | JD-declared (miner template) |
| Payout custody | None (coinbase) | None (coinbase) |
| Template control | Pool default; DATUM option | Always miner |
| Standard BTC fee | 2% (1% DATUM) | Not published |
| rBTC side-payout | n/a | 0% pool cut |
| Spec doc | Public | Public (blog.dmnd.work, 2025) |

These are **complementary, not competing**. TIDES + DATUM is OCEAN's version of the same idea (PPLNS done right + miner templates).

## Miner-verifiable payout (the accounting extension)

PPLNS-JD as described above still asks the miner to *trust* the pool's window arithmetic. The [[sv2-share-accounting-ext|SV2 Share Accounting Extension]] (extension type 32) closes that gap: it is a wire protocol letting a miner **audit** a reward window by spot-sampling slices and shares and proving each one — share PoW validity, merkle inclusion in the slice root, that summed share difficulty does not exceed the slice's, and that share fees stay within the slice's reference-job fees plus a delta. It is the on-the-wire counterpart to the theoretical PPLNS-with-Job-Declaration design (lorbax/pplns-with-job-declaration) that this article's N = 8 × D discussion draws on. Reference implementation: the `demand-share-accounting-ext` Rust crate (v0.0.13, pre-1.0; activation handshake still TODO). It reduces — but does not fully eliminate — trust in the pool, since a single audit samples only part of the window.

## Non-custodial payouts under JD: who actually controls the coinbase

**Added 2026-08-24.** Base JDP cannot express multi-miner coinbase payouts at all, and the reason is an
ordering constraint rather than an oversight: `AllocateMiningJobToken.Success` fixes the coinbase
outputs *before* `NewTemplate` reveals the template revenue `T` that the amounts depend on. What base
JDP negotiates is only **where the pool gets paid**, not whom it pays — §6.4.3 has the pool reserve the
**first** output as the pool payout output, requires JDC to allocate sats into it, and constrains any
further pool-added outputs to **0 value**. So the reward flows to one pool output and per-miner
distribution happens elsewhere.

`sv2-spec` **PR #203** (extension `0x0003`, "Non-Custodial Payouts", open, supersedes the closed #195
and #202) resolves it by having the pool publish `SetPayoutDistribution`: per-miner outputs plus
**relative weights**, with absolute amounts computed deterministically once `T` is known
(`amount[i] = floor(weight[i]·T/W)`, `pay_P` absorbing dust pruning and rounding). Validation is
recompute-and-compare. It targets non-debt accounting explicitly — PPLNS, SLICE, TIDES — and says
PPS/FPPS pools SHOULD NOT use it, which makes it directly relevant to this article.

Three findings worth recording for anyone building on it:

- **The two JD modes differ on scriptSig visibility, and one cannot host a pool tag at all.**
  Full-Template (`DeclareMiningJob` §6.4.4) carries `coinbase_tx_prefix` + `coinbase_tx_suffix`, so the
  pool *sees* the scriptSig and can reject a declaration. Coinbase-only (`SetCustomMiningJob` §5.3.18)
  carries only `coinbase_prefix` (≤8 B, the BIP 34 height), `nSequence`, `coinbase_tx_outputs`,
  `locktime` and `merkle_path` — no arbitrary scriptSig bytes, so the pool can neither specify nor
  validate a tag. Neither mode has a pool → JDC field for scriptSig content, so the pool has
  **approval** authority in one mode and **specification** authority in neither.
- **Its own §9.2 names the privacy cost:** the distribution is pool-wide, so *"every JDC learns the
  payout scripts and relative weights of the Pool's top miners,"* with "use fewer payout slots" as the
  offered mitigation. That is a leak-versus-non-custodiality dial rather than a fixable bug — see the
  impossibility note below.
- **The disclosure cannot be engineered away.** The JDC must hash the coinbase, the extranonce sits
  inside the scriptSig, and everything after it must be re-hashed per roll — so the prefix could in
  principle be collapsed to a SHA-256 midstate but the **suffix cannot**, and the outputs live in the
  suffix. The bytes one would most want to withhold are structurally the ones that must be handed
  over.

See [[../../raw/notes/2026-08-24-ll-jd-coinbase-control-amount-linkability|Lessons: SV2 coinbase
control and amount linkability]] lessons 3 and 4.

**Implementation status (2026-10-12).** PR #203's text has not moved since its single commit `47dbedd` (2026-09-10). On 2026-10-11 warioishere (Blitzpool) reported runnable code pinned to that commit: `extensions_sv2` messages/TLV/payout math ([warioishere/stratum#1](https://github.com/warioishere/stratum/pull/1), `9be4023`, MIT/Apache) and an SRI **JDC** ([warioishere/sv2-apps#3](https://github.com/warioishere/sv2-apps/pull/3), `90b3e3d`), run on mainnet against **Blitzpool's JDS** (release 2.4.1, `b48998f`, AGPL) in both JD modes — self-reported, no artifacts. Two unrelated SRI bugs surfaced: the coinbase-only JDC stops sending custom jobs after a chain-tip update (#1), and `bitcoin_core_sv2` reserves only `max(4·size, 2000)` WU for the coinbase instead of TDP's `1168 + 4·size`, so a 42-output coinbase with a full mempool fails `bad-blk-weight` (#2). No SRI JDS implements `0x0003` yet. Two thread items could still move text: TheBlueMatt's unanswered question on paying only a subset of static `additional_outputs` (would relax §7's no-partial-payment MUST) and a requested §9.3 sentence. Full record: [[../../raw/articles/2026-10-11-sv2-spec-pr203-0x0003-implementations-and-thread|raw]].

## Sources

- [[../../raw/articles/2026-05-23-dmnd-demand-pool|DMND / Demand Pool article]]
- [[../../raw/articles/2026-05-23-dmnd-slice-blog-pplns-jd|DMND blog — Understanding SLICE (PPLNS+JD), 2025-03-18]] — primary spec for N = 8 × D
- [[../../raw/repos/2026-05-23-stratum-v2-spec|Stratum V2 Specification — §6 Job Declaration]]
- [[../../raw/repos/2026-07-14-demand-share-accounting-ext-github|demand-open-source/share-accounting-ext — SV2 Share Accounting Extension]] — the miner-side payout-verification wire protocol

## See also

- [[pplns]]
- [[tides]]
- [[sv2-share-accounting-ext|SV2 Share Accounting Extension]] — the wire protocol that makes this scheme's payout verifiable
- [[../topics/sv2-jd-and-payout-decoupling|SV2 Job Declaration ↔ Payout Decoupling]]
- [[xpub-payout-identity|xpub Payout Identity]] — PPLNS-JD is the natural host: its ledger key is `(slice, share_index)` with **zero identity fields**, so an xpub changes only the identity layer
- [[payout-attribution-privacy|Payout Attribution Privacy]] — what the pool still learns from validating the shares in each slice
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]]
- [[datum.md|DATUM (OCEAN template-construction)]]
- [[parasite-pool.md|Parasite Pool]]
