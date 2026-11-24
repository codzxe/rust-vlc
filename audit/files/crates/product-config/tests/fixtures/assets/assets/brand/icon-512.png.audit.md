# AUDIT: crates/product-config/tests/fixtures/assets/assets/brand/icon-512.png (binary)
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Size / bytes | 16 bytes: `89 50 4e 47 0d 0a 1a 0a` + 8 zero bytes |
| File verdict | STUB (magic-byte stub, not a renderable PNG) |
| Reachable from shipped entry point? | NO — test fixture asset for tests/semantic.rs asset-existence/magic checks |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `file` -> "data" (not classified as PNG); `od -A x -t x1z` (bytes above); rg for citations |

## 1. Verdict
A 16-byte synthetic file carrying ONLY the 8-byte PNG signature. It passes the validator's magic-byte check (semantic.rs AssetKind::Png `magic_ok`) and serves as one of the "other three assets are valid" in `brand_icon_with_wrong_magic_bytes_is_rejected` (semantic.rs:248-261, baseline PASS 17/17) — but it is not an image: no IHDR chunk, no dimensions, no IEND; `file` classifies it as "data". Legitimate as a validator test asset (the validator checks magic only, by design: "extensions lie"); would be FRAUD if cited as a real brand asset. No doc/matrix row cites it (rg over feature-matrix.toml/docs -> only doc-07 schema examples reference the *path pattern*, not this file).

## 3. Findings
F-001 [S4] The stub cannot render (no IHDR/dimensions); fine for magic-check tests, misleading if ever shipped — the REAL product.toml's `assets/brand/icon-512.png` does not exist at all (config-validator RED, 4 violations). Confidence: HIGH — Verification: ran file/od.

## 9. Open questions: none.
