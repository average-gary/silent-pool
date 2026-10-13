# 2026-10-12 — License: MIT OR Apache-2.0, not AGPL

**Context.** The spec interview chose AGPL-3.0-or-later for one reason: to permit porting Blitzpool's `bp-pplns` ledger / trim / autoscaler code directly. Every other dependency is MIT or Apache (SRI `stratum-core`, the `sv2-apps` forks, `extensions_sv2`, bark / `ark-lib`), and the `spc_ark` prototype is MIT.

**Finding.** As of this date silent-pool uses **no Blitzpool code** — only architecture (role split, pending-credit ledger, keep-largest weight-budget trim, autoscaler constants, payout-list fingerprint, confirmation-gated idempotent apply) and a reading reference for the `0x0003` JDS. Spec §4.6 / §4.8 / §7.4 specify the arithmetic completely, with proptests pinning conservation; re-implementing it is ~200 lines.

**Decision.** MIT OR Apache-2.0. Re-implement the Blitzpool-derived math from the spec under a clean-room rule (spec §7.6): write from the spec text and tests, do not read `crates/bp-pplns/src/distribution.rs` while doing so. Blitzpool remains cited as provenance (`7815884`) and as the `0x0003` JDS reading reference (`b48998f`).

**Consequences.** Roadmap Decision 5 / Phase 0 task / R13 updated; spec §7.6 rewritten; `LICENSE` replaced by `LICENSE-MIT` + `LICENSE-APACHE`; `spc_ark/Cargo.toml` (`license = "MIT"`) is consistent. Operators who modify the pool are not obliged to publish changes.
