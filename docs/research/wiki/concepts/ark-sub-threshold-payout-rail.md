---
title: "Ark (bark) as a Sub-Threshold Payout Rail"
category: concept
created: 2026-10-10
updated: 2026-10-12
verified: 2026-10-12
confidence: medium
volatility: hot
tags: [ark, bark, arkoor, sub-threshold, payout-rail, pending-credits, second-tech, captaind, mailbox, vtxo-expiry, trust-model, payer-equals-asp, silent-pool, arkpool, dual-rail-identity]
aliases: [Ark payout rail, arkoor payouts, sub-threshold Ark payouts, dual-rail payouts]
sources:
  - raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md
  - raw/articles/2026-10-10-second-docs-pricing-lifetime-server-tos.md
  - raw/articles/2026-10-10-arkpool-bitaxe-pool-ark-payouts.md
  - raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md
  - raw/articles/2026-10-10-bark-releases-oct5-exploit-ecosystem-jul-oct-2026.md
  - raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md
  - raw/data/2026-10-10-sub-threshold-payout-economics-tables.md
summary: "Pay earnings above a threshold as a coinbase output and everything below it as an Ark out-of-round (arkoor) payment from a pool-held bark wallet. The plumbing exists and is in production at trivial scale (ArkPool, Oct 2026): one `POST /api/v1/wallet/send` per miner, 0 % fee on Second's server, offline delivery via the server mailbox to never-connected recipients. The constraints are structural, not implementation bugs: Ark addresses are server-bound (130/131 chars), every payout inherits the pool's VTXO expiry (~28 days), the rational miner sits ~26 days under sender+server trust, swept VTXOs are gone, and if the pool runs its own Ark server the 1-of-2 honesty assumption collapses to one party. Verdict: viable as a low-value rail on a third-party server, with the threshold set by what a miner can afford to lose to a sweep rather than by coinbase weight alone."
---

# Ark (bark) as a Sub-Threshold Payout Rail

> **The question this answers**: a coinbase-direct pool (TIDES / SLICE / [[../topics/self-blinding-pool-design-space|SPc]]) cannot put every miner in every coinbase. OCEAN parks sub-threshold earnings as unpaid balance and offers BOLT12; Blitzpool carries them as pending credits. Could the sub-threshold rail be **Ark**, with the pool paying from a bark wallet? Working assumption examined here: **≥ 500 000 sats → coinbase output (silent payment); < 500 000 sats → arkoor payment.**

## 1. What exists today

**A precedent, as of 2026-10-02.** [ArkPool](../../raw/articles/2026-10-10-arkpool-bitaxe-pool-ark-payouts.md) pays Bitaxe-class miners **hourly over Ark via bark against Second's mainnet server**, 1-sat minimum, fractions carried forward, 50 sats/miner/hour cap. The miner's `ark1…` address *is* the Stratum username, preserved "byte-for-byte" by the Tor gateway. It is anonymous and closed-backend; as of a direct fetch on 2026-10-12 it had paid **1 955 sats all-time to 4 active miners at 6.8 TH/s** — evidence that the plumbing works, not that it scales. It also ships **signed, hash-chained payout receipts** verified in the miner's browser — a post-hoc attestation by the operator, which is the lightweight cousin of (not a substitute for) the type-32 share-window audit silent-pool serves. The site's own warning is worth keeping: "An accepted share is not proof of payment." It is explicitly "not affiliated with or endorsed by Second", and Second's own blog, docs, GitHub issues and talks contain **no mining mention** July–October 2026 ([ecosystem timeline](../../raw/articles/2026-10-10-bark-releases-oct5-exploit-ecosystem-jul-oct-2026.md)).

**The sender API is one call per miner.** barkd's only Ark send is `POST /api/v1/wallet/send { destination: "ark1…", amount_sat }`; the Rust equivalent is `Wallet::send_arkoor_payment(&Address, Amount)`. There is **no batch arkoor endpoint** (only on-chain `send-many`) and **no fee parameter**. An arkoor transaction takes **exactly one input VTXO**; if no single VTXO covers the amount bark chains several payments and the miner receives several VTXOs ([bark source read](../../raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md)).

**Fees on Second's server: 0 % to send, 0 % to receive.** Refresh is 0 % only in the last 2 days of a VTXO's life (0.2 / 0.4 / 0.5 % at < 7 / < 14 / ≥ 14 days left) ([Second docs](../../raw/articles/2026-10-10-second-docs-pricing-lifetime-server-tos.md)). This is what makes 1-sat payouts economic where Lightning (20-sat minimum fee on Second; OCEAN's BOLT12 needs inbound liquidity) and on-chain (330-sat dust) cannot be.

## 2. Structural constraints a pool inherits

| Constraint | Source | Consequence for a pool |
|---|---|---|
| **Addresses are server-bound** — `ark_id` = first 4 bytes of SHA256(server pubkey); receivers reject VTXOs "committing to a foreign server"; no cross-server arkoor | bark `address.rs` | Every miner must run a bark wallet pointed at *the pool's chosen server*. ArkPool refuses other servers for this reason. |
| **Minting an address needs the server's `mailbox_pubkey`** (blinded mailbox id = ECDH with the server key) | bark `mailbox.rs` | A miner must fetch `ArkInfo` once from that server; "never connected" is possible only after that one fetch. |
| **No recipient-existence check** on `PostArkoorMessage` | server mailbox handler | The pool can pay a miner who has never registered; delivery waits in the mailbox. (Bug fixed 0.7.0: before Sep 2026 bark could "drop Ark payments that arrive and expire while the receiver is offline".) |
| **Recipient inherits the sender's `expiry_height`** — "All output VTXOs inherit the input's expiry_height" | bark `arkoor/mod.rs`; docs: "inherits the original expiry, and with it the responsibility to refresh" | A payout made from a 20-day-old pool VTXO gives the miner ~8 days. The pool must pay from **freshly refreshed** VTXOs to hand miners a full ~28 days. |
| **One input per arkoor; change split into a tree** (`change_vtxo_split_factor` default 2; server fan-out cap 4; `max_input_exit_depth`) | bark 0.6.2 / 0.7.1 | A high-volume payer's VTXOs reach the depth limit before expiry and must be refreshed in a round — rounds are hourly on Second. |
| **Per-action amount limits** (`max_arkoor_amount` etc., 0.7.1); mainnet values unpublished | server config | Large sub-threshold batches may hit caps; nothing to size against until published. |
| **Second recommends hourly, not per-minute, cadence** — per-minute "build long exit chains … prohibitively expensive to exit on small balances" | community.second.tech, Aug 2026 | Pay once per block or once per hour, never per share or per template. |
| **Dust floor 330 sats** — below it a VTXO cannot be exited on-chain through the relay network | bark dust checkpoint; Roose | Sub-330 balances are safe only while the server lives. Carry them forward instead. |
| **Address length 130 (`ark1`) / 131 (`tark1`) chars** with mailbox delivery | bark test vectors | An SV2 `user_identity` carrying both an SP address (116/117) and an Ark address is 248–249 chars — **inside `STR0_255` by 6–7 chars.** |

## 3. The trust model, and why the pool must not be the server

Second states arkoor security as: **"Payments remain secure as long as at least one party (either sender or Ark server) remains honest."** Detection of a double-spend is post hoc ("when multiple recipients attempt to refresh, offboard, or perform an emergency exit") and the remedy is reputational. The [covenantless-ark wiki](../../../covenantless-ark/wiki/concepts/out-of-round-payments.md) already records this as the statechain-like caveat on all OOR payments.

For a mining pool the sharp question is **what happens when the pool is also the Ark server**, which the sibling plan [JDC-as-Ark-payer](../../../ark-boarding-sv2-mining/output/plan-jdc-ark-payer-external-asp-2026-07-20.md) had defaulted to (self-host `captaind`). The one primary treatment of exactly that configuration — ademan's equivocation-bonds post ([critiques](../../raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md), Delving 2026-08-18) — says it plainly: "the initial VTXO holder and the Ark server are the same entity … The Ark server could reassign these VTXOs arbitrarily many times," with "unfavorable sovereignty and regulatory implications." The fix needs `OP_CHECKSIGFROMSTACK`. A pool-run server can co-sign a conflicting spend of a miner's unrefreshed payout, refresh the parent into its own round, refuse the miner's refresh, or wait and sweep — and the miner's recourse is to publish duplicate signatures.

**So the sub-threshold rail is strictly less trusted on a third-party server than on the pool's own.** That inverts the self-host default. The cost of a third party is a liveness and surveillance dependency, which Second demonstrated five days ago: the **2026-10-05 boarding-flow exploit** double-spent 0.75 BTC of Second's liquidity (no user funds), took the server offline briefly, and an Oct 6 DoS degraded Lightning receives; boards were still closed on Oct 8 ([timeline](../../raw/articles/2026-10-10-bark-releases-oct5-exploit-ecosystem-jul-oct-2026.md)). The alternative implementation is not a live option: Arkade has had no release since 2026-08-25, no blog since March, and lost its Boltz Lightning bridge on Aug 3.

## 4. Liveness: the 28-day clock and the fee curve

- Board/refresh VTXOs live **28 days** (server default `vtxo_lifetime` 4 320 blocks); Lightning-received VTXOs 1–3 days. At expiry the server sweeps the root, exit paths die, and the ToS says **"ANY BITCOIN REMAINING IN IT MAY BE CLAIMED BY US, AND WE HAVE NO OBLIGATION TO RETURN IT TO YOU."** A forum case confirms a swept 175 035-sat VTXO was unrecoverable.
- The **fee curve pushes miners to maximum exposure**: refresh is free only in the last 2 days, so the economically rational miner holds an arkoor payout ~26 days under sender+server trust; bark's "refresh on receive" is off by default ("significantly higher running cost").
- **Delegated refresh does not rescue a dormant miner.** It must be scheduled while online, is "dependent on the Ark server's signature only" (co-signers unbuilt), and Ark Labs' own delegation yields "preconfirmation state" only. There is no proposal that refreshes a fully offline recipient without trusting the server; Erk / hArk-CTV async receiving needs a soft fork with zero mainnet activation movement.
- Net: a miner on the Ark rail **must come online at least once per 28 days** or forfeit that month's sub-threshold earnings to the server. A quarterly hobbyist is swept three times a year.

## 5. Where the threshold should sit

The 500 000-sat assumption is in the right neighbourhood but the number should be derived, not assumed — see [[payout-threshold-economics|Payout Threshold Economics]] for the tables. Three anchors:

1. **Precedent.** OCEAN's coinbase threshold is 2^20 = **1 048 576 sats** (discretionary ≥ 65 536), explained by its ~16-slot firmware-safe coinbase; Braiins' Lightning rail runs **1 sat to 500 000 sats** — the proposed cut coincides with Braiins' Lightning ceiling.
2. **Pool size, not miner size, decides who is sub-threshold.** At 132.7 T difficulty and 3.125 BTC, a miner is below 500k sats per block whenever it is **< 0.16 % of pool hashrate** — a single 100 TH/s ASIC clears the line only in pools ≤ ~60 PH/s; in a 1 EH/s pool everything under 1.6 PH/s is on the Ark rail every block.
3. **Loss tolerance, not block weight, is the binding constraint on the Ark side.** A coinbase output costs 43–860 sats of block space (1–20 sat/vB) and nothing in trust; an Ark payout costs 0 sats and up to 100 % of itself if the miner misses a 28-day window, plus 0.2–11 % to exit unilaterally if the server turns hostile. The threshold is therefore the amount a *typical* sub-threshold miner is willing to have at risk per month — which argues for keeping it **low** (hundreds of thousands of sats, not millions) and for a pool that already has a weight-budget trim ([[lottery-pplns|Blitzpool pattern]]) to treat the threshold as a **floor on pending credits**, not a replacement for the trim.

## 6. Privacy interaction with SPc

The coinbase leg (SPc) hides *which output pays which miner* from chain observers only; the pool already knows ([[payout-attribution-privacy|Payout Attribution Privacy]]). The Ark leg adds a **third party**: the Ark server sees the pool wallet → every miner address → every amount, with the mailbox "private from the public, but not from the Ark server", hArk forfeits hash-linked, and **no privacy policy published by Second**. **SP-deriving the Ark receiver key was evaluated and rejected** ([key-side code read](../../raw/repos/2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md)): the `Pubkey` policy accepts any key and the mailbox blinding is linear enough that a pool could mint the address from `M + D + t·S_m`, but (i) VTXO keys are hot by construction, so `P' = B_spend + t·G` would force `b_spend` hot; (ii) the server unblinds every delivery to the per-seed mailbox, so fresh keys hide nothing from the only observer of off-chain VTXOs; (iii) the miner still has to supply its mailbox pubkey and a static ECDH point — not a single identity. Single-seed recovery is better obtained by deriving the bark seed from the SP master. The amount channel ([[coinbase-amount-linkability|Coinbase Amount Linkability]]) stays open on both rails.

## 7. Alternatives, and when they win

| Rail | Wins when | Loses because |
|---|---|---|
| **Ark (bark, third-party server)** | sub-dust-to-500k amounts, miner online monthly, pool wants 0-fee instant settlement without running Lightning | 28-day sweep risk; server-bound addresses; single-server liveness; third-party surveillance |
| **BOLT12 Lightning** ([OCEAN precedent](../../raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md)) | miner runs a node with inbound liquidity | client compatibility is fragile (Alby Hub ≥ 1.21.2 broken since Mar 2026); needs inbound liquidity; retried every block |
| **Cashu eHash** ([[ehash|hashpool]]) | privacy, zero liquidity | explicit IOU on the mint |
| **Pending credits only** ([[../topics/payout-design-space|Blitzpool]]) | the pool is already carrying `pending_balance` | pool custody until paid; a small miner may wait months |

Second's own staff concede "Lightning can still be better for streaming payment applications"; Corallo's critique is that Ark is only low-trust "when full Lightning channels sit on top."

## 8. Design recommendations for silent-pool

1. **Use a third-party Ark server (Second's) for the rail; do not run `captaind` as the payer.** Payer == ASP removes the only honesty assumption arkoor has (§3).
2. **`user_identity` stays the SP address alone; the Ark address travels in a project-defined SV2 extension** (`SetArkPayout`, rotatable at will, validated fail-closed against the configured server's `ark_id`). The dual-rail-in-`user_identity` variant (248–249 chars, fits `STR0_255`) works but is unnecessary once the miner side is our own binary. Derive the miner's bark seed from its SP master so one backup recovers both rails. See [spec §11](../../output/plan-silent-pool-spec-2026-10-09.md).
3. **Pay sub-threshold credits once per block found (or hourly), one `send` per miner, from VTXOs refreshed within the last few days**, so miners inherit a near-full 28-day window. Never per share.
4. **Keep the weight-budget trim; add the threshold as a policy floor**: `owed ≥ T` → coinbase candidate (subject to trim); `dust ≤ owed < T` → Ark; `owed < dust` → carry forward. Start `T` at or below 500 000 sats and publish it.
5. **Tell miners the deadline**: surface each payout's `expiry_height` in the audit API and in the type-32 receipt; recommend "refresh on receive" for anyone who will be offline > 3 weeks.
6. **Hold the float on Second, not on-chain**: board the pool output (`pay_P`) after maturity; size float to the sub-threshold hashrate share × ~3.13 BTC per expected block; refresh inside the free window.
7. **Record the residual custody honestly**: pending credits sit in `pay_P` until paid (already in the spec §4.6); Ark payouts are "trustodial" for up to 28 days; neither is non-custodial to the sat.

## Open questions

- Second's mainnet values for `max_arkoor_amount`, mailbox retention and throughput — unpublished; needed before sizing a 1 EH/s pool's hourly batch.
- Whether a BOAT-standard (`instagibbs/boats`) will fix the address format so a second server implementation becomes a drop-in.
- ~~Whether an SP-derived Ark receiver key is worth specifying~~ — **closed 2026-10-12**: feasible with a wallet-side key table, not worth it (hot-key mismatch, per-seed mailbox, still needs Ark-specific material). Recorded in spec §11.8.
- Regulatory classification of a pool that routes payouts through an Ark server — no primary text exists ([critiques](../../raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md) §5).

## See also

- [[payout-threshold-economics|Payout Threshold Economics]] — the tables behind §5
- [[ark-for-mining-payouts|Ark for Mining Payouts]] — the May 2026 survey this supersedes in part (status updated Oct 2026)
- [[parasite-pool|Parasite Pool]] — Lightning-custody analogue
- [[ehash|eHash / hashpool]] — Cashu analogue
- [[tides|TIDES (OCEAN)]] — the 2^20 threshold and BOLT12 fallback
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]] — the SPc coinbase leg
- [[../../output/plan-silent-pool-spec-2026-10-09|Spec: silent-pool]] — §4.6 pending ledger and §10 O6 are the integration points
- [[../../../covenantless-ark/_index|covenantless-ark]] — protocol mechanics (July 2026 snapshot; the Oct 2026 sources here supersede its status claims)
- [[../../../ark-boarding-sv2-mining/output/plan-jdc-ark-payer-external-asp-2026-07-20|JDC-as-Ark-payer plan]] — prior plan whose self-host default §3 reverses

## Sources

- [bark 0.7.1 source read](../../raw/repos/2026-10-10-bark-barkd-openapi-source-boat001.md) — send API, arkoor rules, address format, mailbox semantics
- [Second docs snapshot](../../raw/articles/2026-10-10-second-docs-pricing-lifetime-server-tos.md) — fees, lifetime, trust statement, delegated refresh, captaind, ToS
- [ArkPool](../../raw/articles/2026-10-10-arkpool-bitaxe-pool-ark-payouts.md) — the precedent
- [OCEAN threshold and coinbase outputs](../../raw/articles/2026-10-10-ocean-payout-threshold-bolt12-coinbase-outputs.md) — 2^20, BOLT12, measured outputs, Braiins/Luxor
- [Releases, exploit, ecosystem](../../raw/articles/2026-10-10-bark-releases-oct5-exploit-ecosystem-jul-oct-2026.md) — 0.4.0–0.7.1, Oct 5 incident, cadence guidance, Arkade stall
- [Critiques](../../raw/articles/2026-10-10-ark-oor-trust-payer-equals-asp-critiques.md) — payer == ASP, delegation, privacy, alternatives
- [Economics tables](../../raw/data/2026-10-10-sub-threshold-payout-economics-tables.md) — 0.16 % rule, cost comparison, float sizing
