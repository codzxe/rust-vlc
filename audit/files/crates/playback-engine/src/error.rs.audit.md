# AUDIT: crates/playback-engine/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 213 / 9 (2 enums + Display/Error impls, 1 struct + ctor, 1 private leaf, 1 builder fn; no in-file tests — coverage lives in tests/recovery_media.rs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `SessionError::{SessionClosed, InboxBusy, Invariant, GenerationExhausted}` are the live return values of every `SessionHandle` command (session.rs:439-440, 1081, 1262, 1286); `config_error`/`NoDecodableKnown` fire on the no-decodable-track open path (session.rs:2402-2411). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | No unit tests in this file; behavior verified by executing `./target/debug/deps/recovery_media-2d1b70ceb0ed612e` → 7 passed (the three G1 tests pin this module's exact output shapes — see my recovery_media.rs audit). `rg` for construction sites of every variant (see F-001). |

## 1. One-paragraph verdict
A small, honest error surface: transport/construction failures (`SessionError`) cleanly separated from state-machine rejections (TransitionRejected in state.rs) and from media failures (EngineError as data) — the three-family split the module docs promise, and the split the code actually implements (verified: every variant has a live production constructor except one). The G1 enrichment (`NoDecodableKnown` + `NoDecodableLeaf` + `config_error`) renders exactly what the recovery tests pin (container in `FormatInfo`, honest empty sniff list, per-track kind+codec list, "(none reported)" instead of fabrication). Two S4 blemishes: `SessionError::Config` is never constructed anywhere in production code (hosts carry dead match arms for it), and `Invariant(String)` is stringly-typed.

## 2. What actually works (verified)
- G1 output shapes executed: `./target/debug/deps/recovery_media-2d1b70ceb0ed612e` → 7 passed, including `no_decodable_track_names_container_and_codecs` (asserts `FormatInfo{name: Some("avi"), sniffed: []}`, source leaf `container `avi`` + `data `subp`, `data `txts``) and the "(none reported)" and un-enriched back-compat pins — every string this module renders is asserted somewhere.
- Variant construction sites verified by rg: `InboxBusy`/`SessionClosed` at session.rs:439-440 (bounded-inbox try_send), `Invariant` at session.rs:1081 and 1262 (table-breach paths, honored as Result instead of unwrap), `GenerationExhausted` via session.rs:1286 (`generation_exhausted_boxed`) on epoch-counter exhaustion in `Engine::open`.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/src/error.rs:66-69 (vs session.rs:1061-1080, 1428+)
Code:
```
pub enum SessionError {
    /// The session configuration is unusable (see
    /// [`SessionConfigError`] for the typed causes).
    Config(SessionConfigError),
```
What is wrong: `SessionError::Config` has no production constructor. `Engine::open` (session.rs:1061) performs no config validation that yields it; `Session::new` consumes the config by value unchecked; the `NoDecodableTrack` failure travels as a Fatal `EngineError` built by `config_error` (session.rs:2408), not as this variant. The only constructors are test files (tests/soak/dispatch.rs, tests/scenarios_b/host.rs — rg verified). Meanwhile the shipped hosts match it: apps/cli/src/host.rs:1538 and apps/web/src/host.rs:1929 carry handling arms for an error the engine cannot produce — defensive dead code that implies a failure mode that does not exist. The doc comment ("The session configuration is unusable") asserts producibility the code lacks. What correct looks like: either a validation step in `Engine::open` that actually refuses unusable configs with this variant, or delete the variant and the host arms. Confidence: HIGH — Verification: read + rg (exhaustive constructor search over crates/ and apps/). UNVERIFIED: none.

F-002 [S4]
Where: crates/playback-engine/src/error.rs:84-89
Code:
```
    /// An engine-internal invariant breach: an input the table pins as
    /// always-legal was refused (only reachable through a caller bug
    /// or table corruption — e.g. applying `Open` to a freshly
    /// constructed `Idle` session). Carries the rejection's rendered
    /// form; surfaced as a typed error, never panicked on.
    Invariant(String),
```
What is wrong: the payload is a rendered string, not the typed `TransitionRejected` it wraps (session.rs:1081 does `format!("{rejection}")` — the structure is destroyed at the boundary). A host cannot programmatically distinguish which pinned row was breached. The doc comment honestly scopes it to caller-bug-only, so the blast radius is diagnostics quality, not correctness. What correct looks like: `Invariant(TransitionRejected)` (the type is already `Copy`-able data per state.rs) with Display doing the rendering. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NoDecodableKnown` (struct + `from_tracks`) | 34-61 | REAL | — | Container (probe truth) + per-track (kind, codec) pairs in demuxer order; consumed at session.rs:2402. |
| `SessionError` (enum) | 65-90 | REAL | S4 | F-001 dead `Config` variant; F-002 stringly `Invariant`. Other four variants live and reachable. |
| `SessionError::Display` / `Error::source` | 92-118 | REAL | — | Actionable messages (InboxBusy tells the host to poll-then-retry); source chaining for the two wrapped variants. |
| `SessionConfigError` + Display/Error | 120-139 | REAL | — | Single variant; message states the no-fabrication policy. Used via `config_error`, not `SessionError::Config` (F-001). |
| `NoDecodableLeaf` + Display | 145-174 | REAL | — | The source-leaf renderer: `container `x` tracks: video `a`, data `b`` / `(none reported)`. Output pinned by executed recovery tests. |
| `config_error` | 186-213 | REAL | — | Stage Control, `Unsupported{format}` with honest empty `sniffed`, code `session.no-decodable-track`, leaf attached only when `known` is Some. |

## 5. Test honesty
No in-file tests. Cross-file coverage (executed by me, this session): recovery_media.rs's three G1 tests assert this module's exact Display/kind/leaf output — they would fail on any wording regression, on dropping the leaf, or on fabricating sniff entries. `SessionError` transport variants are exercised by model_walk.rs (InboxBusy retry loop, SessionClosed dead-handle refusals — both in the suite I ran) and the generation storm (SessionClosed refusal law).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two families, kept distinct on purpose: SessionError — transport and construction failures … TransitionRejected — semantic rejections" | error.rs:5-16 | REAL | session.rs:439-440/1081/1262 (SessionError) vs state.rs's rejected-receipt path; the model-walk receipt-shape law asserts the split on the wire. |
| "Media/pipeline failures are not errors here at all: they are data carried by the Failed input and the Fatal event" | 18-22 | REAL | pipeline TickReport.fatal → session fail_with → Fatal event + Error state (traced in my pipeline.rs audit); no EngineError escapes as SessionError. |
| "The session configuration is unusable" (Config variant) | 66-69 | FALSE (as producibility) | No production constructor (F-001). |
| "an empty sniff list is the honest form, never a fabricated one" | 196-199 | REAL | `sniffed: Vec::new()`; pinned by executed test (asserts `sniffed: Vec::new()`). |

## 7. Dead / duplicated / unreachable code
- `SessionError::Config`: production-dead (F-001); hosts' match arms for it are dead defensively.
- Nothing else; no `todo!`/`unimplemented!`.

## 8. Dependencies & flags
- `media_types` (EngineError, PipelineStage, MediaKind, error vocabulary), `commands` indirectly through wire mapping elsewhere. No features, no unsafe, std-only.

## 9. Open questions for the next agent
- If a config-validation card ever lands (e.g. refusing a SessionConfig with a null consumer or a zero-budget tick), wire it to `SessionError::Config` or delete the variant — today it is a phantom.
- state.rs's `TransitionRejected` should be audited for whether `Invariant(TransitionRejected)` (F-002's fix) would break any host match.
