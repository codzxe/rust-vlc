# AUDIT: crates/platform-macos/src/macos/coreaudio.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 767 / 16 functions + 8 extern FFI decls + 13 ABI constants |
| File verdict | UNREACHABLE (real, careful code; zero shipped reachability; zero committed compile/run evidence) |
| Reachable from shipped entry point? | NO. Chain ends at `MacOsPlatform::discover` (lib.rs:98) → `audio_seam` (lib.rs:143) → `CoreAudioDevices::new` (lib.rs:146, `#[cfg(target_os = "macos")]`). Callers of `MacOsPlatform::discover`: ONLY `crates/platform-macos/tests/seam_behaviour.rs:393`. No Cargo.toml in the repo depends on `platform-macos` (`rg -n 'platform-macos' --glob 'Cargo.toml'` → only its own manifest); neither `apps/cli` (`lumen`) nor `apps/desktop-tauri` reaches it. Double-gated: even on macOS the crate is an orphan. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `sed -n` (full read 1-767; quote verification 684-692, 39-47, 562-568) · `rg -n 'coreaudio\|CoreAudioDevices' crates` (users: lib.rs:146 + mod.rs:16 only) · `rg -n 'platform-macos' --glob 'Cargo.toml'` (zero consumers — re-verified W01-A09's orphan finding) · `rg -n 'apple-darwin'` repo-wide (only prose claims + worklog.md:556; NO committed run log) · `rg -n 'unsafe_code' Cargo.toml` (workspace deny → the file-scoped `#![allow(unsafe_code)]` at :52 is required, matches crate design) · read of `src/lib.rs` (cfg gate at :61-62), `src/events.rs`, `src/audio.rs` (trait contracts), `tests/seam_behaviour.rs` |

## 1. One-paragraph verdict
This is carefully engineered FFI: hand-rolled CoreAudio/CoreFoundation ABI with correct four-char-code constants (`'dev#'` 0x64657623, `'dOut'` 0x644f7574, `'name'` 0x6e616d65, `'uid '` 0x75696420, `'str#'` 0x73747223, `'nsr '` 0x6e737220, `'glob'`, `'outp'`, kCFStringEncodingUTF8 0x08000100 — all cross-checked against the real headers), a 64 KiB allocation bound, per-buffer bounds checks against the HAL-reported size, no panics in the extern callback, CFRelease paired with the HAL property-read ownership, a rollback of the first listener on a failed second registration, and a documented mutex-across-HAL-call prohibition the code obeys. But none of it is reachable from any shipped entry point (orphan crate, cfg-macOS), and no committed evidence proves it even compiles: the Linux baseline build never sees this file, the "compile-verified via cargo check --target aarch64-apple-darwin" claim has no committed run log (only a worklog sentence), and runtime behavior is admitted `[VERIFY]`. 767 lines of unsafe FFI with a product blast radius of exactly zero.

## 2. What actually works (verified)
— verified by read/trace only (nothing executable in this sandbox touches this file):
- ABI constants correct against CoreAudioBaseTypes.h/AudioObjectBase.h (cross-checked by this auditor against header knowledge; single remaining doubt — `ELEMENT_MAIN = 1` — is flagged by the code itself at :133-137).
- `get_property_bytes` (:235-293): size-bounded allocation (`MAX_PROPERTY_BYTES` 65_536, :253-259), zero-initialized buffer, growth-between-size-and-read refused (:284-290) — no unbounded allocation from HAL-controlled sizes.
- `on_property_change` (:548-590): zero `unwrap`/`expect`/index/panic paths in the extern callback (every lock is `if let Ok`); lock never held across a HAL call; `Drop for Watch` (:623-656) removes both listeners with the identical proc+client-data triple before the state `Arc` can drop — matches the module's soundness story.
- `output_channels` (:496-530): bounds-checks every `AudioBuffer` read against the actual byte count (:521-523); `saturating_add` on channel sum; `header = size_of::<AudioBufferList>() - size_of::<AudioBuffer>()` is a correct offsetof computation for a repr(C) trailing-array struct.
- `subscribe` rollback path (:735-756): failed second registration removes the first before returning Err.
- `poll()` returning `Ok(0)` matches the seam contract verbatim (`audio.rs:48-52`: "Push-based integrations (… CoreAudio listeners) return 0").
- The bounded `DeviceEventQueue::push` the listener calls never blocks or panics on a full/poisoned queue (`events.rs:92-110`) — callback-safe by construction.

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/macos/coreaudio.rs:684-691
Code:
```
    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError> {
        // A second subscribe replaces the first watch (the old queue
        // stops receiving events — Watch::drop unregisters).
        self.watch = None;
        // Prime the translation map through the same enumeration the
        // seam exposes (also the lane's probe: a failure here is the
        // capability verdict).
        let found = list_devices()?;
```
What is wrong: A re-subscribe drops the live `Watch` (unregistering both HAL listeners) BEFORE the new subscription's probe (`list_devices()?`) has succeeded. If the probe fails — a completely realistic HAL refusal this very file models with typed errors — the method returns `Err` having already destroyed a working notification subscription, and `self.watch` stays `None`: the adapter silently loses all device-change events with no path back except a future successful `subscribe`.
Why it matters: Destroying a working subscription because a new one failed to start is wrong ordering, not graceful degradation; a consumer that treats `Err` as "keep the old one" (the natural reading of an error return) is betrayed. Blast radius today is zero (orphan crate), but the moment any host wires this seam the bug ships.
What correct looks like: Probe first into a local (`let found = list_devices()?` before touching `self.watch`), and only then swap the old `Watch` out — or re-register the old listeners on failure. The trait doc (`audio.rs:40-41` "Re-subscribing replaces the previous queue") says nothing about the failure case destroying the previous queue.
Confidence: HIGH — Verification: read (ordering is unambiguous in the source).

### F-002 [S3]
Where: crates/platform-macos/src/macos/coreaudio.rs:39-47
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]` on
//! real-device behavior (in particular the numeric value of
//! `kAudioObjectPropertyElementMaster` and the claim that reading
//! `kAudioHardwarePropertyDefaultOutputDevice` inside its own
//! listener is deadlock-free — the HAL is documented to invoke
//! property listeners without holding its global lock, but that
//! claim needs hardware confirmation).
```
What is wrong: The file's only assurance that it typechecks is an executed-evidence claim with no committed artifact. The wave-0 baseline (audit/runs/baseline/INDEX.md, 26 runs) contains no `cargo check --target *-apple-darwin`; repo-wide `rg 'apple-darwin'` finds only prose claims in crate docs and one worklog sentence (worklog.md:556, "Cross-target ACTUAL: … clean and warning-free"). Worse: `lib.rs:61-62` gates `pub mod macos` behind `#[cfg(target_os = "macos")]`, so the committed Linux workspace build (baseline #1, exit 0) and both workspace test runs (baseline #3/#4 and orchrun-test-workspace-nofailfast.log.txt — 4534 passed / 7 failed / 49 ignored) never compiled a single line of this file. Within all committed evidence this file has never been compiled by anyone.
Why it matters: 767 lines of `unsafe extern "C"` FFI whose compilation status rests on an unverifiable sentence — the exact "evidence strings that prove nothing" pattern this audit exists to catch. W01-A09 rated the identical claim S3 in the manifest (F-003); this is the same defect at the file level, now also covering the unverified ABI constant (`ELEMENT_MAIN = 1`, :133-137) on which every property address depends.
What correct looks like: A committed run log (like every baseline run got) or the claim reworded to "compile expected, not evidenced".
Confidence: HIGH — Verification: read + exhaustive rg (no run log exists); baseline INDEX cited.

### F-003 [S3]
Where: crates/platform-macos/src/macos/coreaudio.rs:562-567
Code:
```
        if selector == SELECTOR_DEFAULT_OUTPUT {
            // Read the new default inline. Reading a *different*
            // property than the one being notified (devices-set vs
            // default-device) is the listener-reentrancy pattern
            // Apple's own samples use; `[VERIFY]` on hardware.
            let new_default = default_output_device_id().ok().flatten();
```
What is wrong: The listener callback calls back into the HAL (`default_output_device_id()` → `AudioObjectGetPropertyDataSize`/`AudioObjectGetPropertyData`) from a HAL-owned notification thread while being notified about that very property. The module docs (:42-47) admit the deadlock-freedom of this pattern is unconfirmed on hardware. If the HAL ever invokes this listener while holding state the property read needs, the HAL's notification thread deadlocks — device-change delivery dies process-wide and the deadlock is in FFI with no Rust stack to diagnose.
Why it matters: An admitted-unverified deadlock hazard inside an OS callback is the single highest-consequence unknown in this file; the `.ok()` also swallows the failure, so a refused read degrades to `AudioSetChanged` (acceptable) but a hung read hangs forever (not acceptable). Blast radius zero today (orphan crate) — otherwise this would be S1/S2 material.
What correct looks like: Do not read properties inside the listener: push an `AudioSetChanged`-class event (or a pending flag) and resolve the new default on the consumer thread, where a HAL call cannot deadlock a HAL thread. That also removes the `[VERIFY]`.
Confidence: LOW (hazard real, occurrence UNVERIFIABLE in this sandbox) — Verification: read; no macOS hardware available; flagged UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block CoreAudio (4 fns) | 175-203 | REAL-UNPROVEN | — | Declarations match the real ABI (names/arity/types); never linked in any committed run. |
| extern block CoreFoundation (2 fns) | 214-223 | REAL-UNPROVEN | — | `CFStringGetCString`/`CFRelease` signatures correct; ownership comment (:205-213) honestly reasons the Get-rule-vs-property-contract tension. |
| `get_property_bytes` | 235-293 | REAL-UNPROVEN | — | Bounded alloc (65_536), zero-init, growth refused. No panic paths. Sound. |
| `get_property_string` | 301-344 | REAL-UNPROVEN | — | Pointer-size check, null check, 4096-byte copy bound, release-on-failure (no leak), UTF-8 validated. Sound. |
| `get_property_f64` | 350-366 | REAL-UNPROVEN | — | Exact 8-byte size check; `from_ne_bytes` matches ABI. Sound. |
| `system_device_ids` | 369-395 | REAL-UNPROVEN | — | Rejects non-u32-multiple arrays; `chunks_exact(4)`. Sound. |
| `default_output_device_id` | 398-421 | REAL-UNPROVEN | — | 4-byte check; `id==0` → `None` (kAudioObjectUnknown). Sound. |
| `describe_device` | 426-462 | REAL-UNPROVEN | — | Skips UID-less devices (`Ok(None)`), never guesses; name failure swallowed to a synthetic name — documented, cosmetic field. |
| `preferred_format` | 467-492 | REAL-UNPROVEN | — | Rate errors → documented 48 kHz fallback; channels clamped `max(1)` at :488. Suggestion-only data. |
| `output_channels` | 496-530 | REAL-UNPROVEN | — | Per-buffer bounds check vs actual size; `saturating_add`; breaks on first out-of-bounds buffer. Sound parsing of a variable-length C struct. |
| `on_property_change` (extern "C") | 548-590 | REAL-UNPROVEN | S3 | No panics/unwrap in callback (verified line-by-line); F-003 reentrancy hazard. |
| `CoreAudioDevices::new` | 605-607 | REAL-UNPROVEN | — | Trivial constructor. |
| `Default for CoreAudioDevices` | 610-614 | REAL-UNPROVEN | — | Delegates to `new`. |
| `Drop for Watch` | 623-656 | REAL-UNPROVEN | — | Removes both listeners with identical triple, no mutex held; ignores remove status (HAL best-effort teardown — a failed removal would leak the registration until process exit; acceptable for teardown, noted not flagged). |
| `list_devices` | 660-665 | REAL-UNPROVEN | — | `filter_map(…transpose())` propagates errors, skips None. Correct. |
| `AudioDevices::enumerate` | 668-682 | REAL-UNPROVEN | — | Refreshes listener translation map under lock only, never across HAL calls. Correct. |
| `AudioDevices::subscribe` | 684-759 | REAL-UNPROVEN | S3 | F-001 teardown-before-probe; rollback of first listener on second's failure correct. |
| `AudioDevices::poll` | 761-765 | REAL-UNPROVEN | — | `Ok(0)` — matches the push-based contract in `audio.rs:48-52` verbatim; NOT a fake (documented seam semantics). |

## 5. Test honesty
No test in the repository executes or even compiles this file: `tests/seam_behaviour.rs` runs on Linux where `mod macos` is cfg'd out (lib.rs:61-62); its CoreAudio-adjacent tests exercise only `DeviceEventQueue`, `stable_device_id`, the typed fallbacks, and the wrong-OS `discover()`. Those tests are honest for what they test (queue bound/drop accounting is real; the thread-safety test spawns real threads) but they prove nothing about this file. The file has zero test coverage and zero compile coverage in every committed run.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "CoreAudio (device enumeration + change listeners): the HAL's object-property API … no coreaudio-sys crate" | lib.rs:26-29 | REAL-UNPROVEN | Code exists (this file, 767 LOC, hand-rolled ABI); never compiled in any committed run; unreachable from any shipped binary. |
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | coreaudio.rs:39-41, worklog.md:556 | REAL-UNPROVEN | No committed run log exists (F-002); worklog sentence is a claim, not evidence. |
| Listeners "push PlatformEvents (push-based — AudioDevices::poll returns 0)" | coreaudio.rs:20-22 | REAL-UNPROVEN | Code matches (poll :761-765); behavior never executed. |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable (see reachability field): the only caller chain terminates at `MacOsPlatform::discover`, which no app invokes. Parallel macOS FFI also exists in `crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs` (separate hand-rolled ABI) — duplicated-FFI drift risk already flagged by W01-A09 (manifest F-002).

## 8. Dependencies & flags
`use audio_output::{DeviceDesc, DeviceId}` + `use media_types::frame::{AudioFormat, AudioLayout, SampleFormat}` — both declared in the crate manifest and used. `#![allow(unsafe_code)]` (:52) is required by the workspace `unsafe_code = "deny"` (root Cargo.toml:54) and scoped to this FFI module, matching the crate doc's claim that unsafe lives only in FFI modules. `#[cfg(target_os = "macos")]` at the module root (lib.rs:61) is the compile gate — legitimate for an OS adapter, but it is what removes the file from every committed build.

## 9. Open questions for the next agent
- If the orchestrator can obtain a darwin cross-toolchain: does `cargo check -p platform-macos --target aarch64-apple-darwin` still pass (F-002), and does the default-device read inside its own listener deadlock on real hardware (F-003)? REQUEST-ORCH-RUN: `cargo check -p platform-macos --target aarch64-apple-darwin --all-targets` (compile evidence only; no link/run).
- `platform-macos/src/macos/metal.rs` and `dialogs.rs` repeat the same no-evidence compile-verified claim — same pattern, next auditor should inherit F-002's framing.
