# AUDIT: crates/audio-output/src/cpal.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-23 |
| Lines / functions | 138 / 1 const + 1 compile-assert + 1 marker struct (2 impls) + 1 constructor + 2 tests |
| File verdict | STUB — a typed-refusal gate with ZERO implementation behind it. Complete-if-flipped: NO. Flipping the const alone breaks the build; flipping + deleting the assert panics in `unreachable!()` |
| Reachable from shipped entry point? | NO production caller. `cpal_backend` is called only by tests/pcm_pipeline.rs:244 and re-exported at lib.rs:136; `AUDIO_ADR005_ADMITTED` is read nowhere except its own module. No shipped lane constructs a backend through this seam (all lanes use NullBackend directly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg callers (cpal_backend / impl AudioBackend across repo); sed quote verification; `cargo test -p audio-output` (the two in-file tests pass — 33 lib suite, exit 0) |

## 1. One-paragraph verdict
Direct answer to the assignment's question: SCAFFOLDING, and honest
about it. There is no cpal code anywhere — cpal is not a dependency of
any crate (audio-output's Cargo.toml has zero external deps), no
platform adapter implements `AudioBackend` except NullBackend and the
typed UnsupportedBackend (rg over the workspace: 2 hits, both in
audio-output). The "path behind the flag" is literally
`unreachable!()`: flip `AUDIO_ADR005_ADMITTED` alone and the
compile-time assert rejects the build; delete the assert too and the
constructor panics. ADR-005 itself says real-backend admission is
PENDING and enumerates the post-admission conformance work as
not-yet-done, so the docs, the ADR, the matrix note ("no real-device
evidence exists (typed marker refuses on every platform)") and the
code all agree. The fraud question is therefore settled: no flip-lie
here — the lie lives in product.toml (`audio-output = true`), already
filed as S0 F-001 in audit/files/product.toml.audit.md.

## 2. What actually works (verified)
— verified by `cargo test -p audio-output` (exit 0):
`the_gate_is_recorded_closed` and `the_constructor_refuses_with_the_typed_marker`
pass; the marker renders with cpal/ADR-005/Lead strings; the
compile-time assert is real (const assertion in the crate root of the
module). The refusal is typed, named, and matches the documented
streaming-seam discipline in shape.

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/cpal.rs:98-107.
Code:
```
pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted> {
    if !AUDIO_ADR005_ADMITTED {
        // The refusal arm (the flip point): delete together with the
        // gate and the marker when the admission change lands.
        return Err(BackendNotAdmitted::CPAL);
    }
    // Unreachable while the compile-time gate holds: the admission
    // change replaces this arm with the real adapter's construction.
    unreachable!("the ADR-005 admission change replaces this arm")
}
```
What is wrong: nothing is behind the seam. The flip is not a flag
change — it is a whole implementation wave (write the platform-*
adapter: enumerate, open, the RT callback bridge into `render_into`,
pause/resume, the SampleCounterSource wiring, unsafe-audit per doc 11
§1.2 E2, allowlist row, ADR amendment). "One change" (module doc
line 14-16) means one PR containing all of that. Why it matters:
anyone reading "flip point" as "the cpal path exists and is disabled"
is wrong; the matrix evidence string for row `platform-aout-cpal`
lists this file as partial-status evidence, and the honest state is
"seam + refusal only". The crate docs say so; this finding pins the
hostile answer for the record. Confidence: HIGH — Verification: rg
(zero cpal refs in Cargo.toml tree, zero `impl AudioBackend` outside
audio-output) + read.
F-002 [S4]
Where: crates/audio-output/src/cpal.rs:1-6 vs crates/streaming/src/schemes.rs:650-655.
Code:
```
//! The cpal adapter seam: **ADR-005's admission flip point** (wave 14,
//! issue #1). This module is where the real cpal-backed backend will be
//! constructed when (and only when) the Lead admits it — mirroring the
//! streaming TLS seam (`HTTPS_ADR011_ADMITTED`,
//! `crates/streaming/src/schemes.rs`, ADR-011), whose discipline this
//! file copies exactly:
```
What is wrong: the analogy overstates the parallel. The TLS seam's
story ENDED with admission + a real implementation (ureq+rustls is a
live dependency in crates/streaming/Cargo.toml:52; the HTTPS gate is
deleted; schemes.rs:651-655 documents the deletion and the acceptance
pin). This seam's story ends at the refusal marker. "Copies exactly"
is true of the gate/marker *pattern*, false of the trajectory a reader
would infer (gate → real code behind it). Confidence: HIGH —
Verification: read of both files.
F-003 [S4]
Where: crates/audio-output/src/cpal.rs:36-42 (TODO contract).
Code:
```
/// TODO(admitted-impl): the Lead's admission change flips this to
/// `true` **in the same change** that (a) amends ADR-005 with the
```
What is wrong: the TODO names (a) ADR amendment, (b) allowlist row,
(c) the platform adapter — but not (d) wiring ANY caller: after the
flip, `builtin_backends()` (lib.rs:152, currently dead — see null.rs
audit F-003) or a platform seam must actually construct the backend
and the hosts must adopt it. The TODO's completeness checklist omits
the last mile, which is where this crate's other dead seams
(`builtin_backends`) already stalled. Confidence: HIGH —
Verification: read + rg (builtin_backends callers: none).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AUDIO_ADR005_ADMITTED (const) | 43 | REAL (the gate) | — | false; read nowhere else in repo |
| const _: () assert | 48-52 | REAL | — | genuine compile-time flip guard (verified pattern) |
| BackendNotAdmitted (struct) | 57-63 | REAL | — | typed marker, two fields |
| BackendNotAdmitted::CPAL (const) | 65-72 | REAL | — | |
| Display / Error impls | 74-84 | REAL | — | honest strings (test asserts cpal/ADR-005/Lead) |
| cpal_backend | 98-107 | STUB | F-001 | refusal arm + unreachable! — zero adapter code |
| test the_gate_is_recorded_closed | 114-125 | REAL | — | const-block assert, redundant with 48-52 but harmless |
| test the_constructor_refuses_with_the_typed_marker | 127-137 | REAL | — | pins marker identity + rendering |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_gate_is_recorded_closed | YES (compile) | const assert on the flag |
| the_constructor_refuses_with_the_typed_marker | YES | marker equality + Display content |
| the_cpal_seam_refuses_with_the_typed_marker (pcm_pipeline.rs:244) | YES | integration-level pin of the refusal |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no OS audio library is imported anywhere in the workspace; the only backends are the null backend and the typed not-supported stubs" | cpal.rs:32-34 | REAL | rg: zero cpal dep; `impl AudioBackend` = NullBackend + UnsupportedBackend only |
| "a compile-time assertion makes the gate impossible to open by accident" | cpal.rs:11-13 | REAL | const assert at 48-52 |
| "the admission change is one change: flip the const, land the platform-* adapter…" | cpal.rs:14-16 | REAL (honest scope) | nothing behind the seam (F-001) |
| "Flips on the Lead's ADR-005 admission; no real-device evidence exists" | docs/feature-matrix.toml:2365 | REAL | matches code state |
| "Real-backend admission … PENDING" | docs/adr/ADR-005-audio-output.md:3-7 | REAL | matches code state |
| "audio-output = true # default audio device output" | product.toml:150 | FALSE | this file is the proof of the contradiction (S0 filed in product.toml.audit.md F-001) |

## 7. Dead / duplicated / unreachable code
The whole module is intentionally dead-until-admission (correct usage
of the pattern). `AUDIO_ADR005_ADMITTED` has no external readers, so
the flag could not be queried for capability reporting anywhere —
product surfaces cannot even ask "is real audio admitted?".

## 8. Dependencies & flags
None. The module deliberately carries no cpal dependency (that is the
point of the gate). Std-only, no unsafe, no feature gate — compiled
into every audio-output build including the release lane.

## 9. Open questions for the next agent
- platform-windows/src/windows/wasapi.rs (787 LOC) and
  platform-macos/src/macos/coreaudio.rs (766 LOC) implement the
  `AudioDevices` seam (enumeration + notifications) with real FFI —
  are they wired to any shipped surface, or another dead lane like
  platform-linux? Whoever owns platform-windows/macos should answer;
  note they do NOT implement `AudioBackend` (no playback).
- The ADR's "Admission-spike outcome" recommends "cpal conditional-E2
  + pre-authorized direct adapters" — if the Lead ever picks the
  direct-API alternative, this whole flip point is the wrong seam
  (the name says cpal).
