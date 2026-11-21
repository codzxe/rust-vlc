# AUDIT: apps/cli/src/parse.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 710 / 30 (21 code items + 9 test items) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `main.rs:19 lumen_cli::run::run` → `parse::parse_argv` (run.rs:73) for argv mode; REPL lines → `parse::parse_command` (run.rs:234); `parse::open_command` (run.rs:193 argv open lane, run.rs:803 `to_catalog`); `SeekArg::to_target` (run.rs:816). This file IS the argv front door of the shipped `lumen` binary.) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 1 |
| Commands run | `rg -n "parse_argv|parse_command|open_command|parse::" apps/cli` → all callers live in run.rs (73/193/234/803/816); `rg -n "to_target|SeekArg" apps/cli/src`; `sed -n` verbatim verification of every quoted range (151-175, 215-226, 437-468, 686-692); `rg -n "subtitle-add|audio-track|…|plugins" apps/cli` → zero hits (doc-05 verbs absent from the whole app); `rg` over `audit/runs/baseline/test-workspace.log.txt` → **no lumen_cli test binary ran there** (cargo aborted at desktop zero_alloc_transport); parse tests verified passing in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2873-2880`; `sed -n '498,530p' docs/05-command-layer.md` (CLI table). |

## 1. One-paragraph verdict
This is the real argv/REPL grammar of the shipped `lumen` binary and the first hop of the litmus path (`lumen open x.mp4` → `CliCommand::Open` → `open_command` → `SessionOpen`): pure, allocation-light, typed errors throughout, no panics on any input I traced except one — the duration accumulator `parse_duration_us` can overflow `i128` on a crafted `num+unit` run (`seek 9e300h9e300h`), panicking debug builds and emitting a **negative** µs value in release, violating the function's own "Non-negative" contract. The bigger honesty problem is outward-facing: doc 05 §5.3's "Top-30 subcommand mapping (**stable**)" documents ~18 verbs (`rate`, `step`, `volume`, `mute`, `audio-track`, `video-track`, `subtitle-track`, `subtitle-add`, `chapters`/`chapter`, `devices`/`audio-device`, `screenshot`, `enqueue`, `next`/`prev`, `playlist save/load`, `shuffle`/`repeat`, `plugins`) and the `--rate` open flag that this grammar refuses as `unknown command` — none exist anywhere in apps/cli. Smaller defects: the `--help <area>` detector silently drops non-final area tokens (contradicting the module's own "never a partial parse that silently drops tokens"), the riskiest heuristic in the file (VLC-style bare-source fallback) has zero test coverage, and a test comment asserts a flag-ordering rule that does not exist at the real `parse_argv` layer.

## 2. What actually works (verified)
- All 8 parse unit tests pass: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2873-2880` (`parse::tests::{argv_global_flags_and_help_areas, decode_bench_grammar, no_arg_verbs_and_unknowns, open_flags_parse, percent_bounds_are_enforced, seek_grammar_covers_the_doc_table, setting_grammar, time_grammar_composites_and_decimals} ... ok`). NOTE: the primary baseline `test-workspace.log.txt` contains **no** lumen_cli run at all (cargo stopped at the desktop zero_alloc failure); only the orchestrator's no-fail-fast rerun exercises this file.
- The open lane through this file is the proven 4:2:0 MP4 play path: `audit/runs/baseline/INDEX.md` corrected smoke matrix (`sample-420.mp4 … PLAYS, exit 0`) — that invocation starts as `parse_argv → CliCommand::Open → open_command → SessionOpen`.
- Reachability verified by grep (call chains in header table) — every public item (`parse_argv`, `parse_command`, `open_command`, `to_target`, `USAGE`, `ParseError`) has a live caller in run.rs.

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/parse.rs:437-467
Code:
```
        total_us += (value * mult_us) as i128;
        rest = &tail[unit_len..];
    }
    if total_us > i128::from(i64::MAX / 2) {
        return Some(i64::MAX / 2);
    }
    i64::try_from(total_us).ok()
```
What is wrong: The unit-summing loop has no per-term bound (unlike the bare-number fast path at 430-436, which enforces `0.0..=9.0e15`). `(value * mult_us) as i128` is a saturating f64→i128 cast, so one term like `9e300h` saturates to `i128::MAX`; a second term makes `total_us +=` overflow. In debug builds that add panics (`attempt to add with overflow`); in release it wraps — `i128::MAX * 2 ≡ -2 (mod 2^128)` — and the final guard only rejects *positive* over-max values, so `i64::try_from(-2)` succeeds.
Why it matters: `lumen seek 9e300h9e300h` (or `open x --start-time 9e300h9e300h`) yields `SeekArg::Absolute { us: -2 }` — a **negative** absolute time from a grammar whose own doc (parse.rs:408-412) says "Non-negative; the host clamps against the duration". Downstream clamping (doc 05 §5.3 `playback.seek` row: "clamped to `[0, duration]`") likely turns it into a seek to 0, but the parser contract is broken and any consumer that trusts non-negativity (e.g. u64 casts, watermark comparisons) gets garbage. Debug builds panic on user input.
What correct looks like: Bound each term like the fast path (reject `value * mult_us > 9.0e15`), or accumulate with `checked_add` and return `None` (typed `bad time`) on overflow; assert `total_us >= 0` before the `try_from`.
Confidence: HIGH — Verification: read + arithmetic trace (Rust saturating float→int casts since 1.45; release wrapping / debug panic on i128 overflow). Not executed (no cargo runs allowed) — REQUEST-ORCH-RUN candidate: `cargo test -p lumen-cli parse` plus a one-off assert that `parse_time_us("9e300h9e300h")` is `None`.

### F-002 [S2]
Where: apps/cli/src/parse.rs:196-226 (match arms) and 519-531 (`rate: None`), vs docs/05-command-layer.md:505-527
Code:
```
    match verb.as_str() {
        "open" => parse_open(args),
        "play" => no_args(CliCommand::Play),
        "pause" => no_args(CliCommand::Pause),
        "toggle" | "toggle-play-pause" => no_args(CliCommand::Toggle),
```
```
        options: OpenOptions {
            start_time: start_time.map(Timestamp),
            start_paused,
            rate: None,
```
What is wrong: Doc 05 §5.3's CLI table is introduced as "Top-30 subcommand mapping (**stable**)" and documents: `--rate 1.5` (open flag), `rate`, `step`/`step back`, `volume`/`mute`/`unmute`, `audio-track`/`video-track`/`subtitle-track`/`subtitle-add`, `chapters`/`chapter 2`, `devices`/`audio-device`, `screenshot`, `enqueue`, `next`/`prev`, `playlist save/load`, `shuffle`/`repeat`, `plugins`. The grammar implements **none** of them: `rg "subtitle-add|audio-track|video-track|chapters|rate|step|volume|mute|devices|screenshot|enqueue|next|prev|playlist|shuffle|repeat|plugins" apps/cli` returns only the `seek chapter N` sub-form (parse.rs:298,304). `OpenOptions.rate` is hard-wired `None` (line 524), so `--rate` is unreachable from the CLI even though `OpenOptions` carries the field.
Why it matters: The doc's "stable" CLI surface is ~60% fiction in the shipped binary: a user following doc 05 types `lumen volume 80` and gets `unknown command 'volume'` + exit InvalidArgument. This is exactly the doc-vs-code drift class the audit exists to catch (checklist: "docs disagreeing with workflow", "naming that lies").
What correct looks like: Either implement the documented verbs/flag or cut them from doc 05 §5.3's table and mark the table as the target surface with per-row status; `--rate` should parse into `OpenOptions.rate` or be removed from the doc row.
Confidence: HIGH — Verification: read (grep over apps/cli shows zero implementations; only the two `chapter` sub-form lines exist) + doc table quoted verbatim. The per-verb FALSE verdicts belong in the doc-05 claim ledger (flagged for the docs wave).

### F-003 [S3]
Where: apps/cli/src/parse.rs:159-166
Code:
```
            "-h" | "--help" => {
                invocation.help = Some(None);
                // `lumen --help <area>`: the settings area is the FINAL
                // argv word (`--help open clip.avi` is help, not an
                // area named "open").
                if index + 2 == argv.len() && !argv[index + 1].starts_with('-') {
                    invocation.help = Some(Some(argv[index + 1].clone()));
                }
            }
```
What is wrong: Area detection only fires when `--help` is the second-to-last argv word. `lumen --help engine extra` (or `--help a b`) sets `help = Some(None)`; the trailing words land in `command_words` and are then discarded by the suppression at line 171 — the user asked for help on `engine` and gets the top-level help with both tokens silently dropped. The module doc (lines 5-8) promises "never a partial parse that silently drops tokens".
Why it matters: Contract self-violation in the file's own stated invariant; a help-seeking user gets the wrong output with no error.
What correct looks like: When `--help` is present and extra non-flag words exist that don't form a legal `--help <area>` shape, either take the FIRST word after `--help` as the area or emit a typed `invalid-argument` ("did you mean `--help <area>` alone?").
Confidence: HIGH — Verification: read; traced argv `["--help","engine","extra"]` through lines 154-171 by hand.

### F-004 [S3]
Where: apps/cli/src/parse.rs:215-225
Code:
```
            // VLC-style bare source: `lumen clip.avi` (or any
            // path-looking word) is `lumen open <source>` — a path is
            // not an unknown verb, it is the media. No grammar verb
            // contains a path separator or a dot, so the shape is
            // unambiguous.
            if other.contains('/') || other.contains('\\') || other.contains('.') {
                return parse_open(words);
            }
```
What is wrong: (a) The fallback — the only place the CLI guesses intent — has **zero test coverage**: no test in the file (or in apps/cli/tests, per grep) parses a bare `clip.avi` command; `no_arg_verbs_and_unknowns` only asserts `bogus` errors, which happens to lack `.`, `/`, `\`. (b) The heuristic silently reclassifies any dotted unknown word (`lumen mistyped.verb`) as a media open, in tension with the module doc's "never a guess" (line 7).
Why it matters: The riskiest, most surprise-prone line in the file is the only grammar branch with no mutation protection — a regression that drops the fallback or over-matches (e.g. also matching `-`-prefixed words) would pass the entire test suite. The comment's "unambiguous" claim is unverifiable-by-tests today.
What correct looks like: Add tests: `parse_command(&words("clip.avi")) == Ok(Open{source:"clip.avi",..})`, bare URL form, `./x.mp4`, and the negative `bogus` case; also a comment-level acknowledgment that the guess is a documented VLC-parity tradeoff.
Confidence: HIGH — Verification: read + grep (no test references a bare path command word).
### F-005 [S3]
Where: apps/cli/src/parse.rs:688-691 (test) vs 154-170 (parser)
Code:
```
        assert!(
            parse_command(&words("decode-bench a.avi --json")).is_err(),
            "no flags (global flags precede the verb)"
        );
```
What is wrong: The comment asserts a flag-position rule ("global flags precede the verb") that does not exist at the real entry point: `parse_argv` (lines 154-170) matches `--json`/`--control-stdio`/`--version`/`--help` at **any** argv index and strips them from `command_words`. `lumen decode-bench a.avi --json` therefore parses successfully (json=true + DecodeBench) — the test proves the rule only for the inner `parse_command` API, never for the composed argv layer the user actually invokes.
Why it matters: The test's stated rationale is false at the shipped surface; a reader concludes flag position is enforced when it is not. (Blast radius is small — the parse succeeds either way — but the test is evidence of a rule that isn't real.)
What correct looks like: Either enforce position in `parse_argv` (flags after the verb → typed error) and add an argv-level test, or fix the comment to "decode-bench itself takes no flags; global flags are extracted anywhere by parse_argv" and add an argv-level test pinning the actual behavior.
Confidence: HIGH — Verification: read; integration tests (command_surface.rs, session_script.rs) always pass flags before the verb, so nothing at the binary level pins this either.

### F-006 [S4]
Where: apps/cli/src/parse.rs:145 (and 272, 283, 301, 330, 343, 372, 379)
Code:
```
pub const USAGE: &str = "usage: lumen [OPTIONS] <COMMAND> [ARGS] — try `lumen --help`"; // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```
What is wrong: Seven hard-coded `lumen` identity literals annotated `lumen-allow-literal … identity wiring pending`. The same debt class fails the repo's own gates at the one un-annotated site (baseline INDEX #13/#14: `cargo xtask lint-literals` / `check-identity` FAIL at `apps/cli/src/host.rs:1799`).
Why it matters: Product-identity wiring is declared pending repo-wide; these markers are honest about it, but the shipped strings cannot be rebranded without code edits, and the pending [PROPOSAL] has no deadline.
What correct looks like: Resolve help.rs's identity proposal, route argv0 through one constant, delete the allow markers.
Confidence: HIGH — Verification: read + baseline gate failures cited (audit/runs/baseline/INDEX.md rows 13-14).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum CliCommand` | 22-76 | REAL | — | 19-variant data type; docs map each to catalog/host verb accurately (decode-bench argv-only enforced in run.rs:836, not here) |
| `enum SeekArg` | 79-89 | REAL | — | data; doc comment matches variants |
| `SeekArg::to_target` | 95-106 | REAL | — | pure map; reachable run.rs:816; `#[must_use]` |
| `struct Invocation` | 109-121 | REAL | — | Default derive; plain data |
| `struct ParseError` | 110-131 (124-131) | REAL | — | plain data |
| `ParseError::new` | 136-141 | REAL | — | crate-visible constructor; trivial |
| `USAGE` | 145 | REAL | S4 | F-006 identity literal |
| `parse_argv` | 151-175 | REAL | S3 | F-003 help-area drop; global flags stripped at any position (F-005 context); `--version`+`--help` both set → run.rs resolves order (version first, run.rs:83) |
| `parse_command` | 182-227 | REAL | S3 | F-004 bare-source fallback (untested); closure `no_args` correct; verb set = 18 words + aliases |
| `parse_open` | 231-293 | REAL | S2 | F-002 (no `--rate`); flags may appear before/after source; `--start-time` missing-value and bad-value errors typed; second-source refusal correct |
| `parse_seek` | 297-310 | REAL | — | arity + two-token `chapter` form; `chapter` alone (len 1) falls to `parse_seek_target("chapter")` → `bad time` error (slightly misleading message, harmless) |
| `parse_advance` | 313-323 | REAL | — | default 64 ticks is a magic constant but documented in CliCommand doc |
| `parse_decode_bench` | 326-336 | REAL | — | exactly-one-source; correct |
| `parse_setting` | 339-375 | REAL | — | get/set arity enforced; typed refusals |
| `parse_seek_target` | 378-406 | REAL | — | percent 0..100 enforced; `+`/`-` handled; `50%` → f32 fraction fine |
| `parse_time_us` | 413-415 | REAL | — | alias to `parse_duration_us` |
| `parse_duration_ms` | 419-421 | REAL | — | `us / 1000` floor-truncates sub-ms (by-design wire unit, doc 05 §2.4 `deltaMs`) |
| `parse_duration_us` | 425-468 | REAL | S2 | F-001 i128 overflow → debug panic / release negative; fast path correctly bounded 0..=9e15 µs |
| `parse_f64` | 470-478 | REAL | — | rejects empty/`:`/space, non-finite, negative; `inf`/`NaN` filtered |
| `parse_number` | 480-482 | REAL | — | generic FromStr; out-of-range u32 ("advance 5e9") reports "takes a tick count" — message slightly off, harmless |
| `open_command` | 486-533 | REAL | S2 | F-002 (`rate: None` hard-wired); URL-vs-file split on `scheme://` with RFC3986-ish scheme charset is sound; `file://` delegated to demux_host per comment (verified comment-only — resolver is another file) |
| `tests::words` | 539-541 | REAL | — | split_whitespace helper |
| `tests::seek_grammar_covers_the_doc_table` | 543-568 | REAL | — | value-exact asserts; would fail on regression |
| `tests::time_grammar_composites_and_decimals` | 570-586 | REAL | — | covers composites, decimals, `1m30` rejection, negatives refusal, empty, bare unit; no overflow case (F-001 untested) |
| `tests::percent_bounds_are_enforced` | 588-594 | REAL | — | 101%/-5% err, 0/100 ok |
| `tests::open_flags_parse` | 596-623 | REAL | — | flag matrix + arity errors; no `--rate` case (consistent with F-002) |
| `tests::no_arg_verbs_and_unknowns` | 625-651 | REAL | S3 | F-004: no bare-source case; otherwise solid verb×arity matrix |
| `tests::setting_grammar` | 653-670 | REAL | — | get/set happy + arity errors |
| `tests::decode_bench_grammar` | 672-692 | REAL | S3 | F-005 misleading flag-order comment |
| `tests::argv_global_flags_and_help_areas` | 694-709 | REAL | — | `matches!` on shape only (doesn't pin which command) but would catch help/version suppression regressions; no multi-token help-drop case (F-003 untested) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seek_grammar_covers_the_doc_table | YES | exact value equality on all four seek forms + chapter |
| time_grammar_composites_and_decimals | YES | exact µs values incl. `2h`=7.2e9, `750us`=750, plus 5 rejection cases |
| percent_bounds_are_enforced | YES | boundary pair both sides |
| open_flags_parse | YES | destructures all 5 fields, asserts arity errors |
| no_arg_verbs_and_unknowns | PARTLY | solid for the 13 verbs × (0/1 args), but the bare-source fallback (F-004) is untested — deleting lines 221-222 passes this suite |
| setting_grammar | YES | happy paths + two error shapes |
| decode_bench_grammar | PARTLY | arity checks real; the `--json` case asserts an inner-API rule whose comment misdescribes the argv layer (F-005) |
| argv_global_flags_and_help_areas | PARTLY | pins flag/help/version precedence shapes; misses the F-003 multi-token drop and never checks `command`'s variant |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`lumen open <path\|url> [--start-time 30s] [--rate 1.5] [--start-paused] [--no-video] [--no-audio]`" | docs/05-command-layer.md:506 | FALSE (the `--rate` half) | parse.rs:241-267 has no `--rate` arm; `rate: None` at 524; `lumen open x --rate 1.5` → "open takes exactly one source path" error |
| "`lumen rate 1.5` → playback.set-rate" | docs/05-command-layer.md:511 | FALSE | no `"rate"` arm in parse_command (196-226); grep over apps/cli: zero hits |
| "`lumen step` / `lumen step back` → playback.step-frame" | docs/05-command-layer.md:512 | FALSE | no `"step"` arm; grep zero hits |
| "`lumen volume 80` / `+5` → audio.set-volume" | docs/05-command-layer.md:513 | FALSE | no `"volume"` arm; grep zero hits |
| "`lumen mute` / `lumen unmute` → audio.set-mute" | docs/05-command-layer.md:514 | FALSE | no `"mute"`/`"unmute"` arm; grep zero hits |
| "`lumen audio-track 2`/`video-track 1`/`subtitle-track auto`/`subtitle-add <file>`" | docs/05-command-layer.md:516-518 | FALSE | no such arms; grep zero hits |
| "`lumen chapters` / `lumen chapter 2`" | docs/05-command-layer.md:522 | FALSE | only `seek chapter N` sub-form exists (parse.rs:298,304); top-level `chapter`/`chapters` verbs absent |
| "`lumen devices`/`audio-device`/`screenshot`/`enqueue`/`next`/`prev`/`playlist save/load`/`shuffle`/`repeat`/`plugins`" | docs/05-command-layer.md:523-527 | FALSE | no such arms anywhere in apps/cli (grep zero hits) |
| "`lumen seek 90s`/`+10s`/`-1m`/`50%`/`chapter 3` → playback.seek" | docs/05-command-layer.md:510 | REAL | parse.rs:297-310, 378-406; tests passing in orchrun-test-workspace-nofailfast.log.txt:2878 |
| "`lumen setting get/set <area.key> [value]` … key validation from schema" | docs/05-command-layer.md:528 | REAL | parse.rs:339-375 (schema validation is run.rs `split_setting_key`, out of this file's scope) |
| "every token the CLI accepts maps to a catalog command or a documented host verb … never a partial parse that silently drops tokens" | parse.rs module doc, lines 5-8 | FALSE (the "never silently drops" half) | F-003: `--help engine extra` drops both tokens |
| "`advance [ticks]` pumps the engine deterministically … `quit`/`q` … `help`" | parse.rs module doc, lines 10-16 | REAL | run.rs:144+ REPL loop handles them (grep verified) |

## 7. Dead / duplicated / unreachable code
None. Every public item has a live caller in run.rs (grep table in header). `parse_time_us` is a pure alias of `parse_duration_us` (kept for the doc-comment contract; acceptable).

## 8. Dependencies & flags
- Only dependency: `commands::catalog::types` (OpenOptions, SeekTarget, SourceSpec, Timestamp) — used, minimal.
- No feature flags gate any of this file; shipped build compiles it unconditionally (baseline build #1 clean).
- No unsafe, no allocations beyond `Vec<String>`/`String` clones in argv handling (per-token clone in `parse_argv:168` — fine at argv scale).

## 9. Open questions for the next agent
- run.rs/dispatch.rs auditors: confirm what `SeekTarget::Absolute{Timestamp(-2)}` does downstream (clamp-to-0 vs typed out-of-range) to finalize F-001's blast radius.
- Does any consumer ever set `OpenOptions.rate` to non-None (desktop bridge? control-server)? If nothing does, the field is dead weight from the CLI's perspective.
- doc-05 wave agent: the §5.3 table needs per-row verdicts (18 FALSE rows pre-identified above); check whether the "Top-30" wording or a [PROPOSAL] marker softens the "stable" claim.
- REQUEST-ORCH-RUN (non-blocking, for F-001 execution evidence): `cargo test -p lumen-cli --lib parse` plus a temporary unit assert `assert_eq!(parse_time_us("9e300h9e300h"), None)` — expected to FAIL in release (returns `Some(-2)`), demonstrating F-001.
