# AUDIT: apps/cli/src/run.rs#part1 (lines 1-499)
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 499 audited (of 1054; chunk boundary pinned at the natural seam: part1 = module docs + the three mode drivers + graceful exit, through `teardown_step` (498); part2 = 500-1054, the printing/grammar helpers + tests (A07)) / 8 items (7 fns + 1 const) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` binary → apps/cli/src/main.rs:19 `lumen_cli::run::run(&argv, …)` → one_shot/repl_mode/machine_mode. EXECUTED: every baseline smoke transcript drives repl_mode (mp4-smoke-open-session-420.txt, mp4-smoke-open-session.txt, mp4-smoke-open-session-aac.txt, mp4-smoke-open-session-hevc.txt, avi-firstvertical-open-session.txt — all end in teardown trails + EXIT), and 12 `run::tests` ran ok in the no-fail-fast baseline (orchrun-test-workspace-nofailfast.log.txt:2887-2898)) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `Read` over the full 1054 lines (part2 context) + part1 in detail; `Read` apps/cli/src/main.rs (the caller — thin main, run is the whole binary surface); `sed -n` quote verification (108-115, 205-212, 216-230, 286-296); `rg "const AWAIT_PUMPS|fn settle|fn pump_pipeline_until" apps/cli/src/host.rs` (AWAIT_PUMPS=64 at host.rs:103; settle = bounded pump at 288-298 — the no-hang claims); `rg "control_stdio|control-stdio" apps/cli/src/parse.rs` (parse_argv 151-175 accepts flag+command together; tests at 707-708 assert ONLY the alone case); `rg "control-stdio" apps/cli/src/run.rs tests/` (integration wire_mode exists at tests/integration/first-vertical/tests/wire_mode.rs); `rg "wire_mode|wire-session|first-vertical" orchrun log` (first-vertical integration FAILED in baseline: evidence drift at 2493-2501; journey/recovery_matrix/wire_mode panicked "the `lumen` binary is not built at …/target/debug/lumen" at 2529/2572/2592 — the baseline ran --release, the harness looks for a debug binary); `rg "run::tests::" orchrun log` (12/12 ok). |

## 1. One-paragraph verdict
The runner's core is real, bounded, and heavily executed: all three host modes funnel through the single dispatch entry, every settle/pump/teardown path is bounded by constants (AWAIT_PUMPS=64, TEARDOWN_TAIL_PUMPS=8 — no wall clock, no unbounded loop anywhere in this chunk), and the M17 graceful-exit composition (stop → close, re-drive once through a bounded settle window, success-only tail flush) ran in every recorded smoke session. The chunk's real defects are at the process boundary: (1) `repl_mode` answers 0 unconditionally after the command loop — a session that failed to play (the 4:4:4/HEVC MP4 lanes' `decode.unsupported` → `Error`) exits as SUCCESS because the fatal rides the event stream only and `last_fatal` is never consulted on the quit path (the INDEX's standing defect, anchored here); (2) `--control-stdio` silently swallows an argv command because `run` checks the flag before the command and the parser accepts the combination. The `--json` event half of machine mode (`print_new_events_json`) has no executed evidence anywhere in the baseline — its only driver (the integration wire-mode scenario) panicked on a missing debug binary before reaching it.

## 2. What actually works (verified)
— repl_mode EXECUTED end-to-end in five baseline transcripts: e.g. mp4-smoke-open-session-420.txt (`ok: accepted session=1 gen=0` → events → teardown trail `Playing → Stopped → Idle`, EXIT=0) — the open dispatch, settle, event printing, and the graceful_exit teardown (stop → close → tail flush) all ran in the shipped binary.
— machine_mode EXECUTED (without --json) by unit tests through the real dispatch: `control_stdio_answers_ndjson` and `control_stdio_malformed_request_is_typed` ok (orchrun log 2888-2889) — NDJSON envelope printing, the malformed-request teardown-before-exit path (W2-17), exit 64.
— one_shot EXECUTED: `one_shot_engine_info_and_commands`, `one_shot_play_without_session_is_not_ready` (exit 69, `[session.not-ready]`), `json_one_shot_prints_the_envelope` ok (log 2891-2894) — plus `lumen engine-info` in the wild (rc-smoke-engine-only.log.txt:13-19).
— Boundedness verified by read: settle = `pump_pipeline_until(…, AWAIT_PUMPS=64)` (host.rs:288-298, host.rs:103); graceful_exit worst case `2 × (AWAIT_PUMPS + AWAIT_PUMPS) + TEARDOWN_TAIL_PUMPS` pipeline-only pumps (435-436) — the "exit cannot hang" claim holds structurally; every await rides the same bound.
— The M17 teardown composes only wire-legal rows (stop from Playing/Paused, close otherwise) through `lumen_dispatch` — no host-internal engine bypass (verified: teardown_step is the only exit-path dispatch, 486-498).

## 3. Findings

### F-001 [S2] `repl_mode` answers 0 unconditionally — a session that failed to play (4:4:4/HEVC MP4) exits as success; the fatal is never consulted on the quit path
Where: apps/cli/src/run.rs:288-295
Code:
```
    // M17: the graceful exit — `quit`/EOF must not abandon a live
    // session (doc 10 §6 scenario 16: "exit during playback leaves no
    // temp state"). See [`graceful_exit`].
    graceful_exit(&mut host, err);
    print_new_events(&host, printed, out);
    0
```
What is wrong: the REPL's exit is hard-coded `0` regardless of how the session ended. When the media fails to decode (the 4:4:4 H.264 lane: open is ACCEPTED, then `event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)` → `Buffering → Error → Idle`, mp4-smoke-open-session.txt:8-15; same for HEVC, mp4-smoke-open-session-hevc.txt), the failure surfaces only as an event line — `host.last_fatal` (set by the host's fatal/event drains, host.rs:659/839) is consumed ONLY by the open path (host.rs:998) and never on the quit path, and repl_mode never asks. Scripts (`lumen open broken.mp4 </dev/null`) get EXIT=0 for a file that never played a frame. This is the INDEX's registered standing defect ("The refused paths exit 0 … a failed open is reported as success by exit code" — INDEX.md:139-141); the decision point is this line.
Why it matters: exit code is the only machine-readable verdict a headless consumer has; the CLI's own runner contract (run's doc, 60-65: "0 success, the exit table otherwise") is violated for the most common real failure — the owner's exact MP4 case. Not S0: the open path itself works and the error IS printed visibly; the state machine and teardown are correct.
What correct looks like: on quit/EOF, if the session's lifetime contained a fatal (host.last_fatal or an Error state in the state-history trail), answer `exit::exit_code` of that error (or a dedicated "playback failed" code) instead of `0`; pin it with a test on the yuv444p fixture.
Confidence: HIGH — Verification: read + the defect EXECUTED in two baseline transcripts (mp4-smoke-open-session.txt:15 EXIT=0 after decode.unsupported; mp4-smoke-open-session-hevc.txt EXIT=0) + INDEX standing-defects list.

### F-002 [S3] `--control-stdio` silently swallows an argv command: the flag wins before the command is ever looked at, and the parser accepts the combination
Where: apps/cli/src/run.rs:108-110 (with apps/cli/src/parse.rs:151-175)
Code:
```
    if invocation.control_stdio {
        return machine_mode(input, out, err, invocation.json);
    }
    match invocation.command {
```
What is wrong: `parse_argv` collects `--control-stdio` as a flag AND parses any remaining words as a command (parse.rs:154-173) — it does not reject the combination, and its tests assert only the alone case (parse.rs:707-708: `--control-stdio` with `command.is_none()`). `run` then checks the flag first: `lumen --control-stdio open clip.mp4` silently drops `open clip.mp4`, prints nothing, and blocks reading NDJSON from stdin — to a calling script this is indistinguishable from a hang, and the user's argument is swallowed without a single byte of diagnostics.
Why it matters: a realistic invocation mistake (passing the media on argv in machine mode) produces a mute, blocked process instead of a typed usage error; nothing in the parser, runner, or test suite covers the combination.
What correct looks like: either `parse_argv` rejects `--control-stdio` with a non-empty command (`ParseError`: "machine mode takes no command — requests arrive on stdin"), or the runner prints a warning line before entering machine mode; plus one parse test for the combination.
Confidence: HIGH — Verification: read of both sites + parse tests (707-708) + no coverage found (`rg "control-stdio" apps/cli/src/run.rs` — only json=false unit tests at 1002/1016).

### F-003 [S4] Dead statement: `let _ = interactive;` immediately before the flag is used
Where: apps/cli/src/run.rs:219
Code:
```
    print_new_events(&host, printed, out);
    printed = host.events().len();
    let _ = interactive;
    loop {
        if interactive {
```
What is wrong: a vestigial "explicitly ignore" line left from a refactor (the flag IS read at 221); it suggests the flag is unused when it is not.
Why it matters: pure noise that misleads readers of the pacing code.
What correct looks like: delete the line.
Confidence: HIGH — Verification: read (interactive used at 221).

### F-004 [S4] Response/event print order differs between the open arm and the other-commands arm
Where: apps/cli/src/run.rs:205-206 vs 282-284
Code:
```
    print_response(&response, json, out, err);
    print_new_events(&host, printed, out);
```
vs
```
                print_new_events(&host, printed, out);
                printed = host.events().len();
                print_response(&response, json, out, err);
```
What is wrong: the open prints the receipt BEFORE its settle's events; every other class-A command prints its settle's events BEFORE the receipt. Both orders appear in one stream; the module doc promises "one `event: …` line per newly observed wire event, in causal order" without pinning which side of the `ok:` line events land on.
Why it matters: transcript consumers (the AC/evidence suites) must special-case the first command; the drift the baseline recorded in the integration evidence (committed `open` transcript vs fresh run, orchrun log 2493-2494) shows transcripts are fragile here. UNVERIFIED whether the AC suite intentionally pins both orders (the wire-mode scenario never ran — see §1).
What correct looks like: one order, everywhere (receipt-then-events matches the causal "response lands, effects trail" story of the open path).
Confidence: MEDIUM — Verification: read of both arms; the intent question is UNVERIFIED (AC suite output not run).
### F-005 [S4] stdin read errors are silent EOF in both modes
Where: apps/cli/src/run.rs:226-229 (REPL) and 313-327 (machine mode)
Code:
```
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
```
What is wrong: an I/O ERROR on stdin (not EOF) is conflated with EOF — no diagnostic, exit 0 (after teardown). A machine-mode consumer whose pipe broke loses the distinction between "peer closed cleanly" and "peer died".
Why it matters: limited blast radius (teardown still runs; the process exits cleanly), but the runner's own "every failure is a typed error" posture (main.rs doc) has a hole exactly at its input stream.
What correct looks like: `Err(e) => { graceful_exit(…); typed error line; exit Internal }`.
Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table (production items; the 12 `run::tests` are part2's range — A07)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 66-147 | REAL | — | The whole binary surface; version/help/control-stdio precedence then command dispatch; parse failure → usage exit. Executed by every smoke + 12 unit tests. The control_stdio-before-command precedence is F-002. |
| `one_shot` | 151-171 | REAL | — | Fresh host, single dispatch, exit code from the error table. Executed (log 2891-2894). No session can outlive it (open never routes here). |
| `repl_mode` | 180-296 | REAL | S2 (F-001), S4 (F-003, F-004) | Executed in five transcripts. Open failure → typed exit (208-212, correct); settle-after-open bounded; class-A settle policy per command; Advance host verb prints state+position. The unconditional `0` at 295 is F-001. |
| `machine_mode` | 302-372 | REAL | — | Executed (json=false) by 2 unit tests through the real dispatch; all three exit paths (EOF 314, malformed 334-347, serialization failure 354-365) run graceful_exit first (W2-17) — verified by read, the malformed path by test. The `--json` events arm (367-370) has no executed evidence (see print_new_events_json). |
| `print_new_events_json` | 376-389 | REAL-UNPROVEN | — | Reachable only with `--control-stdio --json`; no unit test passes json=true (run.rs:1002/1016 both plain), and the integration wire_mode scenario that drives it panicked on the missing debug binary in baseline (orchrun log 2583-2593: "the `lumen` binary is not built at …/target/debug/lumen") — never executed in recorded evidence. Serialization-failure fallback prints a JSON error object (never swallowed) — read-verified only. |
| `const TEARDOWN_TAIL_PUMPS` | 395-401 | REAL | — | =8; success-only tail flush; cheap in Idle (each pump a no-op). |
| `graceful_exit` | 446-481 | REAL | — | EXECUTED in every smoke session (the teardown trails in all five transcripts). Two bounded rounds, wire-legal rows only, settle window between rounds, success-only tail flush; a persistent `Buffering` refusal exits with the session live — the registered 3.C1 gap, printed honestly on stderr (never blocks). |
| `teardown_step` | 486-498 | REAL | — | Single dispatch entry, typed failure printed, never blocks the exit. Executed via graceful_exit. |

## 5. Test honesty
Tests live in part2 (889-1053, A07's range); for reachability of MY functions they establish: run/one_shot/machine_mode executed with pinned exit codes (0/64/69) and output substrings — mutation-resistant (each asserts on exact code + content). NOT covered by any executed test: the `--json` machine-mode event stream (print_new_events_json), the REPL mid-session command arms (only the open path is exercised by transcripts), and `--control-stdio` combined with an argv command (F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "0 success, the exit table otherwise" (the honest runner contract) | run's doc, this file 60-65 | FALSE in the failed-play case | mp4-smoke-open-session.txt: EXIT=0 after `decode.unsupported` (F-001); INDEX standing defect. |
| "the exit cannot hang" (graceful exit bounds) | this file 438-440 | REAL | AWAIT_PUMPS=64 (host.rs:103), settle bounded (host.rs:288-298), worst case 2×(64+64)+8 pumps — read-verified; no wall clock. |
| "`quit`/EOF never abandons a live session" | module docs 32-40 | REAL (with a registered gap) | Teardown trail in all five transcripts; the Buffering-refusal case leaves the session live by design (card 3.C1, printed refusal) — honest, bounded. |
| Wave-13 prompt context: "4:4:4 H.264 gets decode.unsupported and the process HANGS forever" | wave-13 AGENT-PROMPT | UNVERIFIED / contradicted for the CLI | Every recorded 4:4:4/HEVC CLI session EXITs 0 with a full teardown trail (quit piped). The only unbounded wait in this chunk is the REPL's `read_line` on stdin — file-independent. No baseline artifact shows a hang; the integration scenarios that might (journey/wire_mode) panicked on a missing debug binary before running (orchrun log 2529/2592). |
| "Output shapes (deterministic, asserted by the AC suite)" | module docs 21-30 | REAL-UNPROVEN | The AC suite's wire scenario never ran in baseline (log 2583-2593); the committed `open` transcript DRIFTED from a fresh run (log 2493-2494, baseline run #4's registered failure) — the assertion machinery exists but its recorded state is red, not green. |

## 7. Dead / duplicated / unreachable code
- `let _ = interactive;` (219) — dead statement (F-003).
- Nothing else; all functions have live callers (run ← main.rs:19; the rest ← run).

## 8. Dependencies & flags
- std only (BufRead/Write) + commands crate types + crate-internal dispatch/exit/help/host/parse/bench. No feature gates, no unsafe, no panics (every failure typed; `let _ =` only on write results — the broken-pipe posture). `lumen-allow-literal` annotations at 222 (the "lumen> " prompt) mirror the pending identity wiring (the lint-literals gate's one failure is host.rs:1799 — A10's file).

## 9. Open questions for the next agent
- A07 (part2 = printing + grammar + the 12 tests): please pin whether the two print orders (F-004) are separately AC-asserted; and `format_us`/`to_catalog`/`parse_setting_value` are yours.
- Integration-test wave: the baseline's first-vertical failures (evidence drift; journey/recovery_matrix/wire_mode "binary is not built at target/debug/lumen" while the baseline built --release) mean the AC evidence for the runner is stale or absent — the drift diff (orchrun log 2495+) should be adjudicated: which side changed?
- Orchestrator: the F-001 fix needs a decision (exit code for failed playback); REQUEST-ORCH-RUN candidate: `printf 'quit\n' | target/release/lumen open audit/runs/baseline/samples/sample.mp4; echo $?` (expected 0 today — the transcript already proves it; a run adds nothing unless the exit policy changes).
