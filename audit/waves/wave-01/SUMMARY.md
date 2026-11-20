# Wave 01 SUMMARY — workspace root, Cargo.tomls, tooling (xtask)

Agents: W01-A01 (orchestrator-as-agent, 3 files + 2 hostile experiments), W01-A02 (subagent, 6 files).
Domain files remaining unassigned-in-depth (recorded): workspace root configs (Cargo.toml, deny.toml,
product.toml, rust-toolchain.toml) and the rest of tools/ — coverage-limited run, see final report.

## Files audited (9)
| File | Auditor | Verdict | Findings |
|---|---|---|---|
| tools/xtask/src/cmd/feature_audit.rs | W01-A01 | REAL (as path-existence checker) | S0:1 S3:2 S4:1 |
| tools/xtask/src/cmd/parity_report.rs | W01-A01 | REAL | S4:2 |
| tools/xtask/src/cmd/config_validator.rs | W01-A01 | REAL (v0, limits disclosed) | S3:1 |
| tools/xtask/src/main.rs | W01-A02 | REAL | S3:1 S4:1 |
| tools/xtask/src/cmd/mod.rs | W01-A02 | REAL | S3:1 S4:2 |
| tools/xtask/src/cmd/toml_lite.rs | W01-A02 | REAL | S3:2 S4:1 |
| tools/xtask/src/cmd/sbom.rs | W01-A02 | REAL | S3:2 S4:1 |
| tools/xtask/src/cmd/gate.rs | W01-A02 | REAL | S3:3 S4:2 |
| tools/xtask/src/cmd/check_architecture.rs | W01-A02 | REAL | S2:1 S3:2 S4:2 |

Totals: S0:1 S1:0 S2:1 S3:13 S4:12 (27).

## Worst findings (verified)
1. **[S0] feature_audit.rs:489-501** — evidence resolution is `Path::exists()`. Executed proof: a fabricated matrix row "implemented on all four platforms" citing a 0-byte file as evidence passes the real tool with `result: PASS`, exit 0 (`audit/runs/baseline/experiment-feature-audit-empty-file.txt`). All 193 "implemented" platform cells on the committed matrix are certified to this standard, while the shipped binary registers decoders `mjpeg, pcm_s16le` only and decodes 0 frames of an H.264 MP4.
2. **[S2] check_architecture.rs:990-993** — duplicate package-name detection via `windows(2)` on path-sorted members; non-adjacent duplicates slip (W01-A02).
3. **[S3] gate.rs:107-110** — `WASM_BINDING_CRATE = "crates/engine-wasm-binding"` points at a nonexistent crate; doc-10 row 10 permanently SKIPped; the "10-command gate" is 9 (W01-A02).
4. **[S3] config_validator.rs:16-18** — pattern constraints not enforced by this tool; "validated" is weaker than the name implies (W01-A01, ran).
5. **[S3] feature_audit.rs:325-326** — violation prefixes cite row indexes as if line numbers (W01-A01).

## Executed tool baseline (Wave 0, cross-referenced)
- feature-audit: FAILURE 2 R4 violations; 252 rows; implemented=193 cells.
- parity-report: implemented∪verified 66/252 windows (26.2%), 61/252 linux, 66/252 macos, 0/252 web; verified-only 0/252 all platforms.
- config-validator: FAILURE 4 violations (brand assets) — docs' "RED with 4" claim TRUE.
- check-architecture: FAILURE edges=fail (stale allowlist: apps/cli `streaming` dep missing from boundaries.toml [crates.lumen-cli]).

## Anti-laziness gate (§7.3 step 4)
Sampled 3 citations from W01-A02 (gate.rs:107-110, toml_lite.rs:192-194, check_architecture.rs:990-993) — all three verified verbatim against source with sed -n. PASS. No fabrications recorded (audit/verify/fabrications.md).

## Agents' blocked items
- W01-A02: none. W01-A01: `scripts/full_gate.sh` script-level run deferred (components executed; see audit/runs/baseline/NOTES.md).
