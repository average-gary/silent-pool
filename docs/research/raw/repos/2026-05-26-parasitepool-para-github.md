---
title: "parasitepool/para — Parasite Pool reference implementation"
source: https://github.com/parasitepool/para
homepage: https://parasite.space
endpoint: stratum+tcp://parasite.wtf:42069
license: CC0-1.0
type: repos
language: Rust (+ vendored ckpool C fork)
created: 2025-04-10
last_commit: 2026-05-24
ingested: 2026-05-26
quality: 5
credibility: high
confidence: high
tags: [parasite-pool, repo, rust, ckpool, stratum-v1, primary]
summary: "Canonical reference implementation for Parasite Pool. CC0-licensed, actively developed (v0.5.3 Dec 2025; latest commit May 2026). ~51 stars, 12 forks, 28 open issues."
---

# parasitepool/para — The Parasite Toolkit

Canonical reference implementation for Parasite Pool. CC0-licensed, actively developed (v0.5.3 Dec 2025; latest commit May 2026). ~51 stars, 12 forks, 28 open issues.

Top contributors (pseudonymous): `paratoxicdev` (~258 commits), `parabitdev` (~64), `parachemist`, `Paraphreak`.

## Repo structure

- `src/` — Rust library + CLI (`para` binary with subcommands: `miner`, `template`, `ping`, `pool`)
- `crates/stratum/` — from-scratch Rust **Stratum V1** library
- `ckpool/` — vendored fork of ckpool (C), the production pool daemon. Modifications:
  - Postgres share-log
  - Custom coinbase logic (BIP34 height + extranonce + `|para|` tag bytes `7c 70 61 72 61 7c`)
  - Signet support
  - Custom config flags
- `src/decay.rs` — **continuous-time exponential-decay share weighting**. `exponential_saturation = 1 - e^(-x)`, normalized EMA. **This is the actual accounting algorithm, not classic-window PPLNS.**
- `src/coinbase_builder.rs` — constructs a single-output pool-controlled coinbase (custodial model).
- Sister repos: `parasitepool/parastats` (Next.js dashboard); `parasitepool/entangle` (early-dev Rust crate, undocumented as of May 2026).

## CHANGELOG / shipping order

1. Vardiff (#283)
2. Account system (#254)
3. Automated Lightning payouts (#270)
4. ZMQ block notifications (#232)
5. Template generator (#230)

## Notable findings vs. founder narrative

- **Stratum V1, not V2**. No Job Declaration support. Miners cannot independently verify templates — operator can MEV-tax or censor.
- **Custodial coinbase**: single output to pool-controlled address. Despite "decentralization" branding, custody is operator-trust between block-find and Lightning fanout.
- Decay window length is a runtime config (`settings.rs`); production value not publicly documented.

## Username schema (auth string)

`<onchain-addr>.<worker>.<lightning-addr>@parasite.sati.pro`

Carries onchain BTC address AND Lightning address (Xverse-derived) inline. No registration. Fallback `@sati.pro`.

## Adoption signal

- Endpoint `parasite.wtf:42069` confirmed in `mweinberg/stratum-speed-test` (~52 ms US latency).
- `mrv777/ParaApp` — third-party React Native client.
- Bitaxe firmware (NerdQAxe+, NerdMiner, acs-esp-miner) feature requests for Parasite presets.

## See also

- [[../articles/2026-05-26-zkshark-parasite-pool-substack]] — founder rationale
- [[../articles/2026-05-26-bitcoin-manual-parasite-pool]] — economic critique
- [[../articles/2026-05-26-blockspace-media-parasite-emerges]]
- [[../articles/2026-05-26-coindesk-parasite-second-block]]
