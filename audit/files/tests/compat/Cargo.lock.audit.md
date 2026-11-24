# AUDIT: tests/compat/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 1155 lines / n-a (lockfile data; assignment listed 2 #part chunks of one file) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (standalone test-infra workspace: tests/compat/Cargo.toml declares `[workspace]` empty); consumed by .github/workflows/compat-matrix.yml:154 `cargo run --manifest-path tests/compat/Cargo.toml --locked -- run` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 (27785 bytes, 5c59c4529ca9e287 — appended to audit/runs/W16-A05.datahashes.tsv); rg package/counts; python3 stats |

Data audit. Cargo lock v4, 1155 lines, 130 `[[package]]` entries: 109 registry-sourced with `checksum = ` lines (109 checksums — every registry dep is tamper-pinned), 21 in-repo path deps (lumen crates incl. bench-vs-vlc reuse of the RFC 1321 MD5). No duplicate-name anomalies beyond expected multi-version pairs (getrandom, syn — normal for lockfiles). The lock backs the standalone compat-matrix runner (tests/compat/Cargo.toml header: deliberately standalone per Lead-frozen root manifest policy), and CI runs it with `--locked` (.github/workflows/compat-matrix.yml:154), so this file is load-bearing for the committed evidence lane. Cargo not installed in sandbox — consistency with Cargo.toml dependency closure UNVERIFIED (reason: cargo-build-budget); internal consistency (packages/checksums/version) verified by parse. No findings.
