# AUDIT: apps/cli/src/help.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-24 |
| Lines / functions | 427 / 8 (+1 const table, 5 tests) |
| File verdict | REAL — but two shipped help claims are false (stale seek prose; advertised flags the parser rejects) |
| Reachable from shipped entry point? | YES — `main.rs:19` → `run.rs:81 version_text`, `run.rs:96/116 help_text` (`--help`), `run.rs:86/137 help_for_area` (`--help <area>`), `run.rs:243` REPL `help` verb. Executed all four. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 2 |
| Commands run | `target/debug/lumen --help` (0); `--version` (0, "lumen 0.1.3"); `--help global` (0); `--help bogus` (error naming 11 known areas); `--global-developer-mode on` (exit 64 "unknown command"); REPL `help`; `seek chapter 3` via REPL (typed not-found). |

## 1. One-paragraph verdict
The generation machinery is real and well-tested: `--help`'s settings half is the G2
generator over the embedded schema, drift-tied to the committed golden and to the seed
TOML by two strong tests, and the subcommand synopsis ids are cross-checked against the
dispatch SERVED table in both directions. But the hand-written prose rows carry two
user-visible lies: the seek row describes the obsolete "reopen at the target" composition
that host.rs's own module docs call FORBIDDEN (Wave 3's flag, confirmed live), and the
"Settings-derived flags" section advertises `--<area>-<key> <value>` flags that the
shipped parser rejects for every one of the 121 schema keys (Wave 11's flag, confirmed
live). Both survived precisely because no test pins the prose — only ids, counts, and
the generated G2 text are checked.

## 2. What actually works (verified)
— `lumen --help`, `--version`, `--help global`, `--help bogus`, REPL `help` all executed
with the expected text (transcripts in audit/runs/W13-A02.log.md).
— `generated_help_matches_the_committed_artifact` (:289-301) and
`areas_block_lists_every_screen_the_seed_declares` (:312-375) are real drift-ties: the
second derives its expectation from an independent raw-TOML read of the seed, so a schema
edit without regeneration fails it.
— `dispatch.rs:349-356` pins synopsis ids ↔ SERVED bidirectionally (15 == 15, verified
count :417-421 here).

## 3. Findings

F-001 [S2]
Where: apps/cli/src/help.rs:55-57 (SUBCOMMANDS row)
Code:
```
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```
What is wrong: two false claims on the shipped `--help` line.
(a) "reopen at the target" describes the 2.D2-era stop→close→reopen composition. The
current code does the engine's NATIVE in-pipeline seek: apps/cli/src/host.rs:26-38 states
the old composition "destroyed all of them and is FORBIDDEN for seek (D-4/D-7: seeking
must never destroy the session, drop hardware handles, or re-open files)", and
`dispatch_seek` (host.rs:1059-1150) submits the engine's `Seek` command with the session,
device, ring and decoders surviving. The help documents the exact anti-pattern the code
bans. Wave 3's stale-seek-claim flag, root-caused and confirmed.
(b) `chapter 3` is listed as seek grammar, but chapters are rejected UNCONDITIONALLY for
every container — host.rs:1080-1088 returns NotFound "no chapter {index}: AVI v0 carries
no chapter markers" (executed: `seek chapter 3` → `error: [session.not-found] …`). A
grammar form that can never succeed is advertised without qualification.
Why it matters: `--help` is the primary user contract; it misdescribes both the mechanism
(suggesting session-destroying behavior that no longer exists) and the accepted grammar.
What correct looks like: "seek (the engine's native in-pipeline flow; the session
survives)" and either drop `chapter N` or annotate "(no chapter support yet)".
Confidence: HIGH — Verification: ran (`--help` output + `seek chapter 3` live) + read
(host.rs:26-38, :1080-1088).

F-002 [S2]
Where: apps/cli/src/help.rs:123-127 (in `help_text`)
Code:
```
    out.push_str("\nSettings-derived flags (--<area>-<key> <value>):\n");
    out.push_str("  `lumen --help` lists areas with counts; `lumen --help <area>` lists that\n"); // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
    out.push_str("  area's flags (the VLC --longhelp pattern, generated).\n");
```
What is wrong: the section header promises settings-derived flags, and `--help <area>`
prints their usage lines (executed: `--help global` → `--global-language <en-US>
[restart required]`), but the CLI parser accepts NONE of them: `lumen
--global-developer-mode on` → `error: unknown command '--global-developer-mode'` (exit
64). Wave 11 (W11-A07 F-001: help.rs:123 advertises settings-derived flags the CLI
rejects as "unknown command") — confirmed live. The generated G2 text is honest about
the schema; the CLI wires zero of it.
Why it matters: every one of the 121 documented flags is a hard error when typed; the
help invites the user to type them. What correct looks like: parse the flags (the
settings seam exists end-to-end otherwise) or label the section "schema reference —
not yet parsed by this build".
Confidence: HIGH — Verification: ran (both sides of the contradiction).

F-003 [S4]
Where: apps/cli/src/help.rs:209 (in `area_section`, :186-223)
Code:
```
        } else if is_header || format!("{line}\n") == header_exact {
```
What is wrong: the second disjunct is dead — `header_exact` is `format!("{area}\n")`
(:191), so `format!("{line}\n") == header_exact` iff `line == area`, which `is_header`
(:196-197, `line == area || …`) already covers. Leftover from an earlier shape.
What correct looks like: delete the redundant disjunct. Confidence: HIGH — read.

F-004 [S4]
Where: apps/cli/src/help.rs:39-94 (SUBCOMMANDS) vs tests :415-426 / dispatch.rs:349-356
Code:
```
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
```
(the test at :415-426 asserts only ids and the count 15)
What is wrong: no test pins the prose column — exactly where F-001's two lies live. The
drift-ties cover ids/counts (dispatch test), the G2 golden, and the Areas block, so the
one-line help strings can rot freely. This is the mechanism by which the stale seek row
survived W1-06's seek rewrite.
What correct looks like: one test asserting each row's help string against a
machine-checked fact (e.g. chapter support = false, seek = native), or at minimum a
grep-style guard on "reopen". Confidence: HIGH — read (test bodies) + ran (drift-ties pass
in baseline while prose is false — proof the net has this hole).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SUBCOMMANDS (const) | 39-94 | REAL data | S2 | F-001 stale/false seek row; 15 ids verified vs SERVED |
| version_text | 98-102 | REAL | — | "lumen 0.1.3" executed; [PROPOSAL] identity honest |
| help_text | 109-130 | REAL | S2 | F-002 false flag advertisement |
| help_for_area | 137-152 | REAL | — | error names all 11 known areas (executed) |
| generated_help | 156-160 | REAL | — | G2 over embedded schema; invalid-schema surfaces loudly |
| areas_block | 165-180 | REAL | — | marker-sliced; strong seed-derived test |
| area_section | 186-223 | REAL | S4 | F-003 dead disjunct; section-boundary heuristic (bare-area-name line would truncate early — no such line in G2 shape, LOW risk) |
| known_areas | 227-245 | REAL | — | parses Areas rows |
| synopsis_catalog_ids | 251-256 | REAL | — | flat_maps 15 ids |
| catalog_ids | 259-279 | REAL | — | exhaustive match, `_ => Vec::new()` silently drops unknown rows (a typo'd id would shrink the count and fail the 15-assert — net effect: caught) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| generated_help_matches_the_committed_artifact :289 | YES | byte-eq vs committed golden; missing file → empty → assert fires |
| areas_block_lists_every_screen_the_seed_declares :312 | YES | expectation derived from independent TOML parse (screens, counts, engine-internal annotations all rebuilt) |
| area_section_slices_engine_and_rejects_unknown :378 | YES | pins section start, one flag line, boundary before next area, prefix-area slicing |
| help_for_area_names_known_areas_on_error :402 | YES | bogus → Err naming playback; global → Ok with flag line |
| synopsis_ids_parse_and_cover_the_served_set :415 | PARTIAL | pins count 15 + 3 spot ids; full bidirectional coverage lives in dispatch.rs:349-356; prose column unpinned (F-004) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`--help` generated from settings-schema" (card 2.D2) | help.rs:3-6 | REAL | G2 over embedded schema; golden-tied test |
| "the ids are cross-checked against the dispatch table by test" | help.rs:37-38 | REAL | dispatch.rs:349-356 bidirectional |
| seek "(v0: reopen at the target through the engine's real transitions)" | help.rs:57 | FALSE | host.rs:26-38 forbids reopen; dispatch_seek is native (F-001) |
| "chapter 3" seek grammar | help.rs:55 | FALSE (always fails) | host.rs:1080-1088 unconditional NotFound; executed |
| "Settings-derived flags (--<area>-<key> <value>)" | help.rs:123 | FALSE | parser rejects all 121 (executed) |
| "--version reports the crate version and says so" | help.rs:29-31 | REAL | [PROPOSAL] + "lumen 0.1.3" executed |

## 7. Dead / duplicated / unreachable code
F-003's dead disjunct is the only dead code. No unreachable paths (all four entry points
verified live).

## 8. Dependencies & flags
Depends on commands::CommandId + settings_schema (generate_str, runtime::embedded). No
feature gates; shipped in the release lane. The five `// lumen-allow-literal` markers are
present and justified (CLI prose) — in contrast to host.rs:1799's missing marker (Wave 1).

## 9. Open questions for the next agent
- parse.rs owns the flag-rejecting side of F-002; if the flags ever get parsed, remember
  parse.rs:353's own error hint currently suggests the non-existent key
  `playback.autoplay-next`.
- G2 emitter (crates/settings-schema) — if a settings description can ever render a line
  that equals a bare area name, `area_section`'s boundary heuristic truncates early
  (defensive note only; no such line today).
