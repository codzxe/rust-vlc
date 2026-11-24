# AUDIT: apps/cli/tests/command_surface.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Lines / functions | 171 / 1 harness + 7 #[test] |
| File verdict | REAL (small, honest, executed; brittle count pin + harness twin divergence) |
| Reachable from shipped entry point? | N/A (test file) — drives the REAL `lumen` binary (`CARGO_BIN_EXE_lumen`): --help/--version/commands/engine-info/--control-stdio lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `./target/debug/deps/command_surface-c9ac8e272ab2ce7d` → 7 passed, 0 failed (0.03 s; binary newer than source); `./target/debug/lumen --help` (full surface dumped, 18 verbs + 4 reserved flags + areas); `./target/debug/lumen commands | rg -c '['` → 65 rows; sed verifications |

## 1. One-paragraph verdict
A compact, honest binary-level suite that pins the CLI's self-description surface: the full
`--help` (version header, usage, catalog-id synopsis rows, the settings-schema-generated areas
index), one generated area (`--help engine` with the schema's own flag spellings and defaults)
plus the unknown-area usage error, exact `--version` output, the catalog introspection (65 ids,
class letters, all 15 served ids), the engine self-description (first-vertical tags, null
device, software compositor, decoder list), and the machine-mode wire envelope twice (plain
not-ready error shape, and `--json` end-to-end open of the real AVI fixture with
session.state-changed + tracks.list-ready events on the NDJSON stream). All 7 pass under my own
run, and the surface it pins is internally consistent with the grammar (18 verbs in help ==
18 verbs in parse.rs; "the playing subset of the command catalog" is the help's own honest
framing). Defects are hygiene-level: a stale "64" in a doc comment against the asserted 65, a
hand-bumped count pin with a documented history of being missed, a harness twin that dropped
the sibling suite's BrokenPipe tolerance, and no help↔grammar sync guard.

## 2. What actually works (verified)
- `./target/debug/deps/command_surface-c9ac8e272ab2ce7d` → "7 passed; 0 failed" (executed).
- `./target/debug/lumen --help` dumped: 18 verb rows (open/close/play/pause/toggle/stop/seek/
  info/time/duration/tracks/engine-info/commands/setting/decode-bench/advance/help/quit) —
  EXACTLY the 18 verbs parse.rs's grammar accepts (parse.rs:196-214); "Reserved (non-settings)
  flags: -h/--help, -v/--version, --json, --control-stdio" == the 4 global flags parse_argv
  strips; areas index present (audio 11, controls 9, engine 6, ...).
- `./target/debug/lumen commands | rg -c '['` → 65 — the asserted count is live-verified
  outside the test too.
- The two machine-mode tests pin byte-shape: `"requestId":42` echo, `"code":"not-ready"`,
  `"stage":"session"`, and the --json lane opens the committed AVI fixture end-to-end through
  the envelope (`"kind":"session.state-changed"`, `"kind":"tracks.list-ready"`).

## 3. Findings
F-001 [S4]
Where: apps/cli/tests/command_surface.rs:89-90 vs :95-100
Code:
```
/// The catalog introspection: all 64 ids with their classes; every
/// id the dispatch table serves is present.
```
```
    assert!(out.contains("ok: 65 command(s)"), "{out}");
```
What is wrong: the test's own doc comment says "all 64 ids" while the body asserts 65 — the
stale cardinality lives one screen above the correction comment (:95-99) that explains the
w31.1 miss. A reader of the doc comment alone is miscounted. Confidence: HIGH — Verification:
read + executed count.
F-002 [S4]
Where: apps/cli/tests/command_surface.rs:100 (the pin), :95-99 (its history)
Code:
```
    // 65 since w31.1 minted session.set-frame-channel (the frame
    // channel bind for the GUI canvas presenter); the wave-8
    // playback-operations verbs (frame-step, chapter nav, Go-to-Time,
    // bookmarks) had made it 64. w31.1 missed THIS pin (its checksum
    // sibling was fixed in w31.6); the consolidation round catches it.
```
What is wrong: the count is a hand-bumped literal with a documented history of being missed
once already; nothing derives it from the catalog at test time (the "checksum sibling" the
comment references lives elsewhere), so the next catalog addition re-creates the same drift
dance. The test name promises "the whole catalog" but pins count + 15 served ids + 2 class
letters — 48 of 65 rows are unverified by this suite. What correct looks like: assert the
count against the schema-generated artifact (commands.checksum.txt or the generated catalog
constant) or drop the cardinality and assert the served-id set + one known-unserved id.
Confidence: HIGH — Verification: read + executed.
F-003 [S4]
Where: apps/cli/tests/command_surface.rs:18-21 vs apps/cli/tests/unsupported_path.rs:26-35
Code:
```
    {
        let stdin = child.stdin.as_mut().must();
        stdin.write_all(script.as_bytes()).must();
    }
```
What is wrong: this `run()` is a copy of unsupported_path.rs's harness MINUS the BrokenPipe
tolerance — there, a failing fast-open that exits before consuming stdin is asserted as the
expected shape; here `.must()` would panic the harness with a BrokenPipe error. Today's lanes
are safe (empty scripts, or machine mode which reads to EOF), but the first added lane with a
non-empty script + early-exiting command (exactly the unsupported_path shape) crashes the
harness instead of asserting the binary's behavior. Two twins, one hardened, one not.
Confidence: HIGH — Verification: read + cross-file comparison.
F-004 [S4]
Where: apps/cli/tests/command_surface.rs:1-3 (scope) — absence of a sync guard
What is wrong: the suite pins help-output CONTENT (specific ids present) but never that the
help's verb table and the parse grammar stay in lockstep — a verb added to parse.rs but
forgotten in help.rs (or vice versa) passes every test here (help.rs's own in-crate tests are
the only other guard). Likewise the doc-05 §5.3 "Top-30" drift (parse.rs F-001: `--rate`,
`rate`, `volume` documented, unimplemented) is invisible to this suite: it pins the shipped
surface against itself, not against the doc that oversells it. No lane for the REPL verbs
(advance/quit) or bare-source at this level either (unsupported_path.rs covers bare-source).
Confidence: HIGH — Verification: read + rg (no such assertions) + the doc-05 executions in
the parse.rs audit.

## 4. Per-function table (test inventory)
| Test | Lines | Verdict | Would it fail if code were broken? |
|---|---|---|---|
| run (harness) | 10-28 | REAL | n/a (see F-003 for the missing BrokenPipe arm) |
| help_lists_commands_and_generated_areas | 33-59 | REAL | YES — version prefix, usage line, 4 id rows, Areas + 6 area names |
| help_area_prints_the_generated_flags | 63-76 | REAL | YES — exact generated flag spelling + settings key + unknown-area usage error (exit 64) |
| version_prints_the_crate_version | 79-87 | REAL | YES — exact equality with CARGO_PKG_VERSION |
| commands_lists_the_whole_catalog | 91-123 | REAL | YES (count + served set + class letters); 48 rows unpinned (F-002) |
| engine_info_names_the_vertical | 127-137 | REAL | YES — host, vertical tag, null device, compositor, decoder list |
| control_stdio_speaks_the_wire_envelope | 141-153 | REAL | YES — requestId echo + exact error envelope fields |
| control_stdio_with_json_emits_events | 157-171 | REAL | YES — end-to-end machine-mode open of the real AVI fixture + two event kinds |

## 5. Test honesty
All 7 tests assert exact output content and exit codes against the real binary; none is
tautological. The weakest is the catalog count (F-002) — a regression that renames an
unserved id passes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`--help` (the schema-generated text + the catalog-sourced synopsis)" | :1-2 | REAL | executed; generated areas + id-suffixed synopsis verified in the dump |
| "all 64 ids" (test doc comment) | :89 | FALSE (stale) | body asserts 65, live `commands` count is 65 (F-001) |
| "the machine mode's envelope shape" (requestId echoed, camelCase, adjacent cmd/args) | :2-3, :139-140 | REAL | executed: `"requestId":42`, `"code":"not-ready"`, `"stage":"session"` |
| "`--json --control-stdio`: the observed events ride the NDJSON stream after the response" | :155-156 | REAL | executed: two event kinds on stdout after the response line |
| help "Commands (the playing subset of the command catalog)" | help output (dumped) | REAL | 18 verbs == parse.rs grammar; the "subset" framing is honest vs the 65-id catalog |

## 7. Dead / duplicated / unreachable code
- The `run()` harness is a near-duplicate of unsupported_path.rs's (F-003) — the third
  binary-driving harness shape in the CLI tests (session_script.rs presumably owns another).
- No dead tests.

## 8. Dependencies & flags
- Test-only: std::process, testkit::Must. Uses `CARGO_BIN_EXE_lumen` and `CARGO_PKG_VERSION`
  compile-time env. No features/unsafe.

## 9. Open questions for the next agent
- help.rs owner: is there an in-crate test pinning help-table ↔ parse-grammar sync (every
  grammar verb listed, no extra rows)? If not, F-004's sync gap is tree-wide.
- The catalog count's "checksum sibling" (mentioned at :98) — where does it live, and does CI
  run it? If it is the generated commands.checksum.txt check, the count pin here is redundant
  and can be derived instead of hand-bumped.
