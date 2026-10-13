---
title: "Ark arkoor trust critiques — payer == ASP collapse (ademan), delegation caveats (Adios Expiry, keymeld #48), privacy (Sovrandy), 'trustodial' framings (Seth, Corallo), alternatives"
url: https://delvingbitcoin.org/t/improving-the-security-of-lark-oor-channels-with-equivocation-bonds/2816
source: "https://delvingbitcoin.org/t/improving-the-security-of-lark-oor-channels-with-equivocation-bonds/2816 (ademan 2026-08-18; replies GeorgeTsagk 08-19, ademan 08-25) ; https://arklabs.xyz/insights/adios-expiry-rethinking-liveness-and-liquidity-in-arkade (2025-07-16) ; https://github.com/tee8z/keymeld/issues/48 (2026) ; https://sovrandy.com/2026/04/19/dog-park-privacy-in-bark.html ; https://bitcoinmagazine.com/technical/spark-and-ark-a-look-at-our-newest-bitcoin-layer-twos (Seth For Privacy, 2025-10-23) ; https://stacker.news/items/1264150 ; https://tangentsdaily.substack.com/p/sunday-jun14 (Corallo, 2026-06-14) ; https://www.spark.money/research/ark-protocol-explained (2026-02-20) ; https://delvingbitcoin.org/t/ecash-tides-using-cashu-and-stratum-v2/870 (EthnTuttle 2024-05-15)"
type: articles
category: critique
date: 2026-10-10
ingested: 2026-10-10
research_path: contrarian
volatility: warm
quality: 4
credibility: medium
confidence: medium
bias_note: "Named engineers (ademan, Corallo, Seth, Naar) but forum posts and op-eds, not peer-reviewed; Spark is a competitor."
tags: [ark, arkoor, trust-model, payer-equals-asp, double-spend, delegation, adios-expiry, privacy, hark, trustodial, regulatory, critique, cashu, lightning, mining-payouts]
summary: "The one primary treatment of arkoor when sender and server are the same entity (ademan, Delving Aug 2026) says such a server 'could reassign these VTXOs arbitrarily many times' with 'unfavorable sovereignty and regulatory implications'; the proposed fix (equivocation bonds) needs CSFS. Delegated refresh is live but converts expiry risk into round-time trust in the delegate/server (Ark Labs: 'preconfirmation state' only; keymeld: operator+delegate can redirect renewed VTXOs). The ASP sees every VTXO owner and IP; hArk forfeits are hash-linked and exits can link sender↔recipient on-chain. Critics call arkoor 'trustodial' / statechain-like (Corallo). No regulator text on Ark operators exists; only ToS self-classification."
---

# Arkoor trust critiques relevant to a pool paying its own miners

## 1. Payer == ASP — ademan, "Improving the Security of lArk OOR Channels with Equivocation Bonds" (Delving, 2026-08-18)

- Names the collapse directly: "VTXO OOR transfers are secure under the assumption that the Ark server and the VTXO holder do not collude, but in the proposed scheme **the initial VTXO holder and the Ark server are the same entity**."
- What a sender-ASP can do: "The Ark server could **reassign these VTXOs arbitrarily many times**, opening LN channels not backed by real Bitcoin."
- "OOR VTXO reassignment has **unfavorable sovereignty and regulatory implications**." / "The sovereignty tradeoff may be acceptable to most users for small amounts, but unilateral control over funds creates regulatory risk for LSPs operating this way."
- Fix: on-chain slashable bond with single-use keys — needs **OP_CHECKSIGFROMSTACK**, not deployable today. Only timing guidance: wait "on the order of hundreds of milliseconds, but a delay of a few seconds" for conflicting half-proofs; no refresh-deadline numbers anywhere. ademan confirms (08-25) the scheme generalises to ordinary OOR transfers.

Inference recorded by the agent (docs + ademan): a sender-ASP can co-sign a conflicting spend of the same VTXO to itself or another miner, refresh the parent VTXO into its own round (invalidating the recipient's pre-signed chain), refuse to co-sign the recipient's refresh/offboard, or simply wait and sweep at expiry; the recipient's recourse is publishing duplicate signatures, i.e. reputation.

## 2. Delegation — Ark Labs "Adios, Expiry" (2025-07-16) + keymeld issue #48

- Delegation shipped in arkd v0.7.0; delegate "Cannot modify outputs you specified in your Intent", "Control what you receive, only when".
- Ark Labs' own caveat: delegated renewals maintain only **"preconfirmation state"** and lack finality; "For Bitcoin-level security guarantees, users should independently participate in batch settlement."
- keymeld #48 (operator == delegate): in Arkade "the delegate is the user's only cosigner in the VTXO tree", forfeits are `ALL|ANYONECANPAY`, so "**Operator + delegate could therefore sign a tree that pays the renewed VTXO elsewhere, while the forfeit still completes.**" Proposed mitigation: TEE attestation (another trust assumption).
- Second's equivalent (`learn/rounds.md`): delegated refresh "dependent on the Ark server's signature only"; co-signers unbuilt; must be scheduled while online. → No proposal rescues a **fully offline** recipient without trusting the server.

## 3. Privacy — Randy Naar (Sovrandy), "Dog Park: Privacy in Bark" (2026-04-19)

- "The ASP of course knows the vtxos that belong to your wallet and the IP addresses you're making requests from"; also boarding inputs and refund keys.
- hArk lost coinjoin-style ambiguity: "a hArk forfeit itself is one side of an atomic swap with another vtxo … there's no ambiguity with respect to the output a forfeit transaction is attached to since both are linked by the hash."
- On-chain linkability at exits: "If the recipient were to perform a unilateral exit after the sender tried to exit their already-spent vtxo, an observer would then be able to link the sender and recipient by matching the hash revealed on both sides."
- Public footprint otherwise indistinguishable taproot; rated comparable to LSP-based Lightning wallets. Corroboration: Roose (SN AMA) "We only share VTXO-specific information with the owners of the VTXOs" — a policy, not a protocol guarantee; Spark: "the ASP sees all transactions it co-signs"; the Dog Pack proposal (community.second.tech, 2026-09-29/10-01) confirms in-round transfers are server-visible. **Second publishes no privacy policy or retention statement.**

## 4. "Trustodial" framings

- Seth For Privacy (Bitcoin Magazine, 2025-10-23): payments "default to … 'out-of-round' payments that lack true finality"; VTXOs "expire regularly (i.e. after 30d) … or risk giving up control of their funds entirely"; delegates help but "if they are offline for any reason or refuse to refresh a given VTXO, the user will be forced to join a round themselves before the expiry period."
- Stacker News thread (items/1264150): "trustodial"; "things are either trusted or they're not"; "like 0-conf Lightning channels with extra scam"; "regulatory loopholes".
- Matt Corallo (Tangents Daily, 2026-06-14): "Arkade and Arkoor rely on **statechain style trust models** similar to Spark, with the distinction that Spark operates as a multisig run by a larger company"; Ark "only delivers low trust and reasonable liquidity when full Lightning channels sit on top"; "builders are making custody assumptions that users inherit, often without realizing it."
- Roose (SN AMA): "Your wallet's database is necessary for you to access your funds"; sub-330-sat amounts "cannot propagate through Bitcoin's relay network, making them unrecoverable on-chain if the server disappears."

## 5. Regulatory — nothing primary

No FinCEN, MiCA/TFR, Coin Center or Bitcoin Policy Institute text on Ark operators surfaced. Closest: ademan's "regulatory risk for LSPs operating this way", SN "trustodial"/"regulatory loopholes", Corallo's "custody assumptions that users inherit", and the operators' own ToS (disclaim custody; assert a right to keep swept funds). Context: FinCEN withdrew its 2020 unhosted-wallet proposal (Oct 2026 digest).

## 6. Alternatives critics propose for micro-payouts

- **Lightning**: OCEAN BOLT12 direct-from-coinbase retry; Braiins 1-sat LN payouts (>1 000 payouts/day since Feb 2024). Corallo: Ark is only low-trust with Lightning channels on top; Second staff concede "Lightning can still be better for streaming payment applications".
- **Cashu eHash** (EthnTuttle, Delving 2024-05-15; Hashpool/Axepool, OpenSats 12th wave 2025-07-08): "zero prior liquidity requirements" but "an IOU … until they are redeemed"; Corallo asked whether faster payouts matter without automated pool-switching.
- **Spark**: dismissed by Corallo as the same statechain trust "run by a larger company".
- **Just accumulate credits** (SN, re ArkPool): hourly 1–3 sat payouts "solve nothing a daily ledger doesn't."

## Source criticism

ademan's post is the only engineering treatment of the sender==server case and is itself a proposal, not a result. Critiques are opinion pieces by credible engineers; Spark is a competitor and used only where it restates Second's own numbers. The regulatory gap is a genuine negative result after four searches.

## See also

- [[2026-10-10-second-docs-pricing-lifetime-server-tos|Second docs (the 1-of-2 honesty statement, ToS)]]
- [[../../wiki/concepts/payout-attribution-privacy|Payout Attribution Privacy]]
- [[../../../covenantless-ark/wiki/topics/clark-limitations-and-trust|covenantless-ark: limitations and trust model]]
