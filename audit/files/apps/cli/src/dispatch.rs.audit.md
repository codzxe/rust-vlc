# AUDIT: apps/cli/src/dispatch.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 359 / 12 (5 production + 7 test fns/tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — this IS the dispatch: `main.rs:19` → `run::run` → `repl_mode`/`one_shot`/`machine_mode` → `lumen_dispatch` (this file). Also the machine-mode transport (`--control-stdio`) feeds it line by line. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:5 |

Commands run:
- `sed -n '62,71p;136,138p' apps/cli/src/dispatch.rs` + `sed -n '368,372p' apps/cli/src/host.rs` (quote verification, exit 0)
- `rg -n "TracksList" apps/cli/tests/command_surface.rs` → no match (the no-session TracksList gap has no test pin)

## 1. One-paragraph verdict

The single dispatch entry is real and honest where it matters: every served arm propagates errors with `?`, the unserved surface answers a typed `unsupported-for-host` with the owning subsystem named (no catch-all silence), and the SERVED table is cross-checked against the `--help` synopsis by a test so the two cannot drift. The hostile question "error swallowing in dispatch" resolves to: NOT here — the one swallow in this wave's files is host.rs:862-864 (`submit` error always reported as `SessionClosed`); this file's arms are clean. The real defect is an honesty inconsistency among the snapshot commands: `tracks.list` with no session answers a successful EMPTY list while every sibling snapshot (`media.get-info`, `get-time`, `get-duration`) refuses `not-ready` — a caller cannot distinguish "no session" from "media with zero tracks". Second tier: a pause failure during `--start-paused` composition discards the successful open outcome; the response envelope fabricates `Generation(0)` pre-session; and user-facing refusal messages leak process jargon ("card 3.A3", "ADR-005", "Phase-4 card").

## 2. What actually works (verified)

- Every served route is a straight propagate-or-forward: no `let _ =`, no `.ok()`, no catch-all arm in `route` (the `other` arm is itself a typed refusal, not silence).
- The unserved-refusal test asserts code + stage + detail rows (`command`, `owner`) — scriptable, not prose-parsing.
- `served_table_matches_the_help_synopsis` (347-357) is a real anti-drift pin in both directions.
- The settings-seam test covers both the valid write and the schema-refused write.
- Not executed (no cargo): assertions read; W00 baseline (1751 pass / 2 fail / 25 ignored) corroborates.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/dispatch.rs:136-138 (vs 133-135; host.rs:370-372)
Code:
```rust
        Command::TracksList => Ok(CommandOutcome::Completed(CommandResult::TrackList {
            tracks: host.track_snapshot(),
        })),
```
What is wrong: `track_snapshot()` returns `self.tracks.clone()` — an empty `Vec` when no session exists (the field is only populated by a session's `TracksChanged` events). Every other snapshot command refuses `not-ready` without a session (`media_info`/`current_time`/`duration` all `?` through host.rs's session-checked getters; the in-file test `snapshots_need_a_session` pins exactly that for `MediaGetInfo`). `tracks.list` answers `Completed(TrackList { tracks: [] })` — success-shaped.
Why it matters: a script (or the desktop bridge) calling `tracks.list` before `session.open` cannot distinguish "no session" from "a media with zero tracks" — the honest-not-ready contract the module's own docs advertise ("later snapshots refuse with not-ready", dispatch.rs:107-108) is violated by this one arm. No test pins it either way (`rg TracksList apps/cli/tests/command_surface.rs` → no match).
What correct looks like: the same not-ready refusal as the sibling snapshots (a session-checked host method), or a documented `tracks: []`-means-no-session convention — but the former matches the file's own stated contract.
Confidence: HIGH — Verification: read + grep (no test pin).

### F-002 [S4]
Where: apps/cli/src/dispatch.rs:88-100
Code:
```rust
            if options.start_paused {
                // `--start-paused` composes as open → settle → pause:
                ...
                host.settle();
                host.submit_and_await(SessionCommandKind::Pause)
                    .and_then(|receipt| host.receipt_outcome(receipt))?;
            }
            Ok(outcome)
```
What is wrong: if the composed `Pause` fails after a successful open, the `?` discards the open's `Accepted` outcome and the whole response becomes the pause error — while the session stays live (the response envelope at line 67 still carries `host.session_id()`). In the REPL the open response was already printed as ok (run.rs:198-212), so the exit path is consistent there, but in `one_shot`/machine mode a caller sees a failed `session.open` with a session id attached — an ambiguous state. The settle-window frame presentation is honestly documented; this failure-composition is not.
Confidence: MEDIUM — Verification: read; the exact one-shot behavior with start_paused unpinned by tests in this file.
Severity: S4 (narrow, compose-only path).

### F-003 [S4]
Where: apps/cli/src/dispatch.rs:68
Code:
```rust
        generation: host.generation().unwrap_or(Generation(0)),
```
What is wrong: pre-session responses fabricate `Generation(0)` — a made-up context value on the wire (same pattern as host.rs:1006). Whether doc 05 §1.6 blesses 0-as-no-generation is unverified here; if it does not, this is a fabricated field.
Confidence: LOW-MEDIUM — Verification: read; doc 05 §1.6 not consulted (docs wave).
Severity: S4.

### F-004 [S4]
Where: apps/cli/src/dispatch.rs:186-188, 190-209
Code:
```rust
        Err(_) => "(unserializable)".to_string(),
```
and
```rust
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
```
What is wrong: (a) the serde failure in `command_id_string` is swallowed into a placeholder string (error-path-only, but the real serialization failure is unreportable); (b) `owner_of`'s refusal messages leak internal work-planning jargon ("card 3.A3", "ADR-005", "Phase-4 card", "Phase-6 card") into user/script-facing errors — the same class as host.rs's AVI-chapter message.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/cli/src/dispatch.rs:212-358 (test module)
Code:
```rust
    #[test]
    fn unserved_commands_refuse_typed() {
```
What is wrong (coverage note, not a code defect): the in-file tests cover refusals (unserved, no-session, blocked scheme), the settings seam, and the help cross-check — but NOT ONE happy-path dispatch of the playing subset (`PlaybackPlay/Pause/Toggle/Stop/Seek` against a live session) exists in this file. Those paths are exercised only by the integration suites (session_script.rs et al., other agents' files). If those suites regress, this file's own tests stay green.
Confidence: HIGH — Verification: read of the full test module.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const `SERVED` | 38-54 | REAL | — | 15 ids; cross-checked vs help by test |
| `lumen_dispatch` | 62-71 | REAL | S4 | correlate + envelope; fabricated Generation(0) (F-003) |
| `route` | 75-157 | REAL | S3 | all arms typed; TracksList gap (F-001); start-paused composition (F-002) |
| `unserved` | 164-174 | REAL | — | typed refusal with detail rows |
| `command_id_string` | 180-189 | REAL | S4 | serde-tag trick (no second table — sound); swallowed Err (F-004a) |
| `owner_of` | 190-210 | REAL | S4 | prefix routing; jargon in messages (F-004b) |
| test `request` | 218-223 | REAL | — | |
| test `unserved_commands_refuse_typed` | 228-247 | REAL | — | code+stage+details asserted |
| test `snapshots_need_a_session` | 251-258 | REAL | — | pins MediaGetInfo only — the gap F-001 sits right next to it |
| test `settings_seam_validates_through_the_schema` | 263-297 | REAL | — | both outcomes asserted (the "applied" is the write-only store — host.rs F-006) |
| test `seek_without_session_refuses` | 301-313 | REAL | — | |
| test `blocked_scheme_url_open_refuses_naming_the_lane` | 317-342 | REAL | — | |
| test `served_table_matches_the_help_synopsis` | 347-357 | REAL | — | bidirectional anti-drift |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed | YES | full error shape asserted |
| snapshots_need_a_session | YES | but covers only ONE of the four snapshot arms — TracksList's divergent behavior (F-001) passes right beside it |
| settings_seam_validates… | YES | both valid and refused writes |
| seek_without_session_refuses | YES | |
| blocked_scheme_url_open… | YES | |
| served_table_matches… | YES | genuine bidirectional set equality |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every other catalog id answers the honest typed refusal `unsupported-for-host`" | module docs 15-18 | REAL | route's `other` arm + unserved() + test |
| "class-S snapshot commands answer Completed with their payload" | module docs 24-26 | REAL with one crack | TracksList completes with an empty payload pre-session (F-001) |
| "later snapshots refuse with not-ready (the close test's honest-not-ready path)" | 105-109 | FALSE for TracksList | F-001 |
| "no per-host re-implementations, no bypasses" (ARCH-5) | module docs 5-6 | REAL (CLI-side) | single dispatch entry; run.rs's three modes all call it (verified in host.rs audit); bench.rs deliberately bypasses (documented, run.rs:42-47) |

## 7. Dead / duplicated / unreachable code

- None found. `command_id_string`'s `Err` arm is believed unreachable (catalog commands are serializable).

## 8. Dependencies & flags

No feature gates. Depends on the `commands` wire types (Command enum, CommandId, WireRequest/Response) and `crate::host`. The Tauri shell's "same single command" claim (module docs 4-6) is cross-file (apps/desktop-tauri, Wave 14).

## 9. Open questions for the next agent

- Does doc 05 §1.6 define the pre-session generation value (F-003)? Wave 19 owns the docs.
- Does the desktop bridge (apps/desktop/src/dispatch.rs) share this SERVED table or re-declare it (the ARCH-5 claim's other half)?
- The `--start-paused` + pause-failure composition (F-002): is there an integration test anywhere pinning the behavior? Check session_script.rs / sync_scenarios.rs (other agents).
