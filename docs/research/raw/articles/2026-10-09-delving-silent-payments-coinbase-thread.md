---
title: "delvingbitcoin — Silent Payments Coinbase (SPc) thread, t/2833"
url: https://delvingbitcoin.org/t/silent-payments-coinbase/2833
source: "https://delvingbitcoin.org/t/silent-payments-coinbase/2833"
type: articles
category: forum-thread
author: marathon-gary (OP); replies by Anzus_GemWallet, Seldenthuis, optout
published: 2026-08-24
last_reply: 2026-09-24
ingested: 2026-10-09
volatility: hot
quality: 4
credibility: medium
confidence: medium
tags: [bip352, silent-payments, coinbase, spc, stratum-v2, pool-tag, scriptsig, bip323, primary-source, delvingbitcoin]
summary: "The public write-up of the coinbase-native silent-payments construction this wiki developed in August 2026 (candidate record coinbase-native-stealth-payout), plus the first external review. Four replies: wallets need a new scanning profile (Anzus); the scriptSig headroom claim is corrected to 100 − 4 − 34 = 62 B (Seldenthuis); BIP 323 suggested as relief (OP); optout asks whether it is a BIP 352 amendment or a new BIP and whether solo pools benefit. Nothing in the thread breaks the construction; nothing in it confirms it beyond 'plausible at first glance'."
---

# delvingbitcoin t/2833 — Silent Payments Coinbase (SPc)

## Original post (marathon-gary, 2026-08-24)

Proposes **SPc**: pay miners in the coinbase to BIP 352 silent-payment addresses.

- Miner supplies a static SP address (`B_scan`, `B_spend`) over Stratum V2 (`user_identity`).
- Coinbase has no inputs, so the **block height replaces the outpoint nonce**: `input_hash = hash(block_height ‖ A_send)`.
- `A_send` is an **ephemeral sending key**, deliberately not tied to any spending key, published in the coinbase **pool tag** (34 B push) inside the 100 B scriptSig, after the BIP 34 height push and disjoint from the extranonce.
- Miners scan with `b_scan` against the published `A_send`; cost is low because SPc lives only in "the 0th transaction in every SPc block".
- Pool obligations: register SP addresses in its accounting DB, do the DHKE math, **scrub share-accounting logs after payment** so a DB compromise exposes as little history as possible, absorb the compute cost.
- Test vectors: links the GitHub Pages site (this wiki's `sp-coinbase-vectors`).
- Acknowledged limits: **amount fingerprinting** (a stable hashrate share is a stable fraction of every payout); DB compromise exposes historic address↔payout correlation; dust forces a complementary rail (Ocean-style BOLT12); **Job Declaration needs an SV2 extension for non-custodial payouts** (PR #203).

## Replies

- **Anzus_GemWallet (08-28)** — would existing SP wallets detect these? **OP: no**, "a new scanning profile" is needed; the ownership data sits in a different place than standard silent payments.
- **Seldenthuis (08-29)** — disputes "plenty of headroom". Height push is 4 B now, 5 B past 16,777,215; `A_send` 34 B; so the extranonce budget is **100 − 4 − 34 = 62 B**, not a surplus.
- **marathon-gary (08-31)** — points at **BIP 323** (24 nVersion bits as extra nonce, Corallo, draft 2026-04) as relief: more header-side nonce space means less scriptSig extranonce is needed.
- **optout (09-24)** — context: ~2 % of miners already take direct coinbase payouts (Ocean), today via **reused addresses as usernames**, which leaks spending patterns. Asks: which components change (libraries / scanners / wallets on receive; mining software on send)? **Amendment to BIP 352 or a new BIP?** Can solo pools benefit? Verdict: the input/nonce substitution "looks plausible at first glance" but needs deeper analysis.

## Unresolved in-thread

Wallet compatibility without breaking existing SP implementations; formal spec pathway (amendment vs. new BIP); solo-mining applicability; long-term scriptSig allocation as heights grow.

## Source criticism

OP is the wiki owner's own post, so this is **self-sourced** for the construction and **externally sourced only for the four replies**. No cryptographer has reviewed the derivation in-thread. The Seldenthuis arithmetic agrees with this wiki's independent 2026-08-16 budget (`~62 B remaining`, [[../notes/2026-08-16-ll-sv2-pool-tag-asend-carrier]]).

## See also

- [[../../inventory/candidates/coinbase-native-stealth-payout|Inventory: coinbase-native stealth payout]] — the design record this thread publishes
- [[../notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce|Lessons: the blocker is the sender pubkey, not the outpoint]]
- [[../notes/2026-08-24-ll-jd-coinbase-control-amount-linkability|Lessons: who controls the SV2 coinbase]]
