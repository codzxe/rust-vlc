# AUDIT: fuzz/fuzz_targets/fuzz_codec_mp3.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 macro invocation |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — only via `cargo +nightly fuzz run fuzz_codec_mp3`, and the campaign is UNWIRED (fuzz/README.md:17 "Status: SCAFFOLDED, NOT WIRED"); compiles under `cargo check` in fuzz/ per README:38-41 (not re-runnable here: no cargo) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read; rg "fn fuzz_mp3_stream" → crates/codecs/src/audio/mp3/fuzz.rs:85; rg 'name = "fuzz_codec_mp3"' fuzz/Cargo.toml:181; ls fuzz/corpus/fuzz_codec_mp3/ (3 seeds) |

## 1. One-paragraph verdict
A 20-line libFuzzer shim whose entire body is one call to the real in-crate
harness `codecs::audio::mp3::fuzz::fuzz_mp3_stream` (verified to exist at
crates/codecs/src/audio/mp3/fuzz.rs:85). The shim is genuine, registered in
fuzz/Cargo.toml:181, covered by the registry gate
(scripts/fuzz_registry_check.sh — bins==files==README rows==harness fns),
and its seed corpus is present (3 files). It has never executed a
coverage-guided campaign — the README is explicit that the nightly wiring is
an outstanding Lead decision, and the header comment of this very file says
so. No dishonesty: the "wiring status (honest)" note is in the file itself.

## 2. What actually works (verified)
— rg: harness fn exists (crates/codecs/src/audio/mp3/fuzz.rs:85
`pub fn fuzz_mp3_stream(data: &[u8]) -> FuzzOutcome {`).
— The in-crate harness logic is exercised on stable by the workspace suite
(crates/codecs/tests/mp3_fuzz_survival.rs / fuzz_loop_stable.rs — run green
in audit/runs/baseline/cargo-test-workspace.txt).
— Registry gate consumes the target + non-empty corpus
(scripts/fuzz_registry_check.sh:12-15, :64).

## 3. Findings
None. The file does exactly what it claims (a shim) and claims exactly what
it does (not yet wired). Baseline cannot run it here (no cargo —
UNVERIFIED that it still compiles on this tree; README records compile
verification 2026-11-09).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target!(data → fuzz_mp3_stream) | 18-20 | REAL-UNPROVEN | — | one full untrusted pass per call |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (no tests in this file) | n/a | the shim is test infrastructure; its logic is covered by in-crate sweeps |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness logic is proven by the in-crate deterministic hostile-input sweeps" | file header :7-10 | REAL | mp3 fuzz survival tests ran green in baseline |
| "building the campaign needs the Lead's nightly wiring" | file header :7-8 | REAL | README:17-42 LEAD-DECISION block |

## 7. Dead / duplicated / unreachable code
Not dead: gate-enforced presence. Unreachable in the release lane by design.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13 (caret-pinned per fuzz/README.md:31-35); own workspace
root (README:170-182).

## 9. Open questions for the next agent
None.
