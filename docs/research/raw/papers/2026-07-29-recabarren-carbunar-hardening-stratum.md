---
title: "Hardening Stratum, the Bitcoin Pool Mining Protocol"
authors: [Ruben Recabarren, Bogdan Carbunar]
year: 2017
venue: "PoPETs / Proceedings on Privacy Enhancing Technologies (arXiv 1703.06545)"
source: https://arxiv.org/abs/1703.06545
type: papers
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [stratum, privacy, side-channel, hashrate-inference, traffic-analysis, StraTap, ISP-Log, bitecoin, mining-cookie, share-theft, primary]
summary: "The only paper with **measured** figures showing a miner's hashrate and earnings are recoverable from Stratum traffic — including from packet timestamps alone, with no payload access. Load-bearing for any claim that transport encryption delivers miner privacy."
---

# Hardening Stratum (Recabarren & Carbunar, PETS 2017)

The only paper with **measured** figures showing a miner's hashrate and earnings are
recoverable from Stratum traffic — including from packet timestamps alone, with no payload
access. Load-bearing for any claim that transport encryption delivers miner privacy.

## Two attacks, with measured accuracy

Core inference: `hashrate = difficulty × 2^32 / time`, where `difficulty` comes from
`mining.set_difficulty` and `time` = (interval between difficulty changes) / (accepted
share count).

| Attack | Access | Payout prediction error | MSE / MPE |
|---|---|---|---|
| **StraTap** | full packet contents | **1.75 – 6.5 %** | 0.062 / −3.46 % |
| **ISP Log** | *inter-packet timestamps of the first 50 packets only* | **0.53 – 34.4 %** | 2.02 / −9.49 % |

Dataset: 138 MB of live Stratum from an AntMiner S7, US (May 27 – Jun 8 2016) and Venezuela
(Mar 8 – Apr 2 2016), 13 days, frequency swept 100–700 MHz.

## VarDiff is itself the side channel

F2Pool sets difficulty to 1024 on authorize, then waits for **~50 share submissions**
(measured 50–67 packets across 100–650 MHz) before the second `set_difficulty`. Inter-packet
time over those 50 packets decreases monotonically with device frequency (Fig. 12). The
calibration handshake the pool *needs* in order to set a sane target is the same signal that
leaks hashrate to anyone counting packets. Every reconnect restarts it (Fig. 5 shows
subscribe/authorize spikes), re-leaking.

The paper's conclusion on encryption is blunt: it is **"not only undesirable but also
ineffective"** against the inference channel. Costs measured: blanket encryption 1.36 h/day
of pool CPU at 16,000 miners; TLS 1.01 h/day plus 58 % more miner→pool bandwidth; their
Bedrock defense 12.03 s/day.

## Stratum V1 leak surface, enumerated

`mining.subscribe` → `extranonce1`, `extranonce2.size`; `mining.authorize` → username and
password **in cleartext** (password "currently ignored by pools"); `mining.set_difficulty`;
`mining.notify` → `job_id`, params, `clean_jobs`; `mining.submit` → username, `job_id`,
`time`, `nonce`, `extranonce2`. F2Pool sets `extranonce1` to a constant rather than
per-connection random.

**Target reconstruction attack**: recover `extranonce1` from subscribe, coinbase1/coinbase2
and Merkle branches from a job, nonce/extranonce2 from a submit → reconstruct the header,
count leading zeros to bound the target — *without ever seeing a difficulty message*.

## BiteCoin and the mining cookie — why identity is currently the anti-theft mechanism

**BiteCoin** is an active attack that TCP-hijacks the miner↔pool connection and **hijacks
shares** and payouts.

The countermeasure, **Bedrock**, binds a per-miner secret into the puzzle: a "mining cookie"
`C_M = H²(R_M, M.uname)` placed in the coinbase's unused previous-input-hash field. It works
precisely *because the miner's username is an input to the hash*: "the pool would use the
attacker's username instead of the victim's username to construct the cookie … The share
will only validate if the attacker managed to find a username that produced a double hash
that was still smaller than the target."

> **Annotation (2026-07-29).** The field description above is *faithful to the paper* — a
> targeted re-read confirmed §8.3 means the literal 32-byte prevout hash and explicitly declines
> the scriptSig. Our separate objection, which the paper never addresses, is that this is
> **consensus-invalid for the share that is a block**. Also verified in that read: the paper
> contains **no theorem, proof, or named hardness assumption** (§7.1's argument is
> work-equivalence), the cookie **rotates only on block-find — ~7.44 years for an S7 by the
> paper's own arithmetic**, and `store(M.uname, K_M, R_M, target)` keys the **vardiff target on
> identity**, so blinding breaks validation and not merely crediting. Everything else in this
> file verified correct against both PDFs. See
> [[2026-07-29-bedrock-primary-read-cookie-construction|the primary read]].

This is the strongest technical argument that share-credit integrity currently *depends on*
identity binding — and it is an argument about theft, not about block withholding.

**Critical limitation for privacy purposes**: the pool generates `R_M` and shares `K_M`.
**Bedrock provides zero privacy from the pool.** Every defense in this paper treats the pool
as a trusted endpoint.
