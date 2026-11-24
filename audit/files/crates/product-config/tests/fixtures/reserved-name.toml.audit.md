# AUDIT: crates/product-config/tests/fixtures/reserved-name.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 226 (negative fixture: product.toml fork + one mutation) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by fixtures_matrix.rs:332-341 (`reserved_name_is_rejected`). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>` (exit 0); baseline fixtures_matrix 27 passed; direct execution blocked (disk full) — baseline cited |

## 1. One-paragraph verdict
Mutation matches README.md:48 ("`name = "VLC"` (reserved list)") and is sed-verified at L19. The consuming test asserts key `identity.name`, rule containing BOTH "reserved" and "VLC", found `"VLC"`, and the exact line via `marker_line(&src, "name = \"VLC\"")` — line-precise negative testing, honest. Not individually executed (disk full); baseline green. Same S4 fixture rot.

## 2. What actually works (verified)
- Mutation sed-verified (fixture L18-19):
```
[identity]
# Full display name: window idle title, About, installer. 1–40 chars.
name = "VLC"
```
- Test read at fixtures_matrix.rs:332-341: `assert!(d.rule.contains("reserved"))`, `assert!(d.rule.contains("VLC"))`, `assert_eq!(d.found, "\"VLC\"")`, line pinned to the mutation.
- Baseline: fixtures_matrix `test result: ok. 27 passed; 0 failed` (audit/runs/baseline/cargo-test-workspace.txt).

## 3. Findings
F-001 [S4] — inherited fork rot vs live product.toml (L27,29,66-67: 0.1.0-dev/1/480x320/streaming-http=false). Confidence: HIGH — Verification: ran diff.

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: would fail if the reserved-name rule stopped naming the reserved token or mis-lined the diagnostic. Doc claims: README.md:48 accurate. Dead code: none. Dependencies: none. Open questions: none.
