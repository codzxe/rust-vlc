# AUDIT: crates/product-config/tests/fixtures/range-limits-multi.toml (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 240 (a full product.toml-copy with a mutated `[limits]` table) |
| File verdict | REAL (test fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/product-config/tests/fixtures_matrix.rs:197 (`range_limits_multi_reports_all_four_violations`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 tomllib parse (OK); python3 range cross-check vs product.schema.json; `diff` vs product.toml; `cargo test -p product-config` BLOCKED (disk full — see log) |

## 1. Purpose / consumer / cross-check
Purpose: exercise the "report EVERY violation, not just the first" contract (doc 07 §3.4 stage 2) with four simultaneous range violations in `[limits]`. Consumer: fixtures_matrix.rs:196-213 asserts one diagnostic per key with the exact bounds. Cross-check vs validator semantics (product.schema.json `limits` minimum/maximum, extracted by python3):
- `cache_demux_mib = 1` vs min 4 → violation (asserted ">= 4")
- `queue_packets_audio = 99999` vs max 8192 → violation (asserted "<= 8192")
- `max_playlist_items = 10` vs min 100 → violation (asserted ">= 100")
- `max_wasm_extension_memory_mib = 999` vs max 256 → violation (asserted "<= 256")
The four remaining limit keys are in-range (64, 512, 16777216, 4) so the fixture isolates exactly the four asserted violations. The fixture ↔ rule map is tests/fixtures/README.md per the test header.

## 2. What actually works (verified)
— python3 tomllib parses the fixture OK (top keys: brand, defaults, features, identity, limits, migrations, playback, schema_version, telemetry, urls, window).
— The four mutated values match the four asserted bounds exactly (schema.json extraction above). The test is honest: `one()` asserts EXACTLY one diagnostic per key and `rejected()` fails if the fixture ever loads clean.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/fixtures/range-limits-multi.toml:193-205 (verbatim):
```
[limits]
cache_demux_mib = 1

cache_network_mib = 64          # network buffer. schema 4..=1024
queue_packets_audio = 99999

queue_packets_video = 512       # bounded video packet queue. schema 64..=4096
probe_max_bytes = 16777216      # probing bound (MP §12). schema 1 MiB..=64 MiB
max_playlist_items = 10

max_open_sessions = 4           # schema 1..=16
max_wasm_extension_memory_mib = 999
```
What is wrong: nothing functional — noted for the record: the in-file comments still document the OLD values' semantics; the four violating rows had their trailing comments stripped, making the mutation points visually obvious (fine for a fixture, noisy vs the product.toml it copies). Confidence: HIGH — Verification: read.

F-002 [S4]
Where: whole file vs product.toml (diff run)
What is wrong: the fixture is a STALE product.toml snapshot: `display_version = "0.1.0-dev"` (product.toml: "0.1.3"), `version_code = 1` (now 4), `min_width = 480`/`min_height = 320` (now 940/600 per the w28 UI-floor fix), `streaming-http = false` (now true, w31.3), no `[web]` section. Harmless today because the consuming test asserts only the four limits keys (extra diagnostics for other keys would not fail `one()`), but the stale copy silently narrows what the fixture proves: if product.toml's current required-field set regressed, this fixture would not notice. Confidence: HIGH — Verification: diff (output in log).

## 4. Fixture verdict
REAL: it fails validation exactly as designed (four diagnostics), is consumed by an honest exact-match test, and its violation set matches the schema's declared bounds 1:1. Executed test run: UNVERIFIED — `cargo test -p product-config` blocked by "No space left on device" (sandbox disk 100%; nothing outside audit/ may be cleaned). Baseline (audit/runs/baseline, per AGENT-SPEC §13) records cargo test per-package 4744 passed / 2 failed — the product-config package tests pass there, which includes this fixture's test.

## 5. Open questions
- None. (Wave 1's product.toml/product.schema.json audits own the ranges themselves.)
