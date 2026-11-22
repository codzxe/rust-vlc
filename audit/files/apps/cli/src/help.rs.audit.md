# AUDIT: apps/cli/src/help.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 427 / 10 production items + 5 tests |
| File verdict | REAL-UNPROVEN (generation mechanism real, tests real and non-circular; one stale user-facing claim) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → run.rs:116 `Some(CliCommand::Help) => match help::help_text()` and run.rs:243 (REPL `help`); W00 baseline captured the real output (audit/runs/baseline/lumen-help.txt). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n`/`rg` verification (SERVED cross-check exists at dispatch.rs:348-355); baseline help output read (lumen-help.txt). NO cargo (adaptation). |

## 1. One-paragraph verdict

The one file in my assignment with REAL runtime evidence: the W00 baseline captured `lumen --help` output (audit/runs/baseline/lumen-help.txt), and it matches this module's construction line-for-line. The mechanism is genuinely generated (settings-schema's G2 over the embedded schema, not a hand-copied table — pinned by a byte-equality test against the committed golden), the areas test derives its expectation from the seed TOML independently (non-circular), and the claimed synopsis/dispatch cross-check exists for real (dispatch.rs:348-355). The defects are small: the `seek` help line still tells the user seek is "v0: reopen at the target" — a composition the native-seek suite (W1-06) explicitly proves is GONE (in-pipeline, zero teardown edges), so the shipped help describes forbidden, dead behavior; `area_section` is fragile string-munging with a dead redundant condition; and `version_text` reports the crate version with an honest [PROPOSAL] marker for product identity.

## 2. What actually works (verified)

- Runtime evidence: audit/runs/baseline/lumen-help.txt (45 lines read) matches `help_text()`'s construction — version line, usage block, all 18 SUBCOMMANDS rows, areas block. The stale seek line appears verbatim at lumen-help.txt:24 (see F-001).
- The generation is real: `settings_schema::runtime::embedded()` + `generate_str(schema, GenTarget::CliHelp)` (help.rs:156-160); the drift-tie test `generated_help_matches_the_committed_artifact` (288-301) byte-compares against `schemas/generated/settings-help.txt`.
- The claim "the ids are cross-checked against the dispatch table by test" (help.rs:37-38) is TRUE: dispatch.rs:348-355 asserts `synopsis_catalog_ids()` vs `SERVED` both ways.
- `areas_block_lists_every_screen_the_seed_declares` (311-375) derives the expected block from `schemas/settings.toml` by an independent raw-TOML read — a genuinely non-circular oracle (the good kind of test).
- `help_for_area("bogus")` refusal names known areas (tested, 401-412).

## 3. Findings

### F-001 [S3] — Shipped help text says seek reopens the session; seek is in-pipeline

Where: apps/cli/src/help.rs:54-58 (the SUBCOMMANDS row); contradicted by apps/cli/tests/native_seek.rs:10-14 and the W00 output

Code (help.rs:55-57, verbatim):
```rust
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```

Code (native_seek.rs:10-14, verbatim):
```rust
//! - **No reopen** (D-4/D-7): the session id and every session-scoped
//!   half (device, ring, clock, decoders, renderer) survive the seek;
//!   the state trail shows the in-pipeline transitions only — a
//!   `Stopped`/`Idle`/`Opening` after the seek would be the forbidden
//!   stop→close→reopen composition.
```

What is wrong: the user-facing `seek` description documents the OLD reopen composition. Since W1-06 (`SessionHandle::seek`, the suite this repo calls "THE C8 acceptance"), seek runs the engine's in-pipeline flow — the native_seek suite asserts `teardown_edges == 0` and one open edge across five seeks. The shipped binary prints the stale sentence (verified in the W00 baseline output, lumen-help.txt:24). lib.rs:52-57 carries the same stale story ("composed host-side as `stop → close → open-at-target`") — the docs and the tests disagree, and the tests are the ones with exact pins.
Why it matters: the help text is the product's self-description; it tells users their session is destroyed by every seek, which is false of the current code.
What correct looks like: "seek (in-pipeline: session and devices survive; reports the actual landing)".
Confidence: HIGH — Verification: read + baseline output + native_seek.rs pins (all static; the native_seek pins themselves are runtime-UNPROVEN per that file's audit — but dispatch.rs:131 routes to `host.dispatch_seek`, the in-pipeline path, regardless).
Severity: S3 — a false capability statement in shipped output, limited blast radius.

### F-002 [S4] — `area_section`: fragile text-slicing with a dead condition

Where: apps/cli/src/help.rs:186-223

Code (help.rs:209-213, verbatim):
```rust
        } else if is_header || format!("{line}\n") == header_exact {
            collecting = true;
            out.push_str(line);
            out.push('\n');
        }
```

What is wrong: two things. (a) `format!("{line}\n") == header_exact` is the same test as `is_header`'s first disjunct (`line == area` where `header_exact = format!("{area}\n")`) — wait, it is not: `is_header` compares `line == area` (no newline) while this compares with the newline appended; both can only be true for the same single line, making the second disjunct dead duplication of the first's intent. (b) The whole function slices generated prose by heuristics (bare header line, or `name (…)` prefix + `)` suffix) — any G2 emitter reformat (e.g. a flag description containing a line that equals an area name) silently truncates a section. The tests pin today's shapes, so drift is caught only if the golden changes shape.
Confidence: HIGH — Verification: read.

### F-003 [S4] — `version_text` reports the crate version, not product identity

Where: apps/cli/src/help.rs:98-102

Code (help.rs:101, verbatim):
```rust
    format!("lumen {}\n", env!("CARGO_PKG_VERSION")) // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```

What is wrong: nothing functionally — honestly recorded as a [PROPOSAL] (product.toml identity not wired in v0). Listed because the "rebrand-propagation card" this defers to is exactly the class of deferred wiring this audit tracks; `lumen 0.1.3` is a crate version presented as the product version.
Confidence: HIGH — Verification: read + baseline output line 1 (`lumen 0.1.3`).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SUBCOMMANDS` | 39-94 | REAL-UNPROVEN | S3 (F-001) | Data table; cross-checked against SERVED by dispatch.rs:348-355 (verified). |
| `version_text` | 98-102 | REAL-UNPROVEN | S4 (F-003) | Crate version; baseline-verified output. |
| `help_text` | 109-130 | REAL-UNPROVEN | — | Assembled; W00 output matches construction. |
| `help_for_area` | 137-152 | REAL-UNPROVEN | — | Unknown-area refusal names known areas (tested). |
| `generated_help` | 156-160 | REAL-UNPROVEN | — | Single funnel through G2 over the embedded schema. |
| `areas_block` | 165-180 | REAL-UNPROVEN | — | Marker-to-blank-line slice; tested. |
| `area_section` | 186-223 | REAL-UNPROVEN | S4 (F-002) | Heuristic slicing; dead disjunct at :209. |
| `known_areas` | 227-245 | REAL-UNPROVEN | — | Trim + split_once; tested against seed-derived list. |
| `synopsis_catalog_ids` | 251-256 | REAL-UNPROVEN | — | |
| `catalog_ids` | 259-279 | REAL-UNPROVEN | — | Total match; host verbs → empty. |
| tests ×5 | 288-426 | REAL-UNPROVEN | — | Golden byte-tie, seed-derived areas oracle (non-circular), section slicing, area refusal, id coverage. Runtime-UNPROVEN (crate never ran in baseline) — but the baseline `lumen --help` output corroborates the rendered result. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| generated_help_matches_the_committed_artifact | YES | Byte equality against the committed golden. |
| areas_block_lists_every_screen_the_seed_declares | YES | Independent TOML-derived oracle — would catch emitter regressions and seed/schema drift. |
| area_section_slices_engine_and_rejects_unknown | YES | Prefix/exclusion pins incl. the prefix-trap (`playback` vs longer names). |
| help_for_area_names_known_areas_on_error | YES | |
| synopsis_ids_parse_and_cover_the_served_set | PARTIAL | Len + 3 spot-pins; the full cross-check lives in dispatch.rs:348-355 (which does the real both-ways assertion). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "--help generated from settings-schema" (the card) | :3-6 | REAL-UNPROVEN | G2 call + golden tie test + baseline output. |
| "the ids are cross-checked against the dispatch table by test" | :37-38 | REAL | dispatch.rs:348-355. |
| "seek (v0: reopen at the target…)" (the help string itself) | :57 | FALSE (stale) | native_seek.rs:10-14; F-001. |
| "--version reports the crate version and says so" | :29-31 | REAL | [PROPOSAL] marker + code. |

## 7. Dead / duplicated / unreachable code

- The dead disjunct at :209 (F-002). No other dead code; no duplication (the flag metadata genuinely lives once, in the schema).

## 8. Dependencies & flags

- `settings_schema` (workspace path dep) — the generation is unconditional; `toml` dev-usage in tests only for the oracle. No feature gates.

## 9. Open questions for the next agent

- lib.rs:52-57 repeats the stale reopen story (audited next in this agent's lib.rs report — cross-reference F-001).
- Does `--help <area>` for an area whose flags include another area's name as a literal line exist in the current schema? (F-002's fragility is hypothetical — the golden's current shape is pinned.)
