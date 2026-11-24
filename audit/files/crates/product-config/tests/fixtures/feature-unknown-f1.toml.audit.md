# AUDIT: crates/product-config/tests/fixtures/feature-unknown-f1.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 226 (negative fixture: product.toml fork + one mutation) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; reached by `cargo test -p product-config` (tests/semantic.rs:199,232). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/semantic-c370408f252f53a1 f1_unknown` → 1 passed; `... f1_f2_are_deferred` → 1 passed; `diff product.toml <fixture>`; baseline cargo-test-workspace.txt |

## 1. One-paragraph verdict
Does what the README map says (fixtures/README.md:63: "`telepathy = true` (not in the registry)"). The single mutation is present and I EXECUTED both tests that consume it: `f1_unknown_feature_id_is_rejected` (asserts key `features.telepathy`, rule F1, found `"telepathy"`, hint "config alone enables nothing") and `f1_f2_are_deferred_when_no_registry_is_supplied` (asserts NO telepathy diagnostic without a registry — proving F1 is a semantic-layer rule, not schema) — both pass from the prebuilt binary. The only defect is inherited rot: the fixture is a fork of an OLD product.toml (display_version "0.1.0-dev", version_code 1, `library` feature absent, `streaming-http = false # Phase 6`) — 4+ stale deltas vs the live product.toml that have nothing to do with the mutation.

## 2. What actually works (verified)
- Mutation verified by diff: fixture L152 `telepathy = true` (replaces the live `library = false`/`streaming-http = true` block). sed-verified:
```
playlists = false               # Phase 4
telepathy = true
```
- Executed (prebuilt binary): `f1_unknown_feature_id_is_rejected ... ok`, `f1_f2_are_deferred_when_no_registry_is_supplied ... ok` (semantic.rs:194-240).
- Baseline agreement: audit/runs/baseline/cargo-test-workspace.txt — product-config semantic 17 passed, exit_code=0.

## 3. Findings
F-001 [S4]
Where: fixture L27,29,66-67,152-154 vs live product.toml (same lines)
What is wrong: fixture fork predates w28/w31 product.toml changes (display_version 0.1.3, version_code 4, min_width 940/min_height 600, library/streaming-http features). Still passes because every stale value remains schema-valid, but each future product.toml tightening risks breaking ~40 fixtures for reasons unrelated to their mutations (or worse, masking the intended diagnostic behind an earlier stage error).
Why it matters / correct looks like: regenerate fixtures from the live product.toml when re-pinning, or build fixtures by patching the live file in the test harness instead of committing forks.
Confidence: HIGH — Verification: ran diff (exit 0)

## 4-9. (short audit)
Per-function table: N/A (data). Tests: honest — `one()` in fixtures_matrix/semantic asserts exactly-one diagnostic with rendered full output on mismatch. Doc claims: README.md:63 map is accurate. Dead code: none. Dependencies: none (TOML). Open questions: none.
