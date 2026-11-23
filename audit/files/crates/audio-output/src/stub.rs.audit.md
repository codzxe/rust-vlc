# AUDIT: crates/audio-output/src/stub.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 165 / 12 (4 inherent methods incl. private `pending`, 2 AudioSink impls, 6 AudioBackend impls, 1 test) |
| File verdict | STUB (by design and by declaration) + UNREACHABLE from shipped entry points (its only constructor path `builtin_backends()` has zero callers) |
| Reachable from shipped entry point? | NO — `UnsupportedBackend` is constructed only in `audio-output/src/lib.rs:155` (`builtin_backends()`), and `rg builtin_backends` over the whole repo returns only that definition and a worklog mention; shipped hosts (`apps/cli/src/pipeline_host.rs:52-53`, `apps/cli/src/host.rs:79`, `apps/web/src/host.rs:75-76`) construct `NullBackend`/`NullStream` directly and never touch the stub |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p audio-output --lib stub` → `stub::tests::every_operation_answers_typed_not_supported ... ok`, 1 passed, exit 0; `rg -n "builtin_backends\|UnsupportedBackend"` (repo-wide, reachability); `sed -n` quote verification |

## 1. One-paragraph verdict
This file is a deliberately honest typed-refusal stub and its code does exactly what a typed stub should: `write`/`open`/`pause`/`resume` all return a typed `Unsupported` error (`audio.os_backend_pending`, naming the platform), `enumerate` returns an empty list, no panic anywhere, no fake success — verified by reading every impl and by running its test. The doc is MOSTLY honest but overclaims twice: "every operation answers with a typed Unsupported error" is false for `latency()`/`clock_source()` (return `None`) and `underruns()` (returns `0`), and the error payload does not actually name "the pending decision" (ADR-005 appears only in a source comment, not in the error the user sees). The structural problem is reachability: the "selection slot" this stub claims to occupy — `builtin_backends()` — is dead code with zero callers, so the stub never runs in any shipped lane; the product's real answer to "play to my speakers" is not this stub's typed error but the silent use of the null (discard) backend everywhere.

## 2. What actually works (verified)
— verified by `cargo test -p audio-output --lib stub` → `every_operation_answers_typed_not_supported ... ok` (1 passed, exit 0). The test asserts: `enumerate().len()==0`, `clock_source().is_none()`, `underruns()==0`, `latency()==None`, `pause()`/`open()`/`resume()` return `ErrorKind::Unsupported` with code `audio.os_backend_pending`, and the rendered error contains `os-audio-backend/<platform>`.

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/lib.rs:152-157 (the slot stub.rs:9 claims to occupy)
Code:
```
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
```
What is wrong: zero callers anywhere in the repo (rg over apps/ + crates/ + tools/: only the definition and docs/worklog/archive-2026-10.md:1441). The stub doc's claim "this stub occupies the selection slot truthfully" (stub.rs:9) describes a slot nobody selects from. In every shipped host the audio path is hard-wired to `NullBackend` — the user never receives this stub's typed refusal; audio is silently discarded instead. The honest typed refusal is thus UNREACHABLE in practice: the product's actual behavior (null discard) is less honest than the stub designed to be its honesty mechanism. Confidence: HIGH — Verification: read (rg) + ran the stub test.
F-002 [S4]
Where: crates/audio-output/src/stub.rs:9-13
Code:
```
//! land, this stub occupies the selection slot truthfully: it
//! enumerates **no devices** and every operation answers with a
//! typed [`Unsupported`](media_types::ErrorKind::Unsupported) error
```
What is wrong: "every operation answers with a typed Unsupported error" is false — `latency()` returns `None` (stub.rs:82-84), `clock_source()` returns `None` (110-112), `underruns()` returns `0` (114-116); only write/open/pause/resume error. The trait's signatures force Option/u64, so defaults are the best available, but the doc sentence overclaims. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/audio-output/src/stub.rs:61-74
Code:
```
    fn pending(&self) -> EngineError {
        // The stable, honest refusal: this backend's implementation
        // is pending ADR-005 + the platform-* cards.
```
What is wrong: the doc promises the error "nam[es] the platform and the pending decision" (stub.rs:12); the constructed error carries only `os-audio-backend/<platform>` and code `audio.os_backend_pending` — no ADR-005, no pointer to the decision record. The decision reference lives only in this comment, invisible to the caller. Confidence: HIGH — Verification: read (error payload at 64-73).
F-004 [S4]
Where: crates/audio-output/src/stub.rs:77-80 and 124-164
Code:
```
impl AudioSink for UnsupportedBackend {
    fn write(&mut self, _buf: &AudioBuffer) -> Result<(), EngineError> {
        Err(self.pending())
    }
```
What is wrong: the module test covers pause/open/resume refusals but never calls `write()` — the AudioSink refusal arm (the one the pump would hit first in a real selection loop) is untested; if it returned `Ok(())` the test still passes. Confidence: HIGH — Verification: read + ran the test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| UnsupportedBackend::for_current_os | 37-48 | REAL (executed) | — | compile-time cfg! name, no OS type leaks |
| UnsupportedBackend::named | 52-54 | REAL (executed) | — | test round-trips "plan9" |
| UnsupportedBackend::platform | 57-59 | REAL (executed) | — | accessor |
| UnsupportedBackend::pending (private) | 61-74 | REAL | S4 | typed error; missing ADR-005 reference (F-003) |
| AudioSink::write | 78-80 | STUB (refusal) | S4 | untested refusal (F-004) |
| AudioSink::latency | 82-84 | STUB (None) | S4 | doc overclaim (F-002) |
| AudioBackend::enumerate | 88-91 | STUB (empty) | — | honest empty list, matches doc |
| AudioBackend::open | 93-100 | STUB (refusal) | — | typed error, tested |
| AudioBackend::pause | 102-104 | STUB (refusal) | — | typed error, tested |
| AudioBackend::resume | 106-108 | STUB (refusal) | — | typed error, tested |
| AudioBackend::clock_source | 110-112 | STUB (None) | S4 | doc overclaim (F-002) |
| AudioBackend::underruns | 114-116 | STUB (0) | S4 | doc overclaim (F-002) |
| tests::every_operation_answers_typed_not_supported | 125-164 | REAL (executed) | S4 | passes; misses write() (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_operation_answers_typed_not_supported | Mostly | Asserts typed errors for pause/open/resume, empty enumerate, None clock/latency, 0 underruns, platform round-trip. Gaps: `write()` never called (F-004); would not catch `write` returning `Ok(())`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "it enumerates no devices" | stub.rs:10 | TRUE | stub.rs:88-91 returns `Vec::new()`; test asserts len 0 |
| "every operation answers with a typed Unsupported error" | stub.rs:10-11 | FALSE (3 of 8 ops return defaults) | F-002: latency/clock_source/underruns |
| "naming the platform and the pending decision" | stub.rs:12 | HALF-TRUE | platform yes (`os-audio-backend/linux`); decision (ADR-005) absent from payload (F-003) |
| "never a fake success, never a panic" | stub.rs:12-13 | TRUE | all impls read; no panic path; test run passed |
| "this stub occupies the selection slot truthfully" | stub.rs:9 | FALSE in effect | the slot (`builtin_backends`) has zero callers (F-001) |
| "Until those cards land" (platform-* backend cards pending) | stub.rs:8-9 | TRUE | platform-* crates exist but implement their own `AudioDevices` seam, not `audio_output::AudioBackend`; no real backend against this trait exists anywhere (rg) |

## 7. Dead / duplicated / unreachable code
- The file itself is unreachable from shipped entry points (see header; F-001). It is exercised only by its own unit test and by `builtin_backends()` (itself dead).
- Architecture note (S4-level observation): the workspace now has TWO disconnected audio vocabularies — `audio_output::AudioBackend` (the engine's output seam, backed only by NullBackend + this stub) and `platform-*::audio::AudioDevices` (enumeration seams: pulse.rs, coreaudio.rs, windows). PulseAudio can enumerate sinks (pulse.rs, my W08-A03 audit) but nothing can play through them: no bridging adapter exists between the two trait systems.

## 8. Dependencies & flags
audio-engine, media-types, media-clock (path deps); testkit dev-only. No feature gates. Std-only, no unsafe. `cpal` is NOT a dependency — `src/cpal.rs` is the ADR-005 flip-point marker (`AUDIO_ADR005_ADMITTED = false`, compile-time assert), verified by reading cpal.rs:43-52.

## 9. Open questions for the next agent
- `apps/cli/src/pipeline_host.rs` and `apps/web/src/host.rs` (whoever audits them): confirm the shipped audio path is null-only and that no user-facing surface ever reports `audio.os_backend_pending` — if the feature matrix cites this stub as evidence of "honest audio fallback", the user never sees it (F-001).
- Matrix row evidence string at docs/feature-matrix.toml:2361 cites this stub + cpal.rs as the ADR-005 seam — check whether the row claims real-device audio output anywhere.
