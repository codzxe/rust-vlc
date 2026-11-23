# AUDIT: crates/commands/tests/catalog_emission_ledger.rs
| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-24 |
| Lines / functions | 399 / 2 consts + 2 tests + 3 helpers + 1 struct |
| File verdict | REAL (intra-file contract + engine-proven rows) with a false cross-file "tripwire" claim |
| Reachable from shipped entry point? | N/A — test binary; runs in the test lane (`cargo test -p commands --test catalog_emission_ledger`), which I ran: 2 passed, exit 0. Not linked into shipped binaries (test files never are). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p commands --test catalog_emission_ledger` -> 2 passed, exit 0; `cargo test -p playback-engine --lib events::tests::the_completeness_map_partitions_the_catalog_exactly` -> 1 passed, exit 0; rg per-EventKind emission-site counts in playback-engine src+tests; Read-tool quote verification. |

## 1. One-paragraph verdict
Hostile question: is the emission ledger a real contract or paperwork? Answer: it is a real contract INSIDE its own file and its "emitted" claims are genuinely engine-proven — but it is NOT the cross-file contract it claims to be, and it is knowingly stale by one row. What is real: the partition test machine-checks that all 26 EventIds are covered exactly once with non-empty owners (I ran it); every one of the 14 "emitted-now" rows has live assertion sites in the playback-engine test suite (verified per-kind: e.g. events_contract.rs:1054 asserts the duration value, hotplug.rs:609 asserts the video backend report); and the payload-shape test pins the exact sorted JSON key set of each emitted wire payload against doc 05 §3.2. What is paperwork: the doc claims "the two files must not drift; the partition test below is the tripwire" — false. The partition test checks only this file's own two arrays; the engine-side twin (`events.rs` `CATALOG_EMISSIONS`, itself machine-checked at 15 emitted / 11 later) disagrees with this ledger (14 / 12) on `tracks.changed` today, both tests green, nothing cross-checks them, and the engine genuinely emits that event in production (session.rs:2310/2715 -> event.rs:585 wire mapping). The file admits the divergence in its header and blames "the owning card's bookkeeping" — honest prose, uncorrected contract.

## 2. What actually works (verified)
- `cargo test -p commands --test catalog_emission_ledger` -> `test result: ok. 2 passed; 0 failed` (partition + payload shapes).
- `cargo test -p playback-engine --lib events::tests::the_completeness_map_partitions_the_catalog_exactly` -> 1 passed — the engine-side twin is itself machine-checked (asserts `ENGINE_EMITTED_ROWS = 15`, `later = 11`, bijection onto `EventId::ALL` in doc-05 order).
- Emitted-bucket proof cross-check (rg over crates/playback-engine/tests): all 14 EventKind variants appear with assertion semantics — SessionStateChanged 25 refs, PlaybackError 28, TimingSeekCompleted 14, AudioDeviceLost 12, TimingDurationKnown asserted-by-value at events_contract.rs:1054, VideoBackendChanged asserted at hotplug.rs:609, TracksListReady/MediaInfoReady matched at contract_suite.rs:766/774.
- Later-bucket spot check: 11 of 12 "wired-by-later" kinds have zero production emission paths (only the exhaustive `wire_event_id` match arm, events.rs:816-840, and test tables). The 12th (TracksChanged) is the documented divergence (F-002).

## 3. Findings
F-001 [S3]
Where: crates/commands/tests/catalog_emission_ledger.rs:121-123
Code:
```
/// The 12 rows honestly owned by later cards — the same notes the
/// engine-side module-doc table carries (the two files must not
/// drift; the partition test below is the tripwire).
```
What is wrong: two false statements. (1) The engine side is no longer a "module-doc audit table" — `crates/playback-engine/src/events.rs:646` defines `pub const CATALOG_EMISSIONS` with its own partition test (events.rs:1956+). (2) The partition test in THIS file cannot detect drift between the two files: it iterates `EventId::ALL` and counts membership in this file's two arrays only. Proof the tripwire fails at its one job: the files HAVE drifted (15/11 vs 14/12) and both partition tests are green (I ran both). Why it matters: a reader trusting the comment believes cross-ledger drift is machine-prevented; it is prevented by nothing. What correct looks like: one shared test (in whichever crate has both as dev-deps, or via a generated common artifact) asserting both maps agree row-by-row, or delete the claim. Confidence: HIGH — Verification: ran both tests + read both files.
F-002 [S3]
Where: crates/commands/tests/catalog_emission_ledger.rs:129-132 (vs crates/playback-engine/src/event.rs:582-588, session.rs:2310)
Code:
```
    Row {
        id: EventId::TracksChanged,
        owner: "3.A3 (track-selection commands; no engine-side track-loss trigger exists in v0)",
    },
```
What is wrong: the row sits in WIRED_BY_LATER ("the catalog row exists (frozen data) but no engine trigger exists yet") while the engine emits `tracks.changed` in production today: session.rs emits `EngineEventKind::TrackSelectionChanged` (lines 2310, 2715) and event.rs:585 maps it to wire `EventKind::TracksChanged`. The owner note is technically precise about *track-loss* but the bucket classification is false for selection-driven emission — stale since 3.A3 by the file's own admission (lines 35-40). Why it matters: the ledger's headline arithmetic ("14 emitted / 12 later", line 26) understates real engine behavior (15/11); anyone using this file as the index of "what fires today" gets one row wrong. What correct looks like: move the row to EMITTED with owner "3.A3 selection terminal" and a loss-only sub-note, reconciling with the engine map. Confidence: HIGH — Verification: read (emission chain cited) + ran (engine-side test asserting TracksChanged::Emitted passes).
F-003 [S4]
Where: crates/commands/tests/catalog_emission_ledger.rs:175-179
Code:
```
/// The ledger partitions the catalog exactly: every one of the 26
/// `EventId`s (doc 05 §3.2's 23 + the issue #42 additive trio) appears
/// in exactly one bucket, no bucket mentions an id
/// that is not in the catalog (typo guard), and the arithmetic is the
```
What is wrong: the "typo guard" is credited to the test but is actually enforced by the compiler — `Row.id` is typed `EventId`, so a bucket cannot name a non-catalog id; the test loop over `EventId::ALL` is structurally incapable of seeing foreign ids. Minor credit misassignment (test claims a guarantee the type system provides). The owner-note check is likewise weak: `row_has_empty_owner` only rejects blank/whitespace — "TODO" would pass. What correct looks like: drop the typo-guard clause or move it to a compile-time note. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct Row | 53-58 | REAL | — | id + free-text owner. |
| const EMITTED (14 rows) | 62-119 | REAL | — | every row engine-proven (see §2); owners are wave tags. |
| const WIRED_BY_LATER (12 rows) | 124-173 | REAL except one row | S3 | TracksChanged mis-bucketed (F-002); other 11 verified unemitted. |
| the_ledger_partitions_the_catalog_exactly | 180-204 | REAL | S3 | genuine intra-file bijection check; cannot catch wrong-bucket rows or cross-file drift (F-001). |
| row_has_empty_owner | 206-209 | REAL | S4 | blank-only check (F-003). |
| kind_keys | 212-231 | REAL | — | serializes one Event, strips envelope keys, sorts — correct; envelope really serializes `gen` (test passes). |
| sample_info | 236-268 | REAL | — | representative MediaInfo fixture. |
| emitted_rows_carry_the_doc_05_payload_shapes | 275-399 | REAL | — | key-set pinning per emitted row; fails on rename/add/skip; keys-only (types not checked) — doc says exactly that. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_ledger_partitions_the_catalog_exactly | YES for: new EventId without ledger row (sum), duplicate row, blank owner. NO for: wrong bucket (F-002 is live proof — green with a mis-bucketed row), cross-file drift (F-001 live proof). | self-partition arithmetic over EventId::ALL. |
| emitted_rows_carry_the_doc_05_payload_shapes | YES for wire-key drift (rename, added field, serde(skip)) on any of the 14; NO for value/type changes (keys only). | serializes a representative payload per row and compares sorted key sets against doc-05 columns. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every catalog EventId is accounted for exactly once" | lines 5-6 | REAL | partition test, ran green. |
| "emitted-now — the playback engine produces the event with its doc-05 payload, proven live by the contract suite" | lines 9-13 | REAL | all 14 kinds have assertion sites in playback-engine tests (rg counts + spot reads, §2). |
| "the two files must not drift; the partition test below is the tripwire" | lines 121-123 | FALSE | both files' tests green while they disagree 14/12 vs 15/11 (F-001). |
| "Arithmetic ... 14 emitted / 12 later" | line 26 | FALSE (stale) | engine emits 15 today (TracksChanged); engine-side test asserts 15/11 (F-002). |
| "Known divergence, recorded for the Lead ... since 3.A3" | lines 35-40 | REAL | accurate self-report; the divergence is real and open. |

## 7. Dead / duplicated / unreachable code
None dead. Duplication of the ledger concept across two crates is the architecture's own choice (dev-edge cycle avoidance, lines 19-24) — defensible, but it is exactly what makes F-001 possible: two hand-maintained maps, no shared check.

## 8. Dependencies & flags
Test-only; uses commands crate public API + serde_json (dev-dependency). No features, no unsafe.

## 9. Open questions for the next agent
- Who owns closing the tracks.changed reconciliation — the ledger says "the owning card's bookkeeping, not 7.C1's"; if that card (3.A3) is considered closed, the divergence is orphaned and should be escalated to the Lead.
- events.rs's `is_engine_emitted` API (events.rs:798-800) — is it consumed by any host (control-server/desktop) to filter events, or is it query-only surface? If consumed, the engine-side map becomes runtime behavior, raising the cost of the divergence.
