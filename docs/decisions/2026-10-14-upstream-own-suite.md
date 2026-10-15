# 2026-10-14 — silent-pool implements the entire tooling suite itself; upstream forks are reference/starting-point pins, not gates

**Trigger.** Implementation-readiness review B3/H4 (`docs/reviews/2026-10-14-implementation-readiness-review.md`): Phase 4 read as hard-depending on an unmerged spec PR and open fork PRs, with no stated contingency and no way to verify pins.

## Decision

silent-pool implements the **entire tooling suite** itself: pool+JDS module, JDC, translator, scanner, agent. sv2-spec PR #203 text and the warioishere forks are REFERENCE / STARTING-POINT pins, not gates. Upstream drift or conflict is addressed in-tree in our own implementation.

## Why

- **No upstream merge gate.** PR #203 has been open for months; waiting for it lands, stabilises and reaches our fork points would delay implementation indefinitely.
- **Own suite is a project requirement.** The unique JDC+translator+agent bundling and SPc enforcement exist nowhere in upstream; making them work against a moving upstream target is more work than maintaining isolated pin.
- **Pin guarantees known-good starting point.** The SHA-pinned text at 47dbedd gives a concrete, unchanging target for the JD payout semantics; the warioishere forks demonstrate a working JDC against it.
- **Upstream deviation manageable in-tree.** If PR #203 merges with incompatible changes, we already have our own implementation that diverged; we evaluate those changes as patches to our own code, not as mandates to rebase.

## Consequences

- `UPSTREAM_DEPS.md` pins (47dbedd, 9be4023, 90b3e3d, etc.) are reproducibility / maximum-reuse pins, not gating dependencies. Breaking changes from upstream can be applied selectively in-tree or ignored.
- Phase 4 (JD mode) depends only on our own JDS module (`spc-jds` inside `front`) and our own JDC (`spc-agent`'s JDC role) implementing the spec; the PR's openness does not block it.
- Roadmap language that read as an upstream dependency is reworded as "spec text per PR #203 frozen at 47dbedd is what we implement".
- No "may need rebase if upstream merges" hedging remains; the implementation is ours.
- Pins stay normative for reproducibility and are verified mechanically (`UPSTREAM_DEPS.md` "Verification"); re-pinning is a deliberate change citing a decision record.

## Edits applied

- `UPSTREAM_DEPS.md`: framing line under the header; new "Verification" section (checkout, Cargo.lock, submodule and spec-text checks; re-pin policy); the Blitzpool row refers to the in-`front` JDS module.
- `docs/ROADMAP.md`: R3 risk text and mitigation; Phase 4 **Dependencies** line; changelog entry 2026-10-14 (b).
