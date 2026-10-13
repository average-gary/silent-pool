---
title: "demand-open-source/share-accounting-ext — SV2 Share Accounting Extension"
url: https://github.com/demand-open-source/share-accounting-ext
source: "https://github.com/demand-open-source/share-accounting-ext"
type: repos
language: Rust (no_std-capable)
license: MIT OR Apache-2.0
crate_version: 0.0.13
last_commit: 2025-06-21
ingested: 2026-07-14
quality: 4
credibility: high
confidence: high
tags: [demand, stratum-v2, sv2-extension, pplns-jd, job-declaration, share-accounting, payout-verification, transaction-fees, merkle-proof, rust]
summary: "Rust crate defining an SV2 protocol extension (extension type 32) that lets miners cryptographically verify a pool's PPLNS payout accounting over both provided hashpower AND miner-selected transactions (Job Declaration). Defines wire messages for windows, slices, shares, and transaction-fee verification. Depends on the demand-open-source fork of the SRI stratum stack (roles_logic_sv2 / binary_sv2 / framing_sv2)."
---

# demand-open-source/share-accounting-ext — SV2 Share Accounting Extension

Rust crate `demand-share-accounting-ext` (v0.0.13, MIT OR Apache-2.0) implementing a Stratum V2 **extension** whose purpose is to let a miner *verify the pool's payout for a given reward window*. Classic pool payout schemes account only for provided hashpower; this extension is designed for a world where miners select their own transactions (SV2 Job Declaration) and transaction fees become a significant part of the reward, so payouts must reflect **both hashpower and the fees of the transactions each miner chose**. It is the wire-protocol substrate for the PPLNS-with-Job-Declaration (PPLNS-JD) scheme described in demand's [PPLNS-with-JD paper](https://www.dmnd.work/pplns-with-job-declaration/pplns-with-job-declaration.pdf).

- **Extension type:** `32` (`EXTENSION_TYPE` in `src/const.rs`).
- **Transport:** messages MUST be sent over an already-established SV2 **Mining** connection; this is an extension of the Mining subprotocol, not a standalone protocol.
- **Repo note:** the git remote is `demand-open-source/share-accounting-ext`; the `Cargo.toml` `repository` field points at `demand-open-source/demand-share-accounting-ext` (crate name). Original author frontmatter: `fi3`. Last commit 2025-06-21 ("Use SRI lib from master branch not anymore from ImproveCoinbase").

## Verification protocol (miner's view)

The extension lets a miner audit one reward window:

0. Ask for the window it wants to verify (`GetWindow` by `block_hash` of a pool-found block).
1. Randomly select some **slices** to check; for each selected slice:
   1. randomly select some shares in the slice,
   2. fetch the transactions not already cached for each selected share,
   3. verify each share is valid PoW,
   4. verify `merkle_path(share) + share_hash == slice.root`,
   5. verify the sum of verified share difficulties does not exceed the slice difficulty,
   6. verify the fees in the shares are within `slice.ref_job_fees + delta`.

This is a probabilistic spot-check design — the miner samples slices/shares rather than re-deriving the whole window.

## Core data types (`src/data_types/`)

- **Slice** (`slice.rs`) — a group of shares mined while the mempool's maximum extractable fees (MMEF) can be treated as constant. Fields: `number_of_shares:U32`, `difficulty:U64` (sum of member share diffs), `fees:U64` (fees of the slice's reference job), `root:Hash256` (merkle root over the slice's shares), `job_id:U64`.
- **Share** (`share.rs`) — a submitted share: `nonce:U32`, `ntime:U32`, `version:U32`, `extranonce:B032`, `job_id:U64`, `reference_job_id:U64`, `share_index:U32`, `merkle_path:B064K`.
- **PHash** (`phash.rs`) — a previous-block hash plus `index_start:u32`, the index of the first share in the window using that phash. Returned in `GetWindowSuccess` so a miner can map each share to the correct prev-hash for PoW verification.
- **Hash256** (`mod.rs`) — 32-byte value stored as four little-endian `u64` limbs (`first/second/third/forth`), with `From<[u8;32]>` conversions. Marked `#[already_sized]` for the SV2 binary codec.

## Message set

Message-type bytes (`src/const.rs`), all with `channel_msg` bit `false`. Wire spec is documented in the repo's `extension.md`.

| 0x | Message | Dir | Purpose |
|----|---------|-----|---------|
| 00 | Activate | C→S | Opt into the extension on a live mining connection |
| 01 | Activate.Success | S→C | Ack (a `msg_type 0xff, len 0` frame means "unsupported → send nothing else") |
| 02 | ShareOk | S→C | Per-share ack (`ref_job_id`, `share_index`); MAY replace `SubmitShares.Success` |
| 03 | NewBlockFound | S→C | Pool found a block (`block_hash`); triggers a miner `GetWindow` |
| 04 | GetWindow | C→S | Request a window by `block_hash` |
| 05 | GetWindow.Success | S→C | `slices: Seq0_64K[Slice]` + `phashes: Seq0_64K[PHash]` |
| 06 | GetWindow.Busy | S→C | Backpressure: `retry_in_seconds` (older windows) |
| 07 | GetShares | C→S | Fetch shares by window-relative id list |
| 08 | GetShares.Success | S→C | `shares: Seq0_64K[Share]` |
| 09 | GetTransactionsInJob | C→S | Request the tx set for a job |
| 0A | GetTransactionsInJob.Success | S→C | `coinbase_id`, `tx_short_hash_nonce`, `tx_short_hash_list` (SipHash short IDs, BIP-152 style), `tx_hash_list_hash` |
| 0B | IdentifyTransaction | C→S | Ask which txs the miner must supply |
| 0C | IdentifyTransaction.Success | S→C | `tx_data_hashes` used to build the job |
| 0D | ProvideMissingTransactions | C→S | (paired success below) |
| 0E | ProvideMissingTransactions.Success | S→C | `transaction_list: Seq0_64K[B0_16M]` full txs in requested order |
| 0F | NewTxs | S→C | Pool pushes fee-increasing (MMEF-raising) txs; persistent front-running by miner is a signal to switch pools |
| 10 | ErrorMessage | both | Single irrecoverable-error type; a `STR0_255` log string. Receiver MUST close the connection |

Errors are handled per-message where recoverable (e.g. `GetWindow` → `GetWindow.Busy`); only irrecoverable failures use the general `ErrorMessage`.

## Code layout (`src/`, ~1.3k LOC)

- `lib.rs` — `#![cfg_attr(feature = "no_std", no_std)]`; re-exports all message + data types.
- `const.rs` — extension type, message-type bytes, channel bits.
- `parser.rs` (~470 LOC) — `ShareAccountingMessages<'a>` enum + `IsSv2Message`; frame encode/decode integrating with `roles_logic_sv2` parsers (`CommonMessages`, `Mining`, `JobDeclaration`, `TemplateDistribution`) and `framing_sv2::Sv2Frame`.
- One module per message group: `activate_ext.rs`, `share_ok.rs`, `new_block_found.rs`, `get_window.rs`, `get_shares.rs`, `new_txs.rs`, `error_message.rs`, `verify_fees.rs` (the `GetTransationsInJob*` / `IdentifyTransations*` / `ProvideMissinTransactions*` families — note the crate's spelling).
- `data_types/` — `slice.rs`, `share.rs`, `phash.rs`, `mod.rs` (Hash256).

## Dependencies

Depends on the **demand-open-source fork** of the SRI stratum stack (not crates.io), pinned by git subdirectory:
- `roles_logic_sv2`, `binary_sv2`, `framing_sv2` from `github.com/demand-open-source/stratum` (as of the last commit, tracking that fork's `master`).
- Optional `serde` (default-off). Features: `no_std`, and a `with_serde` codec path used throughout via `#[cfg(feature = "with_serde")]`.

## Status / caveats

- Early-stage (v0.0.13). The activation handshake (§1.2 of `extension.md`) is explicitly TODO, pending [sv2-spec issue #95](https://github.com/stratum-mining/sv2-spec/issues/95).
- Spelling inconsistencies in the API (`GetTransationsInJob`, `ProvideMissinTransactions`) are baked into the public type names.
- Assumes all pool miners mine on the same block/branch (window integrity depends on it).

## See also

- [[../repos/2026-05-23-stratum-v2-spec|Stratum V2 Specification]] — base Mining + Job Declaration protocol this extends
- [[../repos/2026-05-24-p2poolv2-accounting-modules|p2poolv2 accounting modules]] — alternative code-level PPLNS accounting
- [[../repos/2026-05-26-parasitepool-para-github|parasitepool/para]] — contrasting SV1, custodial, decay-based accounting
