# 2026-10-13 — The JDS is a module of the `spc-pool` binary; JD is always on

**Trigger.** Operator decision during architecture mapping (docs/ARCHITECTURE.md): the SRI pool bundles JDS functionality in the pool binary, and silent-pool will always run JD — no opt-out.

## Decision

1. **`spc-jds` is a module of the `spc-pool` binary's `front` role, not a separate binary.** This matches the SRI layout, where the pool carries JDS functionality in-process. The roles list stays `front / payout / api / ark`; "spc-jds" remains the name of the JD-serving module inside `front` (JDP listener, `0x0003` publication and validation).
2. **JD is always on.** There is no `[jd] enabled` flag; the JDP listener, `SetPayoutDistribution` publication and `distribution_id` TLV validation are unconditional parts of `front`. (Pool-template mode still exists for miners without their own template; this decision is about the *server* always offering JD, not about forcing miners to use it.)

## Why

- The only reason the spec ever allowed a separate `spc-jds` process was fork provenance from `sv2-apps/pool-apps/jd-server`. With the SRI pool bundling JDS in the pool binary, that provenance argument disappears.
- SPEC §2.2 requires one `a_send` per height shared by `front` and the JDS. In-process, that sharing is a `&SendKey` reference — the "local authenticated channel, never Redis" hedge and its attendant interface, key-hygiene and test surface are deleted.
- One binary fewer to build, deploy, pin and monitor; the JDS's failure domain was always `front`'s anyway (co-location was already a MUST).

## Consequences

- `front` owns: SV2 MP listeners, the JDP listener, template stream, job building, distribution building **and** publication (`0x0003`), share validation, slice aggregation, block submit, `a_send[H]`.
- `spc:dist:current` (was "jds → front" for cross-process TLV validation) becomes intra-`front` state; it is no longer a Redis stream.
- `[jd] enabled` is removed from config; `distribution_interval_secs`, `distribution_rebalance_bps`, `grace_same_height_only` remain as `[jd]` parameters.
- **Multi-region caveat (resolved 2026-10-14):** SPEC §9 runs one `front` per region. With the JDS in-process, each regional `front` generates its own `a_send[H]`; that region's blocks carry that region's `A_send`. This is safe — miners scan `A_send` from the coinbase carrier, and channels/distributions are per-`front` — but it means `a_send` is per-`front`-process per height, not literally pool-wide, and `front` is not fully stateless. Full multi-region design: `docs/MULTI-REGION.md`.

## Edits applied

- `docs/ARCHITECTURE.md`: JDS folded into `front` throughout (diagrams, component table, crate graph, interfaces, secrets table, scoping notes).
- `docs/SPEC.md`: §1 architecture diagram and components table, §2.2 key-sharing bullet, §5.2 `spc:dist:current`, §6.3 `[jd]` config, §7.1 workspace tree, §8.4 harness roster, §9 topology; changelog entry.
- `docs/ROADMAP.md`: Phase 0 workspace bullet, Phase 4 heading/first bullet, Phase 8 harness; changelog entry.
- `README.md`: components table.
- `UPSTREAM_DEPS.md`: the Blitzpool `b48998f` row's "reading reference for spc-jds" now refers to the in-`front` JDS module.
