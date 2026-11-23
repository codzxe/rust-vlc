# AUDIT: crates/commands/tests/wire_forms.rs (test honesty audit)
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-23 |
| Lines / functions | 188 / 1 helper + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — the tests RUN: baseline cargo-test-workspace.txt:1534-1540 (7/7 ok) and I executed the prebuilt binary directly |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/wire_forms-596a955b745156da` -> `7 passed; 0 failed` (exit 0); `target/debug/lumen --control-stdio` cross-checks (file-3 audit); doc 05 §5.1 example comparison |

## 1. One-paragraph verdict
These are real, strong tests: every one pins a byte-exact compact-JSON golden AND round-trips it back through deserialization with an equality assert, so they fail on any rename, reorder, tag-shape, or information-loss regression in the shared serde path — mutation reasoning: change `gen`→`generation`, break `result_wire` into std's `{"ok":…}`, drop the `session` skip, or break the flatten, and a specific golden fails. The three doc-05 §5.1 examples are faithfully pinned (I compared them against docs/05-command-layer.md:471-476: identical keys, order, and values; `position: 63000000`, `seekKind: "command"` included). The defects are all in the file's own commentary and its coverage perimeter: the module doc calls the envelopes "internally-tagged" when `Command` is adjacently tagged (`cmd`/`args` — envelope.rs:51-53 says so; only `EventKind` is internally tagged), "BYTE FOR BYTE" is false against the doc's pretty-printed examples (the pinning is content+key-order exact, whitespace-normalized), and there is not a single negative test — nothing pins that unknown command ids, malformed args, or missing `requestId` are rejected (I verified those rejections currently hold by hand through the shipped binary, so the behavior is real but has no regression net). Also `schemaMeta`/`restartRequired`: `restartRequired` is exercised; `schemaMeta` is named in the doc comment but appears in NO golden in this file — the comment promises a fixture it does not contain.

## 2. What actually works (verified)
— verified by `./target/debug/deps/wire_forms-596a955b745156da` -> `test result: ok. 7 passed; 0 failed` (exit 0); baseline :1534-1540 corroborates.
— verified by cross-checking the executed shipped-binary wire forms (file-3 audit: `{"requestId":1,"session":1,"gen":0,"result":{"accepted":{"session":1,"gen":0}}}` etc.) against the goldens here: same shapes — the goldens pin what production actually emits, not an aspirational form.
— verified by reading doc 05 §5.1 (docs/05-command-layer.md:471-476): request/response/event examples match the three `doc_05_*` goldens key-for-key.

## 3. Findings
F-001 [S4]
Where: crates/commands/tests/wire_forms.rs:3-4
Code:
```
//! The one serde path every host shares: camelCase field names,
//! internally-tagged envelopes (`"cmd"`/`"kind"`), kebab-case variant
```
What is wrong: `Command` is ADJACENTLY tagged (`"cmd"`/`"args"` — crates/commands/src/envelope.rs:51-53 states it and the goldens here prove it: `"args":{...}` is a separate key); only `EventKind` is internally tagged (`"kind"` with payload fields beside it). The comment repeats doc 05 §5.1's own terminology error ("internally-tagged enums (`"cmd"`, `"kind"`)"). No behavioral impact — the goldens pin the true shapes — but the comment teaches a reader the wrong model. Confidence: HIGH — Verification: read (envelope.rs + goldens + doc 05:469).
F-002 [S4]
Where: crates/commands/tests/wire_forms.rs:5-7
Code:
```
//! tags. The doc's own §5.1 examples are pinned BYTE FOR BYTE (key
//! order included — serde emits struct fields in declaration order, the
//! doc examples follow the same order), plus representative payload
```
What is wrong: the doc's examples are pretty-printed with spaces (`{ "requestId": 42, ... }`, doc 05:471); the goldens are compact (`{"requestId":42,...}`). The pinning is exact in keys, order, and values — not byte-identical to the doc text. Overstated claim, trivial substance. Confidence: HIGH — Verification: read (doc 05:471-476 vs goldens).
F-003 [S4]
Where: crates/commands/tests/wire_forms.rs (whole file — absence)
Code:
```
fn round_trip<T>(value: &T, golden: &str) -> Result<(), Box<dyn Error>>
```
What is wrong: the helper only proves the happy path. No test asserts a REJECTION: unknown command id, wrong arg type, missing `requestId`, `args` on a unit variant, unknown top-level field. I executed those through the shipped binary (`{"cmd":"playback.play","args":{}}` -> typed invalid-argument; missing `requestId` -> missing-field error; `evilField` -> silently ignored per envelope F-001), so today's behavior holds — but a regression to fully permissive parsing would pass this suite untouched. Doc 05 §5.4's "unknown fields/IDs rejected" has no regression net anywhere in crates/commands/tests (catalog_shape.rs checks schema structure/counts, not wire rejection). What correct looks like: a `rejected_wire_forms` test with a table of malformed inputs asserting typed errors. Confidence: HIGH — Verification: ran (rejections via lumen binary) + read (no negative asserts in any of the 4 test files).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| round_trip (helper) | 34-44 | REAL | — | serialize→exact-assert→deserialize→equality; strongest pattern available; "Serde failures propagate as test errors" is true (`?` on to_string/from_str) |
| doc_05_request_example | 47-59 | REAL | — | golden == doc 05:471; catches flatten/camelCase/kebab/deltaMs drift |
| doc_05_response_example | 63-78 | REAL | — | golden == doc 05:473; catches `gen` rename + accepted shape |
| doc_05_event_example | 82-98 | REAL | — | golden == doc 05:475-476 incl. position/seekKind; catches event flatten + origin |
| failure_response_is_the_error_envelope | 102-118 | REAL | — | pins `{"error":{stage,code,message,details}}` and NOT std `{"ok":…}` — the result_wire negative contract |
| completed_response_carries_the_result_variant | 123-138 | REAL | — | internally-tagged result `kind`, kebab tag, `restartRequired` camelCase |
| sessionless_event_omits_optional_fields | 142-157 | REAL | — | pins skip_serializing_if for session/gen/origin; `deviceId` camelCase |
| inline_args_are_camel_case_and_units_are_bare | 161-188 | REAL | — | 3 round-trips: `toIndex`, `delayMs`, bare unit (no `args` key) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| doc_05_request_example | YES | byte-exact golden: any rename/reorder/tag change fails |
| doc_05_response_example | YES | `gen` rename, session presence, accepted nesting all pinned |
| doc_05_event_example | YES | seq/session/gen/origin/kind/position/seekKind all pinned in one string |
| failure_response_is_the_error_envelope | YES | swapping result_wire for std Result serde changes the shape and fails |
| completed_response_carries_the_result_variant | YES | kind tag + camelCase fields pinned |
| sessionless_event_omits_optional_fields | YES | removing skip_serializing_if emits `"session":null` and fails |
| inline_args_are_camel_case_and_units_are_bare | YES | snake_case leak, or an `args` key on unit variants, fails |
| (absent) negative tests | — | nothing pins rejections (F-003); rejections verified only by my manual execution |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc §5.1 examples pinned "BYTE FOR BYTE" | wire_forms.rs:5-6 | SHALLOW | content+order exact; doc text is pretty-printed so not byte-identical (F-002) |
| "internally-tagged envelopes (`"cmd"`/`"kind"`)" | wire_forms.rs:4 | FALSE (for `cmd`) | Command is adjacently tagged (envelope.rs:51-53; goldens show separate `args` key) (F-001) |
| fixtures exercise `deltaMs`, `seekKind`, `deviceId`, `toIndex`, `schemaMeta`, `restartRequired` | wire_forms.rs:8-10 | SHALLOW | 5 of 6 present in goldens; `schemaMeta` appears only in that doc comment — zero golden/test-body hits in the file (rg) |
| goldens == production wire forms | implied by file purpose | REAL | executed lumen output matches golden shapes (file-3 audit §2) |

## 7. Dead / duplicated / unreachable code
- None. All 7 tests run (executed; baseline). No #[ignore], no cfg gates, no tautologies.
- The only "dead" thing is the promised-but-absent `schemaMeta` fixture (§6).

## 8. Dependencies & flags
- Test uses only the crate's public API (commands::*) + serde/serde_json — no testkit dependency, no features. Runs in the per-package test lane (baseline :1534-1540).

## 9. Open questions for the next agent
- Where is `schemaMeta` actually pinned? If nowhere, the rename rule for that field has no golden anywhere (check ui/packages/commands TS types and catalog tests).
- If envelope-level strictness is added (envelope.rs F-001), add the companion negative tests here (F-003) in the same change so the contract is pinned from both sides.
