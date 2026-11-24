# AUDIT: apps/cli/src/run.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 1053 / 21 (+13 tests, 2 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: apps/cli/src/main.rs:19 `lumen_cli::run::run` ← `fn main` of the `lumen` binary (release lane builds `lumen-cli`) → one_shot/repl_mode/machine_mode → `lumen_dispatch` (dispatch.rs) → `CliHost` (host.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `./target/debug/deps/lumen_cli-9887c2f6df9cd333 run::` → 13 passed / 0 failed (exit 0); `./target/debug/deps/session_script-871be5754d608fa7 tears` → 6 passed / 0 failed (exit 0); rg/sed/awk (many). Binaries dated 2026-11-23 18:07, newer than source 17:35. |

## 1. One-paragraph verdict
The three host modes (argv one-shot, REPL, `--control-stdio` NDJSON) are genuinely implemented and genuinely driven: every command goes through `lumen_dispatch`, the M17 graceful-exit teardown (stop→close, double round with a bounded settle window, 8-pump tail flush) is real code with real integration coverage, and I executed both the 13 unit tests and the 6 teardown integration tests — all green. The file contains **no decoder registry at all**: the assignment prompt's "the 'landed:' registry string lives here per Wave 5" is FALSE — that string lives in apps/cli/src/pipeline_host.rs:376-377 (verified by repo-wide rg; run.rs's only registry contact is printing the EngineInfo decoder list at run.rs:636-642, which faithfully displays host.rs's stale 2-entry hard-coded list). Defects found are small: a swallowed stdin read error, a dead statement, a transcript-ordering asymmetry, and a JSON-failure exit-code inconsistency.

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-9887c2f6df9cd333 run::` → `test result: ok. 13 passed; 0 failed` (version/help/usage exits, one-shot engine-info/commands/play-not-ready exit 69, --json envelope, settings get/set, control-stdio NDJSON + malformed-request, format_us, setting grammar, host-verbs REPL-only).
— `./target/debug/deps/session_script-871be5754d608fa7 tears` → `test result: ok. 6 passed; 0 failed` — quit/EOF/malformed teardowns (REPL + machine mode, Playing and Paused variants) all assert the full `Playing → Stopped → Idle` trail and exit 0.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/run.rs:226-229 (repl_mode), 313-327 (machine_mode)
Code:
```
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
```
(the machine_mode twin at 313-314 is `Ok(0) | Err(_) => {` … `return 0;`)
What is wrong: an I/O failure on stdin (EIO, EISDIR via redirect) is indistinguishable from EOF: no diagnostic, teardown runs, process answers 0. A piped producer crashing mid-session is reported as a clean quit. Why it matters: swallowed-error class (checklist 6.2); machine-machine consumers cannot distinguish "client hung up" from "stdin broke". What correct looks like: `Err(e) => { print to err; return exit_code(Internal) }` alongside the `Ok(0)` EOF arm. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: apps/cli/src/run.rs:219
Code:
```
    let _ = interactive;
```
What is wrong: dead statement; `interactive` is genuinely used at line 221 (`if interactive {`). The `let _ =` is a leftover no-op (bool is Copy, nothing is silenced that isn't already used). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: apps/cli/src/run.rs:205-206 vs 282-284
Code:
```
    print_response(&response, json, out, err);
    print_new_events(&host, printed, out);
```
vs (the REPL loop):
```
                print_new_events(&host, printed, out);
                printed = host.events().len();
                print_response(&response, json, out, err);
```
What is wrong: the seed `open` prints response→events; every subsequent loop command prints events→response. Deterministic but asymmetric; transcript consumers pinning "response first" break on command 2+. Cosmetic only (both orders are causal). Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/cli/src/run.rs:512-522 (print_response json arm) + 165-170 (one_shot exit)
Code:
```
            Err(failure) => {
                let _ = writeln!(err, "error: [control.internal] {failure}");
                return;
            }
```
What is wrong: in `one_shot --json`, a response-serialization failure prints to stderr but `one_shot` still answers `Ok(_) => 0` with EMPTY stdout; the machine_mode twin of the same failure exits `Internal`. Practically unreachable (`WireResponse` has no non-serializable shape), but the inconsistency is real. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: apps/cli/src/run.rs:750-752
Code:
```
fn format_us(us: i64) -> String {
    format!("{:.6}s", us as f64 / 1.0e6)
}
```
What is wrong: `as f64` loses exactness above 2^53 µs (≈104 days); the 6th decimal can round for pathological durations. No realistic media hits it. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 66-147 | REAL | — | router; version/help/control-stdio/open/decode-bench/one-shot; tests executed |
| one_shot | 151-171 | REAL | — | dispatch → print → exit code from `error.code` |
| repl_mode | 180-296 | REAL | S3 | seed open + loop; F-001 read swallow; events/response order F-003 |
| machine_mode | 302-372 | REAL | S3 | NDJSON loop; EOF/malformed/serialize failures all teardown-first (W2-17); F-001 twin |
| print_new_events_json | 376-389 | REAL | — | serialization failure surfaced, never swallowed |
| graceful_exit | 446-481 | REAL | — | 2 rounds, settle between, success-only tail flush; 6 integration tests green |
| teardown_step | 486-498 | REAL | — | refusal → stderr, never blocks exit |
| print_response | 506-528 | REAL | S4 | F-004 json-failure exit-code gap |
| print_outcome | 531-550 | REAL | — | Accepted w/ and w/o session |
| print_result | 553-676 | REAL | — | 10 variants + honest Debug fallback (`other =>`) |
| print_track | 679-709 | REAL | — | codec desc/fourcc fallback "unknown" |
| print_new_events | 712-716 | REAL | — | slice `[printed..]` safe: host.rs `events` vec is push-only (host.rs:172,678) |
| print_event | 720-741 | REAL | — | 6 kinds + Debug fallback |
| print_parse_error | 744-747 | REAL | — | message + hint |
| format_us | 750-752 | REAL | S4 | F-005 |
| format_setting_value | 755-763 | REAL | — | |
| format_setting_number | 766-772 | REAL | — | fract/abs guard then `as i64` |
| is_class_a | 776-787 | REAL | — | 7 commands settle after dispatch |
| to_catalog | 795-845 | REAL | — | full grammar map; decode-bench/host-verbs refused typed |
| split_setting_key | 852-860 | REAL | — | split_once('.') |
| parse_setting_value | 864-887 | REAL | — | on/true/off/false, finite f64, `[a, b]` list (byte-slice safe: len≥2 guaranteed) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| version_and_help_exit_zero | YES | asserts exit 0 + transcript substrings through real parse/help |
| unknown_area_is_a_usage_error | YES | exit 64 + "unknown settings area" |
| bare_invocation_is_usage | YES | exit 64 + usage |
| unknown_verb_is_a_usage_error | YES | exit 64 + typed message |
| one_shot_engine_info_and_commands | YES | real dispatch; pins "ok: 65 command(s)" (catalog size pin — brittle but honest) |
| one_shot_play_without_session_is_not_ready | YES | exit 69 + `[session.not-ready]` through real engine |
| json_one_shot_prints_the_envelope | YES | asserts requestId/completed/kind JSON fields |
| settings_get_set_through_the_grammar | YES | real get (exit 0), invalid set (exit 64 `[config.invalid-argument]`), unknown key (exit 64) |
| control_stdio_answers_ndjson | YES | 2 responses incl. `unsupported-for-host` refusal for set-rate |
| control_stdio_malformed_request_is_typed | YES | exit 64 + `[control.invalid-argument]` |
| microsecond_formatting_is_fixed_decimal | YES | pure-fn pins |
| setting_value_grammar | YES | pure-fn pins on all 4 variants |
| host_verbs_are_repl_only | YES | exit 64 + "REPL-only" |
Coverage gap (not a finding grade): no unit test in run.rs exercises `print_event`/`print_track`/`advance` REPL verb — covered instead by apps/cli/tests/session_script.rs and container_conformance.rs (real `run` with fixtures).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 'landed:' registry string lives here (run.rs) per Wave 5" | W13-A07 task prompt | FALSE | rg -n 'landed:' → apps/cli/src/pipeline_host.rs:376 and apps/web/src/host.rs:684 only; run.rs's sole match is the comment "The teardown landed:" (run.rs:473). Registry A lives in pipeline_host.rs. |
| "decode-bench … bypasses the catalog dispatch by design … drives the open path + decoders directly" | run.rs:42-47 module doc | REAL | run.rs:144 `Some(CliCommand::DecodeBench { source }) => bench::run_bench(&source, out, err)` — no WireRequest built |
| "Human mode prints, per command: ok/error … plus one event: … line per newly observed wire event" | run.rs:21-24 | REAL | print_response + print_new_events at 205-206/282-284 (order asymmetry = F-003) |
| engine-info "decoders: mjpeg, pcm_s16le" | baseline mp4-smoke.md; printed by run.rs:636-642 | STALE SOURCE (host.rs) | run.rs is a faithful printer; the 2-entry list is hard-coded in host.rs:1631-1660 (`decoders: vec![ DecoderInfo { name: "mjpeg" …}, DecoderInfo { name: "pcm_s16le" …}]`) while pipeline_host.rs:239-381 wires mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_* |
| "Never panics; every failure is a typed CommandError" | run.rs:60-65 | REAL-UNPROVEN | no unwrap/expect/index-on-parsed-input found (read); `[printed..]` safe because host.rs events vec is push-only; parse_setting_value slice guarded by len≥2. Not exhaustively proven by execution. |

## 7. Dead / duplicated / unreachable code
- `let _ = interactive;` (219) — F-002.
- Everything else is called; no dead arms found (rg each fn name).

## 8. Dependencies & flags
- Uses crate::{bench, dispatch, exit, help, host, parse} + commands (workspace crate) + serde_json. No feature gates in this file; release lane compiles it unconditionally (baseline: release build exit 0).

## 9. Open questions for the next agent
- Registry reconciliation ownership: engine-info's stale 2-entry list is host.rs:1631-1660 (host.rs agent); "landed:" string omits vp9/dnxhd at pipeline_host.rs:376-377 (pipeline_host.rs agent). bench.rs (mine, next) carries registry C.
- Does any doc claim the REPL prints events before responses? If a doc pins one order, F-003 upgrades.
