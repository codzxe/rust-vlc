# AUDIT: crates/audio-output/src/cpal.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-21 |
| Lines / functions | 138 / 10 (module docs, 7 code items, 2 tests) |
| File verdict | STUB (an honest, disciplined, typed-refusal placeholder — and production-dead) |
| Reachable from shipped entry point? | NO (zero production callers; only `crates/audio-output/src/cpal.rs`'s own tests and `crates/audio-output/tests/pcm_pipeline.rs:244` call `cpal_backend()`; every shipped app hard-wires `NullBackend::open_stream` — `apps/cli/src/pipeline_host.rs:1598-1599`, `apps/web/src/host.rs:1731-1732`; `apps/desktop-tauri/src` has zero `audio_output` references) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |

## 1. One-paragraph verdict

This module is the cpal seam: a named const gate (`AUDIO_ADR005_ADMITTED = false`), a compile-time assertion pinning it shut, a typed error marker, and a constructor `cpal_backend()` whose only live behavior is `Err(BackendNotAdmitted::CPAL)`. It is a stub by design, it never fakes success, and its self-description is 90% accurate — but it is **dead code from every shipped entry point**: no app, no engine crate, no `platform-*` crate calls it. The module's central promise — "the single point platform wiring **will call** to obtain the real backend" — describes wiring that does not exist, and the advertised "one change" admission recipe omits the app-side rewiring it would actually need. Domain answer to the hostile question: the ring (`ring.rs`), RT callback (`render.rs`), sample counter (`counter.rs`), null backend (`null.rs`) and pump (`pump.rs`) are real code, executed in the baseline (rc-smoke #20, AVI first-vertical `a_frames=60 a_decoded=60`); the last mile to any speaker is this file's typed refusal plus three apps hard-wired to the null device. **A user never hears sound, on any platform, in this build.** The shipped audio path terminates at `apps/cli/src/host.rs:613-618`: the pump calls `device.render(&mut buffer)` into a scratch `vec![0.0_f32; …]` and drops the samples — the counter advances (so the clock and the "Playing" state are honest), but no byte reaches an OS audio API. `engine-info`'s `audio.null-device` feature string is honest; `cpal` is absent from every lockfile.

## 2. What actually works (verified)

— The gate is really closed and compiles closed: baseline #1 `cargo build --release --locked --workspace` exit 0 (audit/runs/baseline/INDEX.md #1), which requires `AUDIO_ADR005_ADMITTED = false` to satisfy the const assert at :48-52.
— "no OS audio library is imported anywhere in the workspace" (:32-34): verified — `rg '"cpal"|name = "cpal"' Cargo.lock apps/desktop-tauri/Cargo.lock` → no match in either lockfile; audio-output's external allowlist is empty (`boundaries.toml:408-413`, verified by W01-A02).
— The refusal is typed and panic-free on the only reachable path: `cpal_backend()` returns `Err(BackendNotAdmitted::CPAL)` unconditionally while the gate holds; the `unreachable!` arm (:106) is dead by construction (flipping the const without editing this file fails the build at :48-52 first). Both unit tests and `tests/pcm_pipeline.rs` executed in baseline #3/#4 (1681 passed / 1751 passed; the failures are unrelated files).

## 3. Findings

### F-001 [S2]
Where: crates/audio-output/src/cpal.rs:13-16, 86-88 (against apps/cli/src/pipeline_host.rs:1598-1599, apps/web/src/host.rs:1731-1732)
Code:
```
//! - the admission change is **one change**: flip the const, land the
//!   `platform-*` adapter behind [`cpal_backend`], amend ADR-005, add
//!   the allowlist row, delete the gate + refusal arm + marker.
```
and:
```
/// The cpal adapter's construction gate — the single point platform
/// wiring will call to obtain the real backend.
```
What is wrong: The "single point platform wiring will call" has no caller. Repo-wide grep (`rg 'cpal_backend|AUDIO_ADR005_ADMITTED|BackendNotAdmitted'`, excluding audit/) matches only: this file, `lib.rs`'s re-export (:136), `tests/pcm_pipeline.rs`, docs, and the feature matrix. `crates/platform-{linux,macos,windows}` import only `DeviceDesc`/`DeviceId` (enumeration vocabulary) — none touches this seam. `apps/cli` and `apps/web` bypass the entire backend-selection layer and hard-wire `NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })`; `apps/desktop-tauri/src` contains no `audio_output` reference at all. The advertised admission recipe ("one change") therefore omits a mandatory step: rewiring every app's hard-coded null path to actually consult the backend. Executing the documented recipe exactly as written produces a build marked "admitted" that still emits silence.
Why it matters: This file is the recorded plan for how the project's most user-visible missing capability (sound) gets turned on. A Lead following it lands ADR-005 "admitted" + a platform adapter + zero audible difference, because nothing in the change list touches `pipeline_host.rs:1598`.
What correct looks like: Either wire a caller today (e.g. `builtin_backends()`-based selection in the apps, which itself has zero callers — same disease) or amend the module doc to state that app-side device selection is a fifth, unlanded piece of the admission change.
Confidence: HIGH — Verification: read + ran (`rg` caller census above; `sed -n '1596,1600p' apps/cli/src/pipeline_host.rs`, `sed -n '1729,1733p' apps/web/src/host.rs`).

### F-002 [S3]
Where: crates/audio-output/src/cpal.rs:1-6
Code:
```
//! The cpal adapter seam: **ADR-005's admission flip point** (wave 14,
//! issue #1). This module is where the real cpal-backed backend will be
//! constructed when (and only when) the Lead admits it — mirroring the
//! streaming TLS seam (`HTTPS_ADR011_ADMITTED`,
//! `crates/streaming/src/schemes.rs`, ADR-011), whose discipline this
//! file copies exactly:
```
What is wrong: "Copies exactly" is false in the one respect that matters. The ADR-011 seam lived inside the live classification path — the https match arm in `crates/streaming/src/schemes.rs:306-315` consulted it at runtime for every URL, so its refusal was user-visible behavior, and its deletion (worklog ff2df61) actually turned HTTPS on. This seam gates no runtime path; its refusal is observable only to two test files. The shapes match (const + typed marker + compile-time assert); the properties do not.
Why it matters: The comparison is the module's evidence that the pattern is proven; a reviewer who accepts "exactly" believes flipping this seam has a demonstrated activation path. It does not (see F-001).
What correct looks like: "copies the TLS seam's shape, but unlike that seam it has no runtime consumer yet."
Confidence: HIGH — Verification: read both sites (`sed -n '300,330p' crates/streaming/src/schemes.rs`; feature-matrix.toml:1427 records the ADR-011 seam deletion).

### F-003 [S4]
Where: crates/audio-output/src/cpal.rs:114-125 (dup of :48-52; third copy at tests/pcm_pipeline.rs:235-240)
Code:
```
    fn the_gate_is_recorded_closed() {
        // The flip-point discipline: flipping the const without the
        // admission change breaks the compile-time gate (and this
        // test) — the same-change rule, enforced.
        const {
            assert!(
                !AUDIO_ADR005_ADMITTED,
                "the ADR-005 gate is recorded closed"
            )
        };
    }
```
What is wrong: Enforcement tautology. The identical const assertion already exists as a production compile-time gate (:48-52); any state in which this test would fail is a state in which the crate does not compile. The test enforces nothing the compiler does not, and the same block is pasted a third time in `tests/pcm_pipeline.rs:235-240`.
Why it matters: Three copies of one assertion across two files is drift bait (the admission change must now delete all three); the test's name promises enforcement it cannot deliver independently.
What correct looks like: Delete the test (or keep exactly one copy); rely on the production const assert.
Confidence: HIGH — Verification: read all three sites.

### F-004 [S4]
Where: crates/audio-output/src/cpal.rs:43, 98 (module name `cpal`; fn `cpal_backend`)
Code:
```
pub const AUDIO_ADR005_ADMITTED: bool = false;
```
What is wrong: Naming that implies an implementation: a module named `cpal` and a "construction gate" named `cpal_backend`, while no cpal exists anywhere in the dependency graph (absent from both lockfiles) and nothing is ever constructed — every call in every possible build returns the single hard-coded marker. The honesty lives entirely in the doc comments; the names alone oversell.
Why it matters: `rg cpal` from the repo root suggests an audio backend exists; only reading 138 lines of docs reveals it is a refusal letter.
What correct looks like: `backend_admission.rs` / `admitted_backend()` or similar refusal-flavored naming (the crate's own `stub.rs` got this right).
Confidence: HIGH — Verification: read; lockfile greps in §2.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-23 | REAL (prose) | — | 90% accurate; the two false claims are F-001/F-002. |
| `AUDIO_ADR005_ADMITTED` (const) | 43 | REAL | — | `false`, consistent with lockfiles (§2). Read only by the gate, `cpal_backend`, tests, docs. |
| `const _: () = assert!(!AUDIO_ADR005_ADMITTED, …)` (compile-time gate) | 48-52 | REAL | — | Enforces same-change flip discipline at compile time; verified closed in baseline build #1. |
| `BackendNotAdmitted` (struct + 2 fields) | 57-63 | REAL | — | Plain data (`&'static str` pair); no behavior to get wrong. |
| `BackendNotAdmitted::CPAL` (const) | 68-71 | REAL | — | The only value ever constructed in any build. |
| `impl fmt::Display for BackendNotAdmitted::fmt` | 74-82 | REAL | — | Formats the refusal message; asserts on its content in the unit test. |
| `impl std::error::Error for BackendNotAdmitted` | 84 | REAL | — | Trivial, correct. |
| `cpal_backend()` | 98-107 | STUB | — | Always `Err(BackendNotAdmitted::CPAL)` while the gate holds; `unreachable!` arm dead by construction; **zero production callers** (F-001). No ignored params, no panics reachable, no unsafe. |
| test `the_gate_is_recorded_closed` | 114-125 | SHALLOW | S4 | Tautology with the production const gate (F-003). |
| test `the_constructor_refuses_with_the_typed_marker` | 127-137 | REAL | — | Genuinely fails under the fake-success mutation (§5); proves the refusal, which is all the module claims. |

## 5. Test honesty (if tests exist in/for this file)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_gate_is_recorded_closed` (:114-125) | NO (independently) | The only mutation it catches (const → true) fails the build at :48-52 first; the test body's own const assert is also compile-time. Compiler-tautology (F-003). |
| `the_constructor_refuses_with_the_typed_marker` (:127-137) | YES | Mutation: make `cpal_backend` return `Ok(boxed_null)` — `.err().must()` panics on `None`. Verifies marker identity, fields, and the Display string including "cpal", "ADR-005", "Lead". |
| `the_cpal_seam_refuses_with_the_typed_marker` (tests/pcm_pipeline.rs:233-245) | YES (refusal half) / NO (gate half) | Same split: the `err().must()` half catches fake success; its const-assert half is the third copy of the compile-time gate. |

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`false` means: no OS audio library is imported anywhere in the workspace" | cpal.rs:32-34 | REAL | `cpal` absent from `Cargo.lock` and `apps/desktop-tauri/Cargo.lock`; allowlist `[]` (boundaries.toml:408-413). |
| "the admission change is **one change**" | cpal.rs:13-16 | FALSE | Omits app wiring: apps hard-wire `NullBackend` (pipeline_host.rs:1598; apps/web/src/host.rs:1731); no caller exists to flip (F-001). |
| "the single point platform wiring will call to obtain the real backend" | cpal.rs:86-87 | FALSE | Zero production callers (caller census in F-001); platform-* crates import only `DeviceDesc`/`DeviceId`. |
| "mirroring the streaming TLS seam … whose discipline this file copies exactly" | cpal.rs:1-6 | SHALLOW | TLS seam sat in the live path (schemes.rs:306-315) and its deletion activated HTTPS; this seam gates no runtime path (F-002). |
| "The flip point is already coded … the Lead's admission change flips it in the same commit that lands the adapter" | docs/dependencies/records/audio-output-cpal.md:215-217 | REAL (existence) / FALSE (sufficiency) | Code exists as described; but "lands the adapter" without app wiring changes nothing audible (F-001). |
| "no real-backend claim exists anywhere today, and the typed stubs say so on every platform" | docs/adr/ADR-005-audio-output.md (Consequences) | REAL | Consistent with code and lockfiles; baseline INDEX corrected matrix: sample-aac.mp4 "audible output: none — null device". |
| platform-aout-cpal: "Default audio output path (cpal-first…) on all desktops" status `partial` (win/linux/macos) | docs/feature-matrix.toml:2348-2361 | SHALLOW | The seam/pump/null half of the evidence is real; the output path itself is null-only on every platform. Notes disclose ("partial-seam, not done"; "no real-device evidence exists") — the status cell still reads as partially-working audio to a casual reader. Row owner should reconcile. |

## 7. Dead / duplicated / unreachable code

- The entire module is production-dead: `cpal_backend` called only by its own tests and `tests/pcm_pipeline.rs:244`; `lib.rs:136` re-exports it to zero consumers.
- Same disease next door (for lib.rs's owner): `builtin_backends()` (lib.rs:152-157) and `UnsupportedBackend` have zero production callers — apps never consult a backend list; `apps/cli/tests/container_conformance.rs:232`'s `[audio-output.unsupported]` expectation is the only place the stub vocabulary surfaces, in tests.
- The gate assertion exists three times: :48-52 (production), :119-124 (unit test), pcm_pipeline.rs:235-240 (integration test).

## 8. Dependencies & flags

- No external dependencies; `[dependencies]` = 3 internal path crates (audio-engine, media-clock, media-types) — matches boundaries.toml exactly (W01-A02 verified).
- No feature flag gates this file; it compiles unconditionally in the workspace build (baseline #1 exit 0). No cfg-gating, no `#[allow(`, no unsafe.
- `cpal` (the crate this file is named after) is not a dependency of anything — verified absent from both lockfiles.

## 9. Open questions for the next agent

- `crates/platform-{linux,macos,windows}` import only `DeviceDesc`/`DeviceId`: are `linux/pulse.rs`, `macos/coreaudio.rs`, `windows/wasapi.rs` real enumeration or their own seams? (platform-crate owner; the W01-A02 log already flags a substrate mismatch: PulseAudio enumeration vs a cpal/ALSA output story.)
- Does any UI settings screen expose audio-device selection (doc 06 §5.9 promises `devices = ["default", "null"]` capability) wired to these dead constructors?
- Matrix row `platform-aout-cpal` (feature-matrix.toml:2348-2361): the `partial` status vs the null-only reality deserves a row-owner ruling; this audit supplies the code-side evidence.
