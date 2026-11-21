# AUDIT: apps/cli/src/run.rs#part2 (lines 500–1053)
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-21 |
| Lines / functions | 554 (of 1053) / 29 items (14 production + 15 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` main.rs → `run` → one_shot/repl_mode/machine_mode (part1, lines 151–390) → this chunk's printers at run.rs:166/205/206/217/253/282/284/294 and grammar at 153/263/270; executed by every baseline smoke transcript) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "print_response(|to_catalog(|is_class_a(|print_new_events("` (callers: run.rs part1 sites listed above); `rg -n "fn events|self.events"` in host.rs (append-only verified: push at host.rs:678 only, no clear/truncate → `[printed..]` slice safe); `rg -n "TimingDurationKnown|MediaInfoReady"` in crates/commands/src (event.rs:96/286 — real catalog variants); baseline transcripts rg'd for the Debug-fallback evidence; `sed -n` on all quoted ranges |

**Chunk boundary note (for A05):** **part2 (this audit) = lines 500–1053** (the
"Printing" section divider at 500–502 through end-of-file: printers, the
class-A predicate, the CLI→catalog grammar, `mod tests`); part1 = lines 1–499
(`run`, `one_shot`, `repl_mode`, `machine_mode`, `print_new_events_json`, the
teardown section). Split at the section divider nearest 1053/2 ≈ 526.

## 1. One-paragraph verdict
The deterministic output/grammar half of the runner: response envelopes
(human + NDJSON), event lines, track rows, the settings key/value grammar,
and a genuinely strong in-process test harness (`run_lines` drives the real
`run` with in-memory I/O; the tests assert exit codes, error codes, and
output shapes — they would fail under the mutations I reasoned through).
One S3: `print_event` has human arms for only six of the catalog's event
kinds; `TimingDurationKnown` and `MediaInfoReady` — both emitted by every
`open` — fall through to raw `{other:?}` Debug dumps, proven verbatim in the
baseline transcripts (a 500+ character `MediaInfoReady { info: MediaInfo {
… } }` line). The doc comment above the function claims "the stable event
vocabulary of doc 05 §3.2". The rest is honest, verified, reachable.

## 2. What actually works (verified)
- Output path executed by baseline: `audit/runs/baseline/mp4-smoke-open-session.txt`
  lines 1–2 (`ok: accepted session=1 gen=0`, `event: state-changed Idle →
  Opening`) are `print_outcome`'s 538–542 and `print_event`'s 723–725 shapes
  verbatim; `format_us`'s fixed-decimal form is pinned by test 1023–1027.
- `print_new_events`' `[printed..]` slice cannot panic: `host.events()` is
  append-only (host.rs:678 `self.events.push(wire)` is the only mutation;
  no clear/truncate/drain found by rg).
- The settings grammar round-trips the catalog types (test 1031–1044) and the
  invalid paths answer typed errors with exit 64 (tests 989–994).

## 3. Findings

### F-001 [S3] — `print_event` prints two routinely-emitted event kinds as raw Debug dumps
Where: apps/cli/src/run.rs:718–741
Code:
```rust
/// One wire event's human line (the stable event vocabulary of doc 05
/// §3.2, this host's subset).
fn print_event<W: Write>(event: &WireEvent, out: &mut W) {
    use commands::EventKind;
    let what = match &event.kind {
```
(arms for SessionStateChanged / SessionBuffering / TracksListReady /
PlaybackEndOfMedia / PlaybackError / TimingTimeTick only, then)
```rust
        other => format!("{other:?}"),
    };
    let _ = writeln!(out, "event: {what}");
```
What is wrong: `EventKind` (crates/commands/src/catalog/event.rs:59) carries
`TimingDurationKnown` (:96) and `MediaInfoReady` (:286) as first-class catalog
events, and every `open` emits both — the baseline transcripts show them
printed as Debug dumps, e.g. `audit/runs/baseline/mp4-smoke-open-session.txt:8`
`event: TimingDurationKnown { duration: Timestamp(3000000) }` and :9 a
500+ character `event: MediaInfoReady { info: MediaInfo { … tracks:
[TrackInfo { … language: Some("voe") … }] … } }` single line.
Why it matters: the doc comment promises the doc 05 §3.2 stable vocabulary;
users get derive-Debug output that changes shape whenever any nested struct
gains a field (the committed transcripts pin the Debug shape, making every
vocabulary extension a transcript-rewrite). The garbage `language:
Some("voe")` (baseline standing defect) is also only visible because of this
dump — the human printer hides it (F-003).
What correct looks like: human arms — `duration-known {us}` and
`media-info ready` (container/duration/track-count) — leaving Debug only for
genuinely unserved kinds.
Confidence: HIGH — Verification: read + baseline run (both transcripts cited).

### F-002 [S4] — `print_result`'s Debug catch-all for "unserved" result variants
Where: apps/cli/src/run.rs:669–674
Code:
```rust
        other => {
            // A result variant outside the served subset cannot be
            // produced by this host's dispatch — printed generically
            // rather than panicked on.
            let _ = writeln!(out, "ok: {other:?}");
        }
```
What is wrong: same Debug-fallback pattern for results; the comment's
"cannot be produced by this host's dispatch" is a claim about dispatch.rs
(this wave's next file for me — the served-subset check is recorded there).
Defensive printing is fine; Debug output labeled `ok:` for an unexpected
result masks surprises instead of typing them.
Why it matters: an unexpected result prints as success-shaped Debug noise.
What correct looks like: print to `err` with a `control.internal` style code,
or enumerate the remaining variants.
Confidence: HIGH — Verification: read; dispatch-side subset verified in the
dispatch.rs audit (this wave).
Severity S4 not S3: the arm is dead in the shipped dispatch (verified in
dispatch.rs — see that audit's per-command table).

### F-003 [S4] — `print_track` drops language/profile/level; `SettingResult` can print empty parens
Where: apps/cli/src/run.rs:679–709 and 650–653
Code:
```rust
    let mut line = format!("  #{} {} {}", track.id.0, kind, codec);
```
```rust
        CommandResult::SettingResult { value, schema_meta } => {
            let name = schema_meta.get("flag").cloned().unwrap_or_default();
            let _ = writeln!(out, "ok: {} ({name})", format_setting_value(value));
```
What is wrong: (a) `TrackInfo` carries `language`, `profile`, `level`
(baseline transcript shows the fields), but the human track row prints
neither — honest metadata is invisible and the "voe" garbage default hides
from human users while polluting machine output; (b) when `schema_meta` has
no "flag" key the settings line prints `ok: on ()` — empty parentheses.
Why it matters: output-shape gaps on the two most user-facing tables.
What correct looks like: print language/profile when present; omit the parens
when the flag name is absent.
Confidence: HIGH — Verification: read + baseline transcript field evidence.

### F-004 [S4] — The CLI settings-value grammar cannot express bracketed/comma strings
Where: apps/cli/src/run.rs:875–883
Code:
```rust
            if value.starts_with('[') && value.ends_with(']') {
                let items = value[1..value.len() - 1]
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
```
What is wrong: any value that starts with `[` and ends with `]` is forced
into `List` — a string setting whose value looks like a list (or contains a
comma inside brackets) is unrepresentable from the CLI; no quoting/escape
exists. Documented as "the CLI scalar grammar", so this is a recorded limit,
but the failure mode is silent coercion (a String arrives as a List and the
set either fails typed downstream or sets the wrong type).
Why it matters: silent type coercion on user input.
What correct looks like: attempt the list parse only when the schema declares
a list type (the schema is reachable via `ConfigGetSetting`), or reject the
ambiguous shape typed.
Confidence: HIGH — Verification: read (no schema consultation in the parse).
Note: byte-slicing `value[1..len-1]` is safe here — the guards guarantee the
boundary bytes are ASCII `[`/`]`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `print_response` | 506–528 | REAL | — | NDJSON via serde_json; serialization failure prints `[control.internal]` and lets the caller own the exit code; human path splits outcome/error honestly |
| `print_outcome` | 531–550 | REAL | — | `ok: accepted session=N gen=M` matches baseline transcript verbatim |
| `print_result` | 553–676 | REAL | S4 | F-002 catch-all; MediaInfo/EngineInfo/Settings arms traced against baseline shapes; `position * 100.0` f-format fine |
| `print_track` | 679–709 | REAL | S4 | F-003 (language/profile/level dropped); codec fallback chain description→fourcc→"unknown" |
| `print_new_events` | 712–716 | REAL | — | `[printed..]` safe (events append-only, host.rs:678) |
| `print_event` | 720–741 | REAL | S3 | F-001 (Debug fallback for TimingDurationKnown/MediaInfoReady — baseline-proven) |
| `print_parse_error` | 744–747 | REAL | — | message + hint, two lines |
| `format_us` | 750–752 | REAL | — | f64 µs→s fixed 6-decimal; i64→f64 precision fine for media-scale values; negatives format sanely |
| `format_setting_value` | 755–763 | REAL | — | on/off/number/string/list forms |
| `format_setting_number` | 766–772 | REAL | — | integral <9e15 prints bare; NaN/inf would print via Display (unreachable from the is_finite parse gate) |
| `is_class_a` | 776–787 | REAL | — | six commands (open/close/play/pause/toggle/stop/seek); used by repl/machine to bound settling — the settle loop itself is part1 |
| `to_catalog` | 795–845 | REAL | — | 1:1 mapping; DecodeBench → argv-only typed error; host verbs → REPL-only typed error (both with `lumen-allow-literal` annotations referencing the pending identity wiring — matches the baseline lint-literals gate posture) |
| `split_setting_key` | 852–860 | REAL | — | splits at FIRST dot; empty area/key halves pass through and are rejected typed downstream (test 992–994 shows "unknown setting key") |
| `parse_setting_value` | 864–887 | REAL | S4 | F-004; `is_finite` gate keeps NaN/inf out of Number; safe byte slicing |
| test `words` | 893–895 | REAL | — | splitter |
| test `run_lines` | 898–908 | REAL | — | the real `run` with in-memory I/O — no mocks of the subject |
| test `version_and_help_exit_zero` | 910–924 | REAL | — | would fail if version/help text or exit codes broke; asserts help area keywords |
| test `unknown_area_is_a_usage_error` | 926–931 | REAL | — | exit 64 + message |
| test `bare_invocation_is_usage` | 933–938 | REAL | — | exit 64 + usage |
| test `unknown_verb_is_a_usage_error` | 940–945 | REAL | — | exit 64 + "unknown command `bogus`" |
| test `one_shot_engine_info_and_commands` | 947–963 | REAL | — | see §5: pins hard-coded engine-info features and the "65 command(s)" count |
| test `one_shot_play_without_session_is_not_ready` | 965–971 | REAL | — | exit 69 + `[session.not-ready]` — pins the honest no-session refusal |
| test `json_one_shot_prints_the_envelope` | 973–981 | REAL | — | envelope field presence asserts |
| test `settings_get_set_through_the_grammar` | 983–995 | REAL | — | real settings seam incl. two typed failure paths |
| test `control_stdio_answers_ndjson` | 997–1011 | REAL | — | two-request NDJSON session incl. the `unsupported-for-host` refusal path |
| test `control_stdio_malformed_request_is_typed` | 1013–1019 | REAL | — | `[control.invalid-argument]` + exit 64 |
| test `microsecond_formatting_is_fixed_decimal` | 1021–1027 | REAL | — | direct unit pins on `format_us` |
| test `setting_value_grammar` | 1029–1044 | REAL | — | covers on/false/250/[en, de]/auto; misses `true`, negatives, `[]` (empty list → List([]) — filter drops empties), bracketed strings (F-004) |
| test `host_verbs_are_repl_only` | 1046–1052 | REAL | — | exit 64 + "REPL-only" |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| one_shot_engine_info_and_commands | YES for the path; NO for engine-info honesty | asserts `audio.null-device` / `video.software-compositor` appear — these come from host.rs's hard-coded literal (baseline standing defect: engine-info under-reports decoders); the test would FAIL if engine-info were made registry-derived, i.e. it pins the hard-coding. The "ok: 65 command(s)" count is a brittle-but-commented change-detector (958–960). |
| all other tests | YES | exit codes + message substrings through the real run path; mutations (drop the exit-code mapping, break format_us, accept host verbs in one-shot) each fail a specific assert |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Human mode prints … one `event: …` line per newly observed wire event" | module doc 21–25 | REAL | print_new_events wiring (part1 sites 206/217/253/282/294) + baseline transcripts |
| "the stable event vocabulary of doc 05 §3.2, this host's subset" | print_event doc 718–719 | SHALLOW | two catalog events (TimingDurationKnown, MediaInfoReady) print as Debug dumps — baseline mp4-smoke-open-session.txt:8–9 (F-001) |
| "deterministic shapes; the AC suite pins them" | section header 501 | REAL (tautologically) | the suite pins the CURRENT shapes including the Debug dumps — every vocabulary change rewrites transcripts |
| "quit/EOF … answers 0" (graceful exit) | module doc 32–40 | REAL (part1's code; my chunk only prints the trail) | teardown printers ride print_response/print_event; baseline AVI transcript ends `Stopped → Idle`, EXIT=0 |
| "`--json` emits every event as one NDJSON line" | module doc 28–30 | REAL (part1 print_new_events_json at 376) | test json_one_shot_prints_the_envelope + control_stdio tests |

## 7. Dead / duplicated / unreachable code
- `print_result`'s `other` arm and `print_event`'s `other` arm: dead in the
  shipped dispatch (subset verified in dispatch.rs audit) but kept as
  defensive Debug printers (F-001/F-002).
- No duplication found in my chunk; `print_track` is shared by info+tracks
  as designed.

## 8. Dependencies & flags
- Consumes: `commands` (catalog types, WireEvent/Response, CommandOutcome),
  `serde_json`, `crate::{bench, dispatch, exit, help, host, parse}`.
- `lumen-allow-literal` annotations at 837–838, 914 — the identity-literal
  escape hatch the baseline lint-literals gate (#13) tracks; annotated use is
  the sanctioned shape pending the identity-wiring card.
- No feature gates; all of this ships in the `lumen` binary.

## 9. Open questions for the next agent
- A05 (part1 of this file): the settle loops in repl/machine mode are the
  prime suspect for the wave's HANG context (4:4:4 MP4 → decode.unsupported
  → Error state; if `settle` waits for a state change that never comes or
  the teardown's `TEARDOWN_TAIL_PUMPS` loop pumps forever on an Error-state
  session, that is the root). My chunk's printers cannot hang.
- A09/A10 (host.rs): `MediaInfo.language Some("voe")` garbage default — the
  TrackInfo is minted host-side; my printers only reveal it via the Debug
  dump (F-001/F-003).
- A01 (parse.rs): the `CliCommand::Seek` `to_target()` conversion — verify
  the percent/absolute grammar this chunk forwards at 815–817.
