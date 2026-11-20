# crates/audio-output/src — combined index audit: THE AUDIO VERDICT (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| Date | 2026-11-20 |
| Scope | `crates/audio-output/src/` — full listing + verdicts; in depth: `lib.rs`, `null.rs`, `cpal.rs`, `stub.rs` (the backend-selection story). The remaining modules are pure-logic cores with no OS-audio seam (rg over the crate for `cpal\|alsa\|wasapi\|coreaudio\|pulse\|jack` → hits ONLY inside `cpal.rs`'s doc/seam text; zero OS-audio API code anywhere). |
| Reachability | The GUI path: lumen-tauri → ShellRuntime → CliHost → `open_audio_session` (apps/cli/src/pipeline_host.rs:1544-1575) → `NullBackend::open_stream(..., NullConfig { capture: false })` (line 1555-1556, sed-verified). The CLI path: identical (`lumen_cli` is the same host). The web path: same engine core compiled to wasm. **Every shipped surface routes here, and here there is no OS audio.** |
| Verdict | **NULL-ONLY. A user HEARS NOTHING from any shipped binary at HEAD — CLI, lumen-tauri GUI, or web. The cpal seam is a compile-time-closed admission gate; the "real device" backend is a typed-refusal stub. This is recorded honestly by the repo itself (worklog.md:2294-2296: "audio output remains the null device engine-wide (the cpal lane's typed-error foundation is stashed — `git stash list`: \"w31-audio-cpal-partial\")").** |

## Directory listing (complete, with LOC and verdict)

```
src/lib.rs      157  REAL — the adapter-trait layer + the backend-set truth (builtin_backends)
src/null.rs     748  REAL — the null backend: full AudioBackend, discard-by-default, capture-for-tests
src/cpal.rs     138  REAL SEAM, CLOSED GATE — AUDIO_ADR005_ADMITTED = false; cpal_backend() always refuses typed
src/stub.rs     165  REAL STUB — UnsupportedBackend: enumerate()=0 devices, every op = typed Unsupported "audio.os_backend_pending"
src/device.rs   132  trait layer (AudioBackend/DeviceDesc/SinkEvent) — no OS types
src/ring.rs     319  wait-free SPSC ring (atomics-only, no unsafe) — pure logic
src/render.rs   311  real-time render callback core + op-count ledger — pure logic
src/pump.rs     338  PcmPump (frames → bounded queue → sink, playback-clock paced) — pure logic
src/pattern.rs  190  SinePattern test-pattern generator — test-fixture core
src/counter.rs  132  SampleCounter (the media-clock truth source) — pure logic
src/error.rs     96  typed crate errors — pure logic
```

Cargo.toml (sed-verified): deps = `audio-engine`, `media-clock`, `media-types` ONLY (zero external deps; `testkit` is dev-only). "Std-only by C5: zero external dependencies, no unsafe".

## The three quotes that decide the audio question (all sed-verified)

1. The shipped backend set — `lib.rs:152-157`:
   ```
   pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
       vec![
           Box::new(NullBackend::new()),
           Box::new(UnsupportedBackend::for_current_os()),
       ]
   }
   ```
   and its doc (lib.rs:145-147): "The backend set v0 ships: the null backend first (headless/CI), then the typed not-supported stub for the current OS (the real backends' placeholder until ADR-005 lands them in `platform-*`)."

2. The closed admission gate — `cpal.rs:43`:
   ```
   pub const AUDIO_ADR005_ADMITTED: bool = false;
   ```
   with the compile-time assertion (lines 48-52) that makes flipping it impossible without editing this file, and the refusal arm (lines 98-103): `cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted>` returns `Err(BackendNotAdmitted::CPAL)` — `unreachable!("the ADR-005 admission change replaces this arm")` below it.

3. The null device itself — `null.rs:437`:
   ```
   name: "null (headless discard)".to_string(),
   ```
   and `stub.rs:88-91`:
   ```
   fn enumerate(&self) -> Vec<DeviceDesc> {
       // Honest: no devices until a real backend exists.
       Vec::new()
   }
   ```

## What the engine actually does with audio at HEAD (GUI included)

`apps/cli/src/pipeline_host.rs:1544-1575` `open_audio_session` (sed-verified at 1555-1556):
```
let NullStream { sink, device } =
    NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;
```
`capture: false` = headless discard: the decoded AAC/PCM samples are written into the null ring, the null device "renders" them into a scratch buffer, the sample counter advances (which is WHY GUI playback progresses — the pacer's tick is this render quantum), and **the samples never touch a speaker**. The audio limb's entire runtime value at HEAD is (a) the media clock and (b) the A/V scheduler ticks. Sound is a byproduct nobody receives.

## Findings

- **S1 — "hear audio" is FALSE in every shipped surface.** Not a regression: deliberate, recorded, Lead-gated (ADR-005 pending). But the mission question — "can a user HEAR anything from any shipped binary at HEAD?" — the evidence-only answer is NO: `builtin_backends()` is null+stub, `cpal_backend()` refuses, `UnsupportedBackend::open` answers `Err(self.pending())` (stub.rs:93-100) with `audio.os_backend_pending`. Even the baseline MP4 smoke the orchestrator executed (open→Playing→Ended) proves progress, not sound.
- **S2 — the w31.6 headline ("GUI video display end to end") ships beside a silent engine; no KNOWN-LIMITS row for the GUI states "video yes / audio no" as a pair.** The user-visible contract of the released 0.1.3 GUI is "moving pictures, zero sound" — a product state the docs never name in one place for the GUI surface (worklog w31's "Honest open items" names it for the engine; KNOWN-LIMITS.md's audio row exists elsewhere — see doc claims).
- **S3 — no committed test can fail on silence.** The crate's tests prove the null round-trip, the SPSC budget, the pump's queueing — all against the null device; the stub's refusal test (stub.rs:124-164) pins the typed refusals. There is no test anywhere that could catch "cpal admitted but silent", because no real backend exists to test. The evidence posture is honest; the coverage ceiling is total.
- **S4 — `builtin_backends()` itself is called by nobody in the shipped hosts** (`rg -n "builtin_backends"` over apps/ → no production hit; the GUI/CLI hosts construct `NullBackend` directly through `open_audio_session`). The "selection policy ... deliberately the caller's in v0" (lib.rs:149-151) — the selection function is currently library-API-dead weight. Recorded as dead-in-production.

## Test honesty

The crate's own tests are real and adversarial (sabotaged sinks, op-count budgets, poisoned-lock drains). The audio question does not hinge on test honesty — it hinges on backend existence, and the code says: none.

## Doc claims

- lib.rs:26-38 (the "backend decision (recorded honestly)" block) matches the shipped code exactly; cpal NOT admitted, ADR-005 the vehicle.
- Cargo.toml header comment: matches.
- worklog.md:2294-2296 (w31 honest open items): "audio output remains the null device engine-wide" — matches the code at HEAD. The stashed "w31-audio-cpal-partial" is NOT in the tree (stash is uncommitted by definition) — HEAD audio is null-only, full stop.

## Dead code / deps / flags

- Dead-in-production: `builtin_backends` (S4 above). Everything else is live on at least one host path or test.
- No unsafe anywhere in this crate (rg: zero `unsafe` blocks) — the posture claim holds.

## Open questions

1. When ADR-005 flips, the GUI's pacer cadence is DESIGNED to hand off to the device callback ("ADR-005's real backends will own this cadence from the device side", shell.rs:74-75). Until then the 30.625 ms pacer is the audio-clock stand-in — meaning the GUI's A/V sync and its frame cadence are both null-device-shaped today. Any runtime sync verdict is UNVERIFIED by construction.
