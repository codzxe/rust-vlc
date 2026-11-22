AUDIT: apps/cli/src/parse.rs
=============================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 710 / 26 (incl. 8 tests) |
| File verdict | REAL (one arithmetic defect, F-001; otherwise the grammar does what its docs claim) |
| Reachable from shipped entry point? | YES — main.rs:14 argv → `run::run` → `parse_argv` (line 151); every REPL line → `parse_command` (line 182) via run.rs:234; `open_command` (line 486) via run.rs:193/803. |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (2 chunks)
- `sed -n '425,468p' apps/cli/src/parse.rs` (F-001 quote verification, passed)
- Cross-read: containers/mp4.rs:107 (`.clamp(0, duration_us)` — the downstream guard that masks F-001); W00 baseline `mp4-smoke2-stdout.txt` shows `usage: lumen [OPTIONS] <COMMAND> [ARGS]` for a bad invocation (exit 64) — this file's USAGE line + exit table in action.

## 1. One-paragraph verdict

The argv/REPL grammar is small, total, and honest: every token maps to a catalog command or a documented host verb, malformed input answers typed errors with usage hints, and the test suite pins the whole doc 05 §5.3 table (`90s`, `+10s`, `-1m`, `50%`, `chapter 3`) with real assertions. The one genuine defect is arithmetic: the unit-summing path of `parse_duration_us` accumulates `f64→i128` terms with no per-term bound, so two huge terms (e.g. `1e308h1e308h`) overflow `i128` — a debug build panics, a release build wraps and can hand the "non-negative" absolute-time contract a negative value. Everything else verified clean.

## 2. What actually works (verified)

— verified by reading tests 543-709: every grammar claim has a failing-capable test — `seek_grammar_covers_the_doc_table` (543), `time_grammar_composites_and_decimals` (570) including refusal cases (`"1m30"`, `"-5s"`, `""`, `"s"`), `percent_bounds_are_enforced` (589), `open_flags_parse` (597), `no_arg_verbs_and_unknowns` (626), `setting_grammar` (654), `decode_bench_grammar` (673), `argv_global_flags_and_help_areas` (695).
— verified by static trace: the bare-source fallback (`lumen clip.avi` → open, lines 215-225) is reachable from argv mode and matches VLC's UX; `--help`-wins-over-command (171) and `--help <area>` final-word detection (164) both pinned by tests at 698-706.

## 3. Findings

### F-001 [S2] — `parse_duration_us` i128 accumulation can overflow: debug panic / release wraps negative
Where: apps/cli/src/parse.rs:437-467
Code:
```rust
    let mut total_us: i128 = 0;
    let mut rest = token;
    while !rest.is_empty() {
        ...
        total_us += (value * mult_us) as i128;
        rest = &tail[unit_len..];
    }
    if total_us > i128::from(i64::MAX / 2) {
        return Some(i64::MAX / 2);
    }
    i64::try_from(total_us).ok()
```
What is wrong: `(value * mult_us) as i128` saturates at `i128::MAX` for out-of-range f64 (Rust float→int cast saturation), but the accumulator `total_us +=` has no overflow guard: a token with two such terms (e.g. `1e308h1e308h` — parse_f64 accepts `1e308`, finite and ≥0) computes `i128::MAX + i128::MAX`. Debug builds panic ("attempt to add with overflow"); release builds wrap to a negative `i128`, which then passes the `> i64::MAX/2` check as false and converts via `i64::try_from` into a **negative** "absolute" time — violating the documented contract at line 408-412 ("Non-negative; … Saturating at `i64::MAX/2`"). The bare-number fast path (429-436) bounds at 9.0e15 µs; the unit path bounds nothing per-term.
Why it matters: `--start-time 1e308h1e308h` is user input reaching arithmetic with a debug-panic path. The W00 smoke used a release build, where the wrapped negative value then depends on downstream clamps (mp4.rs:107 clamps to 0..duration; a host that trusts `start_time_us` unclamped would seek negative). No test covers multi-term overflow (the tests stop at `2h`).
What correct looks like: bound each term like the fast path (`if !term.is_finite() || term > 9.0e15 { return None; }`) or accumulate in saturating i128 (`total_us = total_us.saturating_add(term)`).
Confidence: HIGH for the arithmetic — Verification: read (Rust cast-saturation + release wrapping semantics; runtime behavior UNVERIFIED — no cargo to run a debug build against the token).

### F-002 [S4] — verb-shadowing: a dot-less media filename can never be opened bare
Where: apps/cli/src/parse.rs:215-225
Code:
```rust
            // VLC-style bare source: `lumen clip.avi` (or any
            // path-looking word) is `lumen open <source>` — a path is
            // not an unknown verb, it is the media. No grammar verb
            // contains a path separator or a dot, so the shape is
            // unambiguous.
            if other.contains('/') || other.contains('\\') || other.contains('.') {
                return parse_open(words);
            }
```
What is wrong: a file named `play`, `stop`, `seek`, `q` (no extension) in the cwd is parsed as the verb, never as a source. The comment's "unambiguous" is only true for dotted/slashed names.
Why it matters: hygiene-level UX trap, documented in-comment, workable via `lumen open play`.
What correct looks like: stat the word as an existing file before classifying it as a verb.
Confidence: HIGH — Verification: read.

### F-003 [S4] — `parse_time_us` is a zero-value alias
Where: apps/cli/src/parse.rs:413-415
Code:
```rust
fn parse_time_us(token: &str) -> Option<i64> {
    parse_duration_us(token)
}
```
What is wrong: pure pass-through kept "for the doc contract"; adds a layer with no behavior. (Its twin `parse_duration_ms` at 419-421 does a real unit conversion, so the asymmetry is not laziness in the pair — but this half is dead weight.)
Why it matters: naming that implies time-vs-duration distinction the code does not make.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CliCommand` (enum) | 21-76 | REAL | — | data. |
| `SeekArg` (enum) + `to_target` | 79-106 | REAL | — | pure classification; resolution deferred to host (correct split). |
| `Invocation` (struct) | 109-121 | REAL | — | data. |
| `ParseError::new` | 133-142 | REAL | — | crate-visible ctor. |
| `USAGE` const | 145 | REAL | — | duplicated in run.rs:838's refusal text (run.rs F-005). |
| `parse_argv` | 151-175 | REAL | — | global flags interleaved with command words; `--help` area = final argv word (164). |
| `parse_command` | 182-227 | REAL | S4 | verb table + bare-source fallback (F-002). |
| `parse_open` | 231-293 | REAL | — | one-source rule enforced (269-274); flags after path allowed. |
| `parse_seek` | 297-310 | REAL | — | 1-token or `chapter N`. |
| `parse_advance` | 313-323 | REAL | — | default 64 ticks; u32 count. |
| `parse_decode_bench` | 326-336 | REAL | — | exactly one source; flags refused (test 688-691). |
| `parse_setting` | 339-375 | REAL | — | get/set shapes. |
| `parse_seek_target` | 378-406 | REAL | — | percent bounds 0..100 (383); '+'/'-' relative; f32 fraction. |
| `parse_time_us` | 413-415 | REAL | S4 | alias (F-003). |
| `parse_duration_ms` | 419-421 | REAL | — | µs→ms truncation (sub-ms relative precision lost — inherent to the ms wire unit). |
| `parse_duration_us` | 425-468 | REAL | S2 | F-001 overflow. |
| `parse_f64` | 470-478 | REAL | — | rejects `:`/space/negative/non-finite. |
| `parse_number` | 480-482 | REAL | — | generic FromStr. |
| `open_command` | 486-533 | REAL | — | URL-vs-file classification (504-509); `file:///` routed downstream (claim in comment — demux_host's resolver is another agent's file; UNVERIFIED here). |
| tests 539-709 | — | REAL | — | 8 tests, all assert exact parses or exact refusals; none covers multi-term overflow (F-001 gap), none covers `advance 0`. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seek_grammar_covers_the_doc_table | YES | exact µs/ms values asserted. |
| time_grammar_composites_and_decimals | YES | exact values + 4 refusal cases. |
| percent_bounds_are_enforced | YES | 101%/-5% refused, 0/100 accepted. |
| open_flags_parse | YES | flag routing + 2 refusal cases. |
| no_arg_verbs_and_unknowns | YES | loop over 13 verbs × (ok, extra-arg-err). |
| setting_grammar | YES | exact shapes + 2 refusals. |
| decode_bench_grammar | YES | incl. flag refusal. |
| argv_global_flags_and_help_areas | YES | matches!-style structural asserts. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every token the CLI accepts maps to a catalog command or a documented host verb, and everything else is a typed `invalid-argument`" | parse.rs:5-8 | REAL | verb table 196-226 + ParseError paths; W00 mp4-smoke2 exit 64 on bad usage. |
| "Non-negative; the host clamps … Saturating at `i64::MAX/2` µs" | parse.rs:408-412 | FALSE (edge) | F-001: release wrap can yield negative; debug panics before saturating. |
| "No grammar verb contains a path separator or a dot, so the shape is unambiguous" | parse.rs:219-220 | REAL (literally true) / S4 gap | dot-less verb names shadow same-named files (F-002). |

## 7. Dead / duplicated / unreachable code

- `parse_time_us` alias (F-003).
- USAGE duplicated in run.rs refusal strings.

## 8. Dependencies & flags

- Depends only on `commands` crate types; no features, no unsafe, no panics in normal operation (F-001 debug-overflow is the exception).

## 9. Open questions for the next agent

1. host.rs `dispatch_seek` — does the relative/percent resolution re-clamp to [0, duration]? F-001's release-mode negative value flows into `SeekTarget::Absolute` if used via `seek` (absolute grammar also goes through `parse_time_us`).
2. demux_host::open_source — confirm the `file://` URL routing claim at 493-503 (resolver behavior).
