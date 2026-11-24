# AUDIT: fuzz/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 321 / 35 [[bin]] targets + 1 package + [package.metadata] + 5 path deps + [profile.release] |
| File verdict | REAL (scaffold — explicitly not gate-wired; build requires nightly, per header and fuzz/README.md:20-27) |
| Reachable from shipped entry point? | NO — standalone workspace (`[workspace]` at :87, root member globs exclude fuzz/); dev-only lane, not in the release build. Its *harness bodies* are exercised through the crates' fuzz_survival integration tests (audit/runs/baseline/cargo-test-workspace.txt:904-1370: aac/adpcm/alac/flac/mp2/mp3/pcm/vorbis/ts fuzz-survival suites all `ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 cross-check declared bins vs fuzz_targets/*.rs and corpus dirs; find/du corpus stats; rg README + full_gate baseline |

## 1. One-paragraph verdict
A real cargo-fuzz scaffold: 35 declared target bins map 1:1 to the 35 on-disk fuzz_targets/*.rs files (python3 cross-check: declared 35, missing 0, undeclared 0), the 35 fuzz/corpus/<target>/ seed dirs all exist, `[package.metadata] cargo-fuzz = true` marker present, libfuzzer-sys pinned ^0.4.13, release profile keeps overflow-checks (a real anti-silent-overflow choice), and the full-gate registry check passes (audit/runs/baseline/full_gate.txt:57-58 `fuzz-registry: CLEAN — 35 targets, diff empty, corpora present`). The header comment still describes the original 3-target scaffold ("Lumen's three registered fuzz targets") while the manifest declares 35 — stale, misleading to the next reader.

## 2. What actually works (verified)
— python3: `declared bins: 35, missing files: []`, `on-disk .rs: 35, undeclared: []` — registry/implementation 1:1.
— Baseline: full_gate.txt:57-58 fuzz-registry PASS (35 targets, corpora present).
— Corpus sanity: 104 files, 1,039,757 bytes total, max file 49,152 bytes — the "< 50 KB every file" invariant (fuzz/README.md:102) holds.

## 3. Findings
F-001 [S4]
Where: fuzz/Cargo.toml:1-10
Code:
```
# fuzz/ — the cargo-fuzz campaign for Lumen's three registered fuzz
# targets (doc 04 §8.5; audit row M14, fix card ED-FIX-5, delivered by
# card W1-12).
```
What is wrong: the manifest declares 35 targets (verified 1:1 with fuzz_targets/*.rs); the header's "three registered fuzz targets" describes the original W1-12 scaffold state and is now false by an order of magnitude. Why it matters: header comments are the first thing a reader (or the next wave agent) trusts; this one misstates the campaign's size. What correct looks like: "35 registered fuzz targets (grown from the original three-target W1-12 scaffold)". Confidence: HIGH — Verification: read + python3 count.

Cross-file note (not my assignment to fix): fuzz/README.md:101 claims "97 files, 1.4 MB total" for the seed corpora; actual state is 104 files / 1.04 MB (find + du). Both numbers drifted; the per-file <50KB invariant still holds. Flag to the README's owning agent.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Notes |
|---|---|---|---|
| [package] | 26-30 | REAL | lumen-fuzz 0.0.0, publish=false, edition 2024 |
| [package.metadata] cargo-fuzz | 34-35 | REAL | required marker present |
| libfuzzer-sys dep | 43 | REAL | ^0.4.13 caret pin, nightly requirement documented |
| 5 path deps | 48-54 | REAL | codecs/demux/subtitles/playlists/streaming — the harness crates |
| [profile.release] overflow-checks | 59-60 | REAL | overflow in decoder = finding, not noise |
| 35 [[bin]] entries | 62-320 | REAL | 1:1 with sources; test/doc/bench=false correct for fuzz bins |
| [workspace] empty | 87 | REAL | self-contained root per cargo's prescribed fix (comment :83-86) |

## 5. Test honesty
N/A (manifest). The scaffold itself is covered by the fuzz-registry gate (targets vs implementation diff — full_gate PASS) and the in-crate fuzz_survival tests exercise the same harness bodies in the stable test lane.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "three registered fuzz targets" | fuzz/Cargo.toml:1-2 | FALSE (stale) | 35 bins declared and on disk (python3 count) — F-001 |
| "NOT wired into any gate" | fuzz/Cargo.toml:6-7 | REAL | no fuzz build step in .github/workflows or scripts/full_gate.sh (rg); the gate that exists is the registry *diff*, not a build |
| "requires a nightly toolchain" | fuzz/Cargo.toml:7-9 | REAL | rust-toolchain.toml pins stable 1.99.1 (prior audit exists: audit/files/rust-toolchain.toml.audit.md); cargo-fuzz `-Zsanitizer` flags are nightly-only |
| corpora present, seeds committed | fuzz/README.md:99-102 | REAL (drift in counts) | 104 files on disk; count/size claims stale (97/1.4MB) |

## 7. Dead / duplicated / unreachable code
None: every [[bin]] has a real source file and a corpus dir.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13 (only external dep); 5 in-repo path deps. No Cargo.lock committed for fuzz/ (nightly cargo-fuzz generates one at run time — consistent with the not-gate-wired posture).

## 9. Open questions for the next agent
- The fuzz campaign is NEVER executed in any committed artifact (registry diff + survival tests only) — the 35 corpus seeds' actual libFuzzer runs have no run log in-repo. If a "fuzz ran N execs" claim exists anywhere in docs, it is PAPER-ONLY until a run log lands. (Not found by me in docs/ — UNVERIFIED absence.)
