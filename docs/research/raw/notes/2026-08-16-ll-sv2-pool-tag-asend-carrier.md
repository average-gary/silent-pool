---
title: "Lessons Learned: carrying a rotating A_send in the SV2 coinbase pool tag"
type: lessons-learned
source: session
date: 2026-08-16
tags: [lessons-learned, bip352, silent-payments, coinbase, stratum-v2, pool-tag, scriptsig, forward-secrecy, bip32, key-erasure, sv2-apps, sri]
lesson_count: 6
category: notes
confidence: high
summary: "Two carrier designs for the sender-side ECDH key died on the same requirement (a_send must be destroyable and must authorize nothing) via completely different mechanisms — bare multisig gives it spending authority, BIP 32 wildcard derivation makes it unrecoverable-proof; plus the exact SV2 scriptSig budget from pinned SRI source, where the pool tag is a UTF-8 String with 64 bytes of headroom."
---

# Lessons Learned: carrying a rotating A_send in the SV2 coinbase pool tag

> Extracted from session on 2026-08-16. Six lessons covering carrier selection for the
> sender-side ECDH key `a_send`, the exact Stratum V2 coinbase `scriptSig` byte budget as
> implemented (not as specified), and where the source of truth for byte budgets actually lives.

**The thread that unifies lessons 1 and 2.** `a_send` buys exactly two properties: losing it costs
privacy and never funds, and destroying it destroys history. Every design decision has to be checked
against both. In this session both properties were nearly lost — through two mechanisms with nothing
in common. Lesson 1 is a key gaining *spending authority* it shouldn't have; lesson 2 is a key
becoming *unerasable* because a parent secret regenerates it. Neither was caught by the question that
had been asked ("can the scanner read `A_send`?"). Both are caught by asking two questions instead:
**what does this key authorize, and can it actually be destroyed?**

## Lesson 1: Evaluate a key by what it authorizes, not by what a scanner can read

**Category**: correction
**Context**: Choosing where `A_send` (the pool's per-block ECDH pubkey) travels in a coinbase. A
prior session had concluded that the pool's own fee output could carry it as a bare 1-of-2 multisig,
`OP_1 <A_send> <A_pool> OP_2 OP_CHECKMULTISIG`, at +37 B over a P2TR output. That conclusion was
written into `index.html` ("That works"), `README.md`, the candidate record's variant table, and case
11's own comment in the test vectors — then published.
**Symptom**: No error, no failing test. Case 11 passed and still passes. The claim was simply wrong,
and it was wrong in a published repo for two days.
**Root cause**: The multisig row was reasoned entirely from *"what can a scanner read from this
output?"* — to which bare multisig is a good answer, since it reveals both keys in the scriptPubKey.
The question never asked was *"what does this key now authorize?"* `m = 1` means any one listed key
authorizes a spend. Case 11's comment asserted that `a_send` is "never needed to spend", which is
true and irrelevant: it is **sufficient** to spend. Whoever obtains the erasable privacy key takes
the pool's fee. That is precisely the defect that had already ruled out a taproot carrier one
paragraph earlier (a P2TR output exposes one group element whose dlog is the keypath spend secret),
just arrived at by a different route. Bare multisig separates visibility from spendability
*mechanically* but not *economically*.
**Fix**: Case 11 relabelled `REJECTED CARRIER` — kept, not deleted, because its mechanics (scanner
parses key 0, `A_send` survives the scriptPubKey round-trip byte-exactly, scan+spend path untouched)
are true and worth pinning. Verdict and both failure directions written into the case comment and the
runner comment. `index.html` §6 and `README.md` corrected, table row struck. Deliberately **not**
faked into an executable failure: the defect is a fact about script semantics, not about the
derivation, so no assertion over these vectors can discover it, and a tautological assert would be
theatre. The runner comment says so outright rather than letting passing asserts read as a security
check.
**Rule**: A key whose entire value is that it can be lost cheaply must never appear in a scriptPubKey
that controls money — put it in a data field, never in a spending condition. And raising the
threshold does not rescue such a design: `m = 2` inverts the failure, since `a_send` must then sign
and therefore cannot be erased until the output is spent. Check *what a key authorizes*, not only
*what an observer can read*.

## Lesson 2: BIP 32 unhardened derivation *is* the group-linear forward-secrecy defect

**Category**: discovery
**Context**: The `feat/coinbase-rotation` branch at
`~/repos/stratum-mining-worktrees/sv2-apps-coinbase-rotation` implements per-block coinbase address
rotation for the SRI pool, and was offered as the plumbing to make `A_send` rotate every block. The
obvious move is to reuse its key source, `XpubDerivator`, as well.
**Symptom**: None yet — this was caught before implementation, which is the only reason it is a
lesson and not an incident.
**Root cause**: `stratum-apps/src/config_helpers/xpub_derivation.rs:212` derives via
`descriptor.at_derivation_index(index)` on a wildcard descriptor like `wpkh(xpub.../0/*)` — BIP 32
unhardened child derivation. Written out:

```
BIP 32 unhardened:   a_i = a_par + H(K_par ‖ i)      A_i = A_par + H(K_par ‖ i)·G
vectors' case 9:     a_H = a_0   + H(A_0   ‖ H)      A_H = A_0   + H(A_0   ‖ H)·G
```

These are the **same construction** — an additive tweak by a publicly computable scalar. Case 9 is
already in the test vectors as a *negative control* proving this shape is fatal for `A_send`: holding
`a_0` recovers every `a_H`, every past `ecdh`, and retroactively unmasks every payout the pool ever
made. Wiring `A_send` through `XpubDerivator` would reintroduce, verbatim, the defect the suite
exists to demonstrate. The reason this is easy to miss is that the *same machinery is correct* for
the thing it was written for: for the pool's own payout address, recoverability from the xprv is a
**feature** — the pool must be able to spend those coins, possibly after a disaster. The two keys
have exactly inverted requirements.

| | pool payout key | `a_send` |
|---|---|---|
| recoverable from a seed | **required** — must be able to spend | **fatal** — must be able to destroy |
| correct engine | BIP 32 wildcard descriptor | independent CSPRNG + erasure schedule |

**Fix**: Take the plumbing, not the key engine — confirmed as the settled decision by the user
mid-session ("we don't want to use the key derivation, just the plumbing"). `A_send` values must be
independent CSPRNG output with a retention/erasure schedule, never derived from a common parent.
**Rule**: Any additive-tweak key ladder — BIP 32 unhardened derivation and wildcard descriptors
included — is unusable for a key that must be *destroyable*, because the parent secret regenerates
every child. Before reusing a rotation mechanism, separate its **plumbing** (when and how state
changes per block) from its **key derivation** (where the secrets come from); the first is usually
portable and the second usually is not, because the two use cases invert on whether recoverability is
a feature.

## Lesson 3: When the nonce is already bound, rotation cadence is a privacy knob, not a correctness requirement

**Category**: discovery
**Context**: Auditing whether the rotation branch's trigger is fast enough for `A_send`.
`rotate_coinbase_address()` fires from both share handlers
(`pool-apps/pool/src/lib/channel_manager/mining_message_handler.rs:854`, `:1133`) only when a
`SubmitSolution` is routed to the Template Provider — i.e. when **this pool** finds a block. Initial
read: too coarse, a blocker.
**Root cause**: It conflates two different kinds of freshness. Payout address freshness does not
depend on rotating `A_send` at all, because `input_hash = H(ser32(H) ‖ A_send)` binds the block
height — so every height produces a different shared secret and different outputs even with a
completely static `A_send`. What rotation actually bounds is **forward-secrecy granularity**: a
single `a_send` unmasks every block it covered. At block-found cadence, a small pool's one `a_send`
could cover weeks of blocks.
**Fix**: Reclassified from blocker to tunable. Per-template rotation is the finest useful cadence and
is strictly better: it scans correctly without extra machinery, because the miner reads `A_send` out
of the *winning* block's scriptSig and so never needs to know which templates lost, and the pool can
erase `a_send` the moment a template is superseded.
**Rule**: Distinguish freshness-of-outputs from freshness-of-keys. If a per-event nonce is already
committed into the shared secret, key rotation buys forward-secrecy granularity only — so treat its
cadence as a tunable knob and state what one key's exposure would reveal, rather than asserting a
rotation frequency as a correctness requirement.

## Lesson 4: The SV2 pool tag is a UTF-8 `String`, and its byte budget is exact

**Category**: gotcha
**Context**: Sizing a 32/33-byte `A_send` into the SRI pool's coinbase tag. The tag is real and
first-class — `pool_tag_string`, set from TOML `pool_signature` — not something to be invented.
**Symptom**: A 32-byte x-only pubkey cannot be placed in it at all. Not a size failure; a type
failure.
**Root cause**: `pool_tag_string: Option<String>` (`channels-sv2/src/server/jobs/factory.rs:85`),
fed from `pool_signature: String` (`sv2-apps` `pool-apps/pool/src/lib/config.rs:36`) and consumed via
`.as_bytes()`. A Rust `String` — and a TOML value — must be valid UTF-8, and 32 arbitrary bytes
essentially never are. So `A_send` needs a *text encoding*, and the choice is load-bearing, because
the budget is tight. Assembled layout (`factory.rs:157`, built at `:697`):

```
coinbase_prefix ‖ OP_PUSHBYTES_N ‖ Sv2/<pool>/<miner>/ ‖ OP_PUSHBYTES_M ‖ extranonce
```

`8` (worst-case `coinbase_prefix`, `MAX_COINBASE_PREFIX_SIZE`) `+ 1` (opcode) `+ 3` (`"Sv2"`) `+ 3`
(three `/` delimiters) `+ pool_len + 1` (extranonce opcode) `+ 20` (`FULL_EXTRANONCE_SIZE`) ≤ `100`
(`MAX_SCRIPT_SIG_SIZE`) → **`pool_len ≤ 64`**. The `20` is not an assumption: it is
`POOL_SERVER_BYTES 1 + bytes_needed(16_777_216) = 3 + CLIENT_SEARCH_SPACE_BYTES 16`, a `const` in
`sv2-apps` `pool-apps/pool/src/lib/channel_manager/mod.rs:57-67` — not configurable, so freeing
extranonce bytes needs a recompile.

| encoding of `A_send` | `pool_len` | scriptSig | |
|---|---|---|---|
| current default `"Stratum V2 SRI Pool"` | 19 | 55/100 | baseline |
| hex, x-only (32 B) | 64 | **100/100** | fits with **zero** slack |
| hex, compressed (33 B) | 66 | 102/100 | **fails** |
| bech32, x-only | 52 | 88/100 | 12 B slack |
| base64url, x-only | 43 | 79/100 | 21 B slack |

Hex x-only landing exactly on the ceiling is a coincidence, not a margin — it leaves no room for a
miner tag (the JDC path sets one), any residual ASCII identity, or a larger extranonce. Standard
base64 is disqualified outright: `/` is in its alphabet and `/` is the tag delimiter. A second,
independent cap also applies — the payload must be 1..=75 for the single-byte push opcode
(`factory.rs:129`), which hex x-only passes at 70.
**Fix**: Recommend bech32 or base64url over hex; note that since the code is being patched anyway,
changing the tag to `Vec<u8>` recovers ~20 bytes and removes the question entirely.
**Rule**: Before sizing a binary payload into a protocol "tag"/"signature" field, check the field's
**type**, not just its length limit — a `String`/TOML field is UTF-8-only, so binary needs a text
encoding, and the encoding's alphabet must avoid the format's own delimiters. Requiring x-only rather
than compressed saved 2 bytes and was the difference between fitting and failing.

## Lesson 5: Prefer a carrier whose invariant the wire format enforces over one needing implementer discipline

**Category**: discovery
**Context**: The hard constraint on any scriptSig carrier is that `A_send` must not overlap the
extranonce bytes miners roll millions of times a second — otherwise every hash attempt would change
every payout address. This had been recorded as a hand-written invariant and asserted by hand in the
test vectors (case 12).
**Root cause / discovery**: Under SV2 it is not an invariant anyone has to maintain. For extended
channels the coinbase is assembled as
`coinbase_tx_prefix ‖ extranonce_prefix ‖ extranonce ‖ coinbase_tx_suffix`, so the rolled region is
*defined* as the gap between two pool-set halves. And the tag provably lands in the prefix: the
`coinbase_tx_prefix` split index explicitly adds `pool_miner_tag_len` (`factory.rs:731`). The tag
therefore **cannot** be rolled. Confirmed in code, not just spec prose. Separately, the 100-byte cap
is enforced twice — conservatively at channel construction (`fits_script_sig_budget`, `:184`,
assuming the worst-case 8-byte prefix) and authoritatively per-template in `coinbase()` (`:662`) —
returning `ScriptSigSizeTooLarge`, so an over-budget tag fails loudly at channel setup rather than
silently at block submission.
**Fix**: Ranked the scriptSig-as-pool-tag above the BIP 141 witness-commitment optional-data field
among on-chain carriers, on this ground rather than on bytes.
**Rule**: When choosing between carriers of equal byte cost, prefer the one whose safety invariant is
structurally enforced by the message format over the one that depends on an implementer remembering
it — and say which kind of guarantee you have. A related finding from the same read: SV2 reserves the
worst-case 400 WU / 100 B for `scriptSig` unconditionally, and unused bytes are *not* returned to
fee-paying transactions, so the marginal on-chain cost of a 34-byte push is **0, not 34**. Check
whether a budget is pre-paid before pricing your use of it.

## Lesson 6: For byte budgets, the spec gives caps and the implementation gives allocations — read pinned local source

**Category**: pattern
**Context**: Answering "does the pool tag have room for 33 bytes?" First attempt used the published
spec; the real answer needed the code.
**Symptom**: `https://stratumprotocol.org/specification/` returns **HTTP 403** to fetchers. A
summarizer over the raw spec markdown then reported "No explicit `MAX_EXTRANONCE_LEN` constant is
defined" and "no mention of the 100-byte limit" for a section that does discuss the budget — fetched
paraphrases drop exactly the constants a byte-budget question is about. And the decisive number,
`FULL_EXTRANONCE_SIZE = 20`, does not exist in the spec at all: the spec caps the extranonce by field
type (`B0_32` + `B0_32` = 64 B ceiling) while the actual allocation is a `const` in the pool app.
Reasoning from the spec alone produced a correct *shape* with a headroom estimate off by 3× and no
knowledge of the UTF-8 blocker.
**Root cause**: Specs state ceilings; implementations state allocations. A budget question needs
both, and only one of them is in the spec.
**Fix**: The user pointed at local clones. They are **not** at `~/repos/sv2-apps` — the working set
is a superrepo, `~/repos/stratum-mining/{stratum, sv2-apps, sv2-spec, stratumprotocol.org, sv2-tp,
sv2-wizard}`, with branch worktrees at `~/repos/stratum-mining-worktrees/<name>/` (the rotation work
is `sv2-apps-coinbase-rotation`, branch `feat/coinbase-rotation`). Read the pinned source directly —
`channels_sv2` is a git dependency, so `Cargo.lock` names the exact commit — and verified the
arithmetic with a throwaway script rather than by hand. Worth noting both repos' HEAD commits were
scriptSig audits (`sv2-apps#658` "scriptsig-defects-companion", `stratum#2243`
"scriptsig-serialization-defects"), so these constants are freshly reviewed.
**Rule**: For any "does X fit in Y bytes?" question, get the ceiling from the spec and the actual
allocation from the pinned implementation — never from a fetched summary, which silently drops
constants. Check `Cargo.lock`/lockfile for the exact dependency commit rather than reading whatever
`main` currently says, and check for a local clone before fetching: the local checkout may be newer
than the published spec and is the thing that will actually run.
