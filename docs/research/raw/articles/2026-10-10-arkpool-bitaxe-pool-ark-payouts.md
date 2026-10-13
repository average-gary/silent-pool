---
title: "ArkPool — first live mining pool paying miners over Ark (bark, Second mainnet server), hourly, 1-sat minimum"
url: https://arkpool.org/
source: "https://arkpool.org/ (dashboard snapshot 2026-10-10 13:17 UTC) ; https://gitlab.com/arkpool/arkpool-gateway (README, CHANGELOG; created 2026-10-02, v0.1.1-alpha.1 2026-10-08) ; https://stacker.news/items/1587244 (2026-10-02) ; https://thebitcoinmanual.com/articles/arkpool/ (2026-10-02)"
type: articles
category: precedent
author: "@PiHiker (anonymous operator); The Bitcoin Manual (secondary)"
date: 2026-10-02
ingested: 2026-10-10
research_path: applied+news+contrarian
volatility: hot
quality: 4
credibility: medium
confidence: medium
verified: 2026-10-12
verification_note: "Direct fetch of arkpool.org and the gateway README on 2026-10-12 confirmed the agent-reported claims first-hand; see the dated section below for deltas."
bias_note: "Self-published alpha by an anonymous operator; closed-source backend; corroborated by three independent agents and one secondary write-up."
tags: [arkpool, ark, bark, mining-pool, bitaxe, hourly-payouts, sub-threshold, precedent, second-tech, tor, stratum-v1]
summary: "Launched 2026-10-02: an anonymous 'hashpower purchasing service' that pays Bitaxe-class miners hourly over Ark using bark against Second's mainnet server. Stratum username = the miner's bark-format ark1 address (Pubkey policy + server mailbox on Second's server; other servers/formats refused). 1-sat minimum, fractions carry forward, caps of 50 sats/miner/hour and 250 sats pool-wide exposure. All-time paid 1,362 sats as of 2026-10-10. Explicitly 'not affiliated with or endorsed by Second'. Proves the plumbing (loop of single arkoor sends to miner-supplied addresses, offline delivery via mailbox) at trivial scale."
---

# ArkPool — the live precedent for Ark mining payouts

## What it is

- "ArkPool uses Bark from Second to send Ark payments"; "Miners receive bitcoin sats over Ark, not Lightning." Dashboard badge: "MAINNET ALPHA — Powered by Bark".
- Model: a **"hashpower purchasing service"**, not PPLNS/TIDES. "Alpha rates are not published." Hashpower destination undisclosed.
- Topology: Bitaxe / SHA-256 miners → home Docker **gateway** (MIT, `gitlab.com/arkpool/arkpool-gateway`) → Stratum V1 **over Tor** → pool. "Tor does not make the service trustless: the operator still verifies work and sends payouts." Backend, accounting and website are private: "Pool accounting, Ark payments, and the website are separate services." Gateway "never holds wallet keys or makes payments."
- README (~line 519): **"ArkPool is an independent project, not affiliated with or endorsed by Second."**

## Payout mechanics

- Username = miner's `ark1…` address, password `x`. Requirement: "a Bark-format mainnet address with a public-key policy and server mailbox on its payout Ark server; unsupported formats or servers are refused." I.e. the miner must be on **Second's mainnet server**; relies on bark's server-mailbox delivery for offline recipients. No Lightning addresses accepted.
- Settlement "Hourly at :00 UTC"; "Minimum payout 1 sat"; "Fractions carry forward until at least one sat is payable."
- Caps (Bitcoin Manual): **50 sats per miner per hour** (max 1 200 sats/day); 250 sats pool-wide unattended exposure.
- Operator on SN: "hourly payments have been 1-3 sats for a Bitaxe 601"; "Eligible payouts start at 1 sat"; cites "Second currently lists 0% for Ark sends and receives" as the reason 1-sat payouts are economic.

## Scale (dashboard 2026-10-10)

Total paid all-time **1 362 sats**; last hourly payout 9 sats; paid last 24 h 223 sats; unpaid balance across all miners 2.93 sats; 0 active miners / 0 TH/s in the preceding hour. (Bitcoin Manual, Oct 3: one active miner, 124 sats paid total.) Gateway repo: 56 commits, no tags, Umbrel exporter experimental, StartOS package blocked.

## Caveats the sources themselves raise

- Bitcoin Manual: VTXOs expire in ~28–30 days (miner must refresh); "amounts under 330 sats can't be redeemed on-chain through the normal relay network, so very small balances aren't safe if the server vanishes."
- SN pushback: "We _have_ functioning pools. What does this do that regular pools don't?" — answer offered: hourly cadence, Tor privacy, 1-sat minimum.
- Because ArkPool pays from a bark wallet against Second's server, **payer ≠ ASP** — miners inherit Second's liveness, DoS exposure and surveillance, but keep the 1-of-2 honesty assumption.

## Direct fetch 2026-10-12 — verification and deltas

Fetched `https://arkpool.org/` (dashboard snapshot **2026-10-12 17:17 UTC**) and `gitlab.com/arkpool/arkpool-gateway/-/raw/main/README.md` directly.

**Confirmed verbatim**: "Bark from Second powers the pool's Ark payments"; "Independent project; no affiliation or endorsement implied"; README: "ArkPool is independent and is not affiliated with or endorsed by Second or the Tor Project." Username = "your complete compatible mainnet `ark1...` receive address", password `x`; gateway "Preserves your Ark receive address byte-for-byte as the Stratum username." Hourly at `:00` UTC ("timing may vary"); "1 sat … Fractions carry forward." MIT, "copyright (c) 2026 PiHiker."

**Deltas since 2026-10-10**:

| Metric | 2026-10-10 | 2026-10-12 |
|---|---|---|
| Total paid all-time | 1 362 sats | **1 955 sats** |
| Paid last 24 h | 223 | 280 |
| Last hourly payout | 9 | 12 |
| Unpaid balance (all miners) | 2.93 | 5.88 |
| Active miners (1 h) | 0 | **4** |
| Avg hashrate (1 h) | 0 | **6.82 TH/s**; share acceptance 100 % |
| Gateway version | 0.1.1-alpha.1 | **0.1.2-alpha.1** |

**New details not in the agent reports**:
- **Payout receipts**: "Confirmed payouts include signed, hash-chained accounting receipts that the dashboard verifies in your browser." Public stats are "aggregate only: no wallet seeds, wallet access, or public miner directory." A read-only Tor dashboard (onion address) exists. This is a post-hoc receipt scheme — the operator attests what it paid — not a share-window audit of the kind the SV2 share-accounting extension (type 32) provides.
- **Explicit refusals**: "Lightning invoice, Lightning address, on-chain Bitcoin address, or wallet seed"; mainnet only — "Signet testing guide" explicitly not supported; "Address compatibility is checked when connecting; not every Ark wallet or address format is supported."
- **Trust statements**: "The gateway does not create wallets, claim funds, or verify pool accounting." "An accepted share is not proof of payment." "Sends pool connections only through Tor; there is no clearnet fallback"; two Tor clients "with bounded failover and optional faster-path preference"; miners separated into Tor streams "without logging their source IPs."
- **Supported firmware**: Bitaxe 601, 601a, 403 guides; Docker Engine/Compose v2 recommended, Podman/Umbrel/StartOS experimental.

**Downgraded**: the **50 sats/miner/hour cap** and the 250-sat pool-wide exposure figure appear only in The Bitcoin Manual (Oct 2) and are **not on the site or in the README as of 2026-10-12** — treat as unverified or dropped.

## Why it matters for this wiki

It is the only pool paying over Ark as of 2026-10-10 and the first concrete instance of the pattern the wiki had only theorised: server-side bark wallet, ledger of fractional credits, scheduled cron issuing **one arkoor send per miner** to miner-supplied `ark1` addresses, offline delivery via the mailbox. It also fixes the shape of the identity field problem (the Ark address *is* the username) and shows a cadence-based rather than amount-based payout policy.

## Source criticism

Anonymous operator, closed backend, no published rates, volumes in the hundreds of sats per day — evidence that the plumbing works, not that the model scales or is honest. Corroborated across arkpool.org, the gateway README/CHANGELOG, Stacker News and The Bitcoin Manual. Second has not acknowledged it.

## See also

- [[2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs|OCEAN threshold and BOLT12 precedent]]
- [[../repos/2026-10-10-bark-barkd-openapi-source-boat001|bark sender API (why it's one send per miner)]]
- [[../../wiki/concepts/parasite-pool|Parasite Pool]] — the Lightning-custody analogue
