# AUDIT: crates/product-config/tests/fixtures/pattern-guid.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 226 (negative fixture: product.toml fork + one mutation) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by `cargo test -p product-config --test fixtures_matrix` (fixtures_matrix.rs:272-277). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>` (exit 0, mutation isolated); baseline fixtures_matrix 27 passed (cargo-test-workspace.txt:~5031); direct run blocked: disk full (`No space left on device`) — UNVERIFIED-by-execution for this one test, baseline cited |

## 1. One-paragraph verdict
Mutation matches the README map (fixtures/README.md:43: "`msi_upgrade_code = "not-a-guid"`") and is sed-verified at L43. The consuming test `pattern_guid_is_rejected` (fixtures_matrix.rs:272-277) asserts key path `identity.ids.windows.msi_upgrade_code`, rule "must match the pattern", found `"not-a-guid"` — a real assertion of the schema's GUID pattern (product.schema.json MSI pattern `^[0-9a-f]{8}-…`). Could not execute this specific test myself (disk full; the fixtures_matrix binary was cleaned from target/) — the orchestrator baseline shows the suite green (27 passed), so verdict REAL with that caveat. Same inherited-staleness rot as the other fixtures (S4).

## 2. What actually works (verified)
- Mutation sed-verified (fixture L43-44):
```
msi_upgrade_code = "not-a-guid"
```
- Test assertions read at fixtures_matrix.rs:272-277: exactly-one diagnostic for the key, rule contains "must match the pattern", found equals `"not-a-guid"`.
- Baseline: `Running tests/fixtures_matrix.rs … test result: ok. 27 passed; 0 failed` (audit/runs/baseline/cargo-test-workspace.txt).

## 3. Findings
F-001 [S4] — same fixture-rot class as feature-unknown-f1.toml F-001 (fork of pre-w28 product.toml: 0.1.0-dev/1/480x320/streaming-http=false). Where: L27,29,66-67. Confidence: HIGH — Verification: ran diff. Not re-executed individually (disk-full, see commands).

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: the `one()` helper would fail on any diagnostic-count drift with all diagnostics rendered. Doc claims: README.md:43 accurate. Dead code: none. Dependencies: none. Open questions: none.
