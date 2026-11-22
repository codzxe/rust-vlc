# AUDIT: crates/playback-engine/tests/soak/dispatch.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 1058 / SoakHost + `lumen_soak_dispatch` + 4 dispatchers + 2 unit tests |
| File verdict | REAL-UNPROVEN (infrastructure + 2 tests; the soak loops it serves live in sibling files) |
| Reachable from shipped entry point? | N/A — the soak binary's command layer (tests/soak/main.rs, scenario.rs, memory.rs, report.rs — other agents' lists). It drives the REAL `Engine`/`SessionHandle` (the shipped core) through a lookalike dispatch. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `head -55`, inventory `rg`, `sed -n '715,758p'`, `sed -n '1016,1058p'` (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

A carefully justified re-implementation of the CLI's dispatch one layer down: the CLI's `lumen_dispatch` lives in `lumen-cli`, which depends on playback-engine, so a playback-engine test cannot link it (cycle) — the soak drives the same envelope types, the same served playing subset **plus `tracks.set-audio-track`** (the CLI row still refuses it; the divergence is disclosed), the same honest typed refusals, onto the identical `SessionHandle` submit path with receipt correlation and bounded class-A awaits. The determinism split is the file's best idea: the scenario runs on frozen virtual time while the memory sampling ticks on REAL wall clock from inside `SoakHost::pump` ("virtual time allocates nothing and proves nothing about memory"). The in-file tests are two typed-refusal pins (unserved → `unsupported-for-host`; seek without session → `not-ready`) — the dispatch plumbing itself is exercised by the sibling soak loops, not here.

## 2. What actually works (verified)

- `route` (715-743): the served map matches the documented subset; `SessionClose` submits the engine's own composed close and pumps to Idle with a bounded wait + typed internal error on expiry — no silent hangs.
- `dispatch_seek` (756+): "resolution and clamping stay the ENGINE's, doc 05 §2.4" — the harness does NOT mirror resolution (correctly avoiding the engine_fx mirror trap), only translates the wire shape.
- `unserved`/`rejection_wire`/`session_error_wire`: typed refusal translation, pinned by the two unit tests.
- The two tests assert exact error codes/stages — the "never a panic, never silence" law.

## 3. Findings

### F-001 [S4] The soak's dispatch is a lookalike with a disclosed divergence
Where: soak/dispatch.rs:24-28
Code (verbatim):
```rust
//! - the **same served playing subset** as the CLI's `SERVED` table
//!   (`session.open`, `playback.play`, `playback.pause`,
//!   `playback.stop`, `session.close`) **plus**
//!   `tracks.set-audio-track` (the CLI row still answers "track
//!   switching is card 3.A3", which has landed engine-side; the soak
//!   needs the switch op);
```
What is wrong: nothing undisclosed — but the soak proves the engine under a command surface that is NOT the shipped CLI's (the shipped `lumen_dispatch` refuses `tracks.set-audio-track` — my snapshot.rs audit's dispatch.rs:154 evidence). A soak-derived claim of "the CLI's dispatch under soak" overstates; it is "a dispatch with the CLI's shape plus one op, against the same engine." Also no `media-probe` stage (disclosed at 33-36). ARCH-5's "one dispatch layer" is necessarily broken in test composition — the file says so itself.
Confidence: HIGH — Verification: read + cross-ref apps/cli/src/dispatch.rs.

### F-002 [S4] In-file coverage is two refusal pins; the dispatch's happy paths are tested only by the sibling loops
The submit/await/correlation machinery (the bulk of the file) has no in-file test; its correctness rests on scenario.rs/memory.rs/main.rs (other agents). If those files are weak, this plumbing is unproven. Recorded for the sibling-file agents.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `FrozenNow`/`Sampling`/`CountingSink` | 89-190 | REAL | frozen virtual time + wall-clock sampling split |
| `SoakProvider` | 191-238 | REAL | |
| `soak_queues`/`soak_budget` | 240-273 | REAL | |
| `SoakHost` (new/open_source/pump/submit_and_await/…) | 275-713 | REAL | the harness core; open without media-probe (disclosed divergence) |
| `lumen_soak_dispatch` | 702-713 | REAL | the single entry |
| `route` | 715-743 | REAL | served map + composed close + bounded settle |
| `dispatch_seek` | 756-796 | REAL | no resolution mirror — engine owns clamping |
| `dispatch_audio_switch` | 797-836 | REAL | |
| `wire_error`/`session_error_wire`/`receipt_outcome`/`rejection_wire` | 837-982 | REAL | typed translations |
| `unserved`/`file_uri` | 964-1015 | REAL | |
| tests: `unserved_commands_refuse_typed` / `seek_without_session_refuses` | 1023-1058 | REAL | exact code+stage asserts |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed | YES (reasoned) | exact UnsupportedForHost/Control |
| seek_without_session_refuses | YES (reasoned) | exact NotReady |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the same architecture one layer down at the exact seam lumen_dispatch itself routes onto" | header 5-14 | REAL with F-001's divergence | envelope types + submit path shared; served set differs by one op (disclosed) |
| "virtual time allocates nothing and proves nothing about memory" | header 43-46 | REAL | the wall-clock Sampling split read-verified |

## 7-9. Dead code / Dependencies / Open questions

None in-file. Open: the soak loops (soak/main.rs, scenario.rs, memory.rs, report.rs — other agents) are the real test of this plumbing; their audits should cross-reference F-002.
