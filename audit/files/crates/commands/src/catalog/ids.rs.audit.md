# AUDIT: crates/commands/src/catalog/ids.rs
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-21 |
| Lines / functions | 301 / 3 types + 1 macro (5 generated surfaces × 2 enums) + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for `as_str`/`ALL` (call chain: `lumen` CLI → `apps/cli/src/dispatch.rs:142-144` `SystemListCommands` → `crate::host::command_listings()` → `spec.rs catalog_data()` rows carrying `CommandId` → `CommandListing.id`). `FromStr` production use is control-protocol `policy.rs:190,261` (operator-config validation + per-request `request_command_id`) — real and e2e-tested, but its only consumer binary (`lumen-control-server`) is NOT in the packaging payload (stage-linux-payload.sh ships BIN_CLI + BIN_TAURI only), so `FromStr` is unreachable from the two shipped bins. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n` (all quotes verified); `awk` variant counts (CommandId=65, EventId=26 — match the pinned asserts); `rg -n "ALL_IDS|parse::<CommandId>" crates/ apps/` (consumers); `rg -n "BIN_" scripts/packaging/` (payload contents); baseline `rg -n "catalog::ids::tests" test-workspace.log.txt:1322-1325` — 4/4 ok |

## 1. One-paragraph verdict
A macro-generated stable-id layer done right: one `(Variant => "wire.id")`
table per enum mechanically produces the enum, `ALL`, `ALL_IDS`, `as_str`,
`Display`, and a closed `FromStr` that rejects unknown ids with a typed error —
the id list exists in exactly one place, and four in-file tests plus the
catalog_shape suite (grammar, 65/26 counts, schema↔ids↔spec-table three-way
set equality) all executed and passed in the baseline (log:1322-1325, 1391,
1394). Unknown-id handling is closed (case/whitespace/prefix variants all
rejected — no guessing), uniqueness is asserted, and the drift risk against
`Command`'s serde tags is covered by `spec_table_covers_exactly_the_enum_variants`.
The only defect found is a stale doc header that undercounts the catalog. The
`FromStr` surface's sole production consumer is the control-server binary,
which the packaging payload does not ship — a wiring fact, recorded below, not
a code defect.

## 2. What actually works (verified)
— Verified by baseline `test-workspace.log.txt:1322-1325`: all 4 tests ok
(`catalog_sizes_match_doc_05`, `doc_05_examples_round_trip`,
`every_id_round_trips_and_is_unique`, `unknown_ids_are_rejected_not_guessed`).
— Verified by baseline `test-workspace.log.txt:1391,1394`: `every_id_parses_and_follows_the_grammar`
and `spec_table_covers_exactly_the_enum_variants` ok — the schema tags,
`CommandId::ALL_IDS`, and `catalog_data()` rows are the same 65-element set
(sorted-equality asserts at catalog_shape.rs:215-228), so ids.rs cannot drift
from the payload enum without a committed test failing.
— Verified by baseline `test-workspace.log.txt:1521` (`a_typoed_sensitive_grant_is_a_typed_startup_error ... ok`):
the `FromStr` rejection path runs for real in the control-server e2e startup.
— Verified by `awk` count: 65 CommandId / 26 EventId variants, matching the
pinned asserts at 279-280.

## 3. Findings

### F-001 [S4] `CommandId` doc header undercounts the catalog (64 vs 65)
Where: crates/commands/src/catalog/ids.rs:90-92
Code:
```
    /// One v1 catalog command (doc 05 §2.1 — 61 entries + the issue #32
    /// bookmark trio; the §2.3–2.12 tables define each entry's payload,
    /// class and errors).
```
What is wrong: "61 entries + the issue #32 bookmark trio" = 64, but the enum
has 65 variants — the w31.1 `SessionSetFrameChannel` addition (present at line
100) is omitted from the header. The test comment at 274-278 gets it right
("w31.1 adds the shell-owned frame-channel bind (65 total)").
Why it matters: the header is the first thing a reader consults to size the
catalog; it disagrees with the pinned count three screens below it.
What correct looks like: "…61 entries + the issue #32 bookmark trio + the
w31.1 frame-channel bind (65)".
Confidence: HIGH — Verification: read + counted (awk: 65).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `UnknownId` + `Display` + `Error` | 22-31 | REAL | — | Typed parse error; message pinned by test at 298. |
| `stable_id_enum!` (macro) | 35-87 | REAL | — | Hygienic; generates `#[non_exhaustive]` enum + `ALL`/`ALL_IDS` parallel arrays + `const fn as_str` + `Display` + closed `FromStr` (linear `if` chain, 65 compares worst case — irrelevant at control-request rates; per-request use is `policy.rs:261`). |
| `CommandId` (65 variants, generated impls) | 89-173 | REAL | — | `as_str` reachable from shipped `lumen` CLI (`system.list-commands`); `FromStr` only via control-server (not in payload — see reachability field). Counts pinned by two independent tests (here:279 and catalog_shape.rs:180). |
| `EventId` (26 variants, generated impls) | 175-225 | REAL | — | Includes the 7.C1/7.D1 additive trio, documented at 176-179 (complete and accurate, unlike the CommandId header). Reachable via event serialization on every host. |
| `tests::doc_05_examples_round_trip` | 232-252 | REAL | — | §2.1's four worked examples; executed+passed (log:1323). Mutation: wrong id string → assert_eq fails. |
| `tests::every_id_round_trips_and_is_unique` | 254-270 | REAL | — | Covers ALL × both enums; duplicate detection (260, 267). Executed+passed (1324). O(n²) `contains` — fine for 65. |
| `tests::catalog_sizes_match_doc_05` | 272-281 | REAL | — | Pins 65/26 with provenance comment; executed+passed (1322). |
| `tests::unknown_ids_are_rejected_not_guessed` | 283-300 | REAL | — | 6 adversarial forms (whitespace, case, prefix, empty, no-dot); executed+passed (1325). Mutation: any aliasing/normalization in FromStr → fails. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doc_05_examples_round_trip | YES | Exact id↔variant equality both directions. |
| every_id_round_trips_and_is_unique | YES | All 91 ids parsed back; duplicates abort. |
| catalog_sizes_match_doc_05 | YES | Hard 65/26 pins — any add/remove fails. |
| unknown_ids_are_rejected_not_guessed | YES | Any acceptance of the 6 malformed forms panics via assert. |
All four executed in the baseline run (log:1322-1325) — these are executed
proofs, not paper tests.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "IDs are stable forever within major version 1 (§6); the enums are `#[non_exhaustive]` and hosts must map unknown IDs to `invalid-argument`" | ids.rs:7-9 | REAL | `#[non_exhaustive]` generated (47); rejection pinned by executed test; CLI serde path rejects unknown `cmd` tags (command.rs test, baseline 1316). |
| "the id list exists in exactly one place and the spec table + the checksum consume the same data" | ids.rs:11-15 | REAL | spec.rs:135/153 carry `CommandId`/`EventId`; catalog_shape.rs:215-228 set-equality (executed, log:1394). |
| "61 entries + the issue #32 bookmark trio" (header) | ids.rs:90-92 | FALSE (undercount) | 65 variants counted; test comment 274-278 states the third addition. F-001. |
| "23 entries + the 7.C1/7.D1 additive trio … 26 total" | ids.rs:176-179 | REAL | 26 counted; pinned at 280. |

## 7. Dead / duplicated / unreachable code
- None dead in-file. Note: `FromStr`'s only production consumer is
  `control-protocol::policy` → `lumen-control-server`, which the Linux
  packaging payload does not ship (stage-linux-payload.sh stages BIN_CLI +
  BIN_TAURI only). Not dead code (the binary is built, e2e-tested in baseline),
  but unreachable from the two shipped bins per this audit's reachability rule.
- Count pins duplicated here and in catalog_shape.rs — intentional double
  guard, not drift risk (both executed).

## 8. Dependencies & flags
- std-only (`fmt`, `str::FromStr`); no serde on the id enums themselves (the
  payload twin `Command` owns the wire) — clean layering. No feature gates.

## 9. Open questions for the next agent
- Is `lumen-control-server` intended to ship? If yes, the packaging payload
  (stage-linux-payload.sh) omits it — an S2-class packaging gap for whoever
  audits scripts/packaging. If no, `FromStr` + policy remain dev-surface only.
- token.rs (my next file) sits in control-protocol — same payload-reachability
  question applies to the whole crate.
