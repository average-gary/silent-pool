# 2026-10-14 — O14: default mailbox mode is epoch-derived (variant B)

**Trigger.** SPEC §11.4/§11.8 left the default mailbox mode open (`static` vs `epoch_derived` vs `epoch_registered`). Verified: a stock `captaind` requires zero server-side cooperation for both variants (server only unblinds; no server-side registration endpoint exists), so epoch-derived is compatible with a stock Second server.

## Decision

1. **Default = `epoch_derived` (variant B).** Per-epoch mailbox id `M_i = M + t_mb_i·G` derived by tweaking the registered static `M` with `t_mb_i = hash_SPc/Mailbox(ecdh_i)`. No per-epoch registration window; no fallback-to-static needed.
2. **No server-side registration exists.** "Registration" language refers to pool↔miner bookkeeping only. The Ark server only unblinds (`from_blinded`); it has no registration endpoint.
3. **Variant A (`epoch_registered`) remains documented** as an option for independent per-epoch keys (`m/352'/0'/0'/3'/i'` hardened).

## Why

- **No registration window or fallback.** Variant B derives `M_i` from data both sides already have (registered `M`, `D`, and the block's `ecdh_i`). Nothing new is sent per epoch.
- **Server can't link epochs.** `M_i` is unlinkable to `M` or `M_j` without `ecdh_i` (requires `b_scan` or `a_send`).
- **Stock server compatible.** Zero server-side changes; works with stock `captaind`.

## Caveats recorded

- **(a)** Variant B mailbox keys are additive tweaks of static `m` — anyone holding BOTH static `m` and `b_scan` can derive all `m_i`. Privacy-only exposure: VTXO spending needs `u_i = b_ark + t_ark_i`, which stays independent.
- **(b)** Network-level: reads of `M_i`, `M_j` from one IP/connection are linkable. Mitigate: new gRPC connection per epoch, Tor stream isolation where possible (MAILBOX.md §4).
- **(c)** "Registration" is pool↔miner bookkeeping only; the server has no registration endpoint.

## Edits applied

- `docs/SPEC.md`: §10 O14 row; §11.4 (no server-side registration endpoint); §11.5 Register bullet; §11.8 mailbox table, default and caveats; §11.10 `mailbox_mode` default; changelog entry 2026-10-14 (b).
- `MAILBOX.md`: default noted.
- `docs/ARCHITECTURE.md`: O14 references updated.
- `DESIGN.md`: open question noted as resolved.
