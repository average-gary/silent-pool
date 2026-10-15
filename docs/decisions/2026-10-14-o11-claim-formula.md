# 2026-10-14 — O11: JD-mode claim formula is revenue-scaled

**Trigger.** Phase 4 entry criterion: SPEC §4.6's settlement rule (`pending'(id) = claim(id) − paid(id)`) left `claim` undefined when the JDC's template revenue ≠ the pool's. Analysis: `docs/analysis/O11_formula_analysis.md`, `docs/analysis/O11_decision_report.md` (illustrative; they define `F` as the pool fee, which this record corrects).

## Decision

In JD mode, `claim(id) = floor(owed(id) · (T_actual − F) / W)` where `owed(id) = raw(id) + pending(id)`, `T_actual` = sum of mined coinbase outputs verified on-chain, `F` = the fixed-output total of PR #203 §4 exactly as in SPEC §3.5 validation (0 when `additional_outputs` is only the 0-sat `OP_RETURN`; the pool's percentage fee is already inside `weight_P`), `W` = total distribution weight (`Σ weight_id + weight_P`, SPEC §4.7). `pending'(id) = claim(id) − paid(id)`.

## Why

- **Unsigned ledger preserved.** `pending'(id) ≥ 0` always (non-negative `owed` × non-negative factor), matching §7.6's no-signed-ledger rule.
- **Type-32 auditability.** One publicly verifiable scaling factor `(T_actual − F) / W`; miners verify `claim = owed × factor` from on-chain data.
- **Matches pool-template mode.** When `T_actual = T`, `claim(id) = owed(id)` — the two modes converge.
- **Risk allocation.** Pending balances are future revenue claims; they scale with actual revenue realization. The pool does not absorb variance for carried balances.

## Alternatives rejected

- **Fixed-sat signed ledger** (`claim = floor(raw(id) · (T_actual − F) / W) + pending(id)`): keeps balances in sats but requires a signed ledger (can go negative), breaks single-factor auditability, and puts revenue variance risk on the pool.

## Edits applied

- `docs/SPEC.md`: §4.6 settlement paragraph, §10 O11 row; changelog entry (2026-10-14 (b)).
- `docs/ROADMAP.md`: Phase 4 entry criterion noted.
- `docs/ARCHITECTURE.md`: §7.2, §9, §10 O11 references updated.
- `docs/ROADMAP.md`: also changelog entry (2026-10-14).
