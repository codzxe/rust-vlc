# AUDIT: fuzz/fuzz_targets/fuzz_codec_aac.rs
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 20-23 / 1 fuzz_target! shim |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz campaign not in release lane. Wired: (1) registry gate scripts/fuzz_registry_check.sh (runs in .github/workflows/ci.yml:244-246 "fuzz-registry"); (2) nightly campaign .github/workflows/audit.yml:325-355 (5 shards, cargo-fuzz, 180 s/target) — consumes fuzz/corpus/fuzz_codec_aac/ automatically. Campaign compile UNVERIFIED here (nightly toolchain absent; cargo not installed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg for harness fn; ls corpus dir; sed of README |

## 1. One-paragraph verdict
A thin, honestly-labeled libFuzzer shim: the entire body is one `fuzz_target!` call delegating to the in-crate harness (codecs::audio::aac::fuzz::fuzz_aac_stream at crates/codecs/src/audio/aac/fuzz.rs:76 — verified to exist by rg). The header comment states the exact wiring status (SCAFFOLDED, NOT WIRED — nightly Lead decision pending, per fuzz/README.md "Status" block). The harness logic is independently exercised on stable by in-crate tests (aac 2 / vorbis 3 / mpd 4 / ps 26 — rg counts in the harness modules' test suites; ps via crates/demux/src/ps/tests.rs:512-530). Seed corpus present: fuzz/corpus/fuzz_codec_aac (3 seeds).
## 2. What actually works (verified)
- Harness function exists: rg `pub fn codecs::audio::aac::fuzz::fuzz_aac_stream` -> crates/codecs/src/audio/aac/fuzz.rs:76. Note: displaying crates/codecs/src/audio/aac/fuzz.rs:75 through the Bash tool shows `#ust_use]` — that is a DISPLAY ARTIFACT of this tool (ANSI-sanitizer strips `[m`-style sequences without the ESC byte; proven by `printf 'X[mY'` -> `XY`); the file bytes are `#[must_use]` (verified via od -c hexdump).
- Registry gate wired: .github/workflows/ci.yml:244-246 runs scripts/fuzz_registry_check.sh in the gate lane; scripts/quality/25_fuzz_campaign.sh exists for the nightly audit.yml tier.
## 3. Findings
None beyond the campaign-not-wired status, which the file itself declares honestly (header comment + fuzz/README.md "Status: SCAFFOLDED, NOT WIRED"). The 20-line file does exactly what it claims and nothing more.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target! closure | 15-20 | REAL | — | single delegation call; no local logic to get wrong |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|---|
| (in-crate sweeps, not this file) | YES | stable-runnable sweeps call the same harness over committed fixtures + mutation sweeps (e.g. ps/tests.rs:512-530 truncation/cut variants; baseline cargo-test passes them) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one coverage-guided fuzz target per landed parser … 35 targets" | fuzz/README.md:1-14 | REAL | 35 .rs files in fuzz/fuzz_targets/ (ls) |
| "compiles on stable (cargo check in fuzz/)" | fuzz/README.md Status block | REAL-UNPROVEN | cannot re-run (cargo absent); fuzz/Cargo.lock committed |
| "the harness logic is proven by the in-crate deterministic hostile-input sweeps" | /bin/bash header | REAL | in-crate tests verified to exist + pass in baseline |
## 7. Dead / duplicated / unreachable code
None. Registry-vs-implementation diff pinned empty by scripts/fuzz_registry_check.sh.
## 8. Dependencies & flags
libfuzzer-sys 0.4.13 caret pin (fuzz/README.md dep note); fuzz/ is its own workspace root (fuzz/Cargo.toml + Cargo.lock committed) so the main workspace never links libFuzzer.
## 9. Open questions for the next agent
- The nightly audit.yml fuzz tier has no committed evidence of a green run in audit/runs/baseline/ (runs happen on GitHub infra, artifacts to docs/audit/reports/fuzz/ — not present in the repo). Campaign green status UNVERIFIED.
