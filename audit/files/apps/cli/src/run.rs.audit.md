AUDIT: apps/cli/src/run.rs
==========================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 1053 / 36 (incl. 14 tests) |
| File verdict | REAL (code does exactly what it says — which is the problem; see F-001) |
| Reachable from shipped entry point? | YES — this IS the entry point: main.rs:19 `lumen_cli::run::run(...)` → `run` (line 66) → `repl_mode`/`machine_mode`/`one_shot`/`bench::run_bench`. |
| Findings | S0:1 S1:0 S2:1 S3:1 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (2 chunks)
- `rg -n "pump_wallclock|pump\(\)|advance\(" apps/cli` → production pump callers are ONLY `run.rs:252` (`host.advance(ticks)`) and `host.settle()` paths; `pump_wallclock` (host.rs:523) has zero production callers
- `rg -n "advance" apps/cli/src/help.rs` → help.rs:88 (one line in the verbs list)
- `sed -n '7,19p;216,231p' apps/cli/src/run.rs` (quote verification, passed)
- W00 baseline cited: `audit/runs/baseline/mp4-smoke.txt` (SMOKE 1 one-shot open → `exit=124`, i.e. the process never exited), `mp4-smoke3-stdout.txt` (`ok: time 0.000000s (0.0%)` while Playing), `advance-test.txt` (MJPEG AVI: 0.000000s until `advance`, then 3.000000s/100%).

## 1. One-paragraph verdict

This is the file that answers the wave's core question. `run.rs` is a competently written three-mode runner, and it openly documents — in its own module header — that the interactive player **cannot play**: "Between commands nothing pumps; the explicit `advance [ticks]` host verb is the run-to-end driver … Continuous wall-time pumping joins with the session-thread card (3.B3)" (lines 14-18). The card never landed: a fully implemented, unit-tested wall-clock pump (`CliHost::pump_wallclock`, host.rs:523-536) exists one file away with **no production caller**. So `lumen open h264+aac.mp4` reaches `Playing` (W00 mp4-smoke3) and then sits at `time 0.000000s (0.0%)` forever — matching the owner's litmus complaint. The MP4 does not fall out at open, demux, or decode; it falls out at **pacing**, for every container, not just MP4. Second defect: the event log grows unboundedly in long sessions.

## 2. What actually works (verified)

— verified by static trace + W00 baseline outputs: argv parsing → help/version routing (lines 73-107), REPL read/dispatch/print loop (220-287), machine NDJSON mode (302-372), graceful teardown (446-481, bounded, no wall clock). W00 `mp4-smoke3-stdout.txt` is a real `--control-stdio` transcript of exactly this code: `ok: accepted session=1 gen=0` … `state-changed Buffering → Playing` … `ok: time 0.000000s (0.0%)`.
— verified by reading tests (889-1053): the in-memory `run_lines` harness drives `run` end-to-end and asserts exit codes (0/64/69) and exact substrings — these fail if routing/printing regresses.

## 3. Findings

### F-001 [S0] — the interactive player never advances time: no wall-clock pump, `advance` is the only time mover
Where: apps/cli/src/run.rs:7-19 (admission), 219 (`let _ = interactive;`), 251-261 (the only full-pump caller)
Code:
```rust
//! nothing pumps; the explicit `advance [ticks]` host verb is the
//! run-to-end driver (deterministic, no wall-clock sleeps anywhere).
//! Continuous wall-time pumping joins with the session-thread card
//! (3.B3). `[PROPOSAL: the settle-after-command policy is this
```
```rust
    let _ = interactive;
    loop {
        if interactive {
            let _ = write!(out, "lumen> "); // lumen-allow-literal: ...
```
```rust
            CliCommand::Advance { ticks } => {
                host.advance(ticks);
```
What is wrong: `repl_mode` reads stdin line-by-line and pumps the engine only (a) when a class-A command is dispatched (`host.settle()`, line 280 — pipeline-only pumps, **no time advance** per host.rs:440-446), and (b) via the explicit `advance` verb (line 252). The `interactive` flag (main.rs:15 `stdin().is_terminal()`) is received and then explicitly discarded (line 219). No thread, no ticker, no `pump_wallclock` call exists anywhere in production code (rg over apps/cli: host.rs:523 is defined; host.rs:1892-1933 are `#[cfg(test)]` only).
Why it matters: this is the shipped user-visible core. `lumen open <any media>` in a terminal shows `Playing` with a frozen clock indefinitely — W00 mp4-smoke3 (`ok: time 0.000000s (0.0%)` after `state-changed Buffering → Playing`) and advance-test.txt (same for the *working* MJPEG AVI vertical until `advance`) prove it for MP4 and AVI alike. The one-shot smoke even timed out (`exit=124`, mp4-smoke.txt) because the REPL blocks on stdin while the session sits frozen. The docs' "player" claim (doc 05 §5.3 controller mode) is aspirational: a controller that must be told `advance 512` to move 1.4 s (each pump drains the whole ring — see pipeline_host F-002) is not a player.
What correct looks like: when `interactive` is true, spawn the session thread (card 3.B3): a loop calling `host.pump_wallclock()` at ~10 ms cadence between command reads, exactly the shape host.rs:503-526 already documents ("the GUI session thread's tick … made real").
Confidence: HIGH — Verification: read + W00 baseline runs (cited above).

### F-002 [S2] — machine mode without `--json` never advances `printed`; event log grows unboundedly in every mode
Where: apps/cli/src/run.rs:367-370 (vs 302-372), and host.rs:171-172 (`events: Vec<WireEvent>`)
Code:
```rust
        if json_events {
            print_new_events_json(&host, printed, out);
            printed = host.events().len();
        }
```
What is wrong: in `--control-stdio` without `--json`, events are never printed and `printed` stays 0, but `CliHost.events` (host.rs:172 `events: Vec<WireEvent>`) is append-only for the process lifetime — nothing in run.rs or host.rs ever truncates it. In `repl_mode` the same Vec grows for the whole session (printing does not prune).
Why it matters: a long-lived machine session (the conformance-suite transport this mode exists for) accumulates every throttled time-tick and state event in memory forever. For a 3 s fixture it is invisible; for the mode's stated purpose (long control sessions) it is an unbounded leak. Minor today, structural.
What correct looks like: drain-to-printed (truncate the Vec at `printed`) after each print, and cap the log for the non-json machine mode.
Confidence: MEDIUM — Verification: read (host.rs:172, 678 confirmed append-only; no truncate found by rg; runtime growth UNVERIFIED).
### F-003 [S3] — stdin read error is silently treated as EOF
Where: apps/cli/src/run.rs:225-229 (REPL) and 313-314 (machine mode)
Code:
```rust
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
```
What is wrong: an I/O error on stdin is conflated with EOF — no stderr line, no distinct exit code. The graceful teardown still runs (good), but the operator cannot distinguish "user quit" from "stdin broke".
Why it matters: in `--control-stdio` the stdin pipe breaking IS a protocol event worth an error line (the mode's own W2-17 hardening prints typed errors for parse failures — line 345 — but not for transport failure).
What correct looks like: `Err(e) => { writeln!(err, "error: [control.io] {e}"); break; }`.
Confidence: HIGH — Verification: read.

### F-004 [S4] — `advance` output shape breaks the `ok:` grammar
Where: apps/cli/src/run.rs:255-260
Code:
```rust
                let _ = writeln!(
                    out,
                    "state: {:?} (position {})",
                    host.state(),
                    format_us(host.position_us())
                );
```
What is wrong: every other command prints `ok: …` / `error: …`; `advance` prints a bare `state: …` line. Undocumented divergence in the output contract (the module docs at 21-30 promise "per command: `ok: …` / `error: …`").
Why it matters: transcript-parsing harnesses must special-case one verb; the docs' own promise is violated by the file that states it.
What correct looks like: `ok: state …` or move the status under the response printer.
Confidence: HIGH — Verification: read.

### F-005 [S4] — `one_shot` swallows the parse error's exit-code distinction for `DecodeBench`
Where: apps/cli/src/run.rs:836-839
Code:
```rust
        CliCommand::DecodeBench { .. } => Err(ParseError::new(
            "`decode-bench` is an argv-only host verb (lumen decode-bench <file>)", // lumen-allow-literal: ...
```
What is wrong: nothing wrong in behavior (REPL `decode-bench` is refused with usage), but the refusal text duplicates the usage string that parse.rs already owns — two sources for one usage line (drift risk only).
Why it matters: hygiene; if the argv grammar changes, this message lies.
What correct looks like: reference `USAGE`/parse.rs's constant.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 66-147 | REAL | S0 (caller of F-001 path) | argv → mode router; version/help/usage/one-shot/repl/bench. No panics; write failures `let _ =` (acceptable: CLI best-effort). |
| `one_shot` | 151-171 | REAL | — | fresh host per command; exit code from response. |
| `repl_mode` | 180-296 | REAL | S0/S3/S4 | THE interactive loop: settle-after-class-A only; `interactive` discarded (219); `advance` only time mover (252); read-error==EOF (227). |
| `machine_mode` | 302-372 | REAL | S2 | NDJSON loop; malformed input → teardown+64 (W2-17); non-json events never pruned (F-002). |
| `print_new_events_json` | 376-389 | REAL | — | serialization failure surfaced, not swallowed. |
| `TEARDOWN_TAIL_PUMPS` | 401 | REAL | — | const 8. |
| `graceful_exit` | 446-481 | REAL | — | bounded 2-round stop→close, no wall clock, refusal never blocks exit. Well documented. |
| `teardown_step` | 486-498 | REAL | — | through the single dispatch entry. |
| `print_response` | 506-528 | REAL | — | json/human split; errors via exit printer. |
| `print_outcome` | 531-550 | REAL | — | Accepted/Completed shapes. |
| `print_result` | 553-676 | REAL | — | class-S subset; catch-all Debug arm documented (669-674). |
| `print_track` | 679-709 | REAL | — | note: prints NO language field even though TrackInfo carries one — the W00 "voe" bug (demux/mp4/mod.rs:535-539) is invisible here by omission, not correctness. |
| `print_new_events` | 712-716 | REAL | — | slice from `printed`. |
| `print_event` | 720-741 | REAL | — | human event vocabulary; `other =>` Debug fallback. |
| `print_parse_error` | 744-747 | REAL | — | message + hint. |
| `format_us` | 750-752 | REAL | — | µs→fixed 6-decimal; f64 exact ≤ 2^53 µs (≈285 y) — fine. |
| `format_setting_value` | 755-763 | REAL | — | wire types → strings. |
| `format_setting_number` | 766-772 | REAL | — | integral bare; guard <9e15. |
| `is_class_a` | 776-787 | REAL | — | the settle-after set. |
| `to_catalog` | 795-845 | REAL | — | CLI verb → catalog payload; host verbs refused. |
| `split_setting_key` | 852-860 | REAL | — | first-dot split. |
| `parse_setting_value` | 864-887 | REAL | — | on/off/number/list/string grammar. |
| tests `run_lines` | 898-908 | REAL | — | in-memory I/O harness. |
| `version_and_help_exit_zero` | 911-924 | REAL | — | real assertions on output+code. |
| `unknown_area_is_a_usage_error` | 927-931 | REAL | — | asserts code 64. |
| `bare_invocation_is_usage` | 934-938 | REAL | — | asserts usage. |
| `unknown_verb_is_a_usage_error` | 941-945 | REAL | — | asserts message. |
| `one_shot_engine_info_and_commands` | 949-963 | REAL | — | asserts engine features + "ok: 65 command(s)". |
| `one_shot_play_without_session_is_not_ready` | 967-971 | REAL | — | asserts exit 69 `[session.not-ready]`. |
| `json_one_shot_prints_the_envelope` | 975-981 | REAL | — | NDJSON envelope keys. |
| `settings_get_set_through_the_grammar` | 985-995 | REAL | — | get/set/invalid paths. |
| `control_stdio_answers_ndjson` | 999-1011 | REAL | — | 2-line NDJSON round trip; `set-rate` → "unsupported-for-host" (host refuses rate verb — honest). |
| `control_stdio_malformed_request_is_typed` | 1015-1019 | REAL | — | code 64. |
| `microsecond_formatting_is_fixed_decimal` | 1023-1027 | REAL | — | exact strings. |
| `setting_value_grammar` | 1031-1044 | REAL | — | all four shapes. |
| `host_verbs_are_repl_only` | 1048-1052 | REAL | — | one-shot `advance` refused. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all run_lines tests above | YES | they drive the real `run` with real argv/stdin and assert exact codes/substrings. |
| control_stdio_answers_ndjson | YES | real serde_json round trip through dispatch. |
| NO test asserts that an interactive session advances time without `advance` | — | the F-001 gap is baked into every scripted test (tests/session_script.rs, sync_scenarios.rs drive `advance` explicitly) — the defect is untestable by construction in this suite. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the interactive REPL (doc 05 §5.3's controller mode)" behaves as a controller | lib.rs:46-48, run.rs:1-3 | SHALLOW | it controls, but playback never progresses without `advance` (F-001; W00 mp4-smoke3 / advance-test.txt). |
| "Between commands nothing pumps … Continuous wall-time pumping joins with the session-thread card (3.B3)" | run.rs:14-18 | REAL (the admission) | rg: no production caller of `pump_wallclock`; card 3.B3 absent from the tree (no `spawn`/thread in apps/cli src). |
| "no wall-clock sleeps anywhere" | run.rs:16 | REAL | rg for `sleep` in apps/cli/src: none in run/host paths (host.rs uses synthetic instants in tests). |

## 7. Dead / duplicated / unreachable code

- `interactive` parameter of `repl_mode` (line 190/219): received from main.rs:15, then `let _ = interactive;` — used only for the prompt; the flag that should arm real-time pacing is the one discarded (F-001).
- Usage strings duplicated between parse.rs `USAGE` and run.rs:838.

## 8. Dependencies & flags

- No feature gates; std + `commands` + `serde_json` (wire) + crate-internal modules. Crate-level deps audited in apps/cli/Cargo.toml.audit.md.

## 9. Open questions for the next agent

1. host.rs `settle()` (288-296) — confirm the settle pump is pipeline-only (no `advance_time`), which is what makes post-open position frozen at 0 even after settle (W00 shows exactly that).
2. Where is card 3.B3 (session thread)? If any other app (desktop-tauri) drives `pump_wallclock`, the CLI-only story stands; rg shows apps/desktop has its own host — the desktop wave should confirm whether IT paces in wall time (if it does, the CLI is the only surface frozen).
3. tests/session_script.rs (my file, next) — verify whether any scripted scenario claims to prove "playback progresses", which would be a false claim under F-001.
