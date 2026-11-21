# AUDIT: crates/platform-linux/src/linux/pulse.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 759 / 17 units (13 fns, 3 callbacks, 1 Drop impl) + 14 constants |
| File verdict | UNREACHABLE (code is genuine; the no-server paths are executed by baseline tests; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — no crate or app depends on `platform-linux`: for-loop over all `crates/*/Cargo.toml` + `apps/*/Cargo.toml` for `platform-linux` matches only the crate's own manifest; `rg -n "LinuxPlatform" apps/cli/src apps/control-server/src apps/desktop apps/web/src crates/device-discovery/src crates/playback-engine/src crates/audio-output/src` → 0 hits. Only internal chain: `lib.rs audio_seam()` → `LinuxPlatform::discover()`, itself caller-free, plus the crate's own tests. |
| Findings | S0: 0 S1: 0 S2: 3 S3: 2 S4: 1 |
| Commands run | Read of full file; `rg -n "LinuxPlatform\|platform-linux"` across apps/ crates/ (callers, manifests); `sed -n` quote verification (379-382, 446-464, 514-516, 560-562, 700-716, 734-739); baseline `orchrun-test-workspace-nofailfast.log.txt:3644-3656` (platform-linux tests executed, 13 passed); read of `src/audio.rs` (trait contract), `src/events.rs` (bounded queue D-3), `src/lib.rs` (seam wiring), `tests/linux_integrations.rs:383-445`; pulse/def.h constant cross-check (context states 4/5, sub masks 0x0001/0x0080/0x0200, facility/type masks, sample formats 0/3/5/7/9). |

## 1. One-paragraph verdict
This is a real, hand-rolled dlopen'd PulseAudio FFI client — not a stub — with correct pulse/def.h constants (context states, subscription masks, sample formats all cross-checked), genuinely bounded waits (2s connect/enumeration deadlines, 50ms pump slices), and a teardown discipline that is sound on the paths its tests exercise. The blast radius of its defects is bounded by one fact this audit established: **nothing in the shipped graph depends on `platform-linux`** — the `lumen` CLI cannot reach a single line of this file, making the whole 759-line unsafe FFI module dead weight in the workspace build. Within the module itself, all three long-running loops share the same defect class: a `pump_once(...)?` early-return that bypasses every cleanup the surrounding code carefully performs on its *other* failure paths — leaking the mainloop/context/operation and, in two of the three, leaving a library-registered callback pointing at storage that dies with the return (a use-after-free grenade if a live Watch is ever pumped again). The in-file safety comments explicitly claim those cleanups happen "in every path"; they do not.

## 2. What actually works (verified)
— Absent-library and absent-server paths executed: baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3644-3656` shows `tests/linux_integrations.rs` ran `pulse_probe_reports_this_hosts_truth` and `pulse_subscribe_without_a_server_is_the_typed_error` → `ok` (13 passed total), exercising `probe()` → dlopen of real `libpulse.so.0` → `connect()` → typed `ServiceUnavailable` (this sandbox has libpulse but no daemon).
— Timeout paths are bounded by construction: `CONNECT_DEADLINE`/`ENUM_DEADLINE` (2s) + `PUMP_SLICE` (50ms) constants at lines 77-81, checked at 456-462, 540-546, 700-706 — no unbounded foreign-loop wait (D-2 honored).
— FFI constants correct vs pulse/def.h: `PA_CONTEXT_READY=4`/`FAILED=5` (54-55), subscription mask `0x0001|0x0080|0x0200` (61), facility/type masks (64-70), `PA_SAMPLE_{U8=0,S16LE=3,FLOAT32LE=5,S32LE=7,S24LE=9}` (287-293) — all match the PulseAudio headers.
— Drop ordering of `Watch` (742-758): unregister callback → disconnect → unref ctx → free mainloop → Box<WatchState> frees last; field order (100-105) matches. Sound.
— `preferred_format` defensive zero-guards (297-303) and honest fallback-to-F32 for exotic formats, documented as a suggestion per doc 03 §4.9.

## 3. Findings

### F-001 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:560-562 (contract claimed at 514-516)
Code:
```
        // SAFETY: ctx is READY; the callback userdata points at the
        // stack `state`, and the operation is completed or cancelled
        // before this function returns in every path below.
```
and the loop tail:
```
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
```
What is wrong: the `?` on `pump_once` returns from `list_sinks` immediately when the mainloop reports an error — without `operation_cancel`/`operation_unref` on `op` and without clearing the sink callback. The SAFETY comment's "in every path below" is false. The pending `pa_operation` is leaked with its callback userdata pointing at the now-dead stack `EnumState`.
Why it matters: memory leak on every pump failure; worse, on the **Watch path** (`enumerate` line 592-597 reuses the persistent watch connection), the mainloop survives the error and belongs to a Watch whose `poll()` (line 738) keeps pumping it — a late reply to the leaked pending operation would invoke `on_sink_info` through the dangling userdata: use-after-free. The identical concern is the reason the timeout path at 563-578 does cancel+unref. Downgraded from S1 only because no shipped entry point reaches this crate (module UNREACHABLE); wired into a real app this is an S1.
What correct looks like: replace `pump_once(...)?` with `match pump_once(...) { Ok(_) => {} Err(e) => { failure = Some(e); break; } }` so the existing cancel/unref block runs.
Confidence: HIGH — Verification: read (control flow traced line by line); no live-server execution exists anywhere, so the UAF itself is UNVERIFIED at runtime.

### F-002 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:463 (ownership contract stated at 379-382 and relied on at 611-616, 638-643)
Code:
```
            if Instant::now() >= deadline {
                failure = Some(PlatformError::TimedOut {
                    api: "pa_context_connect",
                    deadline_ms: CONNECT_DEADLINE.as_millis() as u64,
                });
                break;
            }
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
```
(the contract:)
```
            // Ownership: this method owns `ml` for its whole body —
            // every error path frees it exactly once (the callers
            // must not free on Err).
```
What is wrong: when `pump_once` fails, `?` returns from `connect` with `ml` and `ctx` neither freed nor handed back. The timeout and failed-state paths (471-482) correctly tear down; this path does not. Both callers (`enumerate` 611-616, `subscribe` 638-643) trust the stated contract — "connect() freed the context AND the mainloop on every error path" — and free nothing themselves.
Why it matters: the documented ownership contract is false on this path → double-free hazard for any future maintainer who "fixes" the leak by freeing in the caller, and a straight mainloop+context leak (plus the state callback still registered on the leaked ctx pointing at dead stack `conn`) as the code stands.
What correct looks like: same `match`-and-break pattern as F-001, routing pump errors through the `failure` variable so the common teardown at 471-474 runs.
Confidence: HIGH — Verification: read.

### F-003 [S2]
Where: crates/platform-linux/src/linux/pulse.rs:707 (userdata registered at 661-667; cleanup only at 711-716/717-724)
Code:
```
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
        // SAFETY: completed or cancelled; releasing the reference is
        // valid in both outcomes.
        unsafe {
            if failure.is_some() {
                (self.api.operation_cancel)(op);
            }
            (self.api.operation_unref)(op);
        }
```
What is wrong: in the subscribe-acknowledgement wait, a `pump_once` error exits via `?` before the cancel/unref block, and before the `context_set_subscribe_callback(ctx, None, …)` clear at 719-721 — so `op`, `ctx`, and `ml` all leak, and the subscription callback on the leaked ctx points at the `Box<WatchState>` that is dropped when `subscribe` returns (the Box is only kept alive by `self.watch = Some(Watch { … state, … })` on the success path, line 725-730).
Why it matters: same class as F-001/F-002 — the one error path that skips cleanup; here the dangling userdata points at freed heap, and unlike F-001 the teardown the callers would do is also skipped entirely (no caller tears down after `subscribe` errors post-connect). Leaks FFI resources on a realistic failure (server dies mid-subscribe).
What correct looks like: route pump errors through `failure = Some(e); break;` so the existing clear + teardown_context path (717-724) executes.
Confidence: HIGH — Verification: read.

### F-004 [S3]
Where: crates/platform-linux/src/linux/pulse.rs:734-739 (state callback cleared at 468-470; `poll` never re-checks context state, unlike `list_sinks` at 549-560)
Code:
```
    fn poll(&mut self) -> Result<u32, PlatformError> {
        let Some(watch) = self.watch.as_ref() else {
            return Ok(0);
        };
        pump_once(&self.api, watch.ml, 0)
    }
```
What is wrong: if the PulseAudio server dies while a Watch is active, nothing is ever pushed to the `DeviceEventQueue`: the subscription callback only fires on subscription events (not connection death), the connect-phase state callback was deliberately cleared (468-470), and `poll` — unlike `list_sinks` (549-560) — never checks `context_get_state` for `PA_CONTEXT_FAILED`. `poll` returns `Ok(0)` forever on a dead watch.
Why it matters: the seam's entire notification promise ("subscribe to device changes", trait doc in `src/audio.rs:46-52`) silently dies with the server; the consumer's device list goes stale with no signal and no recovery path short of a full re-`subscribe`. Realistic case: PulseAudio restart / user switches PipeWire.
What correct looks like: `poll` checks `context_get_state(watch.ctx)` and, on FAILED/TERMINATED, pushes `PlatformEvent::AudioSetChanged` (advise re-enumeration) and/or tears the watch down so the next `subscribe` can rebuild.
Confidence: HIGH — Verification: read; runtime behavior UNVERIFIED (no live server in any recorded run).

### F-005 [S3]
Where: crates/platform-linux/src/linux/pulse.rs:1-759 (whole file; crate graph position)
Code:
```
//! The PulseAudio client (card 7.A2): sink enumeration + change
//! notifications over the synchronous `pa_mainloop` — no threads,
//! bounded waits (D-2), one short-lived connection per enumeration
//! and one persistent watch per subscription.
```
What is wrong: 759 lines of `unsafe` FFI, the crate's flagship integration, are dead code from the shipped-binary perspective: no workspace member or app depends on `platform-linux` (verified: manifest scan + `LinuxPlatform` caller scan, both empty). `cargo build --workspace` compiles it (baseline build log line 244 `Compiling platform-linux v0.1.0`) and the workspace tests run its 13 integration tests, but no `lumen` command can construct a `PulseAudioDevices`.
Why it matters: the "platform adapter" story (lib.rs: "capability negotiation is a first-class outcome") is theater until some consumer wires `LinuxPlatform::discover()` — the capability report it produces is never read by shipped code; unsafe code with zero shipped callers is pure liability plus compile time.
What correct looks like: either a consumer (device-discovery is the obvious one — its lib.rs:105 comment says the PulseAudio verdict lives in platform-linux but no dependency edge exists) or a workspace-level decision recording the crate as incubating.
Confidence: HIGH — Verification: rg caller scans (0 hits), manifest scan (self only), baseline build log (compiled).

### F-006 [S4]
Where: crates/platform-linux/src/linux/pulse.rs:393-402
Code:
```
        // The PulseAudio client name is the stable protocol identity
        // the sound server shows for this connection. Product-config
        // owns it once the platform crates gain their config edge
        // (Lead decision, recorded in the 7.A2 report).
        // lumen-allow-literal: PulseAudio protocol client identity (config edge is the Lead-gated follow-up).
        let name = CString::new("lumen").map_err(|_| PlatformError::CallFailed {
```
What is wrong: hard-coded `"lumen"` client identity with an inline lint-suppression annotation; the mapped error message ("client name contained an interior NUL") is unreachable for a compile-time constant with no NUL.
Why it matters: the dead error arm is noise; the literal is honestly annotated and Lead-gated, so this is hygiene, not fraud.
What correct looks like: `const CLIENT_NAME: &CStr = c"lumen";` (edition 2024) removes both the closure and the annotation.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `owned` | 144-152 | REAL | — | NULL-guarded lossy copy; correct. |
| `strerror` | 156-167 | REAL | — | NULL-guarded; static-storage copy-out documented. |
| `on_context_state` (cb) | 174-186 | REAL | — | Sets flags only; no panic paths; userdata lifetime depends on caller clearing (cleared at 468-470 — except F-002 path). |
| `on_sink_info` (cb) | 189-228 | REAL | — | eol/null/nameless guards; copies everything it keeps; pushes into EnumState. Dangling-userdata risk is F-001's, not the callback's. |
| `on_subscription_event` (cb) | 231-267 | REAL | — | Correct mask decomposition; honest "re-enumerate" fallback for change/unknown-index; ignored facilities documented. |
| `on_subscribe_done` (cb) | 270-275 | REAL | — | Intentional no-op, documented. |
| `preferred_format` | 286-305 | REAL | — | Constants verified vs pulse/def.h; zero-guards for channels/rate; BE/alaw fallback documented. |
| `pump_once` | 313-346 | REAL | — | prepare/poll/dispatch sequence correct; `dispatched.max(0) as u32` — redundant but safe (negative handled at 337). |
| `PulseAudioDevices::probe` | 354-362 | REAL | — | dlopen chain; executed in baseline tests (3644-3656). |
| `loaded_from` | 366-369 | REAL | — | Pure accessor; asserted by test at linux_integrations.rs:392. |
| `connect` | 375-484 | REAL | — | Bounded; teardown on timeout/failed paths; **pump-error path leaks (F-002)**. |
| `teardown_context` | 490-501 | REAL | — | Correct order (disconnect→unref→free). |
| `list_sinks` | 504-585 | REAL | — | Mid-enumeration FAILED check (549-560) is good; **pump-error path leaks op + dangling callback (F-001)**. |
| `AudioDevices::enumerate` | 589-621 | REAL | — | Watch-reuse path correct; short-lived path pairs new/teardown correctly on Ok and Err (618-620). |
| `AudioDevices::subscribe` | 623-732 | REAL | — | Replaces prior watch (626); primes cache; **pump-error path leaks all three handles (F-003)**. |
| `AudioDevices::poll` | 734-739 | REAL | — | Non-blocking (timeout 0) per trait contract; **blind to context death (F-004)**. |
| `Drop for Watch` | 742-758 | REAL | — | Teardown order verified sound; state Box freed after ctx. |

## 5. Test honesty (tests for this file live in crates/platform-linux/tests/linux_integrations.rs, not in-file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `pulse_probe_reports_this_hosts_truth` (383-417) | Partially | Would catch a probe/dlopen regression or a wrong error type on this host (no server) — it executes real `libpulse.so.0`. Would NOT catch any live-server defect (enumeration contents only asserted non-empty names), nor any of F-001..F-004 (all require a server or a mainloop error). The `Ok(list)` arm's only assertion is `!desc.name.is_empty()` — weak but honest about being `[VERIFY]`. |
| `pulse_subscribe_without_a_server_is_the_typed_error` (423-445) | Partially | Exercises subscribe→connect failure only; the entire post-connect subscription machinery (priming, callback registration, ack wait, F-003's path) never runs in any recorded environment. |

Mutation reasoning (not executed): making `connect()` skip the `pa_context_connect` call entirely would flip both tests to `Ok`-arms → they would still pass on this host (the `Ok` arms assert almost nothing); the no-server honesty is therefore pinned only by the sandbox's environment, not by the test bodies.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Real-server behavior is `[VERIFY]` (this sandbox has libpulse but no PulseAudio daemon; the absent-server path is what is runtime-tested here)" | crates/platform-linux/src/lib.rs runtime-reality section | REAL | Baseline orchrun log 3644-3656: both pulse tests ran `ok` against the absent-server host; no run anywhere shows a live server. |
| "status is partial-seam, not done — the L4 seam + null backend + PCM pipeline … are implemented and tested" | docs/feature-matrix.toml `platform-aout-cpal` notes | REAL-UNPROVEN | Seam implemented (this file + audio.rs), tested only on absent-library/absent-server paths; the sink-enumeration happy path has no execution evidence. |
| "bounded waits (D-2), one short-lived connection per enumeration and one persistent watch per subscription" | pulse.rs module docs 1-4 | REAL | Constants 77-81 + loop guards 456-462/540-546/700-706; watch persists per subscribe (725-730). "One short-lived connection per enumeration" is bypassed when a watch exists (enumeration reuses it — an optimization the doc's phrasing doesn't cover, not a defect). |

## 7. Dead / duplicated / unreachable code
The entire crate is unreachable from shipped entry points (F-005). Within the file: no dead units; no `unreachable!`; no duplication spotted (the three wait-loops are structurally similar but differ in cleanup semantics — which is exactly what F-001..F-003 show they got wrong).

## 8. Dependencies & flags
- `cfg(target_os = "linux")` gates the module (linux/mod.rs:10-19) — on other OSes the seam falls back to `UnavailableAudioDevices` (lib.rs `audio_seam`). Compiled and tested on Linux in baseline.
- Crate is std-only by design (C5) — no libpulse link-time dep; dlopen via `linux::dl`. Verified: Cargo.toml deps are only `audio-output`, `media-types`.
- `#![allow(unsafe_code)]` module-level exception (line 34) per MP §9 / ADR-005 — the workspace denies unsafe; this is the recorded exception. All unsafe blocks carry SAFETY comments (verified by reading; three of those comments are false on their `?` paths — F-001..F-003).

## 9. Open questions for the next agent
- Who is *intended* to consume `LinuxPlatform::discover()`? If device-discovery (its comment references the PulseAudio verdict), the missing dependency edge is a plan-level gap worth a matrix/docs check.
- If any orchestrator run can stand up a PulseAudio daemon, a live-server run would convert the `[VERIFY]` happy path and probe F-001/F-003's leak paths. Optional, low priority given UNREACHABLE verdict (REQUEST-ORCH-RUN noted in W03-A03.log.md).
- `crates/platform-linux/src/linux/pulse_ffi.rs` (API table + struct layouts) is unaudited by me — its `PaSinkInfo` layout correctness underpins `on_sink_info`'s field reads.
