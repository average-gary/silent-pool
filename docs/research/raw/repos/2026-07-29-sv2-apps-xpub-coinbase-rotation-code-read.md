---
title: "xpub/wildcard-descriptor coinbase rotation in sv2-apps — code-level read of feat/coinbase-rotation @ e2930150"
url: https://github.com/stratum-mining/sv2-apps
source: "local git worktree: REPOS/sv2-apps-coinbase-rotation (branch feat/coinbase-rotation) @ e2930150dbb681eec2c3730dc30e6764be687913, parent 817a1057ceb862346d7d915eb39b511737101957, forked off upstream cb3d2dd6"
type: repos
category: repo
language: Rust
license: Apache-2.0 OR MIT
ingested: 2026-07-29
volatility: warm
quality: 4
credibility: medium
confidence: high
tags: [sv2-apps, coinbase-rotation, xpub-derivation, wildcard-descriptor, miniscript, bip-380, bip-32, pool, jd-client, stratum-v2, payout-address, quantum-resistance]
summary: "Code-level read of an unmerged sv2-apps branch that adds wildcard-descriptor (xpub) coinbase address rotation to both the SV2 Pool and JD-client. Adds a reusable 552-line `XpubDerivator` primitive (miniscript `has_wildcard()` gate, `AtomicU32` index, index persisted to a flat file) and reverses an explicit upstream decision to reject wildcards in `CoinbaseRewardScript`. Rotation fires on `SubmitSolution`. This rotates the pool's *own single* coinbase output — it is not per-miner xpub-username derivation, but it supplies the derivation primitive that design would need. Persistence is weaker than para's shipped BDK persist-first design in three specific ways."
---

# xpub/wildcard-descriptor coinbase rotation in sv2-apps

Direct source read of a local git worktree of `stratum-mining/sv2-apps` on branch **`feat/coinbase-rotation`**, two commits ahead of upstream `cb3d2dd6`:

| Commit | Subject | Files | Diff |
|---|---|---|---|
| `817a1057` | feat: coinbase address rotation for mining pools | 17 | +1112 / −30 |
| `e2930150` | docs: add coinbase rotation documentation to config examples | 2 | +20 |

Totals across both: **19 files, +1132 / −30**. Toolchain pinned to Rust **1.88.0** (`rust-toolchain.toml`). License Apache-2.0 OR MIT. Unmerged; not upstream.

## Why this source matters here

This closes the gap flagged as absent in the 2026-07-29 query on xpub-as-miner-username: prior to this ingest the phrase `xpub` had **zero occurrences** anywhere in the hub. The only coinbase-rotation research in reach was the ckpool/BDK design in the `para` repo-local wiki (`plan-coinbase-rotation-2026-06-24.md`, `pr-body-coinbase-rotation.md`), which rotates a *single pool* address over a Unix socket into ckpool's `sdata->dontxnbin`.

This branch does the same job **natively in the SV2 reference apps**, in Rust, with no C hot path and no IPC. Crucially it factors out the derivation step into a standalone, `Send + Sync` primitive that a per-miner design could call once per miner rather than once per block.

**Scope caveat stated up front:** this rotates the pool's own **single** coinbase output. It is *not* per-miner xpub-username derivation, and it does not touch payout accounting at all. See §7.

## 1. The reusable primitive: `XpubDerivator`

New file `stratum-apps/src/config_helpers/xpub_derivation.rs` — **552 lines, 12 tests**, all passing (`cargo test --lib config_helpers` → 28 passed, 0 failed, of which 12 are `xpub_derivation::tests`).

```rust
pub struct XpubDerivator {
    /// The wildcard descriptor string (e.g., "wpkh(xpub.../0/*)")
    descriptor_str: String,
    /// Current derivation index (atomic for thread safety)
    current_index: AtomicU32,
    /// Path to persist the current index
    index_file: PathBuf,
}
```

Error taxonomy (`:39`):

```rust
pub enum XpubDerivationError {
    NoWildcard, ParseError(String), DerivationFailed(String),
    PersistenceError(io::Error), CreateDirectoryError(io::Error),
}
```

### The descriptor is stored as a `String`, not a `Descriptor`

Deliberate, and documented at `:96-98`:

> "Note: The descriptor is stored as a `String` and re-parsed on each derivation to ensure `Send + Sync` compatibility (miniscript's `Descriptor<DescriptorPublicKey>` uses internal `RefCell` for taproot caching which is not thread-safe)."

So `derive_at_index` (`:212-225`) re-parses the descriptor on **every** derivation before calling `at_derivation_index(index)`. That is a real per-call cost paid to get `Send + Sync`, but rotation is once-per-block so it is free at this cadence. It would **not** be free in a per-miner-per-template design — see §7.

Dependency added: `miniscript = { version = "13.0.0", default-features = false, features = ["std"] }` (`stratum-apps/Cargo.toml:28`).

### Construction gates on `has_wildcard()`

`new()` (`:129-159`) parses, then rejects a non-wildcard descriptor outright:

```rust
if !descriptor.has_wildcard() {
    return Err(XpubDerivationError::NoWildcard);
}
```

This is **the same guard, via the same miniscript API, that `para` uses** — `Wallet::open` there rejects any descriptor whose external keychain has no wildcard via `public_descriptor().has_wildcard()`. Both codebases independently identified the identical footgun. Para's PR body states it plainly:

> "A descriptor like `tr(<xpub>)` parses fine but silently returns the same address forever — `setdonaddress` becomes a no-op write of identical bytes, and the operator only notices when they see repeated addresses on chain."

`new()` also creates parent directories for the index file if absent (`:144-149`), then loads the persisted index, falling back to `start_index`.

### Rotation

```rust
pub fn next_script_pubkey(&self) -> Result<ScriptBuf, XpubDerivationError> {
    // fetch_add returns the old value, so add 1 to get the new value
    let new_index = self.current_index.fetch_add(1, Ordering::SeqCst) + 1;
    let script = self.derive_at_index(new_index)?;
    // Don't fail if persistence fails - just log a warning
    if let Err(e) = self.persist_index() {
        tracing::warn!("Failed to persist coinbase rotation index to {:?}: {}", self.index_file, e);
    }
    Ok(script)
}
```

`current_script_pubkey()` (`:173`) derives at the current index *without* incrementing — used at startup so a restart resumes on the persisted address rather than jumping ahead.

### Test vectors are pinned

`test_known_derivation_vectors` and `test_rotation_flow_with_known_vectors` assert exact `script_pubkey` hex against:

```
wpkh(tpubD6NzVbkrYhZ4XgiXtGrdW5XDAPFCL9h7we1vwNCpn8tGbBcgfVYjXyhWo4E1xkh56hjod1RhGjxbaTLV3X4FyWuejifB9jusQ46QzG87VKp/0/*)
```

The rotation-flow test walks index 2 → 3 → 4, reads the file back (`assert_eq!(contents, "4")`), then constructs a **second** `XpubDerivator` from the same path and asserts it resumes at 4. That is a genuine restart test, not a mock.

Full test list: `test_new_with_wildcard`, `test_new_without_wildcard_fails`, `test_new_with_invalid_descriptor_fails`, `test_current_script_pubkey_does_not_increment`, `test_next_script_pubkey_increments`, `test_index_persistence`, `test_start_index`, `test_creates_parent_directories`, `test_mainnet_xpub`, `test_taproot_descriptor`, `test_known_derivation_vectors`, `test_rotation_flow_with_known_vectors`.

## 2. It reverses an explicit upstream design decision

`stratum-apps/src/config_helpers/coinbase_output/mod.rs` — `CoinbaseRewardScript` gains a third field:

```rust
wildcard_descriptor_str: Option<String>,
```

and `from_descriptor` (`:45-61`) now tries the wildcard parse **first**, deriving at index 0 for the initial `script_pubkey`:

```rust
if let Ok(wildcard_desc) = s.parse::<Descriptor<DescriptorPublicKey>>() {
    if wildcard_desc.has_wildcard() {
        let definite = wildcard_desc.at_derivation_index(0)...;
        return Ok(Self { script_pubkey: definite.script_pubkey(),
                         ok_for_mainnet: true,
                         wildcard_descriptor_str: Some(s.to_string()) });
    }
}
```

Plus accessors `has_wildcard()` (`:143`) and `wildcard_descriptor_str()` (`:153`).

**The branch deletes an upstream assertion that wildcards are invalid.** Removed from the test module by `817a1057`:

```rust
// no wildcards allowed (at least for now; gmax thinks it would be cool if we would
// instantiate it with the blockheight or something, but need to work out UX)
assert_eq!(
    CoinbaseRewardScript::from_descriptor("pkh(xpub661My.../*)").unwrap_err().to_string(),
    "Miniscript: key with a wildcard cannot be a DerivedDescriptorKey",
);
```

That comment is the load-bearing provenance in this whole source: upstream was **not unaware** of wildcard descriptors, it deferred them on **UX grounds** and floated block-height instantiation as the interesting variant. This branch resolves the deferred UX question one particular way (a persisted monotonic counter) without engaging the block-height idea. Anyone taking this upstream should expect that conversation to reopen.

Still rejected after the change, and asserted so in tests: **hardened** derivation steps (`"Miniscript: key with hardened derivation steps cannot be a DerivedDescriptorKey"`, `:392`) and multipath `<0;1>` descriptors. So the accepted shape is narrow — unhardened, single-path, wildcard-terminated.

## 3. Pool wiring

`pool-apps/pool/src/lib/channel_manager/mod.rs` (+118):

- `coinbase_outputs: Vec<u8>` → **`SharedLock<Vec<u8>>`** (`:102`), with the comment "Wrapped in `SharedLock` because the xpub derivator" mutates it at runtime.
- New field `xpub_derivator: Option<Arc<XpubDerivator>>` (`:120`).
- Init (`:183-236`): if `coinbase_reward_script().has_wildcard()` and `coinbase_index_file()` is `None` → `PoolError::shutdown(PoolErrorKind::InvalidConfiguration)`. So the required-companion-setting is enforced at startup, not discovered at first block.
- Initial `coinbase_outputs` come from `derivator.current_script_pubkey()`, **not** index 0 — this is what makes the persisted index actually honored on restart, since `CoinbaseRewardScript::from_descriptor` itself hardcodes index 0.

```rust
pub fn rotate_coinbase_address(&self) {
    let Some(derivator) = &self.xpub_derivator else { return };
    match derivator.next_script_pubkey() {
        Ok(new_script) => { /* TxOut { value: Amount::from_sat(0), script_pubkey: new_script },
                              consensus_encode, self.coinbase_outputs.set(new_outputs) */ }
        Err(e) => { error!("Failed to rotate coinbase address: {}", e); }
    }
}
```

`rotate_coinbase_address` is **fail-open**: on any derivation error the pool logs and keeps mining to the previous address. Correct priority (never stop mining over a payout-hygiene feature), but it means silent degradation back to address reuse.

### Trigger: `SubmitSolution`

`pool-apps/pool/src/lib/channel_manager/mining_message_handler.rs`, at **two** sites (`:844`/`:860` and `:1123`/`:1139` — the standard and extended channel share-submit paths):

```rust
let block_found = messages.iter().any(|m| {
    matches!(m, RouteMessageTo::TemplateProvider(TemplateDistribution::SubmitSolution(_)))
});
...
if block_found { self.rotate_coinbase_address(); }
```

## 4. JD-client wiring (solo mode)

`miner-apps/jd-client/src/lib/channel_manager/mod.rs` (+133) mirrors the pool exactly, including the `coinbase_index_file`-required check (`:417-421`, `JDCErrorKind::InvalidConfiguration`) and the `current_script_pubkey()` initial-outputs path.

`downstream_message_handler.rs` (+20) uses a different intra-function signal because the JDC path is single-threaded within the handler:

```rust
let block_found = std::cell::Cell::new(false);
...
block_found.set(true);
...
if block_found.get() { self.rotate_coinbase_address(); }
```

Also at two sites (`:912`/`:1141` and `:1191`/`:1445`).

## 5. Config surface

Three keys, added to both `pool-apps/pool/src/lib/config.rs` (+39) and `miner-apps/jd-client/src/lib/config.rs` (+44):

| Key | Type | Notes |
|---|---|---|
| `coinbase_reward_script` | `CoinbaseRewardScript` | existing; now accepts wildcard descriptors |
| `coinbase_index_file` | `Option<PathBuf>` | **required** when the script has a wildcard |
| `coinbase_start_index` | `u32` | default 0; used only when no persisted file exists |

`e2930150` documents them in the testnet4 examples for both roles. Verbatim from `pool-config-local-sv2-tp-example.toml`:

```toml
# For automatic address rotation (quantum-resistant payout hygiene), use a wildcard descriptor:
#   coinbase_reward_script = "wpkh(tpub.../0/*)"
# This derives a fresh address for each block found. Requires coinbase_index_file.
coinbase_reward_script = "addr(tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8)"

# Coinbase rotation settings (only used when coinbase_reward_script has a wildcard)
# Path to persist the current derivation index (required for rotation)
# coinbase_index_file = "/var/lib/pool/coinbase_index.dat"
# Starting index if no persistence file exists (default: 0)
# coinbase_start_index = 0
```

**The stated motivation differs from para's.** This branch says *"quantum-resistant payout hygiene"* — i.e. don't leave a long-lived pubkey exposed across many blocks. Para's PR body frames the identical mechanism as **on-chain privacy** (don't let observers trivially total a pool's revenue from one address). Same mechanism, two independent justifications; neither wiki article has yet reconciled them, and the quantum framing in particular is an assertion in a config comment with no analysis behind it in-tree.

## 6. Three ways persistence is weaker than para's shipped design

Para solved this problem first and shipped it (476 tests passing). The differences are not stylistic.

**(a) Non-atomic write, silent fallback on corruption.**

```rust
fn persist_index(&self) -> Result<(), XpubDerivationError> {
    let index = self.current_index.load(Ordering::SeqCst);
    fs::write(&self.index_file, index.to_string())
        .map_err(XpubDerivationError::PersistenceError)
}

fn load_index(path: &Path, default: u32) -> u32 {
    match fs::read_to_string(path) {
        Ok(contents) => contents.trim().parse().unwrap_or(default),
        Err(_) => default,
    }
}
```

A bare `fs::write` with no temp-file-plus-rename and no fsync. A crash mid-write, or a truncated/garbled file, parses to `Err` → `unwrap_or(default)` → **the derivation index silently resets to `coinbase_start_index` (default 0) and the pool replays addresses from the beginning.** That is exactly the address-reuse the feature exists to prevent, reached by the quietest possible path. Para's equivalent persists the BDK `ChangeSet` to redb with **`Durability::Immediate`**.

**(b) Increment-then-persist, warn-only.** `next_script_pubkey` advances the in-memory `AtomicU32` *first*, derives, and then only `warn!`s if the write fails. So the pool can mine a block to index N while disk still says N−1; after a restart index N is handed out again. Para explicitly evaluated and **rejected** this ordering — plan Decision 4, option B:

> "**B. Push then persist** — risk: ckpool advertises an address whose derivation index isn't yet on disk; restart loses it."

Para's contract instead is: `reveal_address_with` "persists ChangeSet before returning, rolls back on failure. Address is never returned without being on disk."

**(c) No gap-limit knob at all.** Grepping the whole worktree for `gap_limit` / `gap-limit` across `*.rs` and `*.toml` returns **nothing**. Para ships `--pool-address-gap-limit` (advisory) precisely because, in its own words, "under sustained per-block rotation, the wallet burns O(blocks/day) addresses; descriptor restore on a fresh machine needs a longer scan." That recovery hazard is identical here and undocumented. It is arguably *worse* here, because sv2-apps holds only a bare integer in a flat file rather than a BDK wallet database — if that integer is lost, rescan-from-descriptor is the only recovery, and nothing tells the operator how far to scan.

**(d) Rotation fires on submission, not confirmation.** `SubmitSolution` is the moment the pool hands a solution to the Template Provider — not the moment the block is accepted and buried. An orphaned or rejected block burns a derivation index. This is the *same* accepted tradeoff as para (whose plan notes "revealed-but-never-paid addresses are normal under any rotation policy"), and burning indices is cheap. Worth recording only so it isn't rediscovered as a bug.

## 7. What this does *not* do: per-miner xpub usernames

The design question this source was fetched to answer — a Blitzpool-style PPLNS where a miner's username is an xpub/wildcard descriptor and the pool derives a fresh output address per miner per block — remains **unimplemented anywhere in reach.** This branch rotates *one* address belonging to *the pool*. It contains no accounting, no per-miner state, and no multi-output construction.

What it does supply toward that design:

1. **The derivation primitive, already `Send + Sync`.** `XpubDerivator` is in `stratum-apps` (the shared crate), not in either role, and takes a descriptor string + index file. A per-miner design instantiates N of these, or refactors to one descriptor-per-call.
2. **Proof the SV2 plumbing tolerates a mutable coinbase.** `coinbase_outputs` behind `SharedLock`, swapped between templates, with `CoinbaseOutputConstraints` re-derived — the machinery does not assume a static payout script.
3. **The `has_wildcard()` validation gate**, which any xpub-username design needs at *registration* time. A miner supplying `wpkh(xpub...)` without `/0/*` would otherwise be paid to the same address forever — the identical footgun, now at miner granularity, where the operator has hundreds of chances to miss it.

Two hard blockers that this source does nothing about:

- **The re-parse-per-derivation cost becomes load-bearing.** Free at once-per-block for one address; not free at once-per-miner-per-template. §1's `Send + Sync` workaround would need revisiting (cache `Descriptor` per miner behind the lock, or accept the parse).
- **Ledger identity.** Per the Blitzpool Postgres verification in [[../../output/plan-lottery-pplns-1776-rewiring-2026-07-28|the 1776 rewiring plan]], `pplns_balance` upserts `ON CONFLICT (address) DO UPDATE` — balances are keyed on **address, globally, not per block**. A rotating per-miner address writes a *new* balance row every block and breaks pending-credit carry-forward outright. Any xpub-username design must split identity: descriptor/fingerprint for the ledger, derived address for the coinbase output. Nothing in this branch addresses that, and nothing in it needs to — the pool's own address never enters the miner ledger.

## 8. Provenance caveats

- **Unmerged, single-branch, no upstream review.** No PR discussion was read; the branch may never land, and it reverses a decision upstream took deliberately (§2).
- **Commit metadata is unreliable.** `817a1057` is authored `Test User <test@test.com>`, and the two commits' author dates are non-monotonic (`e2930150` dated 2026-01-29 sits on top of `817a1057` dated 2026-03-20). Treat the dates as noise; the content and hashes are what's verified here.
- **No integration or regtest coverage of rotation.** The 12 unit tests exercise `XpubDerivator` thoroughly in isolation, including restart. Nothing tests a Pool or JDC actually rotating across a found block — no regtest, no e2e. The two-site trigger duplication in each role is the kind of thing an integration test exists to catch.
- **Testnet4 only in the examples**, with `tb1q...` addresses already present upstream. Nothing mainnet-facing is configured.

## See also

- [[2026-07-27-blitzpool-finder-bonus-code-read]] — per-finder speculative coinbase construction; the architecture an xpub-username design would plug into
- [[2026-07-27-blitzpool-server-rust-github]] — the non-custodial in-coinbase payout pool
- [[2026-05-23-stratum-v2-spec]] — Template Distribution / `SubmitSolution`, the rotation trigger
- [[../../output/plan-lottery-pplns-1776-rewiring-2026-07-28|Lottery-PPLNS 1776 rewiring plan]] — the `pplns_balance ON CONFLICT (address)` finding that blocks naive per-miner rotation
- `para` local wiki (`REPOS/para/.wiki/output/`) — `plan-coinbase-rotation-2026-06-24.md` and `pr-body-coinbase-rotation.md`: the shipped ckpool+BDK design this is compared against throughout
