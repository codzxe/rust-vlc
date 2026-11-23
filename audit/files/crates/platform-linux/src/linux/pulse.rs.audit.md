# AUDIT: crates/platform-linux/src/linux/pulse.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 759 / 17 (8 free fns, 5 inherent methods, 3 trait methods, 1 Drop) |
| File verdict | UNREACHABLE (real dlopen'd PA client; absent-server path executed; live-server path REAL-UNPROVEN; no shipped entry point reaches this crate) |
| Reachable from shipped entry point? | NO — `PulseAudioDevices::probe` is called only by `platform-linux/src/lib.rs:144` (`audio_seam` → `LinuxPlatform::discover`); grep of all Cargo.toml + code shows zero crates outside `platform-linux` depend on it; `apps/cli` and `apps/desktop` dependency lists do not include it |
| Findings | S0: 0 S1: 0 S2: 3 S3: 3 S4: 1 |
| Commands run | `cargo test -p platform-linux pulse` → 2 passed, exit 0; `sed -n` quote verification (all quotes below); `rg PulseAudioDevices\|platform_linux crates apps` → only platform-linux + assignments.tsv; `ls /usr/lib/x86_64-linux-gnu/libpulse*` → libpulse.so.0 present, no daemon (`/run/user/1001/pulse` absent, no pactl) |

## 1. One-paragraph verdict
This is a REAL PulseAudio client, not a decorative dlopen seam: `dl.rs` performs genuine `dlopen`/`dlsym`, `pulse_ffi.rs` resolves 20 real `pa_*` symbols with correct frozen-ABI signatures (PA_CONTEXT_READY=4/FAILED=5, subscription masks 0x0001|0x0080|0x0200, facility/type masks — all match pulse/def.h), and this file drives a real connect→enumerate→subscribe lifecycle with bounded waits. The absent-server path is runtime-verified by me (`cargo test -p platform-linux pulse` → 2 passed; this host has libpulse.so.0 and no daemon). However: (a) the whole crate is wired to NOTHING — no shipped binary (lumen-cli, desktop) depends on platform-linux, so the Linux audio-device feature ships to nobody; (b) `subscribe()` ignores the PA success flag, so a server-rejected subscription still returns `Ok(())`; (c) three `pump_once(...)?` sites bypass all cleanup, leaking ctx/mainloop/operation and leaving registered callbacks pointing at dead stack/heap userdata — directly falsifying the file's own ownership-contract comments.

## 2. What actually works (verified)
— verified by `cargo test -p platform-linux pulse` → `test pulse_probe_reports_this_hosts_truth ... ok`, `test pulse_subscribe_without_a_server_is_the_typed_error ... ok` (exit 0). This executes `probe()` (real dlopen of libpulse.so.0), `enumerate()` (connect phase hits no-server → typed `ServiceUnavailable`), and `subscribe()` (same typed error). Live-server device data: NOT executable in this sandbox (no PA daemon) — REAL-UNPROVEN.

## 3. Findings
F-001 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:463, 561, 707 (contract comment at 611-614)
Code:
```
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
```
```
            // connect() freed the context AND the mainloop on
            // every error path (its ownership contract) — nothing
            // left to tear down here.
```
What is wrong: if `pump_once` returns Err (prepare/poll/dispatch failure), `?` returns immediately from `connect` (463), `list_sinks` (561), and `subscribe`'s ack loop (707) WITHOUT running any teardown: the mainloop, context, and operation leak, the state/sink/subscribe callbacks remain registered, and their userdata dangles — at 463 a stack `ConnState`, at 561 a stack `EnumState`, at 707 the about-to-be-dropped `Box<WatchState>`. The comment at 611-614 asserts connect frees everything "on every error path": that is false for this path. Why it matters: the file's entire safety argument (module docs lines 21-29: "every registration that points at stack memory is cleared ... before the owning scope returns") is violated on these paths; a later dispatch of a leaked mainloop would be a use-after-free (mitigated only because nothing pumps a leaked mainloop — the simple `pa_mainloop` has no internal thread). What correct looks like: replace `?` with a failure arm that cancels/unrefs the op, clears the callback, and runs `teardown_context`. Confidence: HIGH — Verification: read.

F-002 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:269-275, 694-699, 725-731
Code:
```
/// No-op success callback for `pa_context_subscribe`.
unsafe extern "C" fn on_subscribe_done(
    _ctx: *mut PaContext,
    _success: c_int,
    _userdata: *mut c_void,
) {
}
```
```
            let op_state = unsafe { (self.api.operation_get_state)(op) };
            if op_state != OPERATION_RUNNING {
                break;
            }
```
What is wrong: `pa_context_subscribe`'s success flag is discarded; the ack loop only waits for the operation to leave RUNNING. A server that refuses the subscription (success=0, operation state DONE) still falls through to `self.watch = Some(Watch { ... }); Ok(())`. Why it matters: `subscribe()` reports success for a subscription the server rejected — the exact "fake success" the crate's docs promise never happens ("it never fakes success", lib.rs:74). What correct looks like: capture the success value in the callback and return a typed error when 0. Confidence: HIGH — Verification: read (PA success-callback semantics; not executable here — no server).

F-003 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:734-739
Code:
```
    fn poll(&mut self) -> Result<u32, PlatformError> {
        let Some(watch) = self.watch.as_ref() else {
            return Ok(0);
        };
        pump_once(&self.api, watch.ml, 0)
    }
```
What is wrong: `poll` never checks the context state. When the PA server dies or restarts (routine on desktop Linux), the watch context goes FAILED, the cleared state callback (cleared after connect, 469) means nobody is notified, dispatch delivers zero events forever, and `poll` returns `Ok(0)` eternally. `list_sinks` checks `PA_CONTEXT_FAILED` (550) but `poll` does not. Why it matters: the notification feature silently dies on server restart with no typed error, no reconnection, and no `AudioSetChanged` event — the realistic desktop case is the broken one. What correct looks like: check `context_get_state` in `poll` and surface `ServiceUnavailable`/teardown so the owner can resubscribe. Confidence: HIGH — Verification: read.

F-004 [S3]
Where: crates/platform-linux/src/linux/pulse.rs:592-597
Code:
```
        if let Some((ctx, ml)) = self.watch.as_ref().map(|watch| (watch.ctx, watch.ml)) {
            let state = self.list_sinks(ctx, ml)?;
            if let Some(watch) = self.watch.as_mut() {
                watch.state.index_ids = state.index_ids;
            }
            return Ok(state.devices);
        }
```
What is wrong: on `list_sinks` error the watch is retained with a possibly-FAILED context; every later `enumerate`/`poll` reuses the corpse (get_sink_info_list on a failed context returns NULL → repeated `CallFailed`), and the index→id cache goes stale. No teardown, no recovery. Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/platform-linux/src/linux/pulse.rs:31-33 and lib.rs dependency reality
Code:
```
// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the dlopen'd
// PulseAudio C ABI is the audited OS-service exception this file
// exercises (ADR-005 is the audio seam's decision record).
```
What is wrong: the entire crate is an orphan. `rg platform_linux|platform-linux` over every Cargo.toml and all source: zero dependents outside the crate itself (apps/cli deps: audio-engine…video-engine, no platform-linux; apps/desktop deps: commands, lumen-cli, media-types, renderer-wgpu, video-engine). The real PA client is compiled and tested only as a workspace member; no shipped entry point (lumen-cli, desktop, desktop-tauri) can reach it. Any doc/matrix row claiming Linux audio-device enumeration as a product feature is UNREACHABLE-by-wiring. Confidence: HIGH — Verification: read (rg over Cargo.toml + code).

F-006 [S3]
Where: crates/platform-linux/src/linux/pulse.rs:286-293
Code:
```
    let sample = match spec.format {
        0 => SampleFormat::U8,  // PA_SAMPLE_U8
        3 => SampleFormat::S16, // PA_SAMPLE_S16LE
```
What is wrong: big-endian formats (4=S16BE, 6=FLOAT32BE, 8=S32BE, 10=S24BE) and alaw/ulaw all silently map to F32. This is honestly documented as "documented approximation", but on a BE sink the "preferred" suggestion is simply wrong, and the LE assumption is baked in ("the linux targets are LE" — false for e.g. s390x/powerpc Linux). Blast radius: suggestion field only. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/platform-linux/src/linux/pulse.rs:397-398
Code:
```
        // lumen-allow-literal: PulseAudio protocol client identity (config edge is the Lead-gated follow-up).
        let name = CString::new("lumen").map_err(|_| PlatformError::CallFailed {
```
What is wrong: hard-coded client identity with an acknowledged-but-unfinished config edge. Honest about it; still debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| owned | 144-152 | REAL | — | null-safe lossy C-string copy |
| strerror | 156-167 | REAL | — | null-guarded static-string copy |
| on_context_state | 174-186 | REAL | — | sets ready/failed flags; userdata cleared before scope exit except F-001 path |
| on_sink_info | 189-228 | REAL | — | copies name/desc/spec; mints stable id; defensive null/eol/nameless handling |
| on_subscription_event | 231-267 | REAL | — | facility/type masks correct vs pulse/def.h; unmapped → AudioSetChanged (honest) |
| on_subscribe_done | 270-275 | STUB-ish | S2 | discards success flag → F-002 |
| preferred_format | 286-305 | REAL | S3 | LE-only mapping, BE wrong (F-006) |
| pump_once | 313-346 | REAL | — | correct prepare→poll→dispatch, typed errors |
| probe | 354-362 | REAL (executed) | — | real dlopen; ran in my test |
| loaded_from | 366-369 | REAL (executed) | — | asserted by passing test |
| connect | 375-484 | REAL | S2 | bounded 2s; F-001 leak on pump error |
| teardown_context | 490-501 | REAL | — | documented order |
| list_sinks | 504-585 | REAL | S2 | F-001 leak; FAILED-context check present here (contrast F-003) |
| enumerate | 589-621 | REAL (absent-server path executed) | S3 | F-004 stale watch |
| subscribe | 623-732 | REAL | S2 | F-001, F-002 |
| poll | 734-739 | REAL | S2 | F-003 silent death |
| Watch::drop | 742-758 | REAL | — | unregisters callback before freeing state; order sound |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/linux_integrations.rs::pulse_probe_reports_this_hosts_truth | Partially | Accepts Ok-with-devices, ServiceUnavailable, or LibraryUnavailable — passes on ANY host; only real assertions: `loaded_from` is a real SONAME and non-empty device names. It proves probe+enumerate run honestly, not that a live server works. |
| tests/linux_integrations.rs::pulse_subscribe_without_a_server_is_the_typed_error | No (weakly) | Accepts both Ok(()) (live server) and ServiceUnavailable; on this host it exercises only the typed-error branch. Cannot catch F-002 (needs a server that rejects). |
Both ran and passed in MY run (2 passed, exit 0). Live-server branch of both: UNVERIFIED (no PA daemon in sandbox).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The library is dlopened ... never a link error or a fake device list" | pulse.rs:8-11 | REAL | dl.rs:92-149 real dlopen/dlsym; my test run |
| "no server gets a typed ServiceUnavailable ... runtime-tested here" | pulse.rs:12-14 | REAL | my `cargo test -p platform-linux pulse` → 2 passed on host with libpulse, no daemon |
| "A machine with a live server gets real sinks as DeviceDescs" | pulse.rs:15-17 | REAL-UNPROVEN | no daemon in sandbox; code path read, not executed |
| "every registration that points at stack memory is cleared ... before the owning scope returns" | pulse.rs:24-26 | FALSE (on error paths) | F-001: pump-error `?` exits leave registrations live (463/561/707) |
| "connect() freed the context AND the mainloop on every error path" | pulse.rs:611-613 | FALSE | F-001: pump error at 463 returns without freeing |
| "it never fakes success" (subscribe) | lib.rs:74, applied here | FALSE | F-002: success flag discarded, Ok(()) on rejection |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped products (F-005): real code, zero shipped callers. No dead functions inside the file; all 17 are called.

## 8. Dependencies & flags
Std-only crate (Cargo.toml: no external deps; audio-output + media-types path deps). No feature gates on this file; `target_os = "linux"` gate on the `linux` module (lib.rs:58) — correct. `#![allow(unsafe_code)]` (line 34) is the workspace-deny exception, documented.

## 9. Open questions for the next agent
- pulse_ffi.rs (W08-A10): doc claims "exactly the 21 functions"; the PulseApi table resolves 20. Off-by-one in the doc — verify.
- Does any docs/feature-matrix row cite pulse.rs as evidence for Linux audio enumeration? If yes, that row is UNREACHABLE-by-wiring (F-005) — matrix auditor should cross-check.
- lib.rs (W08-A07): `audio_seam()` comment says "Available must mean enumeration works end to end" — but `discover_assembles_this_hosts_report` (linux_integrations.rs:453) passes with either outcome; check how the capability report is (not) surfaced to users.
