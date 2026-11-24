# AUDIT: crates/product-config/tests/fixtures/assets/assets/brand/icon.icns (binary)
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Size / bytes | 12 bytes: `69 63 6e 73` ("icns") + 8 zero bytes |
| File verdict | STUB (magic-byte stub, not a real ICNS) |
| Reachable from shipped entry point? | NO — test fixture asset for tests/semantic.rs magic checks |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `file` -> "Mac OS X icon" (magic recognized); `od -A x -t x1z` (bytes above); rg for citations |

## 1. Verdict
A 12-byte synthetic file with only the `icns` magic and a zeroed length field (a real ICNS declares its total length in bytes 4-8 and carries icon elements). `file` classifies it as "Mac OS X icon" off the 4-byte magic alone. It passes semantic.rs AssetKind::Icns `magic_ok` and is one of the three "valid" assets in `brand_icon_with_wrong_magic_bytes_is_rejected` (baseline semantic 17/17 PASS). Legitimate as a validator test asset; not a usable icon — zero icon data. No doc/matrix row cites this file.

## 3. Findings
F-001 [S4] Magic-only stub (zeroed length field, no icon elements); correct for its purpose, must never be treated as a shippable asset — the real `assets/brand/icon.icns` referenced by product.toml does not exist (config-validator RED). Confidence: HIGH — Verification: ran file/od.

## 9. Open questions: none.
