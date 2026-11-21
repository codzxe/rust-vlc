# AUDIT: apps/cli/src/dispatch.rs (whole file)
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-21 |
| Lines / functions | 358 / 13 items (6 production + 7 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` main → run::run → one_shot/repl_mode/machine_mode → `lumen_dispatch` (run.rs:153/263 and machine-mode sites); executed by every baseline transcript and apps/cli/tests/command_surface.rs through the REAL binary) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -n "TogglePlayPause|toggle"` (CLI wiring + the untested arm, evidence below); read crates/commands/src/catalog/spec.rs:164–193 (SessionClose class S), ids.rs:131–135 (subtitle ids), event.rs arms; read apps/cli/tests/command_surface.rs whole (the engine-info pinning test); `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
This is the real single dispatch entry: 15 served catalog ids routed to
`CliHost` methods, everything else answered by a typed
`unsupported-for-host` refusal with an owner string — the "typed, not
silent" posture is genuinely implemented, and the help↔SERVED drift test is
real. Two substantive problems: the `SystemGetEngineInfo` arm serves the
hard-coded `host::engine_info()` table (host.rs:1631–1659: features and a
two-decoder list, both `active: false`) that under-reports the shipped
registry — the module doc's "exactly what the engine and its wired pipeline
can honor today" is contradicted by the build's own decoder dispatch and the
baseline 4:2:0 MP4 playback, and an AC test PINS the false output
(command_surface.rs:136 asserts "mjpeg, pcm_s16le"); and `--start-paused`
composes as open→settle→pause, i.e. it auto-plays a bounded number of frames
before the pause lands — an in-code admission of a broken flag contract.

## 2. What actually works (verified)
- Dispatch executed: baseline `mp4-smoke-open-session.txt` (`ok: accepted
  session=1 gen=0` = SessionOpen arm; `error [decode.unsupported]` event =
  the engine path this dispatcher routes to) and `command_surface.rs`
  through the spawned binary (exit codes + envelope shapes).
- The served/unserved split verified against the catalog: every
  `Command` variant NOT matched in `route` falls to `unserved` — subtitle
  ids (`subtitle.add-file`/`set-sync`/`set-encoding`/`set-scale`,
  `tracks.set-subtitle-track`; commands/src/catalog/ids.rs:131–135, 242) have
  no arms → the wave's "subtitles are refused at dispatch" is exactly this
  typed refusal (`owner_of` "subtitle." → "the subtitle path is a Phase-5
  card").
- `served_table_matches_the_help_synopsis` (347–357) is a real two-way
  containment check between SERVED and the help synopsis.

## 3. Findings

### F-001 [S2] — `SystemGetEngineInfo` serves a hard-coded self-description that under-reports the shipped build, and the AC test pins the false output
Where: apps/cli/src/dispatch.rs:139–141 (root: apps/cli/src/host.rs:1631–1659)
Code:
```rust
        Command::SystemGetEngineInfo => Ok(CommandOutcome::Completed(CommandResult::EngineInfo {
            info: crate::host::engine_info(),
        })),
```
What is wrong: `host::engine_info()` (host.rs:1631–1659, read verbatim) is a
literal table — `features: [first-vertical:avi-mjpeg-pcm, audio.null-device,
video.software-compositor]`, `decoders: [mjpeg, pcm_s16le]` with both entries
`active: false` — derived from nothing. The shipped build dispatches mjpeg,
pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264
(pipeline_host.rs:372–379's own refusal message lists them) and the baseline
proves H.264 decoding (mp4-smoke-open-session-420.txt: 4:2:0 H.264+AAC MP4
reaches Playing). The module doc's claim that SERVED is "exactly what the
engine and its wired pipeline can honor today" (dispatch.rs:10–13) is thereby
false for the engine-info row: the pipeline honors h264/aac and the
self-description says it does not. Worse, `apps/cli/tests/command_surface.rs:136`
asserts `out.contains("mjpeg, pcm_s16le")` — the AC suite certifies the lie
(calling it "the engine self-description names the first vertical's honest
sink decisions").
Why it matters: every consumer of `engine-info` (scripts, the desktop, the
parity tooling) reads capability data that is wrong about the binary; the
baseline already burned Wave-0 with this ("Wave-0 trusted this output",
INDEX.md standing defects).
What correct looks like: derive `decoders`/`features` from the live registry
(the provider's dispatch table is right there in the same crate) or at
minimum mark the entries `active: true` and complete; fix the pinning test
to assert derived content.
Confidence: HIGH — Verification: read (both files verbatim) + baseline run
(INDEX.md standing defects + mp4-smoke-open-session-420.txt).

### F-002 [S3] — `--start-paused` presents frames before pausing (open → settle → pause composition)
Where: apps/cli/src/dispatch.rs:88–99
Code:
```rust
            if options.start_paused {
                // `--start-paused` composes as open → settle → pause:
                // the engine's `Buffering` has no pause row (the
                // transient states are card 3.C1's), so the honest
                // composition lets the open settle (auto-play — the
                // `Primed` signal) and pauses from `Playing`. A
                // bounded number of frames may present during the
                // settle window; recorded here rather than hidden.
                host.settle();
```
What is wrong: the flag named `start_paused` starts by playing. The settle
runs the engine out of its transient states (Buffering → Playing) — during
which frames decode, present, and audio stages into the ring — and only then
submits Pause. The comment admits it ("A bounded number of frames may
present during the settle window").
Why it matters: a user opening with `--start-paused` (frame-exact capture,
sensitive content, bandwidth) gets unbounded-by-contract playback of the
head of the media. "Bounded" here means "whatever the settle loop pumps",
which for the headless pump is the settle bound, not a frame bound.
What correct looks like: a pause-during-Buffering row in the engine's state
machine (the card 3.C1 the comment names), or park the decode before the
first advance instead of settling into Playing.
Confidence: HIGH — Verification: read (composition traced; the settle bound
lives in host.rs — part of A09/A10's files; the presented-frame window is
admitted in the quote).

### F-003 [S4] — The SERVED↔route-arm correspondence is untested (help↔SERVED alone is checked)
Where: apps/cli/src/dispatch.rs:38–54 vs 79–156; test at 344–357
Code:
```rust
pub const SERVED: &[CommandId] = &[
    CommandId::SessionOpen,
```
What is wrong: the drift test proves SERVED == help synopsis. Nothing proves
route's match arms == SERVED. A regression that drops or breaks an arm
(e.g. a typo'd `SessionCommandKind`) makes the id answer `unsupported-for-host`
while help still lists it as served. Concrete instance: the
`PlaybackTogglePlayPause` arm (118–127) has no CLI-side test executing it —
rg found wiring (parse.rs:200, help.rs:48) but no routing test; the engine's
own scenario host (crates/playback-engine/tests/scenarios_b/host.rs:1850) has
a SEPARATE copy of the arm, so it would not catch a CLI-side regression.
Mutation: invert the toggle's state check — no test in this repo fails.
Why it matters: the module doc's "the two cannot drift" is only half-true;
the served-surface contract has an unguarded edge.
What correct looks like: a test that routes every SERVED id (even
expecting not-ready without a session) and asserts none answers
unsupported-for-host.
Confidence: HIGH — Verification: read + rg (toggle untested claim;
scenarios_b arm is a separate implementation).

### F-004 [S4] — The subtitle-lane refusal (this wave's named behavior) has no test
Where: apps/cli/src/dispatch.rs:197–200
Code:
```rust
    } else if id.starts_with("subtitle.") {
        "the subtitle path is a Phase-5 card"
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
```
What is wrong: the wave context's "subtitles are refused at dispatch" is
implemented here (no subtitle arms in route; owner strings name the cards),
but no test issues a subtitle command and asserts the typed refusal. The
`subtitle.`/`tracks.set-` branches of `owner_of` are dead under the suite —
the one behavior the wave was told to verify is unpinned.
Why it matters: a refactor that starts serving `subtitle.add-file` as a
silent no-op (the fakeness pattern) or drops the owner string passes CI.
What correct looks like: one test: `Command::SubtitleAddFile` →
`UnsupportedForHost`, details include owner "the subtitle path is a Phase-5
card".
Confidence: HIGH — Verification: read + rg (no subtitle-command test in
apps/cli).

### F-005 [S4] — Cross-file class labeling contradiction: `session.close` is class S here, member of `is_class_a` in run.rs
Where: apps/cli/src/dispatch.rs:26–27 vs apps/cli/src/run.rs:776–787
Code:
```rust
//! snapshots + the settings seam — exactly what the engine and its
//! wired pipeline can honor today). [`SERVED`] is that set as data;
```
(doc lines 26–27:)
```rust
//! `session.close` is class S in the catalog and the engine's receipt
//! agrees (`Completed`).
```
What is wrong: the catalog agrees (spec.rs:176 `class: CompletionClass::S`
for SessionClose), and this file documents it — but run.rs's predicate named
`is_class_a` ("Whether the catalog command is class A") lists `Command::SessionClose`
as a member. One of the two names lies about the same command; the settle
behavior is fine, the taxonomy is not.
Why it matters: the class-A/S vocabulary is the completion-truth contract
(doc 05 §1.4); contradictory labels in the two halves of the same surface
invite the next reader to "fix" one side.
What correct looks like: rename run.rs's predicate to `needs_settle` (or
drop SessionClose from it and let close's settle be explicit).
Confidence: HIGH — Verification: read (spec.rs:174–181 verified class S).
Severity S4: naming/taxonomy only — behavior traced consistent.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SERVED` const | 38–54 | REAL | — | 15 ids, synopsis order; cross-checked against help by test 347 |
| `lumen_dispatch` | 62–71 | REAL | — | echoes request_id; session/generation at answering time (doc 05 §1.6); `unwrap_or(Generation(0))` pre-session |
| `route` | 75–157 | REAL | S2/S3 | F-001 (engine-info arm 139–141), F-002 (start-paused 88–99); SessionClose `.inspect` clears host halves only on Ok (correct — a failed close keeps the session); toggle arm 118–127 decides by live state — in Error/Opening states toggle issues Play (edge semantics, noted); seek delegates to `dispatch_seek` (host.rs) |
| `unserved` | 164–174 | REAL | — | typed Control-stage UnsupportedForHost + machine-parseable detail rows |
| `command_id_string` | 180–189 | REAL | — | serde "cmd" tag as the id truth; error-path-only allocation justified in comment; "(unnamed)"/"(unserializable)" fallbacks |
| `owner_of` | 190–210 | REAL | S4 | F-004 (subtitle branches dead in tests); prefix ladder readable; `lumen-allow-literal` on the default arm (annotated identity-literal posture) |
| test `request` | 218–223 | REAL | — | WireRequest builder |
| test `unserved_commands_refuse_typed` | 227–247 | REAL | — | strong: code+stage+details asserted |
| test `snapshots_need_a_session` | 250–258 | REAL | — | NotReady pinned |
| test `settings_seam_validates_through_the_schema` | 262–297 | REAL | — | applies debug, refuses bogus enum — real schema validation path |
| test `seek_without_session_refuses` | 300–313 | REAL | — | NotReady, no crash |
| test `blocked_scheme_url_open_refuses_naming_the_lane` | 316–342 | REAL | — | rtsp URL → UnsupportedForHost naming the lane (test-double URL on .invalid host, allow-literal annotated) |
| test `served_table_matches_the_help_synopsis` | 344–357 | REAL | — | two-way containment; the half of the drift guard that exists (F-003 is the missing half) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed | YES | asserts code/stage/details of the refusal — removing `unserved`'s details or code fails it |
| snapshots_need_a_session | YES | a snapshot answering without a session fails NotReady |
| settings_seam_validates_through_the_schema | YES | both the apply and the typed refusal are asserted |
| seek_without_session_refuses | YES | |
| blocked_scheme_url_open_refuses_naming_the_lane | YES | |
| served_table_matches_the_help_synopsis | YES (for help↔SERVED) / NO (for SERVED↔route arms) | F-003 — a dropped route arm keeps the test green |
| (cross-file) command_surface.rs engine_info_names_the_vertical | NO — it certifies the under-reporting | asserts the hard-coded "mjpeg, pcm_s16le" string (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every host surface … builds a WireRequest and calls lumen_dispatch … no bypasses" | module doc 1–6 | REAL | run.rs one_shot/repl/machine all call lumen_dispatch (sites listed in header); decode-bench is the documented argv-only bypass (run.rs:144 → bench) — the module doc itself discloses it at 42–47 |
| SERVED is "exactly what the engine and its wired pipeline can honor today" | module doc 10–13 | FALSE (for the engine-info row) | F-001: the pipeline honors h264/aac/… ; engine-info reports two decoders, both inactive |
| "Every other catalog id answers the honest typed refusal unsupported-for-host" | module doc 15–17 | REAL | route's `other` arm + test 227; subtitle ids included (ids.rs:131–135) |
| "session.close is class S in the catalog and the engine's receipt agrees" | module doc 26–27 | REAL (and contradicts run.rs's predicate name) | spec.rs:176 class S; F-005 |
| "`--help` synopsis … tested against it, so the two cannot drift" | module doc 13–15 | SHALLOW | true for help↔SERVED only; SERVED↔route arms unguarded (F-003) |

## 7. Dead / duplicated / unreachable code
- `owner_of`'s `subtitle.` and `tracks.set-` branches: reachable from any
  subtitle/track-switch request but never exercised by tests (F-004).
- No duplicated dispatch logic found; the desktop/control-server dispatchers
  are separate files (A02/A06 this wave) — ARCH-5's "no per-host
  re-implementations" holds only per-host-shape, each host still routes
  itself (noted, not a defect: the catalog ids are shared).

## 8. Dependencies & flags
- Consumes: `commands` (envelope, catalog ids, CommandError/Result, Wire
  types), `playback_engine::SessionCommandKind`, `serde_json` (error-path id
  extraction), `crate::host`.
- One `lumen-allow-literal` at 208 (default owner string) — annotated
  identity-literal posture consistent with the baseline lint-literals gate.
- No feature gates; ships in the `lumen` binary.

## 9. Open questions for the next agent
- A09/A10 (host.rs): the root of F-001 (`engine_info()` at 1631–1659) and
  the `settle()` bound that F-002's frame window depends on — both live in
  your files.
- A05 (run.rs part1): machine-mode's handling of an `unsupported-for-host`
  response (baseline control_stdio test shows `"unsupported-for-host"` in
  the envelope — confirm the machine loop continues after it, exit 0).
- Wave lead: F-001's fix changes a pinned AC output (command_surface.rs:136)
  — coordinate the test rewrite with the registry-derived engine-info.
