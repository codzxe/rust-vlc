# AUDIT: fuzz/fuzz_targets/fuzz_codec_adpcm.rs
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 20 / 1 fuzz_target! macro invocation |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz/ is its own workspace root (fuzz/Cargo.toml:87 [workspace]); the release lane (lumen-cli/desktop) never builds it; nothing in production calls the harness fn |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | read shim + harness; ls fuzz/{artifacts,coverage,target} (all absent); rg in scripts/fuzz_registry_check.sh; baseline greps (full_gate.txt:56-58, cargo-test-workspace.txt:390-393, :937-945) |

## 1. One-paragraph verdict
A 20-line libFuzzer shim whose entire value proposition — a coverage-guided
campaign over the ADPCM flavor zoo — has NEVER run, and the repo says so honestly
(fuzz/README.md:165-168 "No such run has happened yet"). What exists and is
verified: the shim is registered ([[bin]] fuzz/Cargo.toml:216-220), the registry
gate (scripts/fuzz_registry_check.sh) rg-proves the called harness fn exists and
passed in the orchestrator baseline (full_gate.txt:57-58), and the harness body
(crates/codecs/src/audio/adpcm/fuzz.rs:87-127, 8 flavors × configure→packet→
sink→flush) is genuinely executed by deterministic green tests (:390-393, :941-945).
The compile-on-stable claim for the shim itself rests on dated manual runs only.

## 2. What actually works (verified)
— wiring at registry level: baseline full_gate.txt:57-58 `fuzz-registry: CLEAN — 35
targets, diff empty, corpora present / fuzz-registry: PASS` (the gate walks
[[bin]]s ↔ target files ↔ README rows ↔ the called fn — scripts/fuzz_registry_check.sh:22-40).
— harness body proven on stable: baseline cargo-test-workspace.txt:390-393
(audio::adpcm::fuzz::tests::empty_and_short_inputs_survive /
raw_facts_skeleton_stays_legal / hostile_wide_inputs_survive ... ok) and :941-945
(adpcm_fuzz_survival.rs, 4 passed: mutated fmt headers / mutated blocks /
unmutated substrate / empty-tiny).
— seed corpus present: fuzz/corpus/fuzz_codec_adpcm/ — 5 seeds (ima_qt, ima_ws,
swf, yamaha .bin + raw_facts_skeleton.bin).

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_codec_adpcm.rs:7-12 and fuzz/README.md:17, :165-168
Code: `// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.`
What is wrong: the campaign has never executed — zero campaign artifacts exist
(verified: `ls fuzz/artifacts fuzz/coverage fuzz/target` → all "No such file or
directory"); no CI job builds campaign mode (ci.yml:244-246 runs the registry
check only; audit.yml:325-339 has a `fuzz` tier gated on a nightly schedule with
tool: cargo-fuzz — dispatch/nightly only, no committed green run); the
README:36-41 LEAD-DECISION (nightly toolchain) remains outstanding. The target is
registry-verified scaffolding, not running coverage.
Why it matters: doc 04 §8.5's fuzz row for ADPCM is satisfied by *adjacent*
deterministic tests, not by fuzzing; "35 targets" invites reading as 35 campaigns.
What correct looks like: the Lead decision lands, `cargo +nightly fuzz run
fuzz_codec_adpcm -s address -O -- -max_total_time=600`, artifacts committed.
Confidence: HIGH — Verification: read + executed ls (absence reproduced).

F-002 [S4]
Where: fuzz/README.md:206 (the stable randomized layer) vs this target
Code: `| `crates/codecs/tests/fuzz_loop_stable.rs`     | `fuzz_codec_pcm` + `fuzz_codec_mjpeg`, N each      |`
What is wrong: the seeded xorshift randomized loop layer does NOT cover adpcm —
the "stable-runnable equivalence" for this target is only 3 tiny unit sweeps
(empty/short ≤32 B, 0x00/0xff at 4096) plus the 4 survival tests; no randomized
N-pass loop feeds this harness on stable.
Why it matters: the strongest stable-side proxy for the missing campaign skips
this target.
What correct looks like: one more loop in fuzz_loop_stable.rs (or admit the gap
in the README's equivalence list).
Confidence: HIGH — Verification: read (fuzz_loop_stable.rs loop list in README
table; corpus seeds exist but have NO in-tree consumer — same S3 family flagged
by W16-RR2 for seed_adpcm_yamaha.bin).

## 4. Per-function table
File <100 LOC — single fuzz_target! invocation (:18-20) delegating to
codecs::audio::adpcm::fuzz::fuzz_codec_adpcm (fuzz.rs:87-127: FLAVORS[8] descriptor
cycle, WAV flavors take raw input as extra_data, raw flavors take the synthesized
16-byte skeleton (fuzz.rs:73-82), CountingSink saturating counter (:63-68), typed
errors swallowed by design — the AC suites pin outcomes). Harness logic REAL and
executed; the shim itself REAL-UNPROVEN (never compiled in any record I can cite;
cargo-build-budget + nightly absent in this sandbox).

## 5. Test honesty
N/A (no tests in this file). Its surrogate evidence is honest: the harness
contract is absence-of-panic (fuzz.rs:20-23) and the sweeps assert exactly that
(they would fail on panic/hang/OOM in the flavor cycle).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the harness logic is proven by the in-crate deterministic hostile-input sweeps" | shim :8-10 | REAL | baseline :390-393, :941-945 |
| "compiles on stable (cargo check in fuzz/)" | shim :9 + README:189-193 | REAL-UNPROVEN | dated manual runs (2026-10-24 / 2026-11-09) recorded in README only; no committed gate or baseline record compiles fuzz/ |
| "No such run has happened yet" (campaign) | README:168 | TRUE (honest) | my ls: no artifacts/coverage/target; no CI green run |
| registry row maps shim → codecs::audio::adpcm::fuzz::fuzz_codec_adpcm | README:75 | REAL | registry gate PASS baseline full_gate.txt:57 |

## 7. Dead / duplicated / unreachable code
Not dead — registered and gate-checked; unreachable from shipped build by design.

## 8. Dependencies & flags
libfuzzer-sys = "0.4.13" (caret pin, crates.io sparse-index claim README:31-35);
codecs path dep; [profile.release] overflow-checks = true (fuzz/Cargo.toml:59-60);
nightly needed only by the campaign driver, not the manifest syntax.

## 9. Open questions for the next agent
- audit.yml:325-339 fuzz tier (nightly schedule + workflow_dispatch, tool:
  cargo-fuzz): has ANY scheduled run happened since it landed (Actions history
  unavailable in sandbox — UNVERIFIED)? If yes, F-001 softens to S4.
