# O11 Analysis: JD-Mode Booking Formulas

> **Status: exploratory analysis, not normative.** The decision is `docs/decisions/2026-10-14-o11-claim-formula.md` and SPEC §4.6; they govern. Caveats: the tables below mix units (shares in sats against `W = 100`) and are illustrative only, not checked arithmetic; `F` is described here as the pool fee, whereas the spec's `F` is the PR #203 §4 fixed-output total (the pool's percentage fee sits inside `weight_P`); and Formula B is described here as non-negative while the decision record notes it needs a signed ledger once carried balances are debited. Conclusion (Formula A, revenue-scaled) is unchanged.

**Date:** 2026-10-14  
**Analyst:** Quantitative Analyst Subagent  
**Context:** silent-pool SPEC.md §4.6, §7.6, §10 O11 (line 787)

## Executive Summary

After analyzing both candidate formulas for JD-mode booking, **Formula A (revenue-scaled)** is recommended for silent-pool v0.1. Formula A maintains auditability, aligns with miner expectations of value scaling, and avoids creating a signed ledger with potential complexity. The key rationale is that pending balances represent *future claims on pool revenue*, which should scale with actual revenue realization.

## Formula Definitions

### Current Context (SPEC.md §4.6)
- `owed(id) = raw(id) + pending(id)` where pending ≥ 0
- `pending'(id) = claim(id) - paid(id)`  
- `paid(id)` = actual satoshis in mined coinbase output for identity `id`
- `T_actual` = Σ coinbase outputs (actual mined revenue)
- `T` = snapshot's expected revenue (template revenue)
- `F` = pool fee
- `W` = total window weight

### Candidate A: Revenue-Scaled
```
claim(id) = floor(owed(id) · (T_actual − F) / W)
```
**Properties:**
- All pending balances scale with actual revenue
- Maintains unsigned ledger (pending ≥ 0)
- Matches Blitzpool's "scale everything" approach

### Candidate B: Fixed-Sat Signed Ledger  
```
claim(id) = floor(raw(id) · (T_actual − F) / W) + pending(id)
```
**Properties:**
- Pending balances carried exactly (not scaled)
- Can theoretically result in negative pending (requires signed ledger)
- More complex audit trail

## Concrete Scenarios (3 Identities: sp_A 60%, sp_B 30%, sp_C 10%)

**Parameters:** `T` = 500,000,000 sats, `F` = 1% = 5,000,000 sats, `W` = 100  
**Raw shares (honest):** sp_A = 297M, sp_B = 148.5M, sp_C = 49.5M

### Scenario S1: Honest JDC (T_actual = T_claimed)
| Identity | raw | pending | owed | Formula A claim | Formula B claim | paid | Formula A pending' | Formula B pending' |
|----------|-----|---------|------|-----------------|-----------------|------|-------------------|-------------------|
| sp_A     | 297M| 0       | 297M | 297M            | 297M            | 297M | 0                 | 0                 |
| sp_B     | 148.5M| 0     | 148.5M | 148.5M          | 148.5M          | 148.5M | 0               | 0                 |
| sp_C     | 49.5M| 0       | 49.5M | 49.5M           | 49.5M           | 49.5M | 0                 | 0                 |

**Result:** Both formulas identical when no prior pending.

### Scenario S2: JDC Collects LESS Revenue (T_actual = 450M, T_claimed = 500M)
Revenue drops 10% due to falling feerate.

| Identity | raw | pending | owed | Formula A claim | Formula B claim | paid | Formula A pending' | Formula B pending' |
|----------|-----|---------|------|-----------------|-----------------|------|-------------------|-------------------|
| sp_A     | 297M| 0       | 297M | 267.3M          | 267.3M          | 267.3M | 0                 | 0                 |
| sp_B     | 148.5M| 0     | 148.5M | 133.65M         | 133.65M         | 133.65M | 0               | 0                 |
| sp_C     | 49.5M| 0       | 49.5M | 44.55M          | 44.55M          | 44.55M | 0                 | 0                 |

**Result:** Both identical when no prior pending. Miners share revenue drop proportionally.

### Scenario S3: JDC Collects MORE Revenue (T_actual = 550M, T_claimed = 500M)
Revenue increases 10% from rising feerate.

| Identity | raw | pending | owed | Formula A claim | Formula B claim | paid | Formula A pending' | Formula B pending' |
|----------|-----|---------|------|-----------------|-----------------|------|-------------------|-------------------|
| sp_A     | 297M| 0       | 297M | 326.7M          | 326.7M          | 326.7M | 0                 | 0                 |
| sp_B     | 148.5M| 0     | 148.5M | 163.35M         | 163.35M         | 163.35M | 0               | 0                 |
| sp_C     | 49.5M| 0       | 49.5M | 54.45M          | 54.45M          | 54.45M | 0                 | 0                 |

**Result:** Both identical when no prior pending. Miners share revenue gain.

### Scenario S4: Pending Carry-Over with Revenue Drop
**Block 1:** sp_C accumulates 20M pending (e.g., from Ark tier or trim)  
**Block 2:** Revenue drops to 300M (T_actual = 300M, T_claimed = 500M)

| Identity | raw | pending | owed | Formula A claim | Formula B claim | paid | Formula A pending' | Formula B pending' |
|----------|-----|---------|------|-----------------|-----------------|------|-------------------|-------------------|
| sp_C     | 49.5M| 20M     | 69.5M| 41.42M          | 49.5M           | 29.5M | 11.92M           | 20M               |

**Key Difference:**
- **Formula A:** Pending scaled down (20M → 11.92M) reflecting revenue drop
- **Formula B:** Pending carried exactly (20M → 20M) despite revenue drop

## Failure Modes Analysis

### 1. Negative Pending Risk
- **Formula A:** `pending' ≥ 0` always (scales non-negative owed by non-negative factor)
- **Formula B:** `pending' = pending` (exact carry) - also non-negative if starting non-negative
- **Both** maintain unsigned ledger when starting with pending ≥ 0  
- **Edge case:** If Formula B were modified to allow debits, it would require signed ledger (SPEC.md §7.6 warns of this complexity)

### 2. JDC Gameability
**Both formulas equally vulnerable to:**
- Weight inflation/deflation: JDC can declare wrong `raw(id)` proportions
- Type-32 audit mitigates this by letting miners verify shares
- **No difference** between formulas here

### 3. Revenue Declaration Attacks
**JDC declares T_claimed ≠ T_actual:**
- **Formula A:** Scales ALL pending balances by `T_actual/T_claimed`
  - Honest scaling: reflects actual revenue
  - Attack vector: JDC could declare high T_claimed, collect low T_actual to reduce pool's pending liability
- **Formula B:** Pending balances unaffected by T_actual
  - Pool absorbs revenue variance risk
  - More stable for miners with pending
  - Pool could lose if T_actual < T_claimed

### 4. Rounding Attacks
**Both formulas:** Use `floor()` - rounding dust accumulates to pool
- JDC could manipulate weights to maximize rounding dust
- Attack limited by integer precision (~1 sat per identity per block)
- **No significant difference**

### 5. Ark Tier Interaction (§11.1)
Ark-tier balances accumulate `pending` when below threshold C:
- **Formula A:** Ark pending gets scaled when moved to coinbase tier
  - Miner's "savings" could lose/gain value with revenue changes
  - Economically rational: reflects actual future revenue
- **Formula B:** Ark pending carried exactly  
  - Predictable for miners
  - Pool bears revenue variance risk

### 6. Weight-Budget Trim (§4.8)
When coinbase full, excess owed → pending:
- **Formula A:** Trimmed pending scaled by next block's revenue
  - Fair: reflects actual value of next block
  - Aligns with "value of waiting" depending on next block's revenue
- **Formula B:** Trimmed pending carried exactly
  - Trimmed miner gets guaranteed amount next block
  - Predictable but may over/under-compensate

### 7. Type-32 Auditability (§3.7)
Miners audit via share-accounting extension:
- **Formula A:** `claim = owed × factor`
  - Simple: one multiplication with public factor
  - `factor = (T_actual - F)/W` is publicly verifiable from blockchain
  - Clean audit trail
- **Formula B:** `claim = raw × factor + pending`
  - More complex: two terms, pending doesn't scale
  - Harder to explain/audit
  - Mixed scaling breaks simple proportionality

## Recommendation: Formula A (Revenue-Scaled)

### Rationale
1. **Auditability:** Single scaling factor makes type-32 audits straightforward
2. **Economic Rationality:** Pending balances represent *future claims on pool revenue* - should scale with actual revenue realization
3. **Alignment with Blitzpool:** Matches "scale everything" approach referenced in SPEC.md §4.6
4. **Simplicity:** Maintains unsigned ledger, avoids signed ledger complexity
5. **Risk Allocation:** Pool doesn't bear risk of revenue drops for pending balances - miners share variance proportional to their claims

### Spec-Ready Definition for O11
```
For JD mode, the pool books:
  claim(id) = floor(owed(id) · (T_actual − F) / W)
  pending'(id) = claim(id) − paid(id)

where:
  owed(id) = raw(id) + pending(id)   # from §4.6 window accounting
  T_actual = Σ coinbase outputs       # actual mined revenue
  F = pool fee                        # 1% of T (snapshot's fee_bps)
  W = total window weight             # sum of all identities' weights
  paid(id) = value of coinbase output with script OP_1 ser_x(P_id)
  
Properties:
  - pending(id) ≥ 0 ⇒ pending'(id) ≥ 0 (unsigned ledger preserved)
  - Σ paid(id) + pay_P = T_actual      # conservation
  - When T_actual = T (pool-template mode), claim(id) = owed(id)
```

### Edge Rules
1. **Floor at zero:** `pending'(id) ≥ 0` enforced by formula (since `owed(id) ≥ 0` and `(T_actual - F) ≥ 0`)
2. **Rounding:** Use `floor()` for integer sats, consistent with bitcoin's satoshi precision
3. **Fee basis:** `F` computed as `fee_bps/10000 · T` (snapshot's fee percentage applied to expected revenue T)
4. **Zero protection:** If `T_actual ≤ F`, all `claim(id) = 0` (no revenue after fee)

### Implementation Notes
- **Type-32 verification:** Miners can verify `raw(id)` from shares, compute `owed(id)`, then verify `claim(id) = floor(owed(id) · (T_actual − F) / W)`
- **Ark tier:** Ark pending balances scale when moved to coinbase tier - economically correct
- **Weight trim:** Trimmed pending scales with next block's revenue - fair compensation for waiting

## Conclusion

Formula A provides the cleanest, most auditable solution for silent-pool v0.1. It maintains the unsigned ledger property, aligns with economic principles of revenue scaling, and supports the type-32 audit extension. The pool avoids taking on revenue variance risk for pending balances, which properly belongs to miners whose future claims are affected by actual block revenue.

**Recommendation:** Implement Formula A as defined above, resolving O11 in favor of revenue-scaled booking.