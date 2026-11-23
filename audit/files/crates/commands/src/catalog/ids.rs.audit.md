# AUDIT: crates/commands/src/catalog/ids.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-23 |
| Lines / functions | 301 / 11 items (1 error type + 2 impls + 1 macro + 2 enums (65+26 variants) + 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen-cli` main → apps/cli/src/dispatch.rs:30-38 (`use commands::{… CommandId …}`; `pub const SERVED: &[CommandId] = &[CommandId::SessionOpen, …]`); also apps/web/src/host.rs:762-774 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/commands-84df364c190318ab ids` → 7 passed, EXIT=0 (all 4 ids tests incl. `catalog_sizes_match_doc_05`); `./target/debug/deps/catalog_shape-a6be788a761b3aed` → 4 passed EXIT=0 and `catalog_shape-f10bbbc4930de6e2` → 4 passed EXIT=0 (grammar + three-way set pin); `./target/debug/deps/catalog_drift-eaf83ff7e4d39d14` → 2 passed EXIT=0 |

## 1. One-paragraph verdict
This is the cleanest file audited in this domain: a 33-line macro generates both id enums from single `(Variant => "wire-id")` tables, producing `as_str`/`Display`/`FromStr`/`ALL`/`ALL_IDS` with no hand-duplicated match arms (the exact drift trap protocol.rs's `ProtocolErrorCode` falls into). Every claim is executable and executed: I ran the ids tests (7 passed), the catalog shape tests that pin grammar (`<area>.<verb-noun>`, kebab-case tokens) and three-way set equality between the schema's serde tags, `CommandId::ALL_IDS`, and `catalog_data()` (4 passed ×2 binaries), and the golden-file drift tests (2 passed). Unknown ids are rejected, never guessed — pinned including hostile forms (trailing space, uppercase, empty, area-only). The 65/26 counts are test-pinned. Two hygiene findings only: the `CommandId` doc comment undercounts (says 61+3=64, actual is 65 — the w31.1 `session.set-frame-channel` addition its own test comment documents), and the header's "the id list exists in exactly one place" is overstated (Command's 65 `#[serde(rename)]` literals are a second hand-maintained copy — contained only by the shape test).

## 2. What actually works (verified)
— verified by `./target/debug/deps/commands-84df364c190318ab ids` → `test result: ok. 7 passed; 0 failed` covering `doc_05_examples_round_trip`, `every_id_round_trips_and_is_unique`, `catalog_sizes_match_doc_05` (65/26), `unknown_ids_are_rejected_not_guessed`.
— verified by `./target/debug/deps/catalog_shape-a6be788a761b3aed` (and the second hash) → 4 passed: `every_id_parses_and_follows_the_grammar` + `spec_table_covers_exactly_the_enum_variants` (schema tags == ALL_IDS == spec rows).
— verified by `./target/debug/deps/catalog_drift-eaf83ff7e4d39d14` → 2 passed (committed artifacts byte-identical to regeneration).

## 3. Findings
F-001 [S4]
Where: crates/commands/src/catalog/ids.rs:90-92 (vs :279 and tests/catalog_shape.rs:179)
Code:
```
    /// One v1 catalog command (doc 05 §2.1 — 61 entries + the issue #32
    /// bookmark trio; the §2.3–2.12 tables define each entry's payload,
    /// class and errors).
```
vs the same file's own test:
```
        // Doc 05 §2.1 lists 61 command entries; issue #32 adds the three
        // media.*-bookmark commands (64); w31.1 adds the shell-owned
        // frame-channel bind (65 total).
        assert_eq!(CommandId::ALL.len(), 65);
```
What is wrong: the enum's doc comment counts 64 (61 + issue #32 trio); the actual and test-pinned count is 65 (w31.1's `session.set-frame-channel` is missing from the doc). Secondary: the test NAME `catalog_sizes_match_doc_05` is wrong — D-017 (docs/gates/phase1-exit.md:61) fixes doc 05's authority at 61/23; the body asserts 65/26, i.e. doc-05-plus-three-addenda, not doc 05. Why it matters: doc-code drift in the one file whose entire purpose is being the single stable reference; a reader reconciling against doc 05 or the gate record gets three different numbers (61, 64, 65). What correct looks like: "61 entries + issue #32 trio + the w31.1 frame-channel bind (65)". Confidence: HIGH — Verification: read + executed (`catalog_sizes_match_doc_05` passed with 65).
F-002 [S4]
Where: crates/commands/src/catalog/ids.rs:11-15 (vs crates/commands/src/catalog/command.rs:77 and 64 more renames)
Code:
```
//! `FromStr` (rejecting unknown ids with [`UnknownId`]) and an `ALL`
//! slice — so the id list exists in exactly one place and the spec
//! table ( [`crate::catalog::spec`] ) + the checksum consume the same
//! data.
```
vs the twin's hand-written copies:
```
    /// `session.open` — open a source as a new session (class A).
    #[serde(rename = "session.open")]
    SessionOpen {
```
What is wrong: "exists in exactly one place" is false — every wire id string is written a second time as a `#[serde(rename)]` literal on `Command`'s 65 variants (and `EventKind`'s 26). The spec table consumes `CommandId` (true), but the payload twin does not. Why it matters: the safety claim that lulls reviewers; a rename added on one side only would compile fine. Mitigation (why this is S4, not S3): `spec_table_covers_exactly_the_enum_variants` (tests/catalog_shape.rs:190-219) asserts set-equality between the generated schema's serde tags and `CommandId::ALL_IDS` — I ran it, 4 passed — so drift cannot ship. What correct looks like: generate the serde renames from the same table (or soften the doc claim to name the twin and the pinning test). Confidence: HIGH — Verification: read + executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| UnknownId + Display + Error | 22-31 | REAL | — | message pinned by test (line 298) |
| stable_id_enum! (macro) | 35-87 | REAL | — | generates enum/ALL/ALL_IDS/as_str/Display/FromStr; no duplicated match arms; executed via both enums |
| CommandId (65 variants) | 89-173 | REAL | S4 | count doc stale (F-001); reachable via lumen-cli dispatch.rs:38 |
| EventId (26 variants) | 175-225 | REAL | — | doc count 23+3=26 correct |
| tests::doc_05_examples_round_trip | 232-252 | REAL | — | §2.1's four worked examples |
| tests::every_id_round_trips_and_is_unique | 254-270 | REAL | — | uniqueness across all 91 ids |
| tests::catalog_sizes_match_doc_05 | 272-281 | REAL | S4 | asserts 65/26 (correct values; name overstates, F-001) |
| tests::unknown_ids_are_rejected_not_guessed | 283-300 | REAL | — | 6 hostile forms × both enums |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| doc_05_examples_round_trip | YES | exact string equality both directions |
| every_id_round_trips_and_is_unique | YES | a duplicate id or broken mapping fails (`seen.contains`) |
| catalog_sizes_match_doc_05 | YES | hard 65/26 — adding/removing an entry fails |
| unknown_ids_are_rejected_not_guessed | YES | any lenient parse (trim/case-fold) fails; error text pinned |
| (external) every_id_parses_and_follows_the_grammar | YES | grammar violations (uppercase, empty tokens) fail — catalog_shape.rs:138-183, executed by me |
| (external) spec_table_covers_exactly_the_enum_variants | YES | the Command↔CommandId drift tripwire — catalog_shape.rs:190-219, executed by me |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the id list exists in exactly one place" | ids.rs:13-14 | FALSE (contained by test) | Command's 65 serde renames (command.rs:77+); pinning test executed (F-002) |
| CommandId = "61 entries + issue #32 trio" | ids.rs:90-91 | FALSE | ALL.len()==65, test-pinned and executed (F-001) |
| "unknown ids are rejected, never guessed" | ids.rs:20-21 | REAL | `unknown_ids_are_rejected_not_guessed` passed (my run) |
| D-017: catalog count 61/23, doc 05 authority | docs/gates/phase1-exit.md:61 | STALE vs code | code is 65/26 (addenda documented in test comment + phase7-exit.md:52 shows 64/26 mid-history) |

## 7. Dead / duplicated / unreachable code
No dead code. `ALL_IDS` used by catalog_shape.rs:196-197 (executed). Duplication: `catalog_sizes_match_doc_05` duplicates catalog_shape.rs:176-181's count asserts — acceptable belt-and-braces. Reachability PROVEN: lumen-cli (release lane) → apps/cli/src/dispatch.rs:30,38 (`CommandId` import + `SERVED` const).

## 8. Dependencies & flags
No external deps (std fmt/str only); no features; no unsafe; no ignores. Consumed by spec.rs (tables/checksum — the checksum feeds `EngineInfo.catalog_version` on every host), control-protocol policy, control-server dispatch, lumen-cli dispatch, apps/web host.

## 9. Open questions for the next agent
- Should D-017 (docs/gates/phase1-exit.md, docs/ROADMAP.md:114) be amended to record the 61→64→65 and 23→26 addenda chain, or does doc 05 itself carry the additive rows now?
- `session.set-frame-channel` is shell-owned (command.rs:89-94 says the engine never sees it): does the control-protocol policy classify it correctly (it is NOT in policy.rs's sensitive list — verify intent)?
