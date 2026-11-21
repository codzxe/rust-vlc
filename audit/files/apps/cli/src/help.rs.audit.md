# AUDIT: apps/cli/src/help.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 427 / 10 (1 const table, 9 fns) + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen --help/-v/--help <area>` → parse.rs → run.rs → `help_text`/`version_text`/`help_for_area`; REPL `help` verb → SUBCOMMANDS; executed: baseline command_surface tests `help_lists_commands_and_generated_areas`, `help_area_prints_the_generated_flags`, `version_prints_the_crate_version` all ok, orchrun-test-workspace-nofailfast.log.txt:2929-2931) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "start-paused\|start_paused" apps/cli/src/parse.rs` (flag exists, :256); `rg -n "reopen\|native" apps/cli/src/host.rs` (seek is NATIVE: :1041-1057, :74 "reopen fallback does not exist"); `rg -n "SERVED\|synopsis_catalog_ids" apps/cli/src/dispatch.rs` (cross-check test :348-355 exists); `rg -n "help::tests\|help_lists\|help_area\|version_prints" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 5 unit + 3 integration tests ok :2863-2867, :2929-2931); `ls schemas/generated/settings-help.txt` (golden exists; byte-equality test passed); Read on every quoted range (verbatim verified) |

## 1. One-paragraph verdict
This module is real, generated, and executed: `--help` funnels through the settings-schema G2 generator over the embedded schema (no duplicated flag metadata), the golden tie test asserts byte-equality with the committed `schemas/generated/settings-help.txt`, the areas test derives its expectation independently from `schemas/settings.toml` rather than pinning a second literal, and the synopsis table is cross-checked against `dispatch::SERVED` by a test in dispatch.rs. All 5 unit tests and 3 integration tests passed in the orchestrator's no-fail-fast run. The defects are textual: the shipped `--help` still describes seek as "v0: reopen at the target" when the code path is the engine's native in-session seek that explicitly never reopens (host.rs:1041-1057), and the tagline sells "the engine's first vertical" while eleven container families ship — plus one dead redundancy in `area_section` and the annotated argv0 identity debt.

## 2. What actually works (verified)
- `help::tests::{generated_help_matches_the_committed_artifact, areas_block_lists_every_screen_the_seed_declares, area_section_slices_engine_and_rejects_unknown, help_for_area_names_known_areas_on_error, synopsis_ids_parse_and_cover_the_served_set}` → all `ok`, orchrun-test-workspace-nofailfast.log.txt:2863-2867.
- End-to-end: `help_lists_commands_and_generated_areas ... ok`, `help_area_prints_the_generated_flags ... ok`, `version_prints_the_crate_version ... ok` (command_surface.rs, same log :2929-2931) — the real binary's help/version surfaces run in the recorded evidence.
- `--start-paused` flag advertised in the synopsis exists (parse.rs:256) — the synopsis does not advertise phantom flags.
- The synopsis↔dispatch cross-check claimed at :37-38 is real: dispatch.rs:348-355 asserts `synopsis.len() == SERVED.len()` and per-id containment.

## 3. Findings
### F-001 [S3]
Where: apps/cli/src/help.rs:55-58 (and :113)
Code:
```
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```
and
```
    out.push_str("The Lumen command-line player — headless host of the engine's first\n"); // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```
What is wrong: Both shipped strings are stale. (a) The seek row tells the user seek "reopen[s] at the target", but the implementation is the engine's NATIVE seek flow — host.rs:1041-1043: "The `playback.seek` path — the engine's native seek flow … the session, the device, the ring, the decoders and the observability trail all survive", host.rs:1056-1057 "never a fake success, never a reopen", host.rs:74 "reopen fallback does not exist"; the native_seek integration suite (9 tests, all ok in no-failfast) pins the epoch-preserving behavior. (b) The tagline claims "headless host of the engine's first vertical" while the same binary opens eleven container families (containers/mod.rs:22-32) and decodes H.264/AAC/Vorbis/Opus/VP9/AV1 (pipeline_host.rs:242-346) — the same under-claim disease as engine_info (my host.rs#part3 F-001).
Why it matters: `--help` is the first thing a user reads; it misdescribes both what seek does (reopen = state loss; native = state survives) and what the product is. A user avoiding seek because they believe it restarts the session is misled by the tool itself.
What correct looks like: "seek (native in-session flow; the session and decoders survive)" and a tagline without the frozen "first vertical" framing.
Confidence: HIGH — Verification: read (host.rs:1041-1057, :74; containers/mod.rs arms; pipeline_host arms all read).

### F-002 [S4]
Where: apps/cli/src/help.rs:196-213
Code:
```
        let is_header =
            line == area || (line.starts_with(&header_annotated) && line.ends_with(')'));
        if collecting {
```
and at :209
```
        } else if is_header || format!("{line}\n") == header_exact {
```
What is wrong: The second disjunct `format!("{line}\n") == header_exact` (where `header_exact = format!("{area}\n")`) is byte-equivalent to `line == area`, which `is_header` already tests. Dead condition, plus two allocations per line for the comparison.
Why it matters: Noise; a future reader must prove the two header tests equivalent to know nothing is missed.
What correct looks like: Delete the redundant disjunct and the now-unused `header_exact`.
Confidence: HIGH — Verification: read (equivalence is definitional).

### F-003 [S4]
Where: apps/cli/src/help.rs:98-102
Code:
```
pub fn version_text() -> String {
    // [PROPOSAL: product.toml identity (name/brand) joins with the
    // rebrand-propagation wiring; v0 reports the crate's own version.]
    format!("lumen {}\n", env!("CARGO_PKG_VERSION")) // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```
What is wrong: The argv0/product identity is hard-coded (annotated for the literal lint, [PROPOSAL]-marked) rather than read from product.toml's `[identity] name = "Lumen"`.
Why it matters: Documented, annotated debt — the rebrand-propagation card owes the join; consistent with the repo's own allow-list convention, so hygiene only.
What correct looks like: `product_config` join when the card lands.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `SUBCOMMANDS` (const) | 39-94 | REAL | S3 | 17 rows: 14 catalog rows + 3 host verbs; seek description stale (F-001); count pinned by two tests (help.rs:415-421, dispatch.rs:348-355). |
| `version_text` | 97-102 | REAL | S4 | Hard-coded argv0, annotated + [PROPOSAL] (F-003); executed (version_prints test). |
| `help_text` | 109-130 | REAL | S3 | Assembles version + tagline (stale, F-001) + usage + synopsis + areas block; String errors documented shape; tagline carries its own allow-literal annotation. |
| `help_for_area` | 137-152 | REAL | — | Unknown area → Err naming known areas (tested); header notes the G2 golden provenance. |
| `generated_help` | 156-160 | REAL | — | Single funnel: embedded schema → G2; invalid schema → loud String error. |
| `areas_block` | 165-180 | REAL | — | Slices the `Areas:` block; shape pinned by the seed-derived test. |
| `area_section` | 186-223 | REAL | S4 | Prefix-area discrimination handled (`playback` vs longer names); trailing blank-line trim; redundant header disjunct (F-002). |
| `known_areas` | 227-245 | REAL | — | Parses area ids from the block rows; equality-pinned by test (:374). |
| `synopsis_catalog_ids` | 251-256 | REAL | — | Flat-maps the table; consumed by dispatch cross-check test. |
| `catalog_ids` | 259-279 | REAL | — | Explicit id → enum map; `_ => Vec::new()` silent fall-through for host verbs — a typo'd catalog id would change the count and fail two tests (guarded). |
| `tests::generated_help_matches_the_committed_artifact` | 288-301 | REAL | — | Byte-equality with the committed golden; missing artifact → assert (unwrap_or_default + non-empty guard, honest). |
| `tests::areas_block_lists_every_screen_the_seed_declares` | 311-375 | REAL | — | Derives the expectation from settings.toml by independent TOML read (mirrors emitter shape incl. counts, plural, engine-internal annotation) — not a pinned literal; mutation-proof by construction. |
| `tests::area_section_slices_engine_and_rejects_unknown` | 377-399 | REAL | — | Exact section content, boundary, unknown refusal, prefix-area case. |
| `tests::help_for_area_names_known_areas_on_error` | 401-412 | REAL | — | Error names known areas; happy path content check. |
| `tests::synopsis_ids_parse_and_cover_the_served_set` | 414-426 | REAL | — | Count 15 + spot-pins; the full containment test lives in dispatch.rs:348-355. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| generated_help_matches_the_committed_artifact | YES | Byte equality vs the committed golden; schema edit without regen fails here AND xtask schema-gen --check. |
| areas_block_lists_every_screen_the_seed_declares | YES | Expectation derived from the seed file itself (sorted screens, counts, annotations) — an emitter or slicing regression cannot self-fulfill. |
| area_section_slices_engine_and_rejects_unknown | YES | Content + boundary + refusal + prefix discrimination all asserted. |
| help_for_area_names_known_areas_on_error | YES | Error message content and happy-path flag text asserted. |
| synopsis_ids_parse_and_cover_the_served_set | YES (with dispatch.rs:348-355) | Count + containment; a dropped/typo'd row breaks one of the two. |
All executed: no-failfast log :2863-2867.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "--help generated from settings-schema" (doc 05 §5.3, doc 01 §2.8) | module docs :3-6 | REAL | generated_help:156-160 → `generate_str(schema, GenTarget::CliHelp)`; golden tie test. |
| "the ids are cross-checked against the dispatch table by test" | :37-38 | REAL | dispatch.rs:348-355. |
| "seek (v0: reopen at the target through the engine's real transitions)" | :57 | FALSE (stale) | host.rs:1041-1057 native seek; :74 "reopen fallback does not exist"; native_seek suite. |
| "headless host of the engine's first vertical" | :113 | FALSE (stale scope) | 11 container families (containers/mod.rs:22-32); corrected MP4 smoke matrix (baseline INDEX). |
| "--version reports the crate version and says so [PROPOSAL]" | :29-31 | REAL | version_text does exactly that, annotated. |
| "Product identity not wired in v0" | :29-31 | REAL | Hard-coded argv0 with [PROPOSAL] (F-003). |

## 7. Dead / duplicated / unreachable code
`area_section`'s redundant header disjunct (F-002). No unreachable functions — all of help_text/help_for_area/version_text/areas_block/known_areas/area_section/synopsis_catalog_ids have production or test callers, and the three primary surfaces are exercised end-to-end by command_surface.

## 8. Dependencies & flags
Imports: `commands::CommandId`, `settings_schema::{GenTarget, generate_str}`. No feature gates — compiled into the shipped `lumen` unconditionally. Tests use the `toml` crate (dev-dependency of lumen-cli) to independently parse the seed — a legitimate second reader for test honesty. The argv0 literals carry `// lumen-allow-literal:` annotations (consistent with the lint-literals gate; the gate's only violation is host.rs:1799, not this file).

## 9. Open questions for the next agent
- Who owns refreshing SUBCOMMANDS prose when flows change (F-001's seek row)? A drift test asserting the seek description mentions "native" would pin it.
- run.rs/parse.rs owners: confirm no other user-facing string still says "reopen" for seek (rg "reopen" apps/cli/src → host.rs:889 comment "also the seek reopen" is itself stale the same way).
