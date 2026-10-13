---
title: "Understanding SLICE (PPLNS+JD)"
author: esraa (DMND)
publication: blog.dmnd.work
date: 2025-03-18
source: https://blog.dmnd.work/understanding-slice-pplns-jd/
related_url: https://blog.dmnd.work/slice-making-pplns-work-for-demand-response/
type: articles
ingested: 2026-05-23
quality: 5
credibility: high
confidence: high
tags: [dmnd, SLICE, pplns-jd, N-parameter, primary-source]
summary: "Closes the gap on SLICE's N parameter."
---

# DMND SLICE — Primary Spec (March 2025)

Closes the gap on SLICE's N parameter.

## N parameter (verbatim)

> *"DMND Pool uses the Bitcoin's network difficulty and multiplies it by 8 to create the look back window."*

So **N = 8 × Bitcoin network difficulty** — same multiplier as TIDES.

A second post (April 2025, "SLICE: Making PPLNS Work for Demand Response") confirms from a different angle:

> *"each valid share remains eligible for payout across the next 8 blocks on average … an 8-block rolling window."*

## Other published parameters

- Share-difficulty target: **~6 shares/minute** per client.
- Payout cadence: per-block, weighted by the rolling 8×D share window.
- Each share is paid across ~8 successive blocks on average.

## What is NOT published

- **Pool fee schedule** for the BTC subsidy/fee leg — not on dmnd.work, blog.dmnd.work, GitHub, or delvingbitcoin.
- **Minimum payout / withdrawal threshold** — not published.
- KYB onboarding required (per DMND's 2026-05-12 changelog) — implies fee terms may be **negotiated/contractual** rather than published.

## rBTC side-payout (closest fee-related proxy, May 2026 post)

From "Miners build the block. Miners keep the rBTC." (2026-05-15):

> *"The rBTC reward lands at an address the miner controls. There is no opt in beyond running SV2 JD through DMND. There is no revenue share. There is no pool wallet sitting in the middle."*

Rootstock pays "roughly 79 to 80 percent of transaction fees to miners, in rBTC." So:

- **rBTC leg**: explicit 0% pool cut.
- **BTC leg**: pool fee undisclosed.

## Theoretical paper (lorbax/pplns-with-job-declaration, GitHub)

The companion theoretical paper says:

> *"N is not specified, but is suggested to be such that the sum at the denominator is a multiple [of] the bitcoin difficulty (for Ocean TIDES this multiple is 8)."*

Worked example: `N = 100 shares/min · 8 · 10min = 8000 shares`.

DMND's deployed multiplier matches the paper's TIDES-aligned suggestion.

## Operational

- Reject rate: **0.0151%** (vs 0.5–2% for typical Stratum V1 pools), per DMND Technical Update 2026-01-22.

## Significance

**TIDES (OCEAN) and SLICE (DMND) use the same N = 8 × D parameter.** This is the production consensus for "PPLNS done right with non-custodial coinbase payout" — independent convergence across two non-custodial pool designs.

## See also

- [[2026-05-23-dmnd-demand-pool|DMND landing-page article]]
- [[2026-05-23-ocean-tides-spec|OCEAN TIDES spec]]
- [[../../wiki/concepts/pplns-jd|SLICE / PPLNS-JD concept article]]
- [[../../wiki/concepts/tides|TIDES concept article]]
