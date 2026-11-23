# AUDIT: crates/commands/src/envelope.rs
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-23 |
| Lines / functions | 288 / 14 prod items (3 newtypes, 3 generic structs, 1 enum + 2 methods, 1 event method, result_wire module: 2 types + 2 fns), 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, EXECUTED — apps/cli/src/main.rs:18 `lumen_cli::run::run` → apps/cli/src/run.rs:332 `serde_json::from_str::<WireRequest>` / :350 `serde_json::to_string(&response)` (`--control-stdio` NDJSON mode); commands compiled in the release lane (baseline cargo-build-release-lumen-cli.txt:186). I ran the built binary end-to-end (see §2) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/commands-84df364c190318ab envelope::tests` -> 4 passed (exit 0); `target/debug/lumen --control-stdio` probes (session.open/playback.pause/media.get-duration + unknown-field + unit-args probes) -> wire forms reproduced; python3 schema inspection |

## 1. One-paragraph verdict
This file's claims are true and I proved them by executing the shipped binary: `lumen --control-stdio` answered `{"requestId":1,"session":1,"gen":0,"result":{"accepted":{"session":1,"gen":0}}}` for `session.open` — camelCase, the `gen` rename, `session` skipped when `None`, and the bare-outcome/`{"error":…}` result union are all exactly what the module doc and doc 05 §5.1 (:471-475) promise, and the generated schema's `result` field refs the same `ResultWire` type the runtime adapter uses, so the "cannot drift apart" claim is structural. The envelope is honest about its contracts (RequestId monotonicity is documented as the sender's business, not the engine's). Two real defects, both small: unknown top-level request fields are silently ignored — `#[serde(flatten)]` makes `deny_unknown_fields` impossible, the generated schema correspondingly lacks `additionalProperties:false` at the CommandRequest level (payload-level structs DO have it — 22 `deny_unknown_fields` in types.rs), so doc 05 §5.4's blanket "unknown fields/IDs rejected" is delivered everywhere EXCEPT the envelope's own top level; and `CommandOutcome::requires_completion_event` has zero production callers (definition + own test only, repo-wide). The in-file tests are accessor-level and mostly tautological; the wire-form coverage lives in tests/wire_forms.rs (file 4 of this assignment) — cross-file coverage exists and passes.

## 2. What actually works (verified)
— verified by `./target/debug/deps/commands-84df364c190318ab envelope::tests` -> `4 passed; 0 failed` (exit 0); baseline cargo-test-workspace.txt:1483-1486.
— verified by EXECUTING the shipped binary: `printf '{"requestId":1,"cmd":"session.open","args":{"source":{"kind":"file","path":"tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"},"options":{}}}...' | target/debug/lumen --control-stdio` ->
```
{"requestId":1,"session":1,"gen":0,"result":{"accepted":{"session":1,"gen":0}}}
{"requestId":2,"session":1,"gen":0,"result":{"accepted":{"session":1,"gen":0}}}
{"requestId":3,"session":1,"gen":0,"result":{"completed":{"kind":"duration","duration":{"kind":"known","time":2000000}}}}
```
and a failure case -> `{"requestId":1,"gen":0,"result":{"error":{"stage":"session","code":"not-ready","message":"no session open","details":{}}}}`. This pins every wire-form claim in the module doc: bare `accepted`/`completed` outcomes, `{"error":…}` failure envelope, `gen` rename, `session` skip-if-none, bare-number RequestId. The duration (2,000,000 µs) matches the 2 s fixture.
— verified by python3 inspection of schemas/commands.schema.json: `CommandResponse.result -> {"$ref":"#/$defs/ResultWire"}`, `ResultWire = anyOf[CommandOutcome, ErrorEnvelope]` — schema and runtime share one type; no drift possible.

## 3. Findings
F-001 [S4]
Where: crates/commands/src/envelope.rs:50-57
Code:
```
    pub request_id: RequestId,
    /// Flattened: the concrete `Command` (card 1.D4) is adjacently
    /// tagged (`cmd`/`args`), so the wire object is
    /// `{ "requestId": 42, "cmd": "playback.seek", "args": { … } }`
    /// — exactly the §5.1 example.
```
What is wrong: "exactly" is false — the type accepts MORE. `#[serde(flatten)]` both prevents `deny_unknown_fields` (serde limitation) and silently consumes unknown keys. Executed proof through the shipped CLI: `{"requestId":1,"cmd":"playback.play","evilField":1}` produced a normal response (not-ready), i.e. `evilField` was ignored without complaint. The generated schema mirrors the permissiveness (CommandRequest def has NO `additionalProperties:false`; verified by python3), so doc 05 §5.4's "unknown fields/IDs rejected" holds for command IDs and for fields inside `args` (21 payload defs have `additionalProperties:false`; 22 `deny_unknown_fields` in catalog/types.rs) but NOT for top-level envelope fields. A typo'd extra key in a machine-mode request is silently swallowed — against the repo's own "typos must surface, never silent no-ops" rule (doc 07 §3.5). Blast radius small: extra fields cannot change semantics (serde ignores them), and the surface is a local stdio pipe. What correct looks like: validate the envelope keys explicitly (deserialize to `serde_json::Value` first, reject unknown top-level keys with the typed `invalid-argument`), as the payload level already does. Confidence: HIGH — Verification: ran (lumen binary) + read + schema inspected.
F-002 [S4]
Where: crates/commands/src/envelope.rs:91-94
Code:
```
    /// Whether the host must wait for a completion event (class A).
    pub fn requires_completion_event(&self) -> bool {
        matches!(self, CommandOutcome::Accepted { .. })
    }
```
What is wrong: production-dead API. Repo-wide rg (all crates, apps, ui): the only hits are this definition, its own test (:232,:236), and a worklog mention. Hosts consume the class-A/S split through `accepted_generation` (correlation.rs:67, playback-engine/session.rs:257 mirror) or by pattern-matching; nobody asks "must I wait?". Dead public vocabulary on a card whose doc sells the method as the C2/A-class split's accessor. Confidence: HIGH — Verification: rg sweep (whole tree, audit/ excluded).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RequestId | 24-27 | REAL | — | newtype; serializes as bare number (executed: `"requestId":1`); monotonicity honestly documented as sender's contract |
| SessionId | 31-34 | REAL | — | newtype; executed `session:1` |
| Generation | 39-42 | REAL | — | newtype; `gen` rename executed |
| CommandRequest<C> | 47-57 | REAL | S4 (F-001) | flatten works (executed); cannot reject unknown top-level fields |
| CommandOutcome<R> | 62-77 | REAL | — | externally tagged with camelCase variants — executed `{"accepted":…}`/`{"completed":…}` |
| CommandOutcome::accepted_generation | 84-89 | REAL | — | consumed by correlation.rs:67 + playback-engine + tests |
| CommandOutcome::requires_completion_event | 91-94 | REAL but production-dead | S4 (F-002) | no callers |
| CommandResponse<R> | 99-112 | REAL | — | serde bounds correct; gen rename + session skip executed |
| Event<E> | 119-134 | REAL | — | flatten + optional-field skips; covered by wire_forms `doc_05_event_example` (baseline :1534 ok) |
| Event::is_pipeline_scoped | 136-142 | REAL | — | both-flags rule; 3-case test; used by StalenessFilter semantics (correlation.rs:76-78 duplicates the destructure — consistent) |
| result_wire::ResultWire | 157-164 | REAL | — | untagged union; no ambiguity for the catalog instantiation (externally tagged outcome vs `error` key) |
| result_wire::ErrorEnvelope | 167-171 | REAL | — | executed `{"error":{...}}` |
| result_wire::serialize | 177-187 | REAL | — | executed via binary |
| result_wire::deserialize | 194-204 | REAL | — | executed (stdio requests parsed); untagged fallthrough order safe here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ids_are_plain_newtypes | NO (near-tautology) | pins Eq/Ord/field on constants; catches only derive removal |
| accepted_exposes_its_generation | YES | both methods asserted on both variants |
| pipeline_scoping_requires_session_and_gen | YES | 3 constructive cases incl. both negative shapes |
| response_shape_carries_all_correlation_fields | WEAK | asserts field values set 2 lines above (tautology); only `.result.ok().and_then(accepted_generation)` adds value; NO serialization asserted — the module's headline wire-form claims are NOT tested in this file; they are covered by tests/wire_forms.rs (7/7 pass, baseline :1534-1540, and executed by me) — cross-file net exists |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| wire object is "exactly" `{requestId, cmd, args}` | envelope.rs:53-54 | SHALLOW | shape yes; unknown extra fields silently accepted (F-001, executed) |
| `generation` serializes as `gen` | envelope.rs:8 | REAL | executed: `"gen":0` |
| success = bare outcome `{"accepted":…}`/`{"completed":…}`, failure = `{"error":…}` | envelope.rs:10-13 | REAL | executed (both) |
| "the emitted JSON Schema and the runtime wire form cannot drift apart" | envelope.rs:148-150 | REAL | schema `result` refs the same ResultWire type (python3 inspection) |
| RequestId "starting at 1, strictly increasing… engine echoes it and never interprets it" | envelope.rs:20-23 | REAL | echoed in executed output; no interpretation in CLI path (read run.rs:332-350) |
| Event `seq` engine-global strictly increasing; generation only for pipeline-scoped | envelope.rs:115-118 | REAL | type carries contract; DeliveryLog (correlation.rs:101-120) checks it host-side |
| doc 05 §5.1 examples match the types | docs/05-command-layer.md:471-475 | REAL | executed wire forms are structurally identical; wire_forms.rs pins them byte-for-byte (baseline :1534-1540) |

## 7. Dead / duplicated / unreachable code
- `requires_completion_event` — production-dead (F-002).
- `Event::is_pipeline_scoped` logic is duplicated inline in correlation.rs:76-78 (`let (Some(session), Some(generation)) = … else return true`) — consistent duplication, not drift (both read the same Option pair).
- No stubs, no todo!/unimplemented!, no constant-returning bodies.

## 8. Dependencies & flags
- commands crate deps: serde, serde_json (dev), schemars — no feature gates on this file; compiled into the shipped release lane (baseline :186).
- The schema artifact (schemas/commands.schema.json) is generated from these types (schemars) — the `#[schemars(with = result_wire::ResultWire<…>)]` mirror keeps schema/runtime in one place.

## 9. Open questions for the next agent
- correlation.rs (RequestIdAllocator `self.next += 1` unchecked vs media-types' checked `GenerationId::next`) — same operation, two rigor levels; u64 wrap needs 2^64 requests so impact nil, but the pattern is inconsistent (belongs to correlation.rs's auditor).
- If a future host adds strict envelope validation (F-001), wire_forms.rs needs a negative test pinning the rejection; today no negative wire tests exist (see file-4 audit).
- `gen:0` for the first `session.open` (executed) — engine assigns generation 0 at first accept; confirm doc 03 §5's numbering starts at 0 so hosts' watermark math is right (playback-engine auditor).
