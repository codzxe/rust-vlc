# AUDIT: apps/desktop/src/frame_tap.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2026-11-21 |
| Lines / functions | 233 / 10 items (6 production + 3 tests + 1 Default impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (by construction): `FrameTap::new` ← shell.rs:179-181 `bind_frame_channel` (part1, driven by the `session.set-frame-channel` intercept) ← tauri wrapper `lumen-tauri` app.rs (take_frame_records at app.rs:211). The tap's CLI-side injection point (`CliHost::set_present_tap` → `tap.tap_present(request, self.repeat)?` at apps/cli/src/pipeline_host.rs:754 and :835) is the shipped engine path. Tauri binary build UNVERIFIED in sandbox (baseline #2); the lib lane is proven headlessly: all 3 tests green in BOTH baseline runs (test-workspace.log.txt:2317-2319; nofailfast:2276-2278) and the shell's `frame_channel_proof_pump_once_ships_luf1_records` drives THIS struct end-to-end on a real fixture (baseline :2339/:2298). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read frame_tap.rs whole; rg `tap_present\|present_tap` in apps/cli/src/pipeline_host.rs (call sites 754/835 — `?` propagation + "after a successful present" doc verified); sed bridge.rs:160-190/235-255 (present/present_repeat → ship → serialize+send; typed wire-error mapping); rg `\.stats\(\)` over apps/desktop + apps/desktop-tauri (FrameTap::stats has ZERO callers); rg `Rgba8\|convert\|rgba` in pipeline_host.rs (:1341-1346 — upstream conversion gate: RGBA/BGRA pass, I420 converted, else typed refusal — closes the tap-error-on-foreign-format scenario); rg frame_tap test names in both baseline logs (green ×2); ls apps/desktop/prototype/lumen-frames.mjs (exists — the claimed UI decoder). |

## 1. One-paragraph verdict
The frame tap is a small, correct, correctly-tested adapter: one `Mutex<CanvasBridge<MailboxTransport>>`, a `PresentTap` impl that delegates present/repeat to the bridge, and a drain that re-encodes mailbox frames as complete LUF1 records with a skip-not-ship policy on encoder inconsistency. The dangerous design question — the tap erroring the pump (`tap.tap_present(...)?` propagates into the present path) — is closed by the upstream conversion gate: the CLI host hands the presenter only RGBA/BGRA frames (pipeline_host.rs:1341-1346, I420 converted, everything else refused typed before the tap can see it), which is exactly the bridge wire table's supported set. Poisoned-lock recovery is deliberate and documented ("frames must keep flowing even past a presenter bug"). Both defects are hygiene: the `stats()` accessor has no caller anywhere (while `encode_record`'s doc leans on "the drop is observable via the … stats" — observable through an accessor nobody calls), and one test assert can never fail.

## 2. What actually works (verified)
- All 3 tests green in BOTH baseline runs: `frame_tap::tests::{a_full_mailbox_drops_oldest_never_blocks, repeat_taps_ship_the_repeat_flag, tapped_frames_drain_as_luf1_records}` — test-workspace.log.txt:2317-2319 AND nofailfast:2276-2278.
- End-to-end through the real engine: shell's `frame_channel_proof_pump_once_ships_luf1_records` (shell.rs:677-745, my part2 audit) drives this struct via bind→open→play→pump on the real AVI fixture and asserts LUF1 records flow — baseline green ×2.
- The tap fires only after a successful present, with the repeat bit: `pipeline_host.rs:751-755` (`let info = self.renderer.present(request)?; … if let Some(tap) = self.tap { tap.tap_present(request, self.repeat)?; }`) — exactly the module doc's contract (frame_tap.rs:15-19).
- The tap-error path is unreachable for format reasons: bridge's wire errors (UnknownFormatCode/BadGeometry/PayloadLengthMismatch, bridge.rs:249-255) can only fire for non-RGBA/BGRA frames, and pipeline_host.rs:1341-1346 guarantees the presenter sees only Rgba8/Bgra8 (I420 converted upstream; other formats refused `cli.pixel_format_unconverted` before presenting).
- Latest-wins bound proven: 7 offers into a capacity-2 mailbox all return Ok (the `.must()`s), drain answers 2 — the pump never blocks.
- The repeat flag round-trips through the REAL `WireDecoder` (test 219-232) — not a mock of the wire.
- `lumen-frames.mjs` (the claimed UI-side byte-mirror decoder) exists at apps/desktop/prototype/lumen-frames.mjs (A08's assignment).

## 3. Findings

### F-001 [S4] — `FrameTap::stats()`: public accessor with zero callers, propping up an observability claim
Where: apps/desktop/src/frame_tap.rs:91-97 (accessor); claim at 120-125
Code:
```
    /// The bridge's own accounting (diagnostics; `BridgeStats`).
    pub fn stats(&self) -> crate::bridge::BridgeStats {
        match self.bridge.lock() {
            Ok(guard) => guard.stats(),
            Err(poisoned) => poisoned.into_inner().stats(),
        }
    }
```
What is wrong: rg over apps/desktop and apps/desktop-tauri: nothing calls `FrameTap::stats()` — not the shell, not the tauri wrapper, not any test. Meanwhile `encode_record`'s doc (line 124-125) justifies its silent skip with "the drop is observable via the mailbox/bridge stats" — the only route to those stats from the shipped desktop is this dead accessor.
Why it matters: the skip-drop policy's accountability is nominal; a maintainer debugging missing webview frames has no wired way to see `frames_shipped`/drops. Dead public surface plus a doc claim that depends on it.
What correct looks like: either wire stats into the shell/tauri diagnostics surface (a `frame_stats()` on ShellRuntime), or delete the accessor and weaken the doc to name the real observability path.
Confidence: HIGH — Verification: read + rg (zero call sites).

### F-002 [S4] — An assert that cannot fail, wearing a message that claims the real proof
Where: apps/desktop/src/frame_tap.rs:211-214
Code:
```
        assert_eq!(count, 2, "only the mailbox capacity survives");
        assert!(
            count < 7,
            "the pump was never blocked: all 7 offers returned"
        );
```
What is wrong: `count` is asserted `== 2` one line earlier; `count < 7` is then trivially true — the assert can never fire. The message ("the pump was never blocked: all 7 offers returned") describes the REAL proof, which is the seven `.must()`-wrapped `tap_present` calls in the loop above (a blocked or erroring offer would panic there).
Why it matters: dead assertion; a future reader trusts a guard that guards nothing. (The test as a whole is still sound — the loop's `must()`s carry it.)
What correct looks like: delete the assert, or assert on the loop completing (e.g., a counter of accepted offers == 7 — which `for pts_ns in 100..=106` plus the `must()`s already prove structurally).
Confidence: HIGH — Verification: read (trivially vacuous given the preceding assert).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-29 | REAL | — | tap contract verified at the pipeline call site; "RGBA/BGRA only" table confirmed vs the conversion gate |
| `struct FrameTap` | 45-47 | REAL | — | one Mutex over the bridge; tap thread vs pacer thread serialized |
| `new` | 56-61 | REAL | — | MailboxConfig::default (capacity 2); Err only for structurally invalid config |
| `drain_records` | 72-89 | REAL | — | poison-tolerant lock (documented); exhaustive draw_next loop; count semantics honest (skips not counted) |
| `stats` | 91-97 | REAL | S4 | F-001: zero callers |
| `impl Default` | 100-104 | REAL | — | `unreachable!` on the known-valid default (the repo's stated convention) |
| `impl PresentTap::tap_present` | 106-118 | REAL | — | poison-tolerant; repeat bit routed to present_repeat; `map(|_| ())` discards PresentInfo; error propagation is the documented trait contract ("surfaced by the pump … audio continues") and format-errors are unreachable (conversion gate) |
| `encode_record` | 125-134 | REAL | — | skip-not-ship on encoder inconsistency; with_capacity from trusted internal header; encode_into re-validates |
| test `tapped_frames_drain_as_luf1_records` | 174-195 | REAL | — | 2 taps → 2 records, magic + size, exhaustive drain; would fail if tap/drain broke |
| test `a_full_mailbox_drops_oldest_never_blocks` | 200-215 | REAL | S4 | real proof is the 7 `must()`s; F-002's vacuous assert |
| test `repeat_taps_ship_the_repeat_flag` | 219-232 | REAL | — | round-trips through the real WireDecoder; both flag states asserted |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tapped_frames_drain_as_luf1_records | YES | record count, magic, size, and exhaustive re-drain all asserted |
| a_full_mailbox_drops_oldest_never_blocks | YES (via the `must()`s) | a blocking/erroring offer panics in the loop; capacity breach fails the `== 2`; the second assert itself is vacuous (F-002) |
| repeat_taps_ship_the_repeat_flag | YES | decoded header flags compared both ways through the real decoder |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "called on the pump thread immediately after a successful present … with the scheduler's repeat bit" | frame_tap.rs:15-17 | REAL | pipeline_host.rs:751-755 / 831-836 |
| "must NEVER block the pump — latest-wins absorbs a slow consumer" | frame_tap.rs:17-19 | REAL | mailbox capacity-2 test (7 Ok offers); MailboxTransport semantics (mailbox.rs, A04's audit scope) |
| "the drop is observable via the mailbox/bridge stats" | frame_tap.rs:124-125 | SHALLOW | counters exist (BridgeStats) but the tap's accessor is unwired (F-001) — observability is theoretical in the shipped shell |
| "the UI's decoder is the `lumen-frames.mjs` reference, byte-mirror of `wire.rs`" | frame_tap.rs:25-26 | REAL-UNPROVEN | the file exists (apps/desktop/prototype/); the byte-mirror claim itself is A08's assignment (prototype/lumen-frames.mjs) |
| "Re-encoding … is one memcpy" | frame_tap.rs:68-69 | REAL | encode_record appends header+payload into one record buffer |

## 7. Dead / duplicated / unreachable code
- `stats()` (F-001) — the only dead item; everything else is on the bind→tap→drain lane.

## 8. Dependencies & flags
No feature gates, no unsafe, std-only + workspace crates (`lumen_cli::pipeline_host::PresentTap`, `video_engine`, `crate::{bridge,mailbox,transport,wire}`). The `use std::vec::Vec` (line 32) is a redundant explicit import (edition 2024 prelude) — trivia, not filed.

## 9. Open questions for the next agent
- A04 (mailbox.rs) / A05 (transport.rs) / A06 (bridge.rs): my audit trusts `MailboxTransport::send`'s latest-wins no-block contract and `CanvasBridge::ship`'s error mapping from your files' signatures — the capacity-2 default and `draw_next` oldest-first order are the two facts this file leans on.
- A08 (prototype/lumen-frames.mjs): the "byte-mirror of wire.rs" claim is yours to verify; the tap's records are exactly what that decoder must consume.
- Tauri wave: the ticker "ships the LAST and drops the rest" (shell.rs:214-218) — that drop policy lives on the tauri side; nothing in the headless repo observes it (same unwired-observability family as F-001).
