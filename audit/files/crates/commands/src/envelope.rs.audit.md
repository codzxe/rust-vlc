# AUDIT: crates/commands/src/envelope.rs

| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-21 |
| Lines / functions | 288 / 13 units (3 newtypes, 3 generic envelope types + 3 methods, result_wire module: 2 types + 2 fns, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, EXECUTED (call chain: `lumen` CLI → `apps/cli/src/run.rs:51` `use commands::{… WireEvent, WireRequest}` → `run.rs:507` `print_response(&commands::WireResponse)` — `WireResponse`/`WireRequest` are the envelope aliases (`lib.rs:57-58` re-export of `CommandRequest<Command>`/`CommandResponse<CommandResult>`); run.rs:26-27: "Machine mode (`--control-stdio`) always prints response envelopes as NDJSON". Also `apps/desktop-tauri/src/app.rs:22` ("commands::WireResponse = CommandResponse<CommandResult> — the SAME…"), `apps/control-server/src/{run,dispatch}.rs`, `apps/web/src/protocol.rs:248`. Executed: baseline run #3 `test-workspace.log.txt:1398-1410` — `tests/wire_forms.rs` 7/7 ok (byte-exact goldens through this file's serde attrs), `:1352-1355` envelope::tests 4/4 ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full `Read` of envelope.rs; `rg "result_wire\|CommandResponse\|CommandRequest" crates/commands apps` (consumer map: correlation.rs:5/59/137, tests/wire_forms.rs, tests/catalog_shape.rs:41, tests/catalog_emission_ledger.rs:49, control-server run.rs:100/dispatch.rs:25-26, web protocol.rs:248/393, desktop-tauri app.rs:15-22); `rg "requires_completion_event\|is_pipeline_scoped\|accepted_generation" -t rust` whole repo (caller census → F-001); `rg "CommandOutcome::Accepted" apps` (inline re-derivations: web host.rs:2010/2224, desktop shell.rs:589/597, web abi.rs:517); `sed -n '1,80p' crates/commands/src/correlation.rs` (production staleness filter re-derives scoping inline); `sed -n '1,130p' crates/commands/tests/wire_forms.rs` (wire contract tests + goldens); `sed -n '1330,1425p' audit/runs/baseline/test-workspace.log.txt` (execution evidence: envelope 4/4, wire_forms 7/7, correlation 10/10 ok); `head -c 700 tests/integration/first-vertical/evidence/transcripts/open.json` (transcript record shape — NOT the raw wire form; see §6) |

## 1. One-paragraph verdict
This is a small, high-discipline wire-envelope module whose risky part — the custom `result_wire` serde/schemars adapter that replaces std's `Result` shape with the documented `{"accepted"/"completed"}` / `{"error"}` union — is genuinely proven: the crate-level `tests/wire_forms.rs` pins the §5.1 examples byte-for-byte (camelCase fields, `gen` renames, skip-`session`-when-`None`, the untagged result union) and all 7 tests executed and passed in baseline run #3, and the same envelope is what `lumen --control-stdio` prints and the tauri/web/control-server hosts consume. Two defects, neither touching the wire form: the two semantic helpers `requires_completion_event()` and `is_pipeline_scoped()` are production-dead — every shipped consumer (including this crate's own `StalenessFilter`) re-derives the same predicate inline, which is precisely the drift the helpers exist to prevent — and the file's own test module never serializes anything, so a reader of envelope.rs alone would wrongly conclude the wire contract is tested there (it is tested one file over, and executed).

## 2. What actually works (verified)
— `result_wire` adapter — verified by trace AND execution: `ResultWire` is untagged with `Ok(O)` (CommandOutcome: externally tagged `{"accepted":…}`/`{"completed":…}`) tried before `Err(ErrorEnvelope)` (`{"error":…}`); the variant keys cannot collide with `"error"`, so the union is unambiguous for well-formed inputs; `serialize`/`deserialize` forward correctly (`Ok(Ok(outcome))`/`Ok(Err(envelope.error))` at 200-203). Executed: baseline `test-workspace.log.txt:1398-1410` — `failure_response_is_the_error_envelope`, `doc_05_response_example`, `doc_05_request_example`, `doc_05_event_example`, `completed_response_carries_the_result_variant`, `sessionless_event_omits_optional_fields`, `inline_args_are_camel_case_and_units_are_bare` all `ok`.
— `gen` rename + `skip_serializing_if` — pinned by the executed goldens: `{"requestId":42,"session":7,"gen":3,"result":{"accepted":{"session":7,"gen":4}}}` and the sessionless failure `{"requestId":7,"gen":2,"result":{"error":{…}}}` (wire_forms.rs:86-104, 109-126).
— `#[serde(bound(serialize…))]`/`#[serde(bound(deserialize…))]` on `CommandResponse` (101-102) — required and correct for the generic-with-custom-field combination (compiles and round-trips per executed goldens).
— `accepted_generation()` — production-consumed by `StalenessFilter::observe_response` (correlation.rs:59-67), executed in baseline (correlation::tests 10/10 ok, log:1339-1349) including `accepted_responses_raise_the_generation_watermark` and `error_responses_track_the_envelope_gen`.
— `RequestId` monotonic-from-1 contract — not enforced by the newtype (doc honestly says the engine never interprets it) but by `RequestIdAllocator` (correlation.rs:11-33), executed: `request_ids_are_monotonic_and_never_reused` ok.

## 3. Findings

### F-001 [S3]
Where: crates/commands/src/envelope.rs:91-94 and 136-142; refuting consumers crates/commands/src/correlation.rs:75-77, apps/web/src/host.rs:2010, apps/desktop/src/shell.rs:589, apps/web/src/abi.rs:517
Code:
```rust
    /// Whether the host must wait for a completion event (class A).
    pub fn requires_completion_event(&self) -> bool {
        matches!(self, CommandOutcome::Accepted { .. })
    }
```
and (correlation.rs:75-77 — the production staleness filter re-derives scoping instead of calling the sibling helper):
```rust
    pub fn filter_event<E>(&mut self, event: &Event<E>) -> bool {
        let (Some(session), Some(generation)) = (event.session, event.generation) else {
            return true; // not pipeline-scoped — never dropped
        };
```
What is wrong: Both semantic helpers are dead in production. `requires_completion_event()` has zero production callers (whole-repo `rg` — only its own tests); every shipped host re-derives class-A inline: `apps/web/src/host.rs:2010` `Ok(CommandOutcome::Accepted { .. }) => {}`, `apps/desktop/src/shell.rs:589/597` same, `apps/web/src/abi.rs:517` `matches!(outcome, commands::CommandOutcome::Accepted { .. })`. `is_pipeline_scoped()` is likewise only called by its own tests and one playback-engine test; the production consumer of the §4.2 scoping rule — `StalenessFilter::filter_event` in this very crate — re-implements the predicate inline.
Why it matters: The helpers encode doc-05 hard contracts ("class A waits for an event"; "pipeline-scoped = session AND generation"). With the canonical helpers unused and five-plus inline copies in shipped hosts, a future contract change (e.g. a third outcome class, or scoping gaining a third required field) updates the helper and its in-file tests while every inline copy silently keeps the old rule — the exact drift the vocabulary crate exists to prevent. Dead public API also bloats the review surface of the "most reviewed" crate.
What correct looks like: Either delete both helpers (and their tests) or make the consumers use them: `filter_event` should start with `if !event.is_pipeline_scoped() { return true; }`, and the hosts' class-A matches should call `requires_completion_event()`.
Confidence: HIGH — Verification: read (whole-repo caller census via rg; correlation.rs and three app call sites sed/read-verified).

### F-002 [S4]
Where: crates/commands/src/envelope.rs:207-288 (the file's own `mod tests`)
Code:
```rust
    #[test]
    fn response_shape_carries_all_correlation_fields() {
        let resp: CommandResponse<FakeResult> = CommandResponse {
            request_id: RequestId(42),
            session: Some(SessionId(7)),
            generation: Generation(3),
```
What is wrong: The in-file test module never calls serde. Its four tests pin Rust semantics only — `ids_are_plain_newtypes` asserts derive-generated `Ord`/`Eq` on newtypes (tautology-adjacent), `response_shape_carries_all_correlation_fields` constructs a struct and reads back the fields it just set (only the last assert, `accepted_generation` through `Result`, tests logic). The file's headline contract — the §5.1 wire form, the `gen` renames, the `result_wire` untagged union, flatten-vs-adjacent-tag composition — is tested entirely one file over in `tests/wire_forms.rs` (which does it byte-exactly, executed in baseline).
Why it matters: Not a coverage hole (wire_forms.rs is thorough and executed) but a mis-reading hazard: the module doc advertises the wire forms at length, and a maintainer running only `cargo test -p commands --lib` sees 4 green envelope tests that would all pass if every serde attribute in this file were deleted. The file's own suite proves nothing about the thing the file is for.
What correct looks like: Move one minimal serde round-trip (serialize a `CommandResponse`, assert the JSON, deserialize back) into this file's test module, or add a pointer comment: "wire forms are tested in tests/wire_forms.rs — keep goldens there."
Confidence: HIGH — Verification: read (all four test bodies 217-288; no serde_json usage in-file; wire_forms.rs execution cited from baseline log).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RequestId` (newtype) | 24-27 | REAL | — | pub u64, no invariant (by documented design — engine never interprets; allocator enforces monotonicity) |
| `SessionId` (newtype) | 29-34 | REAL | — | maps to engine session epoch (doc 03 §5) |
| `Generation` (newtype) | 36-42 | REAL | — | maps to media-types GenerationId conceptually; bumped at accept time (engine side) |
| `CommandRequest<C>` | 47-57 | REAL | — | `#[serde(flatten)]` of adjacently-tagged `Command` → `{"requestId":…,"cmd":…,"args":…}` — byte-pinned by executed golden (wire_forms.rs:70-73) |
| `CommandOutcome<R>::Accepted/Completed` | 62-77 | REAL | — | externally tagged, camelCase variants; `Accepted{session: Option, generation}` with `gen` rename; class A/S split per doc 05 §1.4 |
| `CommandOutcome::accepted_generation` | 84-89 | REAL | — | production-used (correlation.rs:67); `None` for Completed documented (envelope gen authoritative) |
| `CommandOutcome::requires_completion_event` | 91-94 | REAL | S3 (F-001) | dead in production; 5+ inline re-derivations |
| `CommandResponse<R>` | 99-112 | REAL | — | echoes request_id; `gen` rename; `session` skipped when None (pinned by sessionless failure golden); serde bounds correct for generic R |
| `Event<E>` | 119-134 | REAL | — | seq/session/gen/origin + flatten of internally-tagged EventKind; byte-pinned by executed event golden; `seq` monotonicity enforced by emitters (correlation tests), not here — correct split |
| `Event::is_pipeline_scoped` | 136-142 | REAL | S3 (F-001) | dead in production; filter_event re-derives inline |
| `result_wire::ResultWire` | 157-164 | REAL | — | untagged Ok-first union; variant keys ("accepted"/"completed") cannot collide with "error" — unambiguous for well-formed input |
| `result_wire::ErrorEnvelope` | 167-171 | REAL | — | `{"error": E}`; no `deny_unknown_fields` (mixed-key objects decode via the Err arm or fail untagged — behavior unpinned; open question §9) |
| `result_wire::serialize` | 177-187 | REAL | — | forwards `&O`/`&E` through ResultWire's derive — no hand-rolled JSON |
| `result_wire::deserialize` | 194-204 | REAL | — | `Ok(Ok)/Ok(Err)` mapping correct; untagged-miss error documented |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ids_are_plain_newtypes (218-223) | NO (derive-level) | asserts compiler-generated Ord/Eq/accessor behavior; would survive deleting every serde attr in the file |
| accepted_exposes_its_generation (226-239) | YES | pins both methods for both variants — a logic swap fails it |
| pipeline_scoping_requires_session_and_gen (242-267) | YES | all three combinations pinned |
| response_shape_carries_all_correlation_fields (270-287) | PARTLY | construct-and-read-back except the last assert (accepted_generation through Result::ok) — no serde, no wire assertion |
| (cross-file) tests/wire_forms.rs — 7 tests | YES | byte-exact goldens both directions incl. result_wire error arm, gen renames, skip-None; EXECUTED in baseline run #3 (log:1398-1410, 7/7 ok) |
| (cross-file) correlation.rs tests — 10 | YES | envelope consumers (watermarks, staleness, echo checks) EXECUTED (log:1339-1349, 10/10 ok) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `result` field … serializes a success as the bare outcome object … and a failure as `{"error": …}` — std's `Result` shape is NOT the documented wire form" | envelope.rs:9-13 (doc 05 §5.1/§1.6) | REAL | executed goldens: `{"result":{"accepted":{"session":7,"gen":4}}}` / `{"result":{"error":{…}}}` (wire_forms.rs:86-104/109-126, baseline log:1398-1410) |
| "the emitted JSON Schema and the runtime wire form cannot drift apart" | envelope.rs:148-150 | REAL (qualified) | both `#[serde(with)]` and `#[schemars(with)]` point at the same `result_wire::ResultWire` type, so the schema and the wire derive from one impl; the residual risk (editing one attribute without the other) is unpinned by tests — catalog_shape.rs checks schema structure only (baseline:1388-1396, 4/4 ok) |
| "wire object is `{ "requestId": 42, "cmd": "playback.seek", "args": { … } }` — exactly the §5.1 example" | envelope.rs:51-54 | REAL | byte-exact executed golden `{"requestId":42,"cmd":"playback.seek","args":{"target":{"kind":"relative","deltaMs":-10000}}}` |
| "RequestId … starting at 1, strictly increasing; reuse is forbidden" | envelope.rs:20-23 | REAL (enforced elsewhere) | newtype holds none of it (by design, doc says so); RequestIdAllocator enforces (correlation.rs:11-33), executed `request_ids_are_monotonic_and_never_reused` ok |
| Transcript evidence shape — NB: `tests/integration/first-vertical/evidence/transcripts/open.json` renders results as `{"form":"accepted",…}`, NOT the envelope's `{"accepted":{…}}` | transcript file (checked) | — (not this file's claim) | the transcript is a derived record schema ("transcript/1"), not the raw NDJSON envelope; do not cite transcripts as wire-form proof — wire_forms.rs is the proof |

## 7. Dead / duplicated / unreachable code
F-001 (both helpers, production-dead with inline re-derivations). Nothing else: every type is consumed (consumer map §header); `result_wire` is consumed via the attribute plus the schemars mirror and has direct tests in wire_forms.rs.

## 8. Dependencies & flags
`serde` (Serialize/Deserialize), `schemars` (JsonSchema) — declared in crates/commands/Cargo.toml (W01-A01/A02 manifest audits). No features, no cfg beyond `#[cfg(test)]`, no unsafe, no I/O. The `#[serde(bound)]` pair is the only generic-serde subtlety and is correct.

## 9. Open questions for the next agent
1. UNVERIFIED: `ErrorEnvelope` has no `deny_unknown_fields`; a mixed object like `{"accepted":{…},"error":{…}}` resolves by untagged order (Ok arm first) or fails — exact serde behavior for multi-key externally-tagged enums is not pinned by any test. A wire_forms fixture asserting the decode of a mixed-key `result` would close it.
2. F-001 fix should land with the correlation.rs owner (same crate): `filter_event` calling `is_pipeline_scoped` is a two-line change that kills the drift risk.
3. control-protocol agent: the frame layer carries these envelopes over the wire — confirm no second serialization boundary re-wraps `result` (web protocol.rs:393 mentions "the result_wire adapter under the transport tag" — one lane, verify the others).
