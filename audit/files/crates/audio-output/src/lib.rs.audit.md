# AUDIT: crates/audio-output/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-21 |
| Lines / functions | 157 / 1 fn (+ 1 doctest, 10 mod decls, 9 re-export lines) |
| File verdict | REAL (crate core real+executed; `builtin_backends` and the cpal/pump/pattern surface UNREACHABLE from shipped entry points; docs carry 3 defects) |
| Reachable from shipped entry point? | PARTIAL — YES for the NullBackend lane: `lumen open/play` → apps/cli/src/pipeline_host.rs:1599 `open_audio_session` → `NullBackend::open_stream` → `NullStream{sink,device}`; producer apps/cli/src/pipeline_host.rs:663 `AudioLimb::write_bytes` → `NullSink::write_bytes` → `SpscRing::push`; consumer apps/cli/src/host.rs:614,726,824,1274 `device.render` → `render_into` (apps/cli/src/host.rs:53,1599; rg "audio_output::" → apps/cli, apps/web, platform-* vocabulary-only). NO for `builtin_backends`, `cpal_backend`, `UnsupportedBackend`, `PcmPump`, `SinePattern`: zero non-test callers repo-wide. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg -n "builtin_backends|NullBackend|UnsupportedBackend|PcmPump|cpal_backend|SinePattern" --glob '!crates/audio-output/**'` (callers: only apps/cli+apps/web use NullBackend; builtin_backends/cpal_backend/PcmPump/SinePattern: none) · `rg -n "unsafe|thread::spawn|std::thread" crates/audio-output/src/` (none — doc comments only) · `sed -n` on lib.rs:44-51,72-81,124-143,145-157; ring.rs:45-52; cpal.rs:43-52; host.rs:610-618 (quote verification) · `rg -n "audio-output" audit/runs/baseline/test-workspace.log.txt` + sed 150-240 (33 unit + 13 AC tests PASS) · `rg "AudioBackend|render_into|AudioSink" crates/platform-*/src/` (no platform crate implements a real backend) · `rg "trusted progress" docs/03-engine-plan.md` (doc quote exists, line 553) · read of all 10 sibling modules |

## 1. One-paragraph verdict
The plumbing is real: the SPSC ring, `render_into`, `SampleCounter`, and the null backend are fully implemented, honestly error-typed, covered by 46 passing tests (baseline test-workspace.log.txt: 33 unit + 4 callback_budget + 1 clock_truth + 4 pcm_pipeline + 3 sine_sink_hash + 1 zero_alloc), and the null lane executes in the shipped CLI (baseline rc-smoke #20; `sample-aac.mp4` reaches Playing). The seams are also real *as seams*: `AUDIO_ADR005_ADMITTED = false` (cpal.rs:43) with a compile-time gate and a typed `BackendNotAdmitted` refusal means **no OS audio library is imported anywhere in the workspace** — every platform-* crate uses audio-output only for `DeviceDesc`/`DeviceId` vocabulary (rg confirms no `AudioBackend`/`AudioSink` impl outside this crate). Therefore the hostile question has one answer: **a user never hears sound from any shipped build** — decoded PCM is written to the ring, "rendered" into a local `Vec<f32>` that is dropped (apps/cli/src/host.rs:613-617), and the CLI reports Playing/exit 0. That is honest-by-design (ADR-005 PENDING, matrix row `platform-aout-cpal` says "partial-seam… no real-device evidence exists"), so no fraud — but it is a user-visible capability hole (S2) and this file's "backend set v0 ships" is dead code nothing calls.

## 2. What actually works (verified)
- Null lane executed end-to-end: baseline INDEX #20 `RC_SMOKE_SKIP_WEB=1 bash scripts/rc-smoke.sh` → PASS, AVI first-vertical with 60/60 audio frames pumped through this crate's NullBackend; `mp4-smoke` `sample-aac.mp4` → "Buffering → Playing → Stopped (audible output: none — null device)".
- Crate tests: baseline test-workspace.log.txt:155-226 → `test result: ok. 33 passed` (unit) + callback_budget 4, clock_truth 1, pcm_pipeline 4, sine_sink_hash 3, zero_alloc_null_sink 1 — all ok, 0 failed.
- `AUDIO_ADR005_ADMITTED=false` + compile-time assert + typed refusal verified by reading cpal.rs:43-52, 98-107; gate tests in baseline log (`cpal::tests::the_gate_is_recorded_closed ... ok`).
- No `unsafe`, no `std::thread` in `crates/audio-output/src/` (rg → only doc-comment mentions), Cargo.toml deps are 3 path crates — "Std-only; no unsafe" (lib.rs:86) holds.

## 3. Findings
### F-001 [S2]
Where: crates/audio-output/src/lib.rs:152-157 (cross-cites apps/cli/src/host.rs:613-617, crates/audio-output/src/cpal.rs:43)
Code:
```
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
```
What is wrong: The complete shipped "backend set" contains no device that can make sound (null discards; stub refuses), and with `AUDIO_ADR005_ADMITTED = false` there is no other: rg shows no `AudioBackend`/`AudioSink` implementor outside this crate (platform-{linux,macos,windows} import only `DeviceDesc`/`DeviceId`). The CLI renders into a dropped scratch buffer:
```
let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
if let Err(error) = device.render(&mut buffer) {
```
(apps/cli/src/host.rs:613-614 — `buffer` is never read afterwards; same discard at 726, 824, 1274).
Why it matters: Playing any audio-bearing media produces silence with Playing/Stopped/exit 0 and no surfaced notice — the most realistic user case ("hear my MP3") is broken-by-design and invisible to the user (baseline INDEX standing defect: "Audio 'plays' only into the null device; no sound output path exists"). Honest docs do not make the behavior correct.
What correct looks like: Either a real backend admitted behind the ADR-005 gate (the documented plan), or — until then — the CLI surfaces "audio output: null device (silent)" on open/engine-info instead of listing `audio.null-device` as an unqualified feature.
Confidence: HIGH — Verification: read + baseline cite (rc-smoke PASS; sample-aac.mp4 transcript).

### F-002 [S3]
Where: crates/audio-output/src/lib.rs:145-157
Code:
```
/// The backend set v0 ships: the null backend first (headless/CI),
/// then the typed not-supported stub for the current OS (the real
/// backends' placeholder until ADR-005 lands them in `platform-*`).
```
What is wrong: `builtin_backends()` has zero callers repo-wide (rg "builtin_backends" → lib.rs:152 definition + docs/worklog archive only). Nothing "ships" it; the CLI constructs `NullBackend` directly (pipeline_host.rs:1599). It is also the only production construction site of `UnsupportedBackend::for_current_os()`, so the entire stub backend and the trait-object dispatch surface are unreachable in shipped builds.
Why it matters: Dead code presented as the shipped backend set misleads readers about how device selection happens; the "Selection policy … is deliberately the caller's in v0" comment documents a caller that does not exist.
What correct looks like: Delete the function, or wire it into actual device selection in the session wiring; retitle the doc to say nothing constructs it yet.
Confidence: HIGH — Verification: rg callers + read.

### F-003 [S3]
Where: crates/audio-output/src/lib.rs:76-81
Code:
```
//! consumer (the device, calling [`render_into`]) — the null backend
//! splits the two halves explicitly ([`NullStream::sink`] vs
//! [`NullStream::device`]) so a wrong-thread use is a type-level
//! mistake, not a data race.
```
What is wrong: False claim. `NullSink` and `NullDevice` are both plain structs over `Arc<Shared>` (null.rs:115-118, 276-280); nothing at the type level prevents calling `sink` methods on the consumer thread or vice versa. The crate's own ring.rs:50-52 states the truth: "violating the discipline is a logic bug the type cannot catch — it is the documented contract."
Why it matters: A concurrency-safety assertion that the type system does not back is exactly the kind of claim a future agent will rely on; the only actual protection is the atomics (which make misuse non-UB but still logically corrupting: concurrent producers can drop/overwrite frames).
What correct looks like: Fix the doc to "the split makes the two halves *visible* as separate values; the SPSC discipline remains a runtime contract the types cannot enforce" — matching ring.rs.
Confidence: HIGH — Verification: read (null.rs struct definitions; ring.rs:45-52).

### F-004 [S3]
Where: crates/audio-output/src/lib.rs:75-78 (cross-cites apps/cli/src/pipeline_host.rs:663)
Code:
```
//! **SPSC discipline** is the contract: exactly one producer (the
//! pump, calling [`AudioSink::write`]) and exactly one consumer (the
//! device, calling [`render_into`])
```
What is wrong: In every shipped build the producer is apps/cli's `AudioLimb` (`self.sink.write_bytes(...)` pipeline_host.rs:663), not `PcmPump`: rg shows `PcmPump`/`SinePattern`/`cpal_backend` have zero non-test callers. The module docs narrate the pump as the pipeline's producer when it is test-only infrastructure (proven only in tests/pcm_pipeline.rs).
Why it matters: Doc drift between the crate's story and the shipped player path; a reader auditing the real producer must look in apps/cli, not here.
What correct looks like: Name both: "the producer is whoever drives the sink (v0 shipped: the CLI's audio limb; the crate's PcmPump proves the pattern in tests)".
Confidence: HIGH — Verification: rg callers + read pipeline_host.rs:663.

### F-005 [S4]
Where: crates/audio-output/src/lib.rs:48-51
Code:
```
//! [`PcmPump`] (decoded frames → bounded queue → sink, paced by the
//! playback clock — the `MediaClock::audio_master` integration),
//! proven in `tests/pcm_pipeline.rs`.
```
What is wrong: Misattribution: tests/pcm_pipeline.rs uses the raw `SampleCounterSource` ("the same SampleCounterSource a real MediaClock::audio_master reads", its own header); the actual `MediaClock::audio_master` integration is proven in tests/clock_truth.rs (`the_device_sample_counter_drives_the_audio_master_clock`, baseline PASS).
Why it matters: Minor citation inaccuracy in an otherwise precise evidence chain.
What correct looks like: Cite tests/clock_truth.rs for the MediaClock integration.
Confidence: HIGH — Verification: read tests/pcm_pipeline.rs:1-30, tests/clock_truth.rs:1-30; baseline log lines 199-212.

### F-006 [S4]
Where: crates/audio-output/src/lib.rs:28-34, 40-42
Code:
```
//! sandbox can neither build it (no ALSA dev package: the alsa-sys
//! pkg-config probe fails) nor test it (no audio device). The full
//! R1–R10 evidence lives in
//! `docs/dependencies/records/cpal-0.18.2.md`.
```
What is wrong: Unverifiable provenance in this audit session: the "wave-14 admission spike (issue #1)" GitHub issue, and the empirical alsa-sys/pkg-config build failure cannot be re-run (no network, no cargo per session rules). The referenced files exist (`docs/dependencies/records/cpal-0.18.2.md`, `audio-output-cpal.md`, `docs/adr/ADR-005-audio-output.md` all present).
Why it matters: The not-admitted verdict rests on records whose experiments this audit cannot reproduce; the gate constant and its tests are verifiable, the substrate rationale is UNVERIFIED.
What correct looks like: Mark these as record-cited, not re-verified (this file does; future waves should treat the records as claims).
Confidence: MEDIUM — Verification: UNVERIFIED (files exist; experiments not re-runnable).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `builtin_backends()` | 152-157 | UNREACHABLE | S3 (F-002) | Dead: 0 callers. Returns null + typed-refusal stub; no audible backend exists. Allocation per call (`vec!`, `to_string` in enumerate) irrelevant since never called. |
| doctest `Example` (null round-trip) | 92-122 | REAL-UNPROVEN | — | Not seen in baseline log (no `Doc-tests` sections captured in test-workspace.log.txt); equivalent executed coverage: `null::tests::open_stream_roundtrip_and_capture` (baseline PASS). Would fail if `render_into` mis-copied (asserts exact `vec![0.5, -0.5]` + capture equality). |
| module decls `counter..stub` | 124-133 | REAL | — | All 10 sibling files exist; contents audited by reading (real, tested implementations). |
| re-exports (135-143) | 135-143 | REAL | — | Names match definitions. `cpal::{AUDIO_ADR005_ADMITTED, BackendNotAdmitted, cpal_backend}`, `SinePattern`, `PcmPump`, `PushOutcome`, `UnsupportedBackend` re-exports have zero external (non-test) consumers — the unreachable surface (F-004). |
| module docs (crate-level `//!`) | 1-122 | REAL w/ defects | S2/S3/S4 (F-001..F-006) | Backend-decision, no-blocking-contract, threading, never-blocks claims verified against code; 3 defects filed. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (92-122) | YES | Asserts exact rendered floats `out == vec![0.5, -0.5]`, `frames_read == 1`, `silence_filled == 0`, and capture byte-equality — mutation of ring copy, silence fill, or counter breaks at least one assert. Not independently seen executed (covered by the unit twin). |
| (cross-ref) null.rs 8 tests, render.rs 4, ring.rs 6, counter.rs 3, pump.rs 4, pattern.rs 4, stub.rs 1, cpal.rs 2 — all in baseline | YES | Value-level assertions (FIFO order, wrap segments ≤ 2, counter deltas bracketed, typed error codes); baseline test-workspace.log.txt:157-190 all `ok`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NullBackend … fully, CI-safe" | lib.rs:14-16 | REAL | 33 unit + 13 AC tests PASS (baseline log 155-226); rc-smoke #20 PASS |
| "UnsupportedBackend (typed not-supported stubs)" | lib.rs:15-16 | REAL (code) / UNREACHABLE (shipped) | stub.rs:77-117 read; only construction site is dead `builtin_backends` (F-002) |
| "cpal 0.18.2 … NOT admitted" | lib.rs:28-38; matrix row platform-aout-cpal | REAL | cpal.rs:43 `false` + compile-time assert :48-52 + refusal tests in baseline |
| "RenderStats … the callback-budget AC asserts" | lib.rs:19-21 | REAL | tests/callback_budget.rs 4 tests PASS (baseline log 191-198) |
| "render_into … takes only borrows … at most two contiguous segments" | lib.rs:58-63 | REAL | render.rs:89-120 signature + segment-budget tests (render.rs:217-289, baseline PASS) |
| "SampleCounter … monotonic … frozen while paused … never rewinds" | lib.rs:21-24 | REAL | counter.rs:59-76, null.rs:591-623, tests/clock_truth.rs PASS |
| "No threads are spawned here" | lib.rs:73-75 | REAL | rg `std::thread|thread::spawn` in src/ → none |
| "Std-only; no `unsafe`" | lib.rs:86 | REAL | rg `unsafe` → doc comments only; Cargo.toml: 3 path deps, no externals |
| "a wrong-thread use is a type-level mistake, not a data race" | lib.rs:78-81 | FALSE | ring.rs:50-52 contradicts; no type machinery enforces SPSC (F-003) |
| "exactly one producer (the pump …)" | lib.rs:76-77 | SHALLOW (for shipped builds) | Producer is apps/cli AudioLimb (pipeline_host.rs:663); PcmPump test-only (F-004) |
| "The backend set v0 ships" | lib.rs:145 | FALSE (nothing ships/calls it) | rg callers → 0 (F-002) |
| "PcmPump … proven in tests/pcm_pipeline.rs" | lib.rs:47-51 | REAL (proof) / UNREACHABLE (product) | 4 tests PASS (baseline log 213-220) |
| "the `MediaClock::audio_master` integration, proven in tests/pcm_pipeline.rs" | lib.rs:49-50 | SHALLOW (misattributed) | clock_truth.rs proves it; pcm_pipeline.rs uses raw SampleCounterSource (F-005) |
| "alsa-sys pkg-config probe fails" (sandbox) | lib.rs:31-33 | REAL-UNPROVEN | Record exists (records/cpal-0.18.2.md); experiment not re-runnable (F-006) |

## 7. Dead / duplicated / unreachable code
- `builtin_backends()` (lib.rs:152-157) — dead, sole constructor of the shipped-stub narrative (F-002).
- `UnsupportedBackend` (stub.rs) — real, honest, but unreachable in shipped builds (only dead + test callers).
- `PcmPump`, `PushOutcome`, `DropPolicy`, `SinePattern` — real and tested, test-only consumers (UNREACHABLE from `lumen`/`lumen-tauri`).
- `cpal_backend()`, `BackendNotAdmitted`, `AUDIO_ADR005_ADMITTED` — intentional seam; the `unreachable!` arm (cpal.rs:106) is dead by the compile-time gate, as designed.
- `RingRenderer` (render.rs:129-154) — constructed only in its own test; the shipped consumer calls `render_into` via `NullDevice::render`. (Noted for the render.rs auditor; listed here because lib.rs:141 re-exports it.)

## 8. Dependencies & flags
- Cargo.toml deps: audio-engine, media-clock, media-types (path-only), dev-dep testkit — matches "Std-only" claim; no feature flags in this crate (the only "gate" is the `AUDIO_ADR005_ADMITTED` const, not a cargo feature). Workspace lints deny `unsafe_code`/`unwrap_used`/`todo` — consistent with the file's "Never" section.
- No cfg-gating hides real code: there is no real device code anywhere to gate (F-001).

## 9. Open questions for the next agent
- apps/web/src/host.rs:1732 opens its own NullBackend — if the web target ever ships, same silence-by-design question applies there.
- platform-linux hand-rolled a PulseAudio *client* for enumeration (per W-prior Cargo.toml.audit note) while output waits on ADR-005/cpal — the substrate reconciliation hazard (enumeration via PulseAudio vs output via cpal/ALSA) is recorded there and still open.
- Doctests do not appear anywhere in test-workspace.log.txt (no `Doc-tests` sections) — did the baseline run capture them? If not, the doctest pass/fail status of the whole workspace is UNVERIFIED-executed, worth an orchestrator check (`REQUEST-ORCH-RUN` not raised: equivalent unit coverage exists for this crate).
