# AUDIT: crates/commands/tests/catalog_shape.rs
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-27 |
| Lines / functions | 244 / 2 helpers + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — runs in the workspace gate; catalog it pins feeds the shipped `lumen` CLI and the control-server dispatch |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p commands --test catalog_shape` -> 4 passed, EXIT=0; python3 independent count of committed schema -> 65 cmd / 26 event tags |

## 1. One-paragraph verdict
Honest, strong shape tests. They read the COMMITTED artifacts (schemas/commands.schema.json, schemas/generated/commands.checksum.txt) and cross-check three independent surfaces — schema wire tags, `CommandId`/`EventId` enums, `catalog_data()` spec rows — for exact set equality in both directions, so an orphan or missing row anywhere fails loudly. I independently re-counted the committed schema with python3 (65 command tags, 26 event tags, draft 2020-12) and re-ran the suite: 4/4 pass, checksum recomputes. The checksum test is the correct use of a golden: the file is `@generated` and the test proves the committed artifact still matches the live catalog (catches drift both ways). Defects are trivia: the module header still says 64 commands while the body correctly pins 65, and two of the four lines in the checksum file (schema/typescript digests) are written by codegen but verified by no test.

## 2. What actually works (verified)
- verified by `cargo test -p commands --test catalog_shape` -> `test result: ok. 4 passed; 0 failed` EXIT=0.
- verified by `python3` (independent parse of schemas/commands.schema.json) -> `command tags: 65`, `event tags: 26`, draft `2020-12` — matches the test's pins and the header of the checksum file (`counts: 65 commands / 26 events`, `catalog-fnv1a64: debb80f9922a192a`).
- Baseline cross-check: cargo-test-workspace.txt commands section includes catalog_shape (4 tests, ok).

## 3. Findings
F-001 [S4]
Where: catalog_shape.rs:4-8 (vs 99-103, 176-180)
Code:
```
//! These pin the *shape* invariants doc 05 §2–§3 define: 61 command
//! entries (+ the issue #32 bookmark trio = 64) / 26 event entries (23
```
What is wrong: the module header says 64 commands; the actual count and the test bodies say 65 (`assert_eq!(schema_command_tags(&schema).len(), 65, ...)` with the w31.1 frame-channel explanation at L95-98 and L176-180). The header was not updated when the 65th command landed — the same "missed pin" failure mode the body comment (L312-313 in e2e.rs's sibling) admits happened elsewhere. Doc drift inside the very file whose job is pinning counts.
Confidence: HIGH — Verification: read + independent count.
F-002 [S4]
Where: catalog_shape.rs:229-243 (vs codegen.rs:145-147)
Code:
```
    let expected_catalog = format!("catalog-fnv1a64: {:016x}", catalog_checksum());
    assert!(
        text.lines().any(|line| line == expected_catalog),
```
What is wrong: the checksum file carries four digest/counts lines; this test verifies only `catalog-fnv1a64` and the counts line. The `schema-fnv1a64` and `typescript-fnv1a64` lines are written by `cargo xtask codegen` (crates/commands/src/catalog/codegen.rs:145-147) and read by NO test (rg across the repo: only the writer and this file's single-line check). Drift between the committed schema/typescript artifacts and their recorded digests is invisible to the test gate.
Confidence: HIGH — Verification: read + `rg 'schema-fnv1a64|typescript-fnv1a64'` -> writer only.

## 4. Per-function table (test file — helpers)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| workspace_root | 25-29 | REAL | — | manifest parent×2; breaks loudly if the crate moves |
| committed_schema | 31-36 | REAL | — | reads the committed schema from disk |
| schema_command_tags | 39-54 | REAL | — | extracts `cmd` consts from the oneOf; empty-on-malformed → count assert fails with message |
| schema_event_tags | 56-72 | REAL | — | same for `kind` |
(Test fns covered in §5; no production code in this file.)

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| schema_has_the_doc_05_structure_and_counts | YES | draft version, envelope sections, 65/26 tag counts, 19/46 class split, 13 pipeline-scoped — all against the committed file (independently reproduced by me) |
| every_id_parses_and_follows_the_grammar | YES | ALL len 65/26, per-id round-trip parse, kebab grammar per token; the None arm of check_grammar is a guaranteed-fail assert (correct idiom, not a tautology that can pass) |
| spec_table_covers_exactly_the_enum_variants | YES | three-way sorted set equality (schema tags == id enums == spec rows) — catches orphan AND missing rows on every surface |
| committed_checksum_recomputes | YES | recomputes fnv1a64 from live `catalog_checksum()` and requires the exact line in the committed file; a regenerated-but-not-committed or hand-edited catalog fails |
Mutation reasoning: deleting a `CommandId` variant breaks tests 2+3; adding a schema variant without an enum variant breaks 3; editing catalog_data without regenerating the checksum breaks 4; renaming an id to bad grammar breaks 2. No mocks of the thing tested; the artifacts are the real committed files.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "61 command entries (+ the issue #32 bookmark trio = 64)" | catalog_shape.rs:4-5 | FALSE (stale) | live count is 65 (w31.1 frame-channel; the file's own L100-103) — F-001 |
| "26 event entries (23 §3.2 rows + the issue #42 additive trio)" | catalog_shape.rs:5-7 | REAL | python3 count: 26 |
| "no orphan and no missing rows between the `CommandId`/`EventId` enums" | catalog_shape.rs:9-10 | REAL | test 3 passes (three-way equality) |
| "the committed checksum recomputes from the same source" | catalog_shape.rs:11-12 | REAL (catalog line only) | test 4 passes; schema/typescript lines unverified — F-002 |
| checksum file "never hand-edit. Regenerate with `cargo xtask codegen`" | schemas/generated/commands.checksum.txt:2 | REAL | @generated header consistent with codegen writer |

## 7. Dead / duplicated / unreachable code
None. All helpers used; no #[ignore]; no cfg gates.

## 8. Dependencies & flags
commands (CommandId/EventId/catalog::spec/catalog::codegen), serde_json, std. No features. The test reads repo-root artifacts via `default_paths` — ties the test to the codegen output layout (schemas/, ui/src/generated/), which is the point.

## 9. Open questions for the next agent
- Who (if anyone) verifies `schema-fnv1a64` / `typescript-fnv1a64` drift? If nothing does, the ui/src/generated/commands.ts artifact can drift from the schema with a green gate (F-002) — check the xtask codegen/CI lanes.
- The 19/46 class split and 13 pipeline-scoped pins duplicate knowledge that also lives in doc 05 tables; if a future command lands, three places must move (doc, schema, spec table). The test will catch two of the three.
