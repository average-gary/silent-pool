---
title: SV2 Share Accounting Extension
category: concept
created: 2026-07-15
updated: 2026-07-15
verified: 2026-07-15
confidence: medium
volatility: warm
tags: [share-accounting, pplns-jd, SLICE, dmnd, stratum-v2, sv2-extension, job-declaration, payout-verification, transaction-fees, merkle-proof]
aliases: [share-accounting-ext, demand-share-accounting-ext, SV2 extension 32, PPLNS-JD verification protocol]
sources:
  - "raw/repos/2026-07-14-demand-share-accounting-ext-github.md"
summary: "SV2 protocol extension (extension type 32) that lets a miner cryptographically audit a pool's PPLNS-JD payout for a reward window, checking share validity, merkle inclusion, difficulty, and transaction fees. It is the verification wire protocol behind DMND's SLICE / PPLNS-JD, implemented as the demand-open-source Rust crate demand-share-accounting-ext."
---

# SV2 Share Accounting Extension

> A Stratum V2 extension (extension type `32`) whose single purpose is to make a pool's payout **verifiable by the miner**. Classic payout schemes credit only hashpower, so a miner has to *trust* the pool's accounting. When miners select their own transactions (SV2 Job Declaration) and fees become a large share of the reward, payout must reflect both hashpower and the fees each miner chose — and this extension gives the miner the messages to check that the pool did so honestly, by spot-sampling shares from a reward window and proving each one. It is the audit-layer wire protocol behind [[pplns-jd|SLICE / PPLNS-JD]], and the on-the-wire counterpart to the theoretical PPLNS-with-Job-Declaration design.

## Where it sits in the stack

This is an **extension of the SV2 Mining subprotocol**, not a standalone protocol: its messages MUST travel over an already-established mining connection. It complements, rather than replaces, [[../topics/sv2-jd-and-payout-decoupling|Job Declaration]]. JD is what lets a miner *build* the block template; this extension is what lets the miner *audit the payout* that results. Reference implementation: the Rust crate `demand-share-accounting-ext` (v0.0.13, MIT OR Apache-2.0) from demand-open-source, `no_std`-capable, built on the demand fork of the SRI/Stratum V2 codec stack (`roles_logic_sv2`, `binary_sv2`, `framing_sv2`).

The design is explicitly the wire realization of the trust-minimization goal that [[pplns-jd|PPLNS-JD]] states but does not itself enforce: PPLNS-JD decouples template construction (miner) from share accounting (pool), but leaves the miner trusting the pool's window arithmetic. This extension closes that gap with a verification handshake.

## The verification model

Rather than re-deriving an entire reward window, a miner **probabilistically spot-checks** it. Verification cost is bounded by how much assurance the miner wants, not by window size:

0. Request the window to audit (`GetWindow`, keyed by the block hash of a pool-found block).
1. Randomly select some **slices** to check. For each selected slice:
   1. randomly select some shares within the slice;
   2. fetch the transactions not already cached for each selected share;
   3. verify each share is valid proof-of-work;
   4. verify that `merkle_path(share) + share_hash == slice.root`;
   5. verify that the summed difficulty of the verified shares does not exceed the slice's declared difficulty;
   6. verify that the fees in the shares are within `slice.ref_job_fees + delta`.

Steps 3–6 are what bind payout to *both* hashpower (3, 5) *and* miner-chosen fees (6), with merkle inclusion (4) proving the sampled shares were actually committed to the window the pool is paying on.

## Core data types

- **Slice** — a group of shares mined while the mempool's maximum extractable fees (MMEF) can be treated as approximately constant. Carries `number_of_shares`, summed `difficulty`, reference-job `fees`, a merkle `root` over its shares, and a `job_id`. Slices are the unit of sampling and the unit of the reward-window arithmetic.
- **Share** — a submitted share with the SV2 mining fields (`nonce`, `ntime`, `version`, `extranonce`) plus `job_id`, `reference_job_id`, a window-relative `share_index`, and the `merkle_path` proving its inclusion in the slice root.
- **PHash** — a previous-block hash plus the index of the first share that uses it, returned inside the window so the miner can map each share to the correct prev-hash when re-checking proof-of-work.

A **window** spans from the first slice whose cumulative difficulty back from the block reaches `N × window_size` up to the slice containing the block-finding share (the last slice is excluded from reward calculation). This is the same N-scaled look-back idea that TIDES and SLICE both instantiate at N = 8 × network difficulty (see [[pplns-jd|PPLNS-JD]]).

## Message set

Extension type `32`; all messages carry `channel_msg` bit `false`. Message-type bytes:

| 0x | Message | Dir | Role |
|----|---------|-----|------|
| 00 | Activate | C→S | Opt into the extension on a live mining connection |
| 01 | Activate.Success | S→C | Ack; an empty `0xff`-type frame means "unsupported → say nothing more" |
| 02 | ShareOk | S→C | Per-share ack (`ref_job_id`, `share_index`); MAY replace `SubmitShares.Success` |
| 03 | NewBlockFound | S→C | Pool found a block (`block_hash`); prompts a `GetWindow` |
| 04 | GetWindow | C→S | Request a window by block hash |
| 05 | GetWindow.Success | S→C | Returns the window's slices and phashes |
| 06 | GetWindow.Busy | S→C | Backpressure for older windows (`retry_in_seconds`) |
| 07 | GetShares | C→S | Fetch shares by window-relative id |
| 08 | GetShares.Success | S→C | The requested shares |
| 09 | GetTransactionsInJob | C→S | Request the tx set of a job |
| 0A | GetTransactionsInJob.Success | S→C | `coinbase_id`, SipHash short-id list (BIP-152 style), full-list hash |
| 0B | IdentifyTransaction | C→S | Ask which txs the miner must supply |
| 0C | IdentifyTransaction.Success | S→C | The tx-data hashes used to build the job |
| 0D | ProvideMissingTransactions | C→S | Supply requested full txs |
| 0E | ProvideMissingTransactions.Success | S→C | Full transactions, in requested order |
| 0F | NewTxs | S→C | Pool pushes fee-increasing (MMEF-raising) txs to miners |
| 10 | ErrorMessage | both | Single irrecoverable-error type; receiver MUST close the connection |

Errors are handled per-message where recoverable (e.g. `GetWindow` → `GetWindow.Busy`); the generic `ErrorMessage` (a log string) is reserved for irrecoverable faults and forces connection close.

## Status and caveats

- **Early / unfinished.** Crate v0.0.13; the activation handshake (§1.2 of the extension spec) is explicitly a TODO pending [sv2-spec issue #95](https://github.com/stratum-mining/sv2-spec/issues/95). Message numbering and semantics may still move.
- **Verification is trust-*reducing*, not fully trustless.** It lets a miner catch a cheating pool statistically over many windows; a single spot-check samples only some slices/shares. It also assumes all pool miners mine on the same block/branch, so window integrity depends on that assumption holding.
- **API spelling is idiosyncratic** — the public type names carry typos (`GetTransationsInJob`, `ProvideMissinTransactions`) that are baked into the crate.
- **Provenance nuance:** the git repo is `demand-open-source/share-accounting-ext`, but the crate/`repository` field names it `demand-share-accounting-ext`.

Confidence is `medium`: the wire format and code are primary and unambiguous, but this is a single pre-1.0 implementation with an unfinished spec and no independent second source, so claims about it being *deployed* or *final* should not be inferred.

## Extending the pattern to off-chain payout layers (session note, 2026-07-18)

A "claim your hashrate share in an off-chain layer (Ark VTXO)" SV2 extension conflates **two** verification layers, and only carrying the first relocates trust rather than reducing it (see [[../../raw/notes/2026-07-18-ll-proxy-held-vtxo-ark-sv2-extension|lessons note]]):

1. **Amount** (hashrate → BTC) — what *this* extension (type 32) already makes trust-reducing via merkle-path spot-checks.
2. **Instrument exclusivity** (the VTXO leaf is exclusively spendable by the claiming miner) — requires the extension to also carry the VTXO tree/leaf/path data and a **leaf-exclusivity proof**. This is the gap [[ark-for-mining-payouts]] flags as "path-exclusivity unverified."

**Rule:** a payout-claim protocol that carries only the accounting amount relocates trust to the proxy/ASP; a trust-*reducing* one must prove the instrument is exclusively yours the way type-32 proves inclusion.

## See also

- [[pplns-jd|PPLNS-JD / SLICE (DMND)]] — the payout scheme this extension is designed to make verifiable
- [[ark-for-mining-payouts|Ark for Mining Payouts]] — off-chain payout layer whose proxy-held-VTXO variant would need this two-layer verification
- [[../topics/sv2-jd-and-payout-decoupling|SV2 Job Declaration ↔ Payout Decoupling]] — the protocol split that makes miner-verifiable payout possible
- [[tides|TIDES (OCEAN)]] — the other non-custodial PPLNS scheme converging on N = 8 × D
- [[payout-schema-taxonomy|Payout Schema Taxonomy]] — where verifiable PPLNS-JD sits in the overall map
- [[payout-attribution-privacy|Payout Attribution Privacy]] — the extension's `Slice`/`Share`/`PHash` types carry **zero identity fields**, making it the one already-decoupled ledger in the ecosystem
- [[xpub-payout-identity|xpub Payout Identity]] — why that decoupling makes this the natural host for a miner-supplied descriptor

## Sources

- [[../../raw/repos/2026-07-14-demand-share-accounting-ext-github|demand-open-source/share-accounting-ext — SV2 Share Accounting Extension]] — crate source, message set, data types, and verification protocol (extension.md)
