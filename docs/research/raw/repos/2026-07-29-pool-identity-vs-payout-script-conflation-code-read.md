---
title: "Identity-vs-payout-script conflation across pool codebases — ckpool, public-pool, DATUM, TIDES, share-accounting-ext"
source: https://github.com/ckolivas/ckpool/blob/master/src/stratifier.c
supporting_sources:
  - https://ocean.xyz/docs/tides
  - https://github.com/demand-open-source/share-accounting-ext
  - https://github.com/benjamin-wilson/public-pool
  - https://github.com/OCEAN-xyz/datum_gateway
  - https://github.com/stratum-mining/stratum/issues/697
  - https://github.com/stratum-mining/stratum/issues/1652
type: repos
ingested: 2026-07-29
quality: 5
credibility: high
confidence: high
tags: [ckpool, public-pool, datum, tides, share-accounting-ext, pplns-jd, xpub, descriptor, primary-key, coinbase-size, firmware-limits, code-read]
summary: "Code-level answer to \"what mechanically changes if a miner supplies an xpub instead of an address?\" The finding: **schemes differ enormously in how coupled identity and payout script are**, and the coupling — not the derivation math — is the work."
---

# Where the ledger key and the payout script are the same string

Code-level answer to "what mechanically changes if a miner supplies an xpub instead of an
address?" The finding: **schemes differ enormously in how coupled identity and payout script
are**, and the coupling — not the derivation math — is the work.

## ckpool (`src/stratifier.c`) — the conflation at its starkest

```c
struct user_instance {
    char username[128];      /* HASH KEY: HASH_ADD_STR(sdata->user_instances, username, user) */
    bool btcaddress; bool script; bool segwit;
    char txnbin[48]; int txnlen;
    struct userwb *userwbs;
};
```

Auth path (~:5773):

```c
if (generator_checkaddr(username, &user->script, &user->segwit)) {
    user->btcaddress = true;
    user->txnlen = address_to_txn(user->txnbin, username, user->script, user->segwit);
}
```

**The username string is converted directly into the payout script.** `generate_userwbs()`
iterates all users (`if (!instance->btcaddress) continue`), and `__generate_userwb()` splices
`user->txnbin` between the shared `coinb2bin` and `coinb3bin` — **one per-user work base per
user per workbase**, already running per-template. This is exactly the machinery an xpub design
would hook.

Blockers: `txnbin[48]` sizes a *script*, not a derivation rule; `username[128]` truncates at 127
chars, below a ~150-char wildcard descriptor; the username is split on `._`
(`strsep(&base_username, "._")`) and both characters collide with descriptor syntax. Coinbase
construction is manual byte-splicing, now BIP54-aware (`nSequence = 0xfffffffe`,
`nLockTime = height - 1`).

## OCEAN TIDES — the coinbase is precomputed at *work-issue* time

The primary spec detail that determines the cost of pool-side derivation:

> "When work is given to a miner, that work is internally tagged with the share ID of the share
> currently at the top of the share log when that work was issued to the miner, **along with a
> generation transaction that would pay all miners appropriately**."

The window is fixed at work-issue, not block-find, to stop share-log padding attacks. So the
pool must resolve **every** miner's scriptPubKey *before* the miner starts hashing, for **every
template refresh** — not once per payout. An xpub means a derivation per miner per template.

Other primary facts: share log window = `8 × network_difficulty` in diff-1 shares ("99.9665 %
chance of being rewarded at least once", rewarded ~8× on average; at current difficulty that is
1,009,852,056,974,945 shares tracked). Reward
`Σ(miner_shares_in_window[i] / share_log_window × current_block_reward) = current_block_reward`,
rounded down per miner. "**Rewards are calculated per user, not per worker.**" Per-share fee
flags: "Every share is tagged with its fee rate flag at the time it is mined … retains the fee
rate flag forever." Explicit scope boundary: "how the pool tracks and handles payouts is beyond
the scope of TIDES itself, as TIDES is the reward system and not the payout system."
**No mention of xpub, descriptors, or address derivation anywhere.**

Dust concession: "a tiny payout isn't worthwhile, since an aggregate of dust payments are not
efficient or cost effective to spend/move later," with pools accruing to a "minimum threshold";
and sub-1-satoshi earnings at block-found "are lost."

Ocean's only non-address payout identity is **BOLT12** — off-chain with on-chain fallback at
0.01048576 BTC (= 2²⁰ sats), keyed by a signed message binding an existing Ocean Bitcoin
address to the offer. It does not change coinbase construction.

## PPLNS-JD / share-accounting-ext (extension type 32, v0.0.13) — already decoupled

```
Slice { number_of_shares:U32, difficulty:U64, fees:U64, root:Hash256, job_id:U64 }
Share { nonce, ntime, version, extranonce:B032, job_id, reference_job_id,
        share_index:U32, merkle_path:B064K }
PHash { phash, index_start:u32 }
```

**Not one identity field.** Grepping the whole spec for address/identity/payout/user yields only
prose in the abstract. The ledger primary key is **(slice, share_index)** — positional, verified
by `merkle_path(share) + share_hash == slice.root`. A slice is "a group of shares mined when the
mempool's maximum extractable fees can be approximated as a constant"; verification is
probabilistic spot-checking of randomly sampled slices and shares. Miner verifies fees are
"lower than fees of the Slice ref job fees + delta."

So identity lives entirely in SV2 `user_identity` (`STR0_255`, `OpenStandardMiningChannel`)
bound to `channel_id`. **An xpub would change only the identity layer — the opposite of
ckpool.** (Note: the PPLNS-JD paper at `dmnd.work/pplns-with-job-declaration/...pdf` returns
**HTTP 404** and could not be located; this read is from `extension.md`.)

## public-pool — address is part of the table primary key

`ClientEntity`: composite `PRIMARY KEY (address varchar(62), clientName varchar(64), sessionId
varchar(8))`, `WITHOUT ROWID`. `AddressSettingsEntity`: `@PrimaryColumn({ length: 62 }) address`
with `shares`, `bestDifficulty`, `miscCoinbaseScriptData`.

`varchar(62)` fits a P2TR address and nothing more — xpub 111, wildcard descriptor ~150,
silent-payment address ≥117. Changing the identity type is a **primary-key schema migration**,
not a column widen. Validation is `bitcoin-address-validation`'s `validate(value, NETWORK)` via
an `IsBitcoinAddress` decorator, so a descriptor fails authorization outright. Coinbase is a
flat `[{address, percent}]` list with no derivation hook.

## DATUM gateway — the two real engineering ceilings

"**DATUM Gateway is designed with the assumption that pool usernames are generally Bitcoin
addresses**"; `mining.pool_address` "must always be a valid Bitcoin address, and the Gateway
will not fully start until it is set to one."

**Username limits (firmware, not protocol)**: 191 characters for stratum usernames; "Avalons
truncate usernames at 63 characters; **Whatsminer has a buffer overflow (which may damage your
miner) if you exceed 127**." Some miners percent-encode non-alphanumerics — mangling descriptor
characters `()[],/*#`. Live TODO at `datum_protocol.c:1296`: "Make sure the usernames are
addresses, and if not use one of the configured addresses."

**Coinbase output count is firmware-bounded, not consensus-bounded**:
`available_coinbase_outputs[512]`, `T_DATUM_TXN_OUTPUT { unsigned char output_script[64]; int
output_script_len; uint64_t value_sats; int sigops; }`. The coinbase-type table caps at "huge —
max 16kB … something like 380 to 530 outputs," while nicehash ≈500 B, antminer ≈730 B,
whatsminer 6500 B, antminer2 2250 B. This is the hard bound on any per-miner-output scheme, and
it is set by ASIC firmware.

The coinbaser is fetched **asynchronously per job**
(`JOB_STATE_FULL_PRIORITY_WAIT_COINBASER`, 5-second timeout, fail-open to `pool_addr_script`) —
so any xpub derivation lands inside a latency budget. `username_modifiers` splits shares across
addresses by PoW-hash prefix range (`~modifier` suffix) — the closest thing to a non-trivial
payout identity in shipping mining software, and still a static list of plain addresses.

## SV2 upstream: wildcards deliberately deferred, and the rotation trigger is the open question

Issue **#697** (GitGab19) states the unresolved problem verbatim: "Descriptors can be ranged,
e.g. `tr(xpub…/*)` so that the coinbase address can be rotated. **The tricky part here is to
decide when to rotate.** Doing it for every `SetNewPrevHash` would generate too many addresses,
which is difficult for wallet software to keep track of." Also: "the current value also needs to
be stored somewhere between starts." Their verdict: "I personally don't think that address
rotation would be that interesting for coinbase tx, **especially in a pooled mining context**."

Issue **#1652** (apoelstra): "**Greg Maxwell suggests that if the descriptor has a wildcard in
it, we use the blockheight as an index. This might confuse wallets that don't have an 800000+
gap limit**, but OTOH I'm not sure what else we ought to do with wildcards, and I like this."

PR **#1720** (merged 2025-07-09) shipped `coinbase_output_descriptors` using BIP385
`addr()`/`raw()`, deprecating `output_script_type` dictionaries. UX preserved: "You just
generate an address and put `addr(<my address>)` as the descriptor." **Wildcards remain rejected
in merged code**: `config_helpers/coinbase_output/mod.rs` asserts "Miniscript: key with a
wildcard cannot be a DerivedDescriptorKey", commented *"no wildcards allowed (at least for now;
gmax thinks it would be cool if we would instantiate it with the blockheight or something, but
need to work out UX)"*. Also rejected: hardened steps, multipath `<0;1>` ("this is not a wallet
with change"), xprv. Scope is the pool's own single fallback/solo output — **not per-miner**.
Related open issue #1793: "standardize rewards to pool/solo mining into a single `scriptPubKey`."

## What mechanically changes under a miner-supplied xpub

1. **Split the ledger key from the payout script.** Pool persists `identity → descriptor` plus a
   **monotonic derivation index** — new mutable state that must survive restart.
2. **Decide the rotation trigger** (per block? per template? per payout?) — unresolved upstream,
   and the actual hard question.
3. **Pool-side derivation only**, since the miner never sees the coinbase before it is built.
4. **Field widths break** everywhere: ckpool's 127 usable chars, public-pool's `varchar(62)`
   primary key, DATUM's 191 (Avalon 63, Whatsminer buffer overflow past 127).
5. **Per-template cost, not per-payout cost**, for TIDES and ckpool — both already construct
   per-user coinbases on every template refresh.
