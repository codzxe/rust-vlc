# AUDIT: apps/cli/src/parse.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Lines / functions | 710 / 16 production fns (+3 type defs) + 8 tests |
| File verdict | REAL (grammar is implemented, panic-free, well-tested for what exists; drifts against doc 05 §5.3 and the streaming registry's scheme grammar) |
| Reachable from shipped entry point? | YES: `lumen` main → run.rs:73 `parse_argv` (argv lane), run.rs:234 `parse_command` (REPL lane), run.rs:193/803 `open_command` (both lanes) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `./target/debug/deps/lumen_cli-5f13e10be716a8b0 parse` → 9 passed (8 in-file; exit 0); binary probes: `lumen open --rate` → input.inaccessible "cannot resolve --rate"; `lumen rate 1.5` / `volume 80` → "unknown command"; `lumen open c://x` → unsupported-for-host scheme c; `lumen open 1://x` → invalid-argument uri.scheme.invalid; `lumen decode-bench /nonexistent.avi --json` → proceeds (no parse error); `lumen advance 10` / `quit` → REPL-only refusal; sed/rg verifications |

## 1. One-paragraph verdict
The argv/REPL grammar is real, total for the verbs it knows, arithmetic-safe (f64→int casts are
saturating, i128 accumulation, i64::MAX/2 saturation documented), and its 8 tests are mostly
honest pins. The defects are drift, not fakeness: (1) doc 05 §5.3:506 advertises `--rate 1.5` on
`open` — the parser has no such flag and swallows it as the SOURCE PATH (executed: `lumen open
--rate` tries to open a file named `--rate`); the doc's separate `lumen rate 1.5` verb is
"unknown command" (executed). (2) The URL/path split (`is_url`) is a THIRD scheme grammar in the
same call chain: it accepts any alphanumeric scheme including single letters, so `c://x` is
classified URL→`UnsupportedForHost` (executed) where the streaming registry's `split_scheme`
(schemes.rs:376-378, the drive-letter rule) says local path — the Wave-10 drift, confirmed live;
`1://x` is classified URL by this grammar and then rejected by `SourceUri::parse`
(uri.scheme.invalid, executed). (3) The decode-bench "no flags" invariant is pinned only at the
`parse_command` level; the shipped `parse_argv` lane strips global flags ANYWHERE, so
`decode-bench x.avi --json` sails through (executed) contradicting the test's own comment.

## 2. What actually works (verified)
- `./target/debug/deps/lumen_cli-5f13e10be716a8b0 parse` → "9 passed; 0 failed" (8 in-file tests;
  binary newer than source). Seek table (`90s`/`+10s`/`-1m`/`50%`/`chapter 3`), composite times
  (`1m30s`=90 s, `2h`, `1.5s`, `750us`, `1m30`→None, `-5s`→None), percent bounds (101%/-5% err,
  0%/100% ok), open 4-flag parsing, no-arg verb arity, setting get/set, argv global flags — all
  executed green.
- Binary probes (all executed, outputs above): unknown verbs typed-error with usage; `lumen`
  bare → usage; REPL-only verbs refused in argv mode (`advance`, `quit`); `lumen help` argv
  prints help (run.rs:116 intercepts before to_catalog's REPL-only arm).
- Overflow safety re-derived by read: `parse_duration_us` fast path bounds `us ∈ 0..=9e15` before
  the `as i64` cast; composite path accumulates in i128 via SATURATING f64→i128 casts
  (`1e308h` → i128::MAX → clamped to i64::MAX/2, no panic, no wrap); negatives impossible
  (`parse_f64` filters `v >= 0.0`, and composite numbers go through the same filter).

## 3. Findings
F-001 [S3]
Where: apps/cli/src/parse.rs:268-277 (the `_` arm), :521-531 (`rate: None`); doc source
docs/05-command-layer.md:506
Code:
```
            _ => {
                if source.is_some() {
                    return Err(ParseError::new(
                        "open takes exactly one source path",
```
```
        options: OpenOptions {
            start_time: start_time.map(Timestamp),
            start_paused,
            rate: None,
```
What is wrong: doc 05 §5.3's "Top-30 subcommand mapping (stable)" row 1 advertises
`[--start-time 30s] [--rate 1.5] [--start-paused] [--no-video] [--no-audio]` — FIVE flags; the
parser implements FOUR and hardcodes `rate: None`. An unknown `--flag` token is not rejected as
a flag: it falls into the `_` arm and becomes the SOURCE. EXECUTED: `lumen open --rate` →
"error: [input.inaccessible]: cannot resolve --rate: No such file or directory" (the advertised
flag is eaten as a filename); `lumen open x.mp4 --rate 1.5` → "open takes exactly one source
path" (misleading — blames path count, not the unknown flag). The doc's separate verb rows are
equally absent: `lumen rate 1.5` → "unknown command `rate`" (executed); `volume 80` → "unknown
command `volume`" (executed). W02-A07 already classified the doc rows C64/C65 SHALLOW
(audit/docs/docs/05-command-layer.md.audit.md:100-101); this adds the parser-side consequence:
the doc-advertised token produces a WRONG ERROR CLASS (input.inaccessible / one-source error,
not invalid-argument naming the flag). help.rs:41 lists only the 4 implemented flags — doc 05
and the CLI's own help disagree, and the shipped grammar sides with help.rs.
What correct looks like: either implement `--rate` (parse + `OpenOptions.rate` + engine lane)
or refuse unknown `--`-prefixed tokens in parse_open's `_` arm with "unknown flag `{arg}`"
(and fix doc 05 row 1). Confidence: HIGH — Verification: ran (3 binary probes) + read.

F-002 [S3]
Where: apps/cli/src/parse.rs:504-509; contradicting grammar crates/streaming/src/schemes.rs:366-380
Code:
```
    let is_url = source.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    });
```
What is wrong: this is the third independent scheme grammar on the open path. It requires only
"non-empty, all alphanumeric(+ - .)" — no alphabetic-first rule, no minimum length. The
registry's `split_scheme` (the documented authority: schemes.rs:358-365 pins the
disambiguation "so Phase-6 dispatch and `input` cannot drift") requires ASCII-alphabetic FIRST
char and rejects single-letter prefixes as the drive-letter shape (:376-378). EXECUTED
divergences: `lumen open c://x` → "the c streaming lane is not implemented (blocked v1 …):
c://x, scheme: c" (UnsupportedForHost) — the registry's grammar would call `c://x` a local
path (and a file with that name would open); `lumen open 1://x` → "the URL source is not a
valid URI: uri.scheme.invalid" (parse.rs calls it a URL; `SourceUri::parse` rejects numeric
schemes; the registry calls it a local path). Three grammars, three answers for the same
token. The module doc's headline claim ":5-8 ... never a guess" is contradicted: the URL/path
split IS a guess, and it disagrees with the registry the CLI otherwise defers to (the
file:-routing comment :493-503 correctly avoids re-classifying file: URLs, but silently
re-invents the general scheme grammar instead of consulting the registry's rule).
What correct looks like: reuse one predicate (the registry's rule) for the File/Url spec split,
or at minimum adopt alphabetic-first + min-length-2 here. Confidence: HIGH — Verification: ran
(2 binary probes) + read (schemes.rs quoted; W10-A10's schemes.rs audit F-002 cross-ref).
Note: the W10-A10 audit of schemes.rs called the CLI side the drifted party — this file
confirms it with executions.

F-003 [S4]
Where: apps/cli/src/parse.rs:686-691 (test intent) vs :154-170 (parse_argv) — shipped behavior
Code:
```
        assert!(
            parse_command(&words("decode-bench a.avi --json")).is_err(),
            "no flags (global flags precede the verb)"
        );
```
What is wrong: the test pins the invariant at the `parse_command` layer only. The shipped argv
lane `parse_argv` strips global flags (`--json`, `--control-stdio`, `--version`, `--help`)
ANYWHERE in argv, including after the verb, so `lumen decode-bench a.avi --json` does NOT
error: EXECUTED → "error: [input.inaccessible]: cannot resolve …/nonexistent.avi" (the command
parsed fine; the file open failed), identical to the flag-first form. The comment's documented
rule ("global flags precede the verb") is not enforced by any shipped path; the test would not
fail if parse_argv accepted post-verb flags — because it already does. What correct looks
like: pin the invariant through parse_argv (assert `parse_argv(&["decode-bench",
"a.avi", "--json"])` errors), or delete the comment and document position-independent global
flags. Confidence: HIGH — Verification: ran (binary probe) + read.

F-004 [S4]
Where: apps/cli/src/parse.rs:159-167
Code:
```
                // `lumen --help <area>`: the settings area is the FINAL
                // argv word (`--help open clip.avi` is help, not an
                // area named "open").
                if index + 2 == argv.len() && !argv[index + 1].starts_with('-') {
                    invocation.help = Some(Some(argv[index + 1].clone()));
                }
```
What is wrong: the area is recognized only when it is the FINAL argv word. `lumen --help
engine --json` (a global flag after the area) silently drops the area and prints general
help. The area token is otherwise unconstrained: `lumen --help clip.avi` treats "clip.avi" as
a settings area (a downstream help_for_area error, but a confusing one). Edge blast radius;
no test covers either shape (the test file pins only `--help engine` and `--help open
clip.avi`). Confidence: HIGH — Verification: read (no execution; behavior follows directly
from the index arithmetic).

F-005 [S4]
Where: apps/cli/src/parse.rs:216-223 (untested), :504-518 (untested in this file)
Code:
```
            // VLC-style bare source: `lumen clip.avi` (or any
            // path-looking word) is `lumen open <source>` — a path is
            // not an unknown verb, it is the media.
```
What is wrong: two headline behaviors have ZERO tests in this file: the bare-source fallback
(`lumen clip.avi` → open — the module doc's VLC-style rule) and `open_command`'s URL/path
classification (the `is_url` predicate of F-002). rg over parse.rs tests: no test constructs a
`SourceSpec::Url`, none parses a bare path. CROSS-REF UPDATE (unsupported_path.rs audit, same
wave): the bare-source fallback IS exercised at the binary level (unsupported_path.rs:67/:81/
:89/:114 pass bare fixture paths through the real binary), so that half is guarded
integration-level; the `is_url` classification is guarded only for the three uncontroversial
schemes (rtsp/file/http — unsupported_path.rs:184-254) and the drift shapes (`c://x`,
`1://x`) are unguarded at EVERY level (unsupported_path.rs F-004). Mutation reasoning stands
for the in-file suite: renaming the predicate's charset or dropping `split_once("://")`
would not fail any parse.rs test. Confidence: HIGH — Verification: read + rg (test-body
survey) + binary executions (drift shapes).

F-006 [S4]
Where: apps/cli/src/parse.rs:5-8 (claim) vs :171-173 (behavior)
Code:
```
//! everything else is a typed `invalid-argument` with a usage hint —
//! never a guess, never a partial parse that silently drops tokens.
```
```
    if invocation.help.is_none() && !invocation.version && !command_words.is_empty() {
        invocation.command = Some(parse_command(&command_words)?);
    }
```
What is wrong: `--version` or `--help` anywhere silently DISCARDS the entire parsed command
(`lumen --version open clip.avi` prints the version; the open never happens, no warning).
The convention is pinned only by a test comment ("--help wins over a command (doc convention)",
:705) — no doc line in doc 05 I could cite states it, and the module doc's "never a partial
parse that silently drops tokens" claims the opposite of what the gate does. F-002's
URL guess and F-001's flag-as-filename also contradict "never a guess ... typed
invalid-argument". What correct looks like: state the precedence rule (version > help >
command) in the module doc and doc 05, and soften "never silently drops tokens" to name the
exception. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CliCommand (enum) | 22-76 | REAL | — | 18 verbs; doc-05 surface is larger (F-001) |
| SeekArg (enum) + to_target | 80-106 | REAL | — | pure classify; executed by tests |
| Invocation (struct) | 110-121 | REAL | — | |
| ParseError::new | 133-142 | REAL | — | |
| USAGE | 145 | REAL | — | |
| parse_argv | 151-175 | REAL | S4 | global flags position-independent (F-003); help/version gate (F-006) |
| parse_command | 182-227 | REAL | S4 | bare-source fallback untested (F-005) |
| parse_open | 231-293 | REAL | S3 | unknown flags eaten as source (F-001); 4 flags only |
| parse_seek | 297-310 | REAL | — | chapter two-token; arity pinned |
| parse_advance | 313-323 | REAL | — | default 64; u32 pinned |
| parse_decode_bench | 326-336 | REAL | — | arity pinned; flag drift is parse_argv's (F-003) |
| parse_setting | 339-375 | REAL | — | get/set arity pinned |
| parse_seek_target | 378-406 | REAL | — | percent bounds enforced; NaN excluded by parse_f64 |
| parse_time_us | 413-415 | REAL | — | alias |
| parse_duration_ms | 419-421 | REAL | — | truncating us→ms (documented wire unit) |
| parse_duration_us | 425-468 | REAL | — | saturating, panic-free (re-derived) |
| parse_f64 | 470-478 | REAL | — | filters negative/NaN/`:`/space |
| parse_number | 480-482 | REAL | — | |
| open_command | 486-533 | REAL | S3 | is_url = third scheme grammar (F-002); rate always None (F-001); untested here (F-005) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seek_grammar_covers_the_doc_table | YES | all 5 doc shapes pinned by value |
| time_grammar_composites_and_decimals | YES | incl. refusals (1m30, -5s, "", "s") |
| percent_bounds_are_enforced | YES | 101%/-5% err, 0%/100% ok |
| open_flags_parse | YES (for the 4 flags) | cannot see the missing --rate (F-001) — pins the implemented subset |
| no_arg_verbs_and_unknowns | YES | 13 verbs × arity + unknown + bare-empty |
| setting_grammar | YES | get/set/bogus pinned |
| decode_bench_grammar | PARTIAL | the no-flags assert bypasses the shipped parse_argv lane (F-003) — false pin |
| argv_global_flags_and_help_areas | YES (for what it pins) | --help-area + trailing-flag edge unpinned (F-004) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every token the CLI accepts maps to a catalog command or a documented host verb, and everything else is a typed invalid-argument ... never a guess" | parse.rs:5-8 | PARTLY FALSE | --rate → input.inaccessible (F-001, executed); URL/path split is a guess contradicting the registry (F-002, executed) |
| "never a partial parse that silently drops tokens" | parse.rs:7-8 | FALSE | --version/--help discard the command silently (F-006) |
| "open <path> [--start-time T] [--start-paused] [--no-video] [--no-audio]" (4 flags) | parse.rs:23-24, help.rs:41 | REAL | tests + binary |
| "`lumen open <path|url> [--start-time 30s] [--rate 1.5] ...`" (5 flags) | docs/05-command-layer.md:506 | FALSE vs this parser | F-001 executed; W02-A07 C65 |
| "Top-30 subcommand mapping (stable)" | docs/05-command-layer.md:504-531 | SHALLOW | 18 verbs implemented (parse.rs:196-214); `rate`/`volume` executed → unknown command; W02-A07 C64 |
| "advance [ticks] pumps the engine ... quit/q leaves the REPL" | parse.rs:12-14 | REAL | parse + run.rs REPL loop; argv mode refuses REPL-only verbs (executed) |
| "decode-bench <path> — host verb (argv-only)" | parse.rs:68-71 | REAL | run.rs to_catalog REPL refusal arm; argv lane run.rs:144 |

## 7. Dead / duplicated / unreachable code
- `open_command`'s `rate: None` — the OpenOptions.rate field is declared, never settable from
  the CLI (F-001); catalog `Rate` type exists (types.rs:248) for a lane the grammar cannot reach.
- The scheme grammar duplicated three ways: parse.rs `is_url` (:504-509) vs
  media_types `SourceUri::parse` vs streaming `split_scheme` — F-002.
- No dead fns; all 16 production fns reachable (parse_argv/parse_command → run.rs:73/234;
  helpers called from those; open_command run.rs:193/803).

## 8. Dependencies & flags
- Uses only crates/commands types; no features, no unsafe, no I/O. `lumen-allow-literal`
  annotations on usage strings (identity wiring pending — help.rs [PROPOSAL]). Release lane
  builds (baseline exit 0).

## 9. Open questions for the next agent
- unsupported_path.rs / command_surface.rs owners: does any integration test pin the
  `c://x`/`1://x` classifications (F-002) or the bare-source fallback (F-005)? If yes, cite from
  this file's audit; if no, the grammar drift is unguarded at every level.
- Doc 05 owner: row 1 of the "stable" table needs `--rate` removed or the flag implemented;
  the `(doc convention)` help/version precedence should be written into §5.3 (F-006).
- ts.rs cross-ref (this wave): parse refuses negative `--start-time` (`-5s` → None → typed
  error), so ts.rs:117's `.max(0)` clamp is dead defensive code — confirmed from this side.
