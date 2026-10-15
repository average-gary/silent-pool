> **Status (2026-10-15): input to SPEC §11.3/§11.10, not normative.** `docs/SPEC.md` governs where they differ. Adopted: BIP340 Schnorr, CSPRNG-generated key, hex secret at `order_key_path` with `0o600`/`0o700`, pubkey via `[ark] order_pubkey`, operator-driven rotation. Differences: `order_pubkey` is the **32-byte x-only** key (not 33-byte compressed); the signed message is `hash_SPc/ArkSendOrder(...)` per `docs/WIRE-FORMATS.md` §3.4 (not `sha256(chain-prefix ‖ …)`); the optional `order_sig_pubkey_prefix` is not adopted; the §2 generation shell snippets are illustrative only — any CSPRNG 32-byte draw (e.g. `openssl rand -hex 32`) is sufficient.

# SPc Order-Signing Key Spec Recommendation

**Research Date:** 2026-10-14  
**Source:** Bark @ c419159, SPEC.md §11, research/raw/repos/*  
**Status:** Ready for SPEC.md insertion at §11.10

---

## 1. Key Type

**Secp256k1 Schnorr (BIP340)** — same as the Ark `vtxo_key` and `bark`'s signing scheme (`lib/src/vtxo/policy/signing.rs:42` signs with `SECP.sign_schnorr_with_aux_rand`).

**Message Hash:** `sha256(chain-prefix ‖ order_bytes)` where `order_bytes` is the canonical serialization of `ArkSendOrder { order_id, ark_address, sats, not_before_height }`. This mirrors bark's sighash construction for VTXO spending.

**Why not ECDSA:** Bark already uses Schnorr for round signatures, arkoor cosigns, and attestations. Using the same curve keeps the crypto surface minimal; BIP340 also has superior batch-verification properties should the `ark` role ever verify multiple orders.

---

## 2. Generation

```bash
# One-shot generation matching bark's convention (server/src/lib.rs:135-147)
bitcoin-cli getnewaddress order-signing bech32m | \
  bitcoin-cli deriveaddresses descriptor "tr([fingerprint/86h/1h]xpub...)" | \
  jq -r '.key' > /run/secrets/spc/ark-order-key
```

Or using `bark-cli` style:
```bash
# Generate fresh mnemonic → xpriv → order key at m/86'/0'/0' hardened
echo "$(openssl rand -hex 32)" | xargs -I {} bark keygen --seed {} --path "m/86'/0'/0'" \
  --output /run/secrets/spc/ark-order-key
```

**Generation Principle:** Fresh CSPRNG draw (64 bytes, same as bark's `Mnemonic::generate(12).to_seed("")`). Key MUST NOT share a seed with the `ark` role's wallet (separation principle, SPEC §11.3 line 842-849).

---

## 3. File Format & Permissions

**Path:** `/run/secrets/spc/ark-order-key` (per SPEC §11.10 line 964)

**Format (newline-terminated):**
```
<privkey_hex:64chars> # 32-byte secp256k1 secret key
<pubkey_hex:66chars>  # 33-byte compressed pubkey, y-parity byte prefix
```

**Permissions:** `0o600` (owner-read/write only), matching bark's `fs_perms::create_new_owner_only` (`bark-common/src/fs_perms.rs:47-52`). On creation, the containing directory MUST be `0o700` (hardened datadir pattern, `bark/server/src/lib.rs:246` hardens datadir to 0o700).

**Atomicity:** Write should use temp-file + rename (`fs_perms::write_atomic_owner_only`) to prevent torn writes during rotation.

---

## 4. Public Key Distribution

The `ark` role learns the corresponding pubkey via **config file**, not via any runtime channel:

```toml
# spc-pool.toml [ark] section
[ark]
order_pubkey = "02deadbeef..."  # 33-byte compressed pubkey, hex
```

**Rationale:** The `payout`→`ark` boundary is a **Host** boundary (SPEC §11.3 line 855). Pairing secrets should not cross it. A config-file pubkey is auditable at startup (`ark` validates via `bitcoin::PublicKey::from_str`).

**Validation on Startup:** `ark` MUST refuse to start if:
- `order_pubkey` is absent
- Pubkey fails parsing
- Pubkey is all-zeros or otherwise invalid

---

## 5. Rotation Procedure

**Frequency:** Operator-driven, typically on security events or quarterly. Automated rotation is NOT required for v0.1 (hot signing keys without hardware attestations have limited rotation value; compromise is detected by `ark` rejecting old-key signatures).

**Procedure:**
1. Generate new keypair on `payout` host (see §2)
2. Update `order_pubkey` in `spc-pool.toml` on `ark` host
3. Restart `ark` role (or send SIGHUP if dynamic reload is implemented)
4. `ark` checks `/api/v1/ark/health` or startup logs for pubkey validation OK
5. Old key file can be shredded: `shred -u /run/secrets/spc/ark-order-key.old`

**Key History:** `payout` keeps no key history. `ark` stores only the current valid pubkey.

---

## 6. In-Flight Orders During Rotation

**Problem:** An `ArkSendOrder` signed with `key_n` is in the Redis queue (`spc:ark:orders`) when rotation to `key_n+1` occurs.

**Resolution:** Orders are **short-lived** (max lifetime = `retry_blocks` = 6 blocks, default, per SPEC §11.10 line 968). Rotation SHOULD occur during a low-activity window (no blocks found in last 12 blocks). Any order signed with `key_n` that arrives at `ark` after rotation is rejected (`spc_ark_orders_rejected_total{bad-signature}` metric, SPEC §9 line 769).

**Retry Behavior:** `payout` sees rejection, treats as transient failure, and re-issues order signed with `key_n+1` (same `order_id` for idempotency, new signature). This follows the existing retry loop for failed sends (SPEC §11.3 lines 869-870).

**Edge Case:** If rotation happens exactly as a block is found and orders are in-flight:
- Orders signed with old key: `ark` rejects → `payout` retries → succeeds on retry
- Maximum additional latency: 1 block (~10 min)

---

## 7. Security Comparison

| Property | bark Server (`captaind`) | SPc Order Key (this spec) |
|----------|--------------------------|---------------------------|
| Seed Storage | BIP39 mnemonic, `0o600` text file (`wallet.rs` pattern) | Single raw key, `0o600` file |
| Key Derivation | BIP32 hardened (`m/2'/0'` server key) | Fixed path, standalone key |
| Signing Surface | Ark rounds, boards, arkoors | Ark send orders only |
| Exposure Window | Online 24/7, but isolated per-wallet | Used only by `payout` host |
| Rotation | Backup+restore wallet (complex) | Simple file swap |

**Rationale for Simplicity:** The order key is a **single-purpose** sentinel key (only signs Ark send orders). It does not manage funds, participate in MuSig2, or derive addresses. This simplicity reduces the attack surface compared to bark's full wallet key hierarchy.

---

## 8. Proposed Config Snippet for SPEC.md

Insert at §11.10 after line 970:

```toml
# Order-signing key configuration (extends [ark] section)
[ark]
# ... existing fields ...

order_pubkey = "02..."                    # 33-byte compressed pubkey, hex. REQUIRED.
                                          # The corresponding secret key lives only on
                                          # the `payout` host at order_key_path below.

order_key_path = "/run/secrets/spc/ark-order-key"  # payout side only. File format:
                                          #   line 1: 64-char hex secret key
                                          #   line 2: 66-char hex pubkey (optional for audit)
                                          # Permissions: 0o600, directory: 0o700

order_sig_pubkey_prefix = "spc/order/v1"  # Optional context string for domain separation.
                                          # Hashed into Schnorr aux-rand if set.
```

---

## 9. References

- SPEC.md §11.3 (lines 850-862): Order signing requirement  
- SPEC.md §11.10 (lines 958-970): Existing `order_key_path` configuration  
- `bark/server/src/lib.rs:135-147`: Server key derivation from BIP32  
- `bark-common/src/fs_perms.rs:47-82`: File permission patterns (`harden`, `create_new_owner_only`)  
- `bark-common/src/secret.rs`: Secret wrapper pattern (prevents accidental Debug printing)  
- `lib/src/vtxo/policy/signing.rs:42`: Bark's Schnorr signing implementation  
- `2026-10-12-bark-key-management-mailbox-blinding-sp-derived-vtxo.md`: Detailed bark key management read  
- `2026-10-10-bark-barkd-openapi-source-boat001.md`: Bark wallet and address construction

---

**Prepared for:** SPEC.md §11 update  
**Output:** `docs/research/output/ark-order-key-spec-recommendation.md`
