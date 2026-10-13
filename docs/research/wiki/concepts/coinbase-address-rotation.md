---
title: Coinbase Address Rotation
category: concept
created: 2026-07-29
confidence: medium
tags: [coinbase-rotation, xpub-derivation, wildcard-descriptor, payout-address, miniscript, bip-380, bip-32, stratum-v2, ckpool, privacy, quantum-resistance]
volatility: warm
updated: 2026-07-29
verified: 2026-07-29
aliases: [Payout Address Rotation, Coinbase Rotation, xpub Coinbase Rotation]
summary: "Deriving a fresh coinbase payout address for every block a pool finds, from a wildcard output descriptor (BIP-380) plus a monotonic derivation index, rather than paying a static address forever. Covers the mechanism, the wildcard footgun both known implementations independently guard against, the index-persistence correctness problem that separates them, and why extending rotation from the pool's own address to per-miner xpub usernames hits a ledger-identity wall."
sources:
  - "raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read.md"
---

# Coinbase Address Rotation

Paying every block a pool wins to the **same** address is the default in nearly every pool implementation, and it makes the pool's entire revenue history trivially totalable by any observer and concentrates all of it behind one long-lived public key. **Coinbase address rotation** replaces that static address with a wildcard output descriptor and a derivation index, so each block's payout output goes to a fresh address.

The mechanism is small — a descriptor, a counter, and a derive call — and the engineering interest is almost entirely in two places: a footgun that silently disables the feature, and the question of whether the counter survives a crash. Both known implementations get the footgun right; they diverge sharply on the counter.

This article is about rotating the **pool's own** payout address. Rotating a *per-miner* address, where each miner's username is itself an xpub the pool derives from, is a different and unbuilt problem — see § Per-miner xpub usernames.

## The mechanism

Three pieces:

1. **A wildcard output descriptor** ([BIP-380](https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki)) whose key path ends in `*` — e.g. `wpkh(tpub…/0/*)`. The wildcard is what makes the descriptor a *family* of addresses rather than one address.
2. **A derivation index**, monotonically increasing, persisted across restarts.
3. **A trigger** that advances the index and swaps the resulting `scriptPubKey` into the coinbase for subsequent templates.

```
block found → index += 1 → derive scriptPubKey at index → swap into coinbase outputs
```

Derivation is unhardened by necessity — the pool holds only an xpub/tpub, so it can derive public keys and scripts but cannot spend. That property is the reason this is safe to run on a pool server: compromising the pool leaks the *sequence of future payout addresses*, which is a privacy loss, not a theft.

### What the trigger actually is

Both known implementations rotate on **block found**, not on a timer and not on block *confirmation*:

| | ckpool + `para` | sv2-apps `feat/coinbase-rotation` |
|---|---|---|
| Trigger | ckpool's `block_solve` callback, pushed over a Unix socket | SV2 `SubmitSolution` message, in-process |
| Direction | C → Rust event, then Rust → C `setdonaddress` | none — same process |
| Swap target | `sdata->dontxnbin` under `instance_lock` | `coinbase_outputs` behind a `SharedLock` |

Rotating on submission rather than confirmation means an **orphaned or rejected block burns an index**. Both implementations accept this, correctly: derivation indices are free, and gaps in a derivation sequence are normal under any rotation policy. `para`'s plan states the principle directly — "revealed-but-never-paid addresses are normal."

## The wildcard footgun

The single most important operational detail, and both implementations independently guard against it with the same API.

A descriptor *without* a wildcard — `tr(<xpub>)`, `wpkh(<xpub>)` — **parses successfully and then returns the same address forever.** Nothing errors. The rotation code runs on every block, derives, swaps in bytes identical to what was already there, and the operator's only signal is noticing repeated addresses on chain weeks later. `para`'s PR body describes the failure precisely:

> "A descriptor like `tr(<xpub>)` parses fine but silently returns the same address forever — `setdonaddress` becomes a no-op write of identical bytes, and the operator only notices when they see repeated addresses on chain."

The guard in both cases is miniscript's `has_wildcard()`, checked at construction and hard-failing:

- **sv2-apps** — `XpubDerivator::new` returns `XpubDerivationError::NoWildcard`, and the Pool/JD-client refuse to start (`InvalidConfiguration` shutdown).
- **para** — `Wallet::open` rejects any descriptor whose external keychain lacks a wildcard, via `public_descriptor().has_wildcard()`.

Two independent codebases arriving at the identical guard via the identical API is strong evidence this is the correct place to fail. **A rotation implementation without this check is broken by default**, because the broken configuration is also the most natural thing an operator would write.

### What else gets rejected

sv2-apps additionally rejects, with tests pinning the error strings:

- **Hardened derivation steps** — `"key with hardened derivation steps cannot be a DerivedDescriptorKey"`. A pool holding only public key material cannot derive through a hardened step.
- **Multipath descriptors** (`<0;1>`) — the receive/change pair convention is meaningless for a coinbase output, which is always "receive."

So the accepted descriptor shape is narrow: unhardened, single-path, wildcard-terminated.

## Index persistence is where implementations diverge

Both implementations do the same thing on the happy path. Under crash or corruption they behave very differently, and this is the part worth copying carefully.

**`para` persists before returning.** Its `reveal_address_with` writes the BDK `ChangeSet` to redb with `Durability::Immediate`, reloads from disk if the write fails, and **never returns an address that isn't already durable.** The design document records the rejected alternative explicitly:

> "**B. Push then persist** — risk: ckpool advertises an address whose derivation index isn't yet on disk; restart loses it."

**sv2-apps advances first and persists opportunistically.** `next_script_pubkey` does `fetch_add(1, SeqCst)` on an `AtomicU32`, derives, then attempts a write whose failure is only logged:

```rust
let new_index = self.current_index.fetch_add(1, Ordering::SeqCst) + 1;
let script = self.derive_at_index(new_index)?;
if let Err(e) = self.persist_index() {
    tracing::warn!("Failed to persist coinbase rotation index to {:?}: {}", self.index_file, e);
}
```

That is `para`'s rejected option B. Three consequences, all reachable in normal operation:

1. **The write is a bare `fs::write`** — no temp-file-and-rename, no fsync. A crash mid-write can leave a truncated file.
2. **Corruption degrades silently to *replay from the start*.** `load_index` is `contents.trim().parse().unwrap_or(default)`, so a garbled file falls back to `coinbase_start_index` — default `0`. The pool then re-derives addresses it has already paid to, which is precisely the address reuse the feature exists to prevent, reached by the quietest available path.
3. **A restart can reuse the current address**, since the in-memory index can be one ahead of disk.

None of this loses funds — the addresses are all derived from the operator's own descriptor and remain spendable. What it loses is the *property being purchased*. A rotation feature that silently stops rotating has failed while continuing to report success.

### The recovery hazard nobody has solved well

Sustained per-block rotation burns `O(blocks/day)` derivation indices. Restoring the wallet from the descriptor alone then requires a **scan longer than any default gap limit**, because the wallet must look far enough ahead to find used addresses.

`para` ships an advisory `--pool-address-gap-limit` setting and flags the concrete default as an open question. sv2-apps has **no gap-limit setting at all**. The exposure is arguably worse there: `para` at least holds a BDK wallet database, whereas sv2-apps holds a single integer in a flat file — if that integer is lost, rescan-from-descriptor is the only recovery path, and nothing in the configuration tells the operator how far to scan.

## Why rotate at all — two different stated motivations

The two implementations describe the identical mechanism with different justifications, and neither has published analysis behind the claim.

- **On-chain privacy** (`para`) — a static payout address lets any observer total the pool's lifetime revenue, size its hashrate independently of its own reporting, and watch its treasury movements. Rotation breaks the trivial linkage. This is the conventional and well-understood argument.
- **"Quantum-resistant payout hygiene"** (sv2-apps) — asserted in a TOML comment, with no supporting analysis in-tree. The underlying idea is presumably that a fresh address per block limits how long any single public key sits exposed. Note that for P2WPKH and P2TR the pubkey is revealed **on spend**, not on receive, so rotation of *receive* addresses bounds exposure only if the outputs are also spent promptly and never consolidated. Treat this framing as an unexamined claim rather than a result.

The privacy argument stands on its own. The quantum argument is plausible in outline but is not established by either source, and a pool that consolidates rotated outputs into one spending transaction has undone most of whatever benefit was claimed.

## Per-miner xpub usernames

The natural extension — a pool where each miner's *username* is an xpub or wildcard descriptor, and the pool derives that miner's payout address fresh each block — is **unimplemented in every implementation surveyed**, and the obstacles are not in the derivation layer.

What the pool-side work does supply toward it:

1. **A reusable derivation primitive.** sv2-apps' `XpubDerivator` lives in the shared `stratum-apps` crate, not in either role, and is `Send + Sync` — so it is callable per-miner in a concurrent server.
2. **Proof the coinbase tolerates runtime mutation.** `coinbase_outputs` behind a `SharedLock`, swapped between templates with `CoinbaseOutputConstraints` re-derived, shows the SV2 plumbing does not assume a static payout script.
3. **A registration-time validation gate.** `has_wildcard()` is exactly the check needed when a miner *registers* an xpub. Without it, a miner supplying `wpkh(xpub…)` with no `/0/*` is paid to one address forever — the same footgun, now at miner granularity, where the operator has hundreds of opportunities to miss it.

Two blockers it does not address:

**Derivation cost stops being free.** sv2-apps stores the descriptor as a `String` and **re-parses it on every derivation**, a deliberate trade documented in-source: miniscript's `Descriptor<DescriptorPublicKey>` uses an internal `RefCell` for taproot caching and so is not `Send + Sync`. At once-per-block for one address this is irrelevant. At once-per-miner-per-template — the cadence [[lottery-pplns|lottery-PPLNS]] already imposes — it becomes a per-template parse storm and the workaround needs revisiting.

**Ledger identity breaks.** This is the hard one. Blitzpool's PPLNS balance table upserts `ON CONFLICT (address) DO UPDATE` — balances are keyed on **address, globally, not per block**. A miner whose address changes every block therefore writes a *new* balance row every block, and pending-credit carry-forward silently stops working: the credit accrued under block *N*'s address is invisible to block *N+1*'s lookup. Any per-miner rotation design must **split identity in two** — a stable key for the ledger (descriptor string, or its fingerprint) and a rotating derived address for the coinbase output.

Notably, the *attribution* half needs no change. A content hash over the payout list — Blitzpool's `payouts_fingerprint` — identifies which distribution a coinbase paid without reference to any address's stability, which is already the right shape. It is the balance ledger, not the block-found lookup, that assumes address permanence.

## Maturity

| | ckpool + `para` | sv2-apps `feat/coinbase-rotation` |
|---|---|---|
| Status | **Shipped**, 476 tests passing | **Unmerged branch**, no PR review read |
| Wallet layer | BDK wallet + redb | none — bare index file |
| Persistence | ChangeSet, `Durability::Immediate`, persist-before-return | `fs::write`, warn-on-failure, increment-first |
| Gap limit | advisory setting + runbook note | absent |
| Rotation tests | restart-mid-rotation in the manual test plan | 12 unit tests incl. pinned vectors and a real restart test; **no integration or regtest coverage** |
| Stated motivation | on-chain privacy | "quantum-resistant payout hygiene" |

sv2-apps also **reverses a deliberate upstream decision**. The branch deletes an upstream test asserting wildcards are invalid, whose comment read: *"no wildcards allowed (at least for now; gmax thinks it would be cool if we would instantiate it with the blockheight or something, but need to work out UX)."* Upstream was not unaware of wildcard descriptors — it deferred them on UX grounds and floated **block-height instantiation** as the more interesting variant. The branch resolves the deferred question with a persisted counter and does not engage the block-height idea, which would be self-synchronizing (no persistence needed at all, since the height *is* the index) at the cost of gaps whenever the pool doesn't win.

That alternative is worth weighing: it dissolves this article's entire § Index persistence section. Its cost is that the derivation range becomes as large as the block height, making descriptor recovery scans correspondingly wide.

### The recovery-scan cost is specific to BIP 32 derivation, not to height-as-index

**Added 2026-08-14.** The wide-scan cost above — and the gap-limit collision recorded against
block-height instantiation throughout this wiki — is a property of using the height as a **BIP 32
child index**, not of using the height. The two roles are categorically different:

- **As a child index** (BIP 380 wildcard descriptor): recovery walks `i = 0, 1, 2, …` and stops
  after 20 consecutive misses per BIP 44's gap limit. A pool winning one block in 800,000 leaves
  799,999-wide holes. Fatal, exactly as this section states.
- **As an ECDH nonce** (stealth/silent-payment-style derivation, occupying the slot `input_hash`
  fills in BIP 352): the receiver never scans forward. Given block *H*, they compute the single
  candidate for *that* height and compare against the block's outputs. Unmined heights are never
  enumerated, so **there is no gap to limit** and skipped blocks cost nothing.

So "self-synchronizing at the cost of gaps" is only half true — the self-synchronization survives
in both constructions, and the gap cost appears in only one. The nonce also need not be secret:
BIP 352's `input_hash` is fully public, and the height is hashed *together with* the shared secret.

This does not make height-as-index viable for descriptors, which is what both implementations here
use. It means open question 2 below should be split in two, because the recovery-scan tradeoff
nobody has written down does not exist for the ECDH variant. See
[[../../raw/notes/2026-08-14-ll-coinbase-silent-payments-ecdh-nonce|Lessons: coinbase silent
payments]] lesson 2 — and note the separate blocker that gates the ECDH variant in a coinbase at
all: a coinbase input carries no public key, so there is no sender point to ECDH with unless one is
supplied deliberately (fee output at index 0, or an ephemeral key in the scriptSig).

### The rotation plumbing is reusable; the derivation is not, and they invert

**Added 2026-08-16.** A second, sharper reason wildcard descriptors don't transfer to the ECDH
variant above. Reading the rotation branch's key source
(`stratum-apps/src/config_helpers/xpub_derivation.rs:212`, `descriptor.at_derivation_index(index)`
on `wpkh(xpub.../0/*)`), BIP 32 **unhardened** derivation is `a_i = a_par + H(K_par ‖ i)` — an
additive tweak by a publicly computable scalar. That is algebraically the *same construction* as the
group-linear key list already proven fatal for an ECDH sender key, where holding one parent secret
recovers every child and retroactively unmasks every past payout.

The trap is that the same machinery is *correct* for what it was written for. The two keys invert on
one requirement:

| | pool payout key (this article) | ECDH sender key `a_send` |
|---|---|---|
| recoverable from a seed | **required** — the pool must be able to spend | **fatal** — the pool must be able to destroy |
| correct engine | BIP 32 wildcard descriptor | independent CSPRNG + erasure schedule |

So when reusing a rotation mechanism, separate its **plumbing** (the `SharedLock`-wrapped coinbase
state mutated at runtime and re-read per template) from its **key derivation** (where secrets come
from). The plumbing ports; the derivation must not.

A related correction to § What the trigger actually is, for the ECDH variant specifically:
`rotate_coinbase_address()` fires only when *this pool* finds a block, which reads as too coarse but
isn't a correctness problem there. Because the height is bound into the shared secret, outputs differ
every block even with a static sender key — so rotation cadence bounds **forward-secrecy
granularity** only, not address freshness. Worth stating as "one key's exposure reveals every block
it covered" rather than asserting a required frequency. See
[[../../raw/notes/2026-08-16-ll-sv2-pool-tag-asend-carrier|Lessons: SV2 pool tag as A_send carrier]]
lessons 2 and 3.

## Open questions

1. **Is the quantum framing sound?** Neither source analyzes it. Given that P2WPKH/P2TR reveal the pubkey on spend, rotation of receive addresses bounds exposure only under assumptions about spending behavior that no source states.
2. **Block-height instantiation vs. a persisted counter.** Upstream's floated alternative removes the persistence problem entirely. No implementation has tried it, and no one has written down the recovery-scan tradeoff.
3. **A principled gap-limit default.** Both implementations leave this to the operator; `para` names it as an open question. A default tied to expected block rate and a stated recovery window would be mechanical to derive and nobody has.
4. **Does any pool actually run this on mainnet?** `para`'s implementation is shipped and tested, but neither source establishes mainnet rotation in production, and the sv2-apps branch is unmerged.
5. **Per-miner rotation end-to-end.** The ledger-identity split above is a design sketch, not a tested design. No implementation splits descriptor-for-ledger from address-for-coinbase.

## See also

- [[lottery-pplns|Lottery-PPLNS (Finder-Bonus Hybrid)]] — imposes per-finder coinbase construction, the cadence at which the re-parse cost above stops being free
- [[payout-schema-taxonomy|Payout Schema Taxonomy]] — rotation is orthogonal to the payout scheme; it changes the address, not the split
- [[tides|TIDES]] — non-custodial coinbase payout, where per-miner addresses are already the payout rail
- [[ctv-coinbase-payout-tree|CTV Coinbase Payout Tree]] — the other direction on coinbase output handling: reducing output *count* rather than rotating output *addresses*
- [[parasite-pool|Parasite Pool]] — pays each finder's own address per block, so its bounty output rotates as a side effect of the payout design rather than by an explicit rotation mechanism
- [[xpub-payout-identity|xpub Payout Identity]] — the per-miner generalization of this article, including field-width and firmware ceilings and why BIP 352 can't substitute
- [[payout-attribution-privacy|Payout Attribution Privacy]] — the empirical case for rotation (92 % of BTC.com miners identified from address reuse) and its limits
- [[block-withholding|Block Withholding]] — the cost side: fresh-address-per-payout defeats the Eligius 2014 clustering detection, the only one that ever worked in production
- [[../topics/self-blinding-pool-design-space|Self-Blinding Pool Design Space]]

## Sources

- [[../../raw/repos/2026-07-29-sv2-apps-xpub-coinbase-rotation-code-read|sv2-apps xpub/wildcard-descriptor coinbase rotation @ e2930150]] — the code-level read this article is compiled from, including the `para` comparison throughout
- `REPOS/para/.wiki/output/plan-coinbase-rotation-2026-06-24.md` and `pr-body-coinbase-rotation.md` — the shipped ckpool + BDK design. **Local wiki, not in this hub**; cited here as the comparison baseline recorded in the raw source above, not as hub-resident evidence.

