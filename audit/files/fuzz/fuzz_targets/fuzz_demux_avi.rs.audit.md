# AUDIT: fuzz/fuzz_targets/fuzz_demux_avi.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 33 / 1 (fuzz_target closure) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz campaign binary; reachable from CI: .github/workflows/audit.yml `fuzz` tier (nightly cron 0 2 * * *, 5 shards) via scripts/quality/25_fuzz_campaign.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -n "pub fn fuzz_demux_avi" crates/demux/src/avi/` → avi/fuzz.rs:52; `rg -n "fuzz_demux_avi" crates/demux/tests/*.rs` → fixtures_class_b.rs:485, fuzz_loop_stable.rs:221,327; baseline cargo-test-workspace.txt: fixtures_class_b 11 passed (L2242-2247), demux fuzz_loop_stable 4 passed/1 ignored (L2271); `ls docs/audit/reports/fuzz` → absent |

## 1. One-paragraph verdict
A real thin libFuzzer shim: one closure, one call, into a real in-crate harness (`demux::avi::fuzz::fuzz_demux_avi`, crates/demux/src/avi/fuzz.rs:52) that IS exercised on stable by committed tests (fixtures_class_b.rs:485 runs every class-B AVI fixture through it; fuzz_loop_stable.rs seeds mutated inputs). The header's wiring claim is STALE in the underclaiming direction: it says "not built by any gate", but .github/workflows/audit.yml defines a nightly fuzz tier (cron `0 2 * * *`, 180s/target, 5 shards) running this exact target. No committed evidence any campaign run ever happened.

## 2. What actually works (verified)
- `rg -n "pub fn fuzz_demux_avi" crates/demux/src/avi/` → `avi/fuzz.rs:52: pub fn fuzz_demux_avi(data: &[u8]) {` — harness exists, same signature the shim calls.
- Stable surrogate runs (committed baseline, cargo-test-workspace.txt): `fixtures_class_b` 11 passed (includes every_avi_fixture_survives_the_fuzz_harness calling this harness); `fuzz_loop_stable` 4 passed / 1 ignored (`mjpeg_seed_loop_long`, `pcm_seed_loop_long` — explicitly gated long runs).
- Not compiled here (cargo absent; libfuzzer needs nightly). UNVERIFIED at the binary level.

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_demux_avi.rs:10-14 (header) vs .github/workflows/audit.yml fuzz job
Code (verbatim, verified `sed -n '7,19p'`):
```
// What is still owed is the *nightly wiring* (a Lead dependency
// decision per AGENTS.md §3.2 / C5 — the repository toolchain is
// stable-pinned, cargo-fuzz needs a nightly for `-Zsanitizer`); the
// exact commands live in the README. Until that lands, this target
// is compile-ready but not built by any gate. The harness logic
```
What is wrong: the claim "not built by any gate" is stale — the nightly wiring HAS landed: .github/workflows/audit.yml `fuzz:` job (cron `0 2 * * *`, 5 shards, dtolnay/rust-toolchain@nightly + cargo-fuzz install) builds and runs this target via `bash scripts/quality/25_fuzz_campaign.sh 180 <shard targets>`. Why it matters: audit-grade docs must state the real wiring; a reader relying on this header would wrongly conclude zero fuzz coverage exists. What correct looks like: update the header to name the nightly audit.yml tier. Confidence: HIGH — Verification: read of both files.
F-002 [S3, campaign-level; recorded here because this target's only true gate is that campaign]
Where: scripts/quality/25_fuzz_campaign.sh:53-61 (evidence context; script not in my assignment)
Code:
```
  cargo fuzz run "$t" -- -max_total_time="$SECS" -rss_limit_mb=4096 > "$log" 2>&1
  rc=$?
```
What is wrong: a target that fails to BUILD (rc != 0, zero artifacts) is later reported `clean (exit=$rc)` and does not fail the campaign — only artifact counts gate the exit (`[ "$crashes" -eq 0 ]`, final line). Build-broken targets are vacuous green. Also NO committed campaign output exists (`ls docs/audit/reports/fuzz` → absent; `ls fuzz/artifacts` → absent), so there is zero evidence the nightly tier has ever run at all. Confidence: HIGH — Verification: read + ls.

## 4. Per-function table (file < 100 LOC; one entry)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target closure | 30-32 | REAL | — | calls demux::avi::fuzz::fuzz_demux_avi; no assertions of its own (by design — libFuzzer detects crashes) |

## 5. Test honesty
No tests inside the target (correct for a fuzz shim). Surrogate stable tests that call the same harness are honest and ran in baseline (see §2). Would they fail if the harness panicked on fixtures? YES (survival tests assert no panic through real fixture bytes + seeded mutations).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "not built by any gate" | header L13-14 | FALSE (stale, underclaims) | audit.yml nightly fuzz tier runs it (F-001) |
| "harness logic ... proven on stable by the in-crate deterministic smoke tests, the committed-corpus run in crates/demux/tests/fixtures_class_b.rs, and the seeded survival loop" | header L15-19 | REAL | baseline: fixtures_class_b 11 passed; fuzz_loop_stable 4 passed (citation in §2) |
| "Seed corpus when the campaign lights up: the committed class-B AVI derivatives ... plus truncations" | header L20-22 | SHALLOW | fuzz/corpus/fuzz_demux_avi/ — separate corpus audit; truncation sweep not verified here |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13 (fuzz/Cargo.toml), demux path dep. Standalone fuzz workspace (own [workspace] table).

## 9. Open questions for the next agent
- scripts/quality/25_fuzz_campaign.sh lacks `set -e` (has `set -uo pipefail`) — combined with the artifacts-only gate the whole campaign cannot fail on build errors (F-002); owner: whoever audits scripts/quality/.
