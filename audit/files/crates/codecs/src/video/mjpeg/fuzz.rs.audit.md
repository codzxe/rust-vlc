# AUDIT: crates/codecs/src/video/mjpeg/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 146 / 8 items in table (CountingSink + impl, fuzz_codec_mjpeg, 4 smoke tests, 1 test-local closure) |
| File verdict | UNREACHABLE from shipped entry points (fuzz/test infrastructure — but heavily EXECUTED by the test suites; the code itself is REAL quality) |
| Reachable from shipped entry point? | NO by design — a fuzz harness is not a shipped surface. Execution chain that DOES exist: (1) the 4 in-module smoke tests (baseline :696-699 all ok); (2) `crates/codecs/tests/fuzz_survival.rs:9 use codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg` — 6/6 ok in baseline (:993-1000: every fixture file, every extracted frame, systematic truncations, single-byte flips, adversarial geometries, AVI1 field-marker mutations); (3) `crates/codecs/tests/fuzz_loop_stable.rs` mjpeg seeded loop ok (:1013); (4) the libFuzzer shim `fuzz/fuzz_targets/fuzz_codec_mjpeg.rs:36` calls this fn — compile-ready but not built by any gate (its own comment: "compile-ready but not built by any gate", nightly wiring owed). The MJPEG DECODER it fuzzes IS the shipped lane (huffman.rs.audit.md chain). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | full-file read; `rg -n "fuzz_codec_mjpeg"` over tests/fuzz_survival.rs, fuzz_loop_stable.rs, fuzz/ (execution chain verified); `ls fuzz/fuzz_targets` (35 targets) + `cat fuzz/fuzz_targets/fuzz_codec_mjpeg.rs` (shim exists, W1-12 wiring note) + `rg -n "libfuzzer" fuzz/Cargo.toml` (:43 `libfuzzer-sys = "0.4.13"` — contradicts this file's module doc); sed of fuzz.rs:14-24 (the stale wiring claim); baseline log sed :696-699, :993-1013 (all fuzz lanes ok); head of scripts/fuzz_registry_check.sh (4-way registry consistency incl. harness-fn existence — baseline row 18 PASS, 35 targets). |

## 1. One-paragraph verdict
This is the MJPEG fuzz harness — two passes per call (the free `decode_image` entry and the full codec-api contract path configure→decode→flush), outcomes deliberately swallowed because the contract is survivability only — and the surrounding infrastructure is genuinely strong: the committed-corpus survival suite runs every fixture, every extracted frame, systematic truncations and byte flips through this exact function in the baseline (6/6 ok), a seeded loop adds deterministic coverage, and the registry gate keeps target/shim/harness/README in 4-way sync (baseline PASS). The harness body is honest, bounded (memory by the decoder's own MAX_PIXELS ceilings, runtime by the MCU-grid argument at :30-35), and panic-free by delegation. Two real defects: the module's "Wiring status (honest)" section is STALE — it claims the `libfuzzer-sys` dependency and `fuzz/Cargo.toml` scaffolding are still-unmade Lead decisions ("until they land"), but both have existed since card W1-12 (2026-10-22, per the target shim's updated note and fuzz/Cargo.toml:43) — the file's central honesty section is the one thing in it that is no longer true; and the `CountingSink` counts frames that nothing ever reads, so even the one cheap outcome pin available in the truncated-valid-stream test (a concealed frame was emitted) goes unasserted.

## 2. What actually works (verified)
- The harness executes both decode surfaces without panic on every hostile input the suites feed it — verified by baseline: `video::mjpeg::fuzz::tests` 4/4 ok (:696-699); `tests/fuzz_survival.rs` 6/6 ok incl. `systematic_truncations_of_a_real_frame_survive` and `systematic_single_byte_flips_of_a_real_frame_survive` (:993-1000); `tests/fuzz_loop_stable.rs::mjpeg_seed_loop_survives` ok (:1013).
- Registry consistency — `scripts/fuzz_registry_check.sh` PASS in baseline (INDEX.md row 18: "35 targets, diff empty"): the harness fn named in fuzz/README.md:77 exists and is called by the shim (rg-verified :36).
- Bounds delegation is real, not asserted: the doc points at `super::decoder::MAX_PIXELS` (fuzz.rs:29) — the decoder's ceilings, not a harness-side budget; the runtime argument (:31-35, MCU grid derived from the dimension cap, no attacker-controlled event count) is the correct reasoning for a single-image codec (contrast with container walks).
- `saturating_add` in the sink (:51) and `Ok(())` acceptance — the sink cannot panic or fail the harness.
- The four smoke inputs are well-chosen shapes: empty, SOI-only, marker-soup with absurd lengths + 4096×0xff tail, and a hand-built minimal grayscale stream truncated mid-entropy (the concealment path, distinct from the error path).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mjpeg/fuzz.rs:14-24
Code:
```
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_codec_mjpeg.rs` exactly as doc 04 §8.5
//! registers it. Building it needs `libfuzzer-sys` (an external
//! dev-dependency) plus the `fuzz/Cargo.toml` scaffolding — both are
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5), so until
//! they land the harness logic is proven by the in-crate deterministic
```
What is wrong: the module's headline honesty section describes a pre-2026-10-22 world: it claims `libfuzzer-sys` and the `fuzz/Cargo.toml` scaffolding are pending Lead decisions — but `fuzz/Cargo.toml` EXISTS and declares `libfuzzer-sys = "0.4.13"` (fuzz/Cargo.toml:43), and the target shim's own comment records "WIRING STATUS (updated 2026-10-22, card W1-12): the cargo-fuzz scaffolding now EXISTS". What is actually still owed is only the nightly/CI wiring ("compile-ready but not built by any gate", fuzz_codec_mjpeg.rs:17-19). The two wiring-status notes in the same tree contradict each other; this one was never updated when W1-12 landed.
Why it matters: a reader auditing fuzz readiness from this file concludes the substrate doesn't exist at all and under-credits (or, planning a fuzz campaign, starts re-doing W1-12's work). The section titled "(honest)" is the stale one — the exact inversion this audit exists to catch, in the conservative direction.
What correct looks like: mirror the shim's updated status: scaffolding exists (libfuzzer-sys 0.4.13, target declared, seed corpus listed), nightly wiring owed; or delete the duplicated status block and point at fuzz/README.md as the single source.
Confidence: HIGH — Verification: read (all three artifacts sed/cat-verified side by side; baseline registry check row 18 confirms the 35-target registry the stale doc says cannot be built).

### F-002 [S4]
Where: crates/codecs/src/video/mjpeg/fuzz.rs:44-54
Code:
```
#[derive(Default)]
struct CountingSink {
    frames: usize,
}

impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}
```
What is wrong: the sink counts frames into a field nothing ever reads — no test, no assertion, no debug print. The doc calls it "the panic-free emission target", but a counter without a reader is a dead field; even the one outcome pin that costs nothing (`truncated_valid_stream…` should emit ≥1 concealed frame) is left unasserted.
Why it matters: one line would upgrade the smoke suite from pure panic-freedom to also pinning "concealment emits a frame" on the one input designed to hit that path; as written, a regression that silently emits nothing on every input passes this file's tests (fuzz_survival.rs's outcome-pinning is the only backstop).
What correct looks like: return the sink from `fuzz_codec_mjpeg` (or a test-visible counter) and assert `frames >= 1` in `truncated_valid_stream_survives_both_passes`; or delete the field.
Confidence: HIGH — Verification: read (all uses of `frames`/`CountingSink` grep-verified within this file; fuzz_survival.rs's use of the harness discards outcomes by design).

### F-003 [S4]
Where: crates/codecs/src/video/mjpeg/fuzz.rs:82
Code:
```
        data: media_types::packet::SharedBytes::from_vec(data.to_vec()),
```
What is wrong: pass 2 copies the whole input per call — under a real libFuzzer campaign that is one full-input allocation per execution on top of the decoder's own work, and under the seeded loop tests it is per-iteration churn the module's "memory is bounded by the decoder's own ceilings" framing doesn't mention.
Why it matters: cosmetic in every current caller (small inputs), but the harness's stated posture is bounded-memory-only-by-the-decoder; a copy the harness itself adds deserves a sentence in the bounds doc — or the SharedBytes::from_slice-style zero-copy constructor if media-types offers one.
What correct looks like: zero-copy construction where the API allows, or a one-line note in the # Bounds section that the harness adds one input-sized transient copy.
Confidence: HIGH — Verification: read (media-types constructor surface not exhaustively checked — UNVERIFIED whether a borrowed variant exists).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct CountingSink` + `impl FrameSink` | 44-54 | REAL | S4 | F-002 dead counter; saturating; cannot fail |
| `fuzz_codec_mjpeg` | 59-90 | REAL | S4 | two-pass design sound; silent-bail on impossible descriptor/generation errors (:66-71) is fine (those are static); configure-error bail (:74-76) skips pass 2 — acceptable, AC suite pins typed outcomes; F-003 copy |
| `tests::empty_input_survives_both_passes` | 96-99 | REAL(test) | — | panic-only assertion (harness); the empty shape |
| `tests::soi_only_survives_both_passes` | 101-104 | REAL(test) | — | panic-only; SOI shape |
| `tests::hostile_marker_soup_survives_both_passes` | 107-115 | REAL(test) | — | panic-only; absurd lengths + 4096×0xff stress |
| `tests::truncated_valid_stream_survives_both_passes` | 118-145 | REAL(test) | S4 | hand-built minimal grayscale stream (DQT/DHT×2/SOF0/SOS + 3 entropy bytes) — the concealment-path input; asserts nothing beyond survival (F-002) |
| (test-local `seg` closure) | 122-127 | REAL | — | correct big-endian length incl. +2 payload |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 4 smoke tests | PANIC CLASS ONLY | the harness contract is explicitly absence-of-panic (:8-12) — a wrong-but-quiet decode passes; this is documented, and the outcome-pinning is delegated to fuzz_survival.rs/AC suites (which do run in baseline: 6/6) |
| `truncated_valid_stream…` | PANIC CLASS ONLY | would also survive a never-emits-anything regression (F-002) |
No `#[ignore]` here (the long loops live in fuzz_loop_stable.rs with documented ignore reasons — baseline shows `mjpeg_seed_loop_long … ignored, long fuzz run`, and `mjpeg_seed_loop_survives` runs).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:8-12 | REAL | the code does exactly this; survival suites run in baseline |
| "Building it needs `libfuzzer-sys` … plus the `fuzz/Cargo.toml` scaffolding — both are Lead-owned … until they land" | fuzz.rs:18-21 | FALSE (stale) | fuzz/Cargo.toml exists with libfuzzer-sys 0.4.13 (:43); shim comment records W1-12 landing 2026-10-22 — F-001 |
| "the libFuzzer campaign itself is owed CI wiring" | fuzz.rs:22-24 | REAL | no gate builds the fuzz package (shim comment "compile-ready but not built by any gate"; baseline gate list has no fuzz lane — only the registry CHECK) |
| "Memory is bounded by the decoder's own ceilings … this harness adds none" | fuzz.rs:28-30 | SHALLOW | the decoder ceilings are real, but the harness adds the input copy (F-003) — "adds none" overstates |
| registry: harness named and 4-way consistent | fuzz/README.md:77 + baseline row 18 | REAL | fuzz_registry_check PASS, 35 targets, diff empty |

## 7. Dead / duplicated / unreachable code
- `CountingSink::frames`: written, never read (F-002).
- No duplication: the harness is the single entry the shim, the survival suite and the loop suite share — good design.
- Unreachable from the shipped binary (by purpose, not defect).

## 8. Dependencies & flags
- codec-api prelude, media-types, super::MjpegDecoder — no external deps in-crate; the libfuzzer-sys dependency lives in the separate fuzz/ package (workspace-excluded, per its own Cargo.toml). No unsafe. No feature gate. Lint-clean per baseline clippy.

## 9. Open questions for the next agent
- fuzz_survival.rs / fuzz_loop_stable.rs owner: the outcome-pinning backstop for the concealment path — confirm it asserts emission (my read of the harness's design says outcomes are pinned there, not here).
- Whoever lands the nightly wiring: F-001's stale block should be fixed in the same card so the two wiring-status notes cannot contradict again.
