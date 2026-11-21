# AUDIT: crates/audio-output/src/device.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-21 |
| Lines / functions | 132 / 11 items (2 structs, 1 enum, 1 trait, 2 fns, 5 trait methods) |
| File verdict | UNREACHABLE (real, compiled, in-crate-tested trait layer; zero shipped call chains reach any item at runtime) |
| Reachable from shipped entry point? | NO — proof in §7 (only `dyn` producers: `builtin_backends` 0 callers, `cpal_backend` refusal-only; hosts use inherent `NullBackend::open_stream`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 1 |

Commands run (what they established):
- `rg -n "dyn AudioBackend" --type rust` → only lib.rs:152 and cpal.rs:98 (the two Box-of-trait producers).
- `rg -n "builtin_backends"` → lib.rs:152 ONLY: zero callers anywhere, tests included.
- `rg -n "cpal_backend|AUDIO_ADR005_ADMITTED"` → cpal.rs + tests/pcm_pipeline.rs:244 (test-only caller).
- `rg -n "SinkEvent" --type rust` → device.rs (def), lib.rs (re-export), playback-engine recovery.rs:726 + hotplug.rs:169 — both DOC COMMENTS.
- `rg -n "AudioBackend|DeviceDesc|SinkEvent" apps/` → no hits; apps use `NullBackend/NullSink/NullDevice` only (pipeline_host.rs:52-53, host.rs:79; web host.rs:75-76).
- `rg -n "platform-" apps/*/Cargo.toml` → no matches: no app depends on platform-* (so platform-* use of DeviceId/DeviceDesc is unshipped).
- `rg -n "audio_output|audio-output" crates/playback-engine` → stage-name strings/comments only; engine never imports the crate.
- `rg -n "\.open\(|\.enumerate\(\)|\.pause\(\)|\.resume\(\)|backend.clock_source\(\)" crates/audio-output` → trait-method calls exist ONLY in null.rs/stub.rs unit tests + tests/{clock_truth,callback_budget}.rs.
- `sed -n` verification of every quote below.

## 1. One-paragraph verdict
This file is the trait-shaped facade of an audio stack that has no audio output. The `AudioBackend` trait, `DeviceId`, `DeviceDesc` and `SinkEvent` are real, well-documented, compile-clean code with two in-crate impls (NullBackend, UnsupportedBackend) — and not one item is reachable at runtime from `lumen` or `lumen-tauri`: both shipped hosts construct the null device through the inherent `NullBackend::open_stream` and never dispatch through the trait. The only producers of `Box<dyn AudioBackend>` are `builtin_backends()` (zero callers in the entire repo) and `cpal_backend()` (a compile-time-gated typed refusal). Answering the wave's hostile question for this file: it is a seam — an honest, self-admitted one (cpal not admitted, ADR-005 pending), but a seam. **A user hears nothing, ever**: decoded PCM is written into the null ring, the CLI's poll loop renders it into throwaway buffers to advance the sample counter (the clock), and the samples are discarded; `product.toml:150`'s claim "audio-output = true # default audio device output" is FALSE (corroborates W01-A08 F-003, S0).

## 2. What actually works (verified)
- The trait layer compiles into the shipped binary and its concrete impls behave as documented under the in-crate test suite: baseline `cargo test --workspace --locked` 1681 passed/1 failed (audit/runs/baseline/test-workspace.log.txt), and `cargo clippy --workspace --all-targets -- -D warnings` CLEAN (clippy-workspace.log.txt) — includes null.rs/stub.rs trait tests (`backend.open(NULL_DEVICE_ID, …)`, `backend.pause()`, counter frozen) and tests/clock_truth.rs:38-135.
- `NullBackend::enumerate`'s single device is truthfully labeled — verified by read of null.rs:434-445 (`name: "null (headless discard)"`).
- The `clock_source` contract (counter handed to `MediaClock::audio_master`) is real on the shipped path — but via NullSink's INHERENT method (null.rs:123-125), not this trait (see F-003).

## 3. Findings

### F-001 [S2] The domain trait is dead surface in every shipped build
Where: crates/audio-output/src/device.rs:88-91 (trait head; body 88-132), with lib.rs:145-157, cpal.rs:98-107, apps/cli/src/pipeline_host.rs:1598-1599
Code (device.rs:88-91):
```rust
pub trait AudioBackend: AudioSink {
    /// Devices this backend can see right now. Empty is honest ("no
    /// devices"), never a placeholder entry.
    fn enumerate(&self) -> Vec<DeviceDesc>;
```
Code (apps/cli/src/pipeline_host.rs:1598-1599):
```rust
    let NullStream { sink, device } =
        NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;
```
What is wrong: No shipped entry point reaches this trait. `rg "dyn AudioBackend"` finds exactly two producers: `builtin_backends()` (lib.rs:152) with ZERO callers repo-wide, and `cpal_backend()` (cpal.rs:98) which can only return `Err(BackendNotAdmitted::CPAL)` and is called solely by tests/pcm_pipeline.rs:244. Both shipped hosts (apps/cli pipeline_host.rs:1598, apps/web host.rs:1731-1732) call the inherent associated fn `NullBackend::open_stream` — static, no trait dispatch. playback-engine does not depend on audio-output at all (rg: stage-name strings only). The trait's six methods are invoked only by audio-output's own tests.
Why it matters: The file presents itself as "the audio-output plugin kind's domain trait" (module doc) — the API every future backend and the plugin host would use. That integration surface has never been executed once in any shipped or tested configuration; when ADR-005 lands, the first real consumer will be its first-ever user. Meanwhile product.toml:150 tells users "default audio device output" exists — the trait that would provide it is unreachable and its only complete impl discards audio.
What correct looks like: Either wire the shipped host through the trait (host selects from `builtin_backends()`/`cpal_backend()` and calls `open`/`pause`/`resume` through `dyn AudioBackend`), or stop shipping the claim that device output exists. Correct also includes at least one test dispatching through `Box<dyn AudioBackend>`.
Confidence: HIGH — Verification: read (call-graph greps above); baseline build/test cited in §2.

### F-002 [S3] `SinkEvent` is vocabulary for an event stream that does not exist
Where: crates/audio-output/src/device.rs:56-70
Code:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkEvent {
    /// The device went away mid-playback (unplug, server restart).
    DeviceLost,
    /// The session switched to another device (counter offset
    /// carried — doc 03 §4.9 e).
    DeviceChanged,
    /// Underrun count observed (cumulative; one event per callback
    /// that could not fill entirely from the ring).
    Underrun(u64),
}
```
What is wrong: `rg "SinkEvent"` over all Rust: defined here, re-exported (lib.rs:137), and cited in two playback-engine DOC COMMENTS (recovery.rs:726, hotplug.rs:169). Never constructed, never matched, never delivered by any code. The doc promises "the event *stream* joins the session event bus in the 3.B cards" — cards that exist nowhere in the tree.
Why it matters: playback-engine's device-loss narrative reads as if `SinkEvent::DeviceLost` were a real signal; in fact its device-loss tests (scenarios_b/scenario_12) synthesize errors by hand. A reader tracing "what happens when the user unplugs the device" lands on an enum with no producer and no consumer.
What correct looks like: Either a `Box<dyn Fn(SinkEvent)>`/channel in the open contract plus a consumer in the session, or delete the enum until the bus exists (its `Underrun(u64)` variant is redundant with `AudioBackend::underruns()` anyway).
Confidence: HIGH — Verification: read (repo-wide grep, zero non-comment uses).

### F-003 [S3] Parallel duplicated API: the trait vs the inherent methods the shipped path actually uses
Where: crates/audio-output/src/device.rs:123-131 vs crates/audio-output/src/null.rs:123-130
Code (device.rs:123-131):
```rust
    fn clock_source(&self) -> Option<Arc<dyn SampleCounterSource>>;

    /// Underrun events so far (one per callback that could not fill
    /// entirely from the ring; atomic, monotonic).
    fn underruns(&self) -> u64;
```
Code (null.rs:123-130):
```rust
    pub fn clock_source(&self) -> Arc<dyn SampleCounterSource> {
        self.shared.counter.clone()
    }

    /// Underrun events so far (monotonic).
    pub fn underruns(&self) -> u64 {
```
What is wrong: The same two operations exist twice with different signatures — trait `clock_source() -> Option<Arc<…>>` vs inherent `NullSink::clock_source() -> Arc<…>` (non-Option), and `underruns()` on the trait AND on NullSink (null.rs:128) AND on NullDevice (null.rs:331). The trait contract also says "`open` is the only transition into the playing state" (device.rs:78-79), yet `NullBackend::open_stream` is a second, shipped, non-trait transition. The shipped session hands the INHERENT handle to `MediaClock::audio_master` (pipeline_host.rs:1605-1606), never the trait method.
Why it matters: Two vocabularies for one operation is drift waiting to happen (an impl can satisfy the trait while the host reads the inherent field), and it documents a contract ("only transition") the crate itself violates. It also explains F-001: the trait was never needed because the concrete path grew its own API.
What correct looks like: NullSink's inherent methods deleted in favor of the trait (or the trait slimmed to what hosts use), with the shipped host going through the trait; one signature for `clock_source`.
Confidence: HIGH — Verification: read (both files, plus pipeline_host.rs:1605-1609).

### F-004 [S3] The polymorphic path has zero test coverage
Where: crates/audio-output/src/device.rs:88-132 (subject), evidence lib.rs:152 and the crate's test files
Code (lib.rs:152-157):
```rust
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
```
What is wrong: Every trait-method call in the crate's tests is on a concrete type (`NullBackend`/`UnsupportedBackend` — static dispatch: null.rs:597-726, stub.rs:127-158, tests/clock_truth.rs:38-135, tests/callback_budget.rs:338-374). No test in the repo calls `builtin_backends()` (zero callers) or invokes any method through `Box<dyn AudioBackend>`. The exact shape a future platform adapter must satisfy (vtable dispatch, `dyn` object safety in practice) is never executed.
Why it matters: When ADR-005 lands a real backend, its integration contract is this untested dispatch path; a trivial breakage (e.g. an added associated type, or `enumerate`'s Vec allocation in the callback context) would surface only in Lead-sandbox integration, not CI.
What correct looks like: One integration test: `for backend in builtin_backends() { … open/pause/resume/underruns through the Box … }`, asserting the null arm works and the stub arm refuses with the typed error.
Confidence: HIGH — Verification: read (rg of all `.open(`/`.pause()`/… call sites; every one static).

### F-005 [S4] `DeviceId` doc contract contradicts its own public constructor
Where: crates/audio-output/src/device.rs:25-39
Code:
```rust
/// A device's stable identifier (opaque; minted by
/// [`AudioBackend::enumerate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);

impl DeviceId {
    /// Wrap a raw handle (backends mint these from their own tables).
    pub const fn new(handle: u64) -> DeviceId {
```
What is wrong: The struct doc says ids are "minted by `enumerate`"; `new` is a public `const fn` anyone may call (null.rs:53 mints `NULL_DEVICE_ID` this way, null.rs:684 shows a foreign mint rejected at RUNTIME with `audio.device_id_unknown` instead of by type).
Why it matters: Low stakes (the runtime refusal is typed and tested), but the doc sells an opacity the type system does not enforce — a reader could assume id forgery is impossible.
What correct looks like: Either `new` is crate-private with backends minting inside the crate, or the struct doc says "minted by backends; forgeable by design, rejected at open".
Confidence: HIGH — Verification: read (null.rs:53, null.rs:684).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct DeviceId` (+derives) | 27-28 | REAL, UNREACHABLE at runtime | — | u64 newtype; constructed only by null.rs const, stub tests, and unshipped platform-* crates |
| `impl DeviceId::new` | 30-34 | REAL, trivially correct | S4 (F-005) | public const mint; ignored-param none |
| `DeviceId::handle` | 36-39 | REAL, trivially correct | — | callers: platform-linux tests only (stable_device_id); zero shipped callers |
| `struct DeviceDesc` (id/name/preferred) | 45-54 | REAL, UNREACHABLE at runtime | — | plain data; constructed by null enumerate (tests) + platform-* (unshipped) |
| `enum SinkEvent` (DeviceLost/DeviceChanged/Underrun) | 60-70 | STUB vocabulary | S3 (F-002) | never constructed/matched anywhere; no event bus exists |
| `trait AudioBackend: AudioSink` | 88-132 | REAL code, UNREACHABLE dispatch | S2 (F-001) | 2 in-crate impls; no shipped caller; see F-001/F-003 |
| `AudioBackend::enumerate` | 89-91 | REAL (null: 1 device; stub: empty) | — | test-only invocation; contract "empty is honest" upheld by stub.rs:88-91 |
| `AudioBackend::open` | 93-108 | REAL (null parks discard driver; stub refuses) | — | validates device id + shape via `Shared::open` (null.rs:447-466); test-only invocation |
| `AudioBackend::pause` | 110-114 | REAL (flag store, null.rs:468-471) | — | counter freeze verified by tests/clock_truth.rs:100; test-only invocation |
| `AudioBackend::resume` | 116-121 | REAL (flag clear, null.rs:473-476) | — | tests/clock_truth.rs:115; test-only invocation |
| `AudioBackend::clock_source` | 123-127 | REAL, duplicates inherent API | S3 (F-003) | returns Option; shipped path uses null.rs:123 non-Option inherent twin |
| `AudioBackend::underruns` | 129-131 | REAL, duplicates inherent API | S3 (F-003) | atomic load; twins at null.rs:128/331 |
No macros, no config keys, no `unsafe`, no executable bodies beyond the two trivial `DeviceId` fns — panics/overflow/drain risks: none present in this file.

## 5. Test honesty
No `#[cfg(test)]` module in this file. Trait exercised indirectly:
| Test (elsewhere) | Would it fail if the code were broken? | Reason |
|---|---|---|
| null.rs tests (597-726): open/pause/resume/counter/underruns via trait on concrete `NullBackend` | YES for the concrete impl | real assertions on counter freeze and underrun counts (mutation: swap pause to no-op → clock_truth-style counter assertions fail) |
| stub.rs tests (124-159): every op → typed `Unsupported` | YES | asserts error code `audio.os_backend_pending` and platform name |
| tests/clock_truth.rs:38-135, tests/callback_budget.rs:338-374 | YES (static dispatch only) | real open/pause/resume/underruns through `NullBackend` |
| tests/pcm_pipeline.rs:244 `cpal_backend().err()` | NO for anything real | asserts the refusal marker only — the adapter behind the gate does not exist (cpal.rs:106 `unreachable!`) |
| NOTHING covers `builtin_backends()` or `dyn AudioBackend` dispatch | — | F-004 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "audio-output = true # default audio device output" | product.toml:150 | FALSE | Only complete impl discards audio (null.rs:434-445 "null (headless discard)"); cpal gate false (cpal.rs:43); hosts render into discard buffers (host.rs:1270-1276); baseline INDEX standing defect: "Audio 'plays' only into the null device". Already S0'd as W01-A08 F-003 — corroborated, not re-counted |
| "v0 implementations: NullBackend (complete) and UnsupportedBackend (typed not-supported stubs…)" | device.rs:13-15 | REAL | null.rs:433-488, stub.rs:87-117 read; behavior under test (§5) |
| "The session hands this [clock_source] to MediaClock::audio_master" | device.rs:123-126 | FALSE for this trait method | The session uses NullSink's inherent `clock_source()` (pipeline_host.rs:1605-1606, null.rs:123); the trait method has zero shipped callers (F-001/F-003) |
| "the event *stream* joins the session event bus in the 3.B cards" (SinkEvent) | device.rs:57-59 | PAPER-ONLY | Zero constructions repo-wide; no bus exists (F-002) |
| engine-info feature `audio.null-device` | apps/cli/src/host.rs:1639 (baseline lumen-engine-info.txt) | REAL (accurate label, hard-coded source) | The null device IS the shipped sink; the string itself is a literal (INDEX standing defect), but its content is true |
| "8 — final audio sink + master clock (domain trait `AudioBackend` in `audio-output`)" | plugin-api/src/kind.rs:50-52 | REAL-UNPROVEN | Trait exists; whether any plugin host ever binds kind 8 to it is UNVERIFIED here (no plugin-api/src audits yet — open question) |

## 7. Dead / duplicated / unreachable code
- **Unreachable (this whole file, at runtime, in shipped builds):** `lumen` → apps/cli → audio-output uses only null.rs inherent items (pipeline_host.rs:52-53, 1598; host.rs:79). No app imports `AudioBackend`/`DeviceId`/`DeviceDesc`/`SinkEvent` (rg over apps/: zero). The only `dyn AudioBackend` producers: lib.rs:152 `builtin_backends` (zero callers) and cpal.rs:98 `cpal_backend` (refusal-only; sole caller tests/pcm_pipeline.rs:244). platform-* (pulse/coreaudio/wasapi) do consume `DeviceId`/`DeviceDesc`, but no app depends on platform-* (rg "platform-" apps/*/Cargo.toml: no matches) — unshipped.
- **Dead:** `SinkEvent` (F-002); `builtin_backends` (F-001); `DeviceId::handle` outside platform tests.
- **Duplicated:** trait `clock_source`/`underruns` vs inherent twins (F-003).

## 8. Dependencies & flags
- Imports: `audio_engine::AudioSink` (exists, buffer.rs:49), `media_clock::SampleCounterSource` (exists, source.rs:35), `media_types::{EngineError, frame::AudioFormat}` (exist; ErrorKind::Device/Unsupported at error.rs:204/193). All doc-links resolve.
- No feature flags gate this file; it is always compiled into every audio-output consumer, including the shipped CLI — which is what makes F-001 a shipped-dead-surface finding rather than cfg'd-out code.
- cpal seam (cpal.rs:43 `AUDIO_ADR005_ADMITTED = false` + compile-time assert) correctly guarantees no OS audio library is linked anywhere — the null discard is not an accident but an enforced build property.

## 9. Open questions for the next agent
- W08-A01 (null.rs): does `Shared::open` honor the "open-time allocation only" contract (device.rs:78-81) — ring allocated once, callback path allocation-free? The budget test (callback_budget.rs) claims to measure it.
- W08-A08 (cpal.rs) / W17-A07 (records/audio-output-cpal.md) / W19-A07 (ADR-005): the admission record's "sandbox cannot build alsa-sys" claim vs the fact that nothing would need to build until the gate flips — is the record evidence or narrative?
- Plugin host (future wave): does anything ever instantiate plugin kind 8 (`AudioOutput`, plugin-api/src/kind.rs:50-52) against `AudioBackend`, or is the "plugin kind" row vocabulary-only like SinkEvent?
- Who is meant to call `builtin_backends()`? If the answer stays "nobody", it should be deleted with the selection policy comment (lib.rs:149-151).
