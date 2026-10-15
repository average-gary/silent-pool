# silent-pool

A Stratum V2 mining pool that pays miners to **BIP 352 silent-payment addresses**: one-time, unlinkable P2TR outputs **directly in the coinbase** for earnings above a threshold, and **Ark (bark) out-of-round payments** below it. Accounting is **SLICE / PPLNS-JD**, so miners build their own block templates via Job Declaration and are paid for both hashrate and the fees they selected.

**Status: design phase.** No pool code yet; the Ark-tier derivation, mailbox and client are prototyped and validated on regtest (`spc_ark/`, `spc_ark_mailbox/`, `spc_ark_client/`, see `RESULTS.md`). The prototype crates are reference material, not throwaway: their frozen test vectors are normative inputs and they are absorbed into `spc-crypto` / `spc-agent` (see `docs/ARCHITECTURE.md` §2.3). The technical specification and roadmap are in `docs/`; the research behind them lives in a separate knowledge base (see *Provenance*).

## The two ideas

1. **SPc — silent payments in the coinbase.** BIP 352 cannot be used in a coinbase because the input has no public key. SPc supplies one: a per-height ephemeral key `A_send`, published in the coinbase itself (scriptSig pool tag in pool-template mode, `OP_RETURN` under Job Declaration), with the BIP 34 block height as the nonce. Each miner's output is `P = B_spend + hash_SPc/TierSpc(ecdh ‖ 0)·G`; a wallet finds it from the block header, the coinbase, and its scan key. The pool erases `a_send` once the coinbase is built. Public write-up: [delvingbitcoin t/2833](https://delvingbitcoin.org/t/silent-payments-coinbase/2833).
2. **A sub-threshold rail that is not the pool's own ledger.** Earnings under the coinbase threshold are paid to a per-block Ark key derived from the same ECDH, once the block has 6 confirmations, from the pool's Ark float on a **third-party** Ark server — deliberately not one the pool runs, because arkoor's only security assumption is "sender or server honest." The block's single pool output (a fresh P2TR under the pool's hot Ark key) refills the float when it matures. Miners who don't register Ark delivery forfeit their below-threshold earnings to the pool.

## Components (planned)

| Binary | Role |
|---|---|
| `spc-pool` | SV2 pool; roles `front` / `payout` / `api` / `ark`; SLICE accounting; SPc coinbase; Ark settlement. `front` includes the `spc-jds` module — Job Declarator Server implementing sv2-spec PR #203 (`0x0003` Coinbase Transaction Payouts), always on |
| `spc-agent` | Miner-side: SV1/SV2 translator + JDC + payout verifier + Ark wallet, one binary, one seed |
| `spc-scan` | Reference scanner / wallet helper for SPc outputs |

Miner identity is the silent-payment address, full stop: it is the only thing that keys a miner's account, and both payout tiers are derived from it per block. Separately, a miner MAY register a private **delivery capability** for the Ark tier (`B_ark` plus mailbox blinding data, signed with the scan key) over a small project-defined SV2 extension; it changes where the miner can be paid, never who they are, and is never published.

## Documents

- [`DESIGN.md`](DESIGN.md) — the one-identity derivation: Ark-tier destination derived from the same per-block ECDH as the coinbase output (authoritative for the Ark-tier key scheme; spec §11.8 defers to it); prototype in [`spc_ark/`](spc_ark/)
- [`docs/SPEC.md`](docs/SPEC.md) — technical specification v0.1 (architecture, SPc protocol, SV2 surface, SLICE accounting, data model, interfaces, tests, deployment, §11 Ark rail)
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — phased build plan with architecture decisions and risks
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — component map, interfaces and scoping for implementation planning
- [`docs/MULTI-REGION.md`](docs/MULTI-REGION.md) — multi-region deployment design (its §10 deltas are mostly adopted into the spec; the spec wins on conflict)
- [`docs/WIRE-FORMATS.md`](docs/WIRE-FORMATS.md) — byte-level formats of the project's own Redis streams, `spc:arkreg`, `spc:ark:*` and the front bootstrap RPC
- [`docs/reviews/`](docs/reviews/) — implementation-readiness review and its resolutions
- [`docs/decisions/`](docs/decisions/) — dated decision records; first entry: the 2026-10-12 assessment of sv2-spec PR #203's reference implementation
- [`UPSTREAM_DEPS.md`](UPSTREAM_DEPS.md) — exact commits this design is pinned to, and why

## Provenance

These documents were produced from a research wiki (`bitcoin-mining-payout-schemas`) that ingested the primary sources — BIP 352, the Stratum V2 spec and PR #203, the SRI/sv2-apps and bark source trees, Blitzpool, DMND's SLICE material, OCEAN's docs, and the relevant Delving/Optech threads. Every article and raw source the spec or roadmap cites is snapshotted under [`docs/research/`](docs/research/) with the wiki's layout preserved, so the documents stand alone; each raw file's frontmatter carries its original URL, ingest date and credibility rating.

## License

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`), matching SRI, bark, `extensions_sv2` and the `spc_ark` prototype. Blitzpool (AGPL) is a design reference only: its ledger, trim and autoscaler patterns are re-implemented from the spec, no code is ported (spec §7.6).
