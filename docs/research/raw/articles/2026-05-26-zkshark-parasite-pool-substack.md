---
title: "Parasite Pool: Igniting the Mining Insurrection (zkshark Substack)"
author: zk-shark (pseudonym; x.com handle @kram_btc)
publication: zkshark.substack.com
source: https://zkshark.substack.com/p/parasite-pool-igniting-the-mining
date: 2025-05-09
type: articles
ingested: 2026-05-26
quality: 4
credibility: medium
confidence: high
tags: [parasite-pool, zk-shark, primary, substack, lightning-payouts, novel-accounting]
summary: "Founder-authored manifesto / informal whitepaper for **Parasite Pool**, a Bitcoin mining pool launched in beta in 2025 by pseudonymous developer **zk-shark** (also creator of Ordinal Maxi Biz). This is the canonical \"design rationale\" document — there is no formal whitepaper."
---

# Parasite Pool — Founder Announcement (Substack)

Founder-authored manifesto / informal whitepaper for **Parasite Pool**, a Bitcoin mining pool launched in beta in 2025 by pseudonymous developer **zk-shark** (also creator of Ordinal Maxi Biz). This is the canonical "design rationale" document — there is no formal whitepaper.

## Mechanism (as described)

- **Block-finder bonus**: the worker that submits the winning share receives a flat **1 BTC** off the top of the coinbase.
- **Residual distribution**: the remaining ~**2.125 BTC + tx fees** (post-2024-halving subsidy 3.125 BTC) is split among all other contributing miners.
- Weighting of the residual: shares accumulated since the pool's last block (no rolling N-window — full inter-block window).
- **Zero pool fee**.
- **Lightning payouts** with **10-sat minimum** (effectively no minimum), executed event-driven on block-find.
- Coinbase-maturity sidestep: pool fronts liquidity via Lightning so payouts settle before the 100-block coinbase matures.

## Stack credits (founder claim)

- **Sati** — custom coinbase / Lightning settlement infrastructure
- **Xverse** — wallet (Lightning + onchain address co-derivation)
- **Jan from Xverse**, **Lucha from Sati** — named collaborators

## Status at writing (May 2025)

- Beta, no block found yet
- No public source code at writing (later released as `parasitepool/para` — see [[2026-05-26-parasitepool-para-github]])
- Phased open-sourcing planned

## Positioning vs existing pools

Founder explicitly positions Parasite as a new category — "warrants a new terminology" — distinct from:
- **FPPS / PPS+** (no operator-borne variance smoothing here)
- **PPLNS classic** (no rolling N-window — full inter-block share log)
- **TIDES / OCEAN** (no on-chain coinbase-output transparency; custodial Lightning fanout)
- **ckpool solo** (block finder gets only 1 BTC of 3.125 BTC, residual to others — vs. solo's full reward to finder)

## Why ingestion-worthy

Only public document explaining the **why** of the 1-BTC-finder + Lightning rail design. Cited by BitDevsNYC Socratic #147 and SF Bitcoin Devs socratic.

## See also

- [[2026-05-26-parasitepool-para-github]] — canonical Rust + ckpool fork implementation
- [[2026-05-26-blockspace-media-parasite-emerges]] — independent technical narrative
- [[2026-05-26-coindesk-parasite-second-block]] — mainnet validation
- [[2026-05-26-bitcoin-manual-parasite-pool]] — practitioner-economic critique
