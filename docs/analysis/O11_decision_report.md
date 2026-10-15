# O11 Decision: JD-Mode Booking Formula

> **Status: exploratory analysis, not normative.** The decision is `docs/decisions/2026-10-14-o11-claim-formula.md` and SPEC §4.6; they govern. Caveats: the tables below mix units (shares in sats against `W = 100`) and are illustrative only, not checked arithmetic; `F` is described here as the pool fee, whereas the spec's `F` is the PR #203 §4 fixed-output total (the pool's percentage fee sits inside `weight_P`); and Formula B is described here as non-negative while the decision record notes it needs a signed ledger once carried balances are debited. Conclusion (Formula A, revenue-scaled) is unchanged.

## Quantitative Analysis Report

### Summary
After modeling both candidate formulas across 4 concrete scenarios, **Formula A (revenue-scaled)** is recommended for silent-pool v0.1.

### Key Scenarios & Results

**Base Parameters:**
- T = 500M sats, F = 1% (5M), W = 100
- Identities: sp_A (60%), sp_B (30%), sp_C (10%)
- Raw shares: A=297M, B=148.5M, C=49.5M

#### Scenario Table

| Scenario | T_actual | Key Condition | Formula A Result | Formula B Result | Verdict |
|----------|----------|---------------|-----------------|-----------------|---------|
| **S1**: Honest JDC | 500M | T_actual = T_claimed | Identical: pending' = 0 | Identical: pending' = 0 | Tie |
| **S2**: Revenue drop | 450M | -10% revenue | Scales all claims down | Scales raw share only | A: Fair scaling B: Pool risk |
| **S3**: Revenue rise | 550M | +10% revenue | Scales all claims up | Scales raw share only | A: Fair scaling B: Miner gain |
| **S4**: Pending carry-over | 300M | C has 20M pending | pending' scaled to ~11.9M | pending' stays 20M | A: Value-adjusted B: Guaranteed |

### Failure Modes Analysis

| Risk Area | Formula A | Formula B | Winner |
|-----------|------------|------------|--------|
| **Negative pending** | None (scales ≥0) | None (carries ≥0) | Tie |
| **JDC weight game** | Vulnerable | Vulnerable | Tie |
| **Revenue attacks** | Self-correcting | Pool bears risk | **A** |
| **Rounding attacks** | Minor dust | Minor dust | Tie |
| **Ark tier scaling** | Economically correct | Predictable but irrational | **A** |
| **Weight trim** | Value-adjusted | Guaranteed amount | **A** |
| **Type-32 audit** | Simple: owed × factor | Complex: raw×factor + pending | **A** |

### Recommendation: Formula A

**Definition for SPEC.md §4.6/O11:**
```
In JD mode, the pool books pending'(id) = claim(id) - paid(id) where:
  claim(id) = floor(owed(id) · (T_actual − F) / W)
  owed(id) = raw(id) + pending(id)
  T_actual = Σ coinbase outputs
  F = pool fee (fee_bps/10000 · T)
  W = total window weight
```

**Rationale:**
1. **Auditability:** Single scaling factor enables clean type-32 verification
2. **Economic rationality:** Pending balances are future revenue claims - should scale with actual revenue
3. **Risk allocation:** Pool doesn't bear revenue variance risk for pending balances
4. **Simplicity:** Maintains unsigned ledger, avoids signed ledger complexity
5. **Blitzpool alignment:** Matches "scale everything" approach referenced in SPEC.md

**Edge Rules:**
- `pending'(id) ≥ 0` guaranteed (non-negative owed × non-negative factor)
- Use `floor()` for integer sats
- If `T_actual ≤ F`, all `claim(id) = 0`

**Implementation Impact:**
- Resolves O11 blocking Phase 4 implementation
- Consistent with Ark tier (pending scales when moving to coinbase)
- Compatible with weight-budget trim (trimmed pending scales with next block)

### Decision
**Approve Formula A** for silent-pool v0.1. Update SPEC.md §4.6 with the definition above, close O11.

---
*Analysis based on SPEC.md §4.6, §7.6, §10 O11; PPLNS-JD documentation; and quantitative modeling of 4 concrete scenarios.*