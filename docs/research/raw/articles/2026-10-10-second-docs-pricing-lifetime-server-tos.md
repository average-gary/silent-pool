---
title: "Second docs — pricing tiers, VTXO lifetime, rounds & delegated refresh, exit economics, captaind self-hosting, SDK, Terms of Service (Oct 2026)"
url: https://second.tech/docs
source: "https://second.tech/pricing ; https://second.tech/docs/learn/lifetime.md ; /docs/learn/payments.md ; /docs/learn/rounds.md ; /docs/learn/exit.md ; /docs/learn/liquidity.md ; /docs/learn/fees.md ; /docs/ark-server/manual-setup.md ; /docs/bark-sdk/rust.md ; /docs/bark-wallet/settings.md ; /docs/barkd/api-reference/wallet/send-a-payment.md ; /docs/swaptopia/fees-and-limits.md ; https://second.tech/terms (rev. 2026-08-10) ; https://arkadeos.com/terms-of-service"
type: articles
category: vendor-docs
publisher: Second (Second Technologies)
date: 2026-10-10
ingested: 2026-10-10
research_path: technical+economics+contrarian
volatility: hot
quality: 5
credibility: high
confidence: high
bias_note: "Vendor documentation; fee and trust statements are the operator's own framing."
tags: [ark, bark, second-tech, pricing, fees, vtxo-lifetime, refresh, delegated-refresh, exit, captaind, self-hosting, terms-of-service, sweep, trust-model, mining-payouts]
summary: "Second's hosted Ark server charges 0% on Ark sends and receives; refresh costs 0% only in the last 2 days of a 28-day VTXO life (0.2%/0.4%/0.5% at <7/<14/≥14 days left), so the rational default is to sit ~26 days under sender+server trust. Lightning-receive VTXOs live 1–3 days. Arkoor security is stated as 'at least one party (either sender or Ark server) remains honest'. Delegated refresh is 'dependent on the Ark server's signature only' and must be scheduled while online. Expired VTXOs can be swept and the ToS says 'WE HAVE NO OBLIGATION TO RETURN IT'. captaind is officially self-hostable (PostgreSQL, Bitcoin Core ≥ v31)."
---

# Second docs — what a pool integrator needs to know (snapshot 2026-10-10)

## Pricing (Second's hosted mainnet server, `second.tech/pricing`)

| Operation | Fee |
|---|---|
| Ark (arkoor) send | **0 %** |
| Ark receive | **0 %** |
| Refresh (wallet maintenance) | **0 % if < 2 days to expiry**; 0.2 % if < 7 d; 0.4 % if < 14 d; 0.5 % if ≥ 14 d (plus the server's on-chain share, priced at ~221 vB per three-output refresh tx as of 0.7.1) |
| Lightning send | 0.2 / 0.4 / 0.5 % by the same time-to-expiry tiers, **minimum 20 sats** |
| Lightning receive | 0 % |
| On-chain send from Ark balance | 0.2–0.5 % + on-chain fee |
| Board (on-chain receive) | on-chain fee only |
| Emergency exit | "multiple on-chain fees" |

Quote: "No cost to hold funds on Ark indefinitely as long as you always refresh within two days of expiry." Implied annualised opportunity rate of the tiers: 0.5 %/(28/365) ≈ 6.5 %/yr on a fresh VTXO; 0.5 % at 14 d left ≈ 13 %/yr — above the docs' 5 % worked example (`liquidity.md`: 100 000 sats × 5 d × 5 % ≈ 68 sats). No explicit `opportunity_rate` is published ("hybrid approach").

## Lifetime and expiry (`learn/lifetime.md`, `learn/exit.md`)

- Board/refresh VTXOs **last 28 days**; **Lightning-received VTXOs last 1–3 days** (server-configurable). Rounds on Second's server: **every hour**.
- "When spending over Ark, the recipient's VTXO inherits the original expiry, and with it the responsibility to refresh."
- After expiry: "An expired VTXO isn't lost but becomes restricted. Users can still perform refreshes, Lightning sends, and on-chain sends, but cannot make Ark payments or use emergency exits. The server then sweeps the underlying on-chain bitcoin to its own wallet." The sweep spends "the root output that the user's pre-signed exit transactions depend on, rendering them invalid."
- Exit economics: "If the balance held in a VTXO is less than the total on-chain fees required to broadcast all exit transactions, it may not be economically viable to recover those bitcoin." Exit needs the wallet DB: "losing this data without a backup of your wallet data would mean permanently losing access to the exited bitcoin."
- Bark Wallet default: auto-refresh when remaining lifetime < 1 day (`vtxo_refresh_expiry_threshold` 144 blocks on mainnet) — "keeps automatic refreshes inside the free window". "Refresh on receive" is **Off** by default ("Maximum security, at a significantly higher running cost").

## Arkoor trust model (`learn/payments.md`)

- "Payments remain secure as long as at least one party (either sender or Ark server) remains honest."
- "Users can always detect double-spending when multiple recipients attempt to refresh, offboard, or perform an emergency exit." Remedy is reputational: "Duplicate signatures can easily be publicly proven."
- Exit doc: "the exit can be prevented if the sender and Ark server collude to double-spend. The exit succeeds if it fully confirms before the VTXO expires and either the sender or the server acts honestly."
- Refresh guidance is qualitative: large payments → "Refresh immediately after receiving payment"; small → wait "until their VTXOs are close to expiring". Change "inherits trust properties from the source VTXO"; chains "increase exit costs and expand the trust surface area".

## Delegated refresh (`learn/rounds.md`)

Standard refresh "requires the wallet app to be online during scheduled round windows." Delegated: "the wallet app submits refresh intents ahead of time … Designated co-signers then sign on the user's behalf during the round"; "1-of-n trust model"; "entirely opt-in". **"Currently, delegated refreshes will be dependent on the Ark server's signature only"**; co-signer support "unlikely to be completed until after our mainnet launch". "If a covenant proposal (like CTV or TEMPLATEHASH) were adopted on bitcoin, delegated refreshes would become fully trustless."

## Running an Ark server (`ark-server/manual-setup.md`, Docker Compose page)

Binary **`captaind`** ("Download the latest `captaind` binary from the releases page" or `cargo install --locked --path ./server`). Dependencies: PostgreSQL, Bitcoin Core (v30.0+ in docs; **0.7.1 changelog: requires v31**), optional Core Lightning. `captaind create --config <toml>` / `captaind start --config <toml>`; config carries network, postgres, bitcoind RPC, public addr `127.0.0.1:3535`, admin `3536`; fund via `captaind rpc wallet | jq -r '.rounds.address'`. "Currently, only Unix-based systems (Linux, macOS) are supported." Warning: **"Operating an Ark server can be significantly more involved than running a Lightning node."**

## SDK (`bark-sdk/rust.md`)

Rust crate **`bark-wallet`** (`cargo add bark-wallet`): `Wallet::create(&mnemonic, network, config, db: Arc<SqliteClient>, false)`, `wallet.new_address()`, `wallet.sync()`, `wallet.balance()`. UniFFI bindings (bark-ffi): React Native, Dart/Flutter, Swift, Kotlin, Go; WASM for web. barkd typed clients TypeScript and C#; Rust `bark-rest-client`.

## Swaptopia limits (same server, useful as published mainnet ceilings)

Min BTC→LN swap 51 000 sats; max per swap 10 000 000 sats; max LN receive 2 750 000 sats; on-chain payout min 1 000 sats after fees; LN payout min 1 sat; 0.5 % service fee; 20-sat LN minimum.

## Terms of Service (Second, rev. 2026-08-10; Arkade for contrast)

- Second: "do not take custody or control of bitcoin held in VTXOs; do not act as your agent, trustee, or intermediary" — and: "UPON EXPIRY OF A VTXO, THE PROTOCOL PERMITS THE OPERATOR … TO SWEEP THE BITCOIN"; **"IF YOUR WALLET REMAINS OFFLINE OR INACTIVE FOR TOO LONG AND A VTXO EXPIRES, ANY BITCOIN REMAINING IN IT MAY BE CLAIMED BY US, AND WE HAVE NO OBLIGATION TO RETURN IT TO YOU."** "It is your responsibility to ensure that your wallet comes online frequently enough." Delaware law, JAMS arbitration. **No privacy policy is linked anywhere on second.tech; no data-retention clause.**
- Arkade: "does not take possession, custody, or control"; Liquidity Provider "reclaiming forfeited VTXO outputs after expiry"; Estonian law; no MSB/VASP statement.

## Source criticism

All first-party. The 0 % arkoor fee is a pricing decision, not a protocol property, and the server has `max_arkoor_amount` / per-action limits whose mainnet values are unpublished. The "isn't lost but becomes restricted" post-expiry regime is at the server's discretion and sits uneasily with the ToS's "no obligation to return".

## See also

- [[../repos/2026-10-10-bark-barkd-openapi-source-boat001|bark source read]]
- [[2026-10-10-ark-oor-trust-payer-equals-asp-critiques|Critiques: payer == ASP, delegation, privacy]]
- [[../../../covenantless-ark/wiki/concepts/vtxo-lifetime-and-expiry|covenantless-ark: VTXO lifetime and liquidity economics]]
