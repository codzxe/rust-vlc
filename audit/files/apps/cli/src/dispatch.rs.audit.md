# AUDIT: apps/cli/src/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 359 / 5 prod (+1 test helper, 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: main.rs -> run::run -> repl_mode/machine_mode -> lumen_dispatch (run.rs:198, 272, 349, 488); desktop delegates to it (apps/desktop/src/dispatch.rs, per W13 desktop audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/lumen commands` -> "ok: 65 command(s)" (counted 65 rows); `lumen_cli-5f13e10be716a8b0 dispatch` -> 6 passed exit 0; machine-mode probes: `playback.set-rate` -> typed `unsupported-for-host` + owner detail; `session.set-frame-channel` -> same refusal (no secret serving); sed quote checks |

## 1. One-paragraph verdict
The hostile-focus question was whether the CLI's SERVED table (Wave 2: 15/65)
is honest. It is. SERVED lists exactly 15 ids; the catalog has exactly 65
(executed: `lumen commands` -> "ok: 65 command(s)"); `route()` has exactly the
15 matching arms and one default arm; and I probed the machine mode with
unserved ids — `playback.set-rate` and `session.set-frame-channel` both answer
the typed `unsupported-for-host` refusal naming command + owner, never a
silent no-op and never a hidden extra serving. The 15/65 posture is typed, not
inflated: the inflation lives in OTHER files (help.rs prose per W13-A02;
host.rs engine-info / bench.rs hardcoded decoder lists per my ogg.rs audit),
not here. The only debt is a doc-precision slip ("every host surface" — the
four host verbs bypass by documented design) and the answered open question
from the host.rs audit about served markers in `system.list-commands`.

## 2. What actually works (verified)
— verified by `./target/debug/deps/lumen_cli-5f13e10be716a8b0 dispatch --test-threads=1` ->
`test result: ok. 6 passed` (incl. `served_table_matches_the_help_synopsis`,
the bidirectional drift guard :347-357). Machine-mode probes (executed):
`{"cmd":"playback.set-rate",...}` ->
`"code":"unsupported-for-host","message":"`playback.set-rate` is not served by
this build's command-line host (rate and frame stepping are card 3.A1)"` with
`details.command` + `details.owner`; `session.set-frame-channel` -> same
typed refusal. `lumen commands` -> 65 rows. The end-to-end serving of all 15
is proven by my session_script.rs run (21/21 through the real binary).

## 3. Findings
F-001 [S4]
Where: apps/cli/src/dispatch.rs:1-6
Code:
```
//! The one dispatch entry (card 2.D2): every host surface — argv
//! subcommands, the interactive REPL, the `--control-stdio` machine
//! mode — builds a [`WireRequest`] and calls [`lumen_dispatch`].
```
What is wrong: "every host surface" is false for the four host verbs —
`advance`, `help`, `quit` (REPL, run.rs:242-261) and `decode-bench` (argv,
run.rs:44-47: "bypasses the catalog dispatch by design") never build a
WireRequest. The carve-out is documented one file away but this file's
headline claim overstates. What correct looks like: "...every CATALOG
command surface...; the four host verbs (advance/help/quit/decode-bench) are
handled by the runner by design". Confidence: HIGH — Verification: read +
ran (advance/decode-bench behavior observed).
F-002 [S4]
Where: apps/cli/src/dispatch.rs:38-54 (SERVED) + host.rs command_listings
Code:
```
pub const SERVED: &[CommandId] = &[
    CommandId::SessionOpen,
    CommandId::SessionClose,
```
What is wrong: nothing in the mechanism — this records the ANSWER to the
host.rs audit's open question ("should system.list-commands carry a served
marker? CommandListing has no field for it"). `lumen commands` enumerates all
65 ids with no served flag, so a script enumerating the surface gets 50 rows
that will refuse; the only machine-readable marker is the refusal itself
(branchable on `unsupported-for-host`) plus `--help`'s 15-verb synopsis
(drift-guarded by :347-357). Defensible layering (the catalog crate must not
know hosts), but the CLI's own `command_listings()` (host.rs:1665) could
annotate rows host-side. Confidence: HIGH — Verification: ran + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SERVED (const) | 38-54 | REAL | — | 15 ids, counted; == route() arms; == --help synopsis (test) |
| lumen_dispatch | 62-71 | REAL | — | correlate, route, echo request_id + session/generation |
| route | 75-157 | REAL | — | 15 arms + default refusal; --start-paused compose (settle+pause) admits the settle-window frame present honestly (:94-95) |
| unserved | 164-174 | REAL | — | typed refusal, command+owner details |
| command_id_string | 180-189 | REAL | — | serde "cmd" tag read, error-path only, "(unnamed)"/"(unserializable)" fallbacks |
| owner_of | 190-210 | REAL | — | prefix buckets; all probed ids name a plausible owner |
| tests x6 | 227-357 | honest | — | refusal pins, not-ready pins, schema-validating settings seam, drift guard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed | YES | pins code/stage/details for set-rate |
| snapshots_need_a_session | YES | NotReady pinned |
| settings_seam_validates_through_the_schema | YES | valid write applies + bogus enum refuses |
| seek_without_session_refuses | YES | NotReady pinned |
| blocked_scheme_url_open_refuses_naming_the_lane | YES | rtsp refusal names the lane |
| served_table_matches_the_help_synopsis | YES | bidirectional set equality — add/remove drift fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every host surface" + "builds a [`WireRequest`] and calls [`lumen_dispatch`]" | :1, :3 | FALSE as stated (4 host verbs bypass, by documented design) | run.rs:44-47, run.rs:242-261; F-001 |
| "the `--help` synopsis ([`crate::help`]) is tested against it, so" + "the two cannot drift." | :14-15 | REAL | test :347-357 passed in my run |
| "Every other catalog id answers the honest" + "typed refusal `unsupported-for-host` naming the wave that owns it" | :15-16 | REAL | executed: set-rate, set-frame-channel both refuse typed; 65-15=50 |
| Wave 2 claim "Served surface: 15/65 commands (CLI/desktop/web)" | wave-02/SUMMARY.md:16 | REAL | counted: 65 catalog rows, 15 SERVED, 15 route arms |
| session.close is class S and answers Completed | :26-27 | REAL-UNPROVEN here | proven by my session_script runs (close/quit teardown tests) |

## 7. Dead / duplicated / unreachable code
None. SERVED is consumed by the drift test and referenced by help.rs docs;
the desktop consumes the whole dispatch by delegation.

## 8. Dependencies & flags
No feature gates; commands + playback-engine only. No unsafe. Release-lane
shipped (baseline cargo-build-release-lumen-cli exit 0).

## 9. Open questions for the next agent
- F-002's served-marker decision belongs to the host layer (host.rs
  command_listings) or a catalog v2 field; not a dispatch.rs defect.
- toggle (:118-127) sends Play for any non-Playing state (incl. Ended) — the
  engine's state table owns the answer; if a "toggle from Ended restarts"
  behavior is desired, that is a product decision nobody has recorded.
