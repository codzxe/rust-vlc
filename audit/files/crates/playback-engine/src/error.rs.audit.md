# AUDIT: crates/playback-engine/src/error.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 213 / 7 production items |
| File verdict | REAL — small, correct typed-error taxonomy; every item reachable |
| Reachable from shipped entry point? | YES: `SessionError` converted to wire errors in both hosts (apps/cli/src/host.rs:1538-1556, apps/web/src/host.rs:1929-1948); `config_error`/`NoDecodableKnown::from_tracks` called on the zero-wired-limb failure (crates/playback-engine/src/session.rs:2403-2408) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

A clean 213-line error taxonomy: `SessionError` (transport/construction:
Config, GenerationExhausted, SessionClosed, InboxBusy, Invariant),
`SessionConfigError` (NoDecodableTrack), and the enriched
`session.no-decodable-track` builder that carries the container name in
`FormatInfo` and the codec list in a `source`-chain leaf. No panics, no
swallowed errors, honest degradation (`None` container → un-enriched
form; empty sniff list documented as honest, :196-199). Two hygiene
notes only. The separation it documents (transport errors vs
`TransitionRejected` vs media failures as `Failed` input data) matches
what the state machine actually does (state.rs:903-959, verified in the
pipeline.rs audit).

## 2. What actually works (verified)

— Reachability: `SessionError` → wire conversion in the CLI host
(host.rs:1538-1556, all five arms matched) and the web host
(host.rs:1929-1948); `config_error` built at session.rs:2408 when
`Pipeline::wire` wires zero limbs; the code string
`session.no-decodable-track` is asserted by integration tests
(tests/recovery_media.rs:432/481/518, tests/session_flow.rs:869).
— `Display`/`source` chains are correct std-error plumbing
(:92-118, :128-137, :150-174); `NoDecodableLeaf` renders
`container \`x\` tracks: video \`c\`, audio \`d\`` — human-usable, and
the structured data rides `source` so machines do not parse strings.

## 3. Findings

### F-001 [S4] `SessionError::Invariant` carries a free-form `String` detail
Where: error.rs:84-89

Code:
```
    /// or table corruption — e.g. applying `Open` to a freshly
    /// constructed `Idle` session). Carries the rejection's rendered
    /// form; surfaced as a typed error, never panicked on.
    Invariant(String),
```
What is wrong: the one engine-internal breach channel is stringly
typed — the caller (host.rs:1554-1556) can only re-wrap the string into
a wire error, not match on a structured cause. For a "caller bug or
table corruption" channel this is tolerable (it should never fire), and
the doc says so; still, the kind of detail (which input, which state)
is unparseable.
What correct looks like: `Invariant { input: SessionInput, state:
SessionState }` when those types are cheap to carry.
Confidence: HIGH — Verification: read.

### F-002 [S4] `SessionConfigError` is a single-variant enum nested in `SessionError`
Where: error.rs:120-126

Code:
```
pub enum SessionConfigError {
    /// Zero tracks survived wiring and the engine cannot play what it
    /// cannot decode (no audio/video limb was constructible).
    NoDecodableTrack,
}
```
What is wrong: one variant, one user, one match arm everywhere
(host.rs:1538-1541). The extra layer buys nothing today; it exists to
make future config causes non-breaking, which is a legitimate but
undocumented rationale. Not a defect — structure debt.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NoDecodableKnown::from_tracks` | 52-60 | REAL | — | collects (kind, codec) pairs; reachable session.rs:2403 |
| `Display for SessionError` | 92-108 | REAL | — | distinct per-variant text; `InboxBusy` tells the caller to poll |
| `Error::source for SessionError` | 110-118 | REAL | — | chains Config/GenerationExhausted |
| `Display for SessionConfigError` | 128-137 | REAL | S4 | single variant (F-002); message honest ("refuses to fabricate playback") |
| `Error for SessionConfigError` | 139 | REAL | — | |
| `NoDecodableLeaf` Display | 150-173 | REAL | — | container + tracked codec list; empty → "(none reported)" |
| `config_error` | 186-213 | REAL | — | FormatInfo from container only (sniffed honestly empty); leaf in source chain; code `session.no-decodable-track` |

## 5. Test honesty

No in-file tests. The file's output contract is covered by integration
tests asserting the code string and wire details
(crates/playback-engine/tests/recovery_media.rs:432/481/518,
tests/session_flow.rs:869, tests/soak/dispatch.rs:851,
tests/scenarios_b/host.rs:838). The `NoDecodableLeaf` Display rendering
(container + codec list) has no direct assertion found — UNVERIFIED
whether any test pins its exact text.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| two families kept distinct (transport vs semantic rejection) | :5-16 | REAL | session.rs applies `TransitionRejected` separately; media failures ride `Failed` (pipeline audit §4) |
| "media/pipeline failures are not errors here at all" | :18-22 | REAL | `EngineError` carried by `SessionInput::Failed` (session.rs:2428 tick path) |
| NoDecodableKnown answers "which formats would I have needed" | :32-33 | REAL | leaf renders the codec list; asserted indirectly via recovery_media tests |

## 7. Dead / duplicated / unreachable code

None — every public item has a caller (§2).

## 8. Dependencies & flags

`media_types::{EngineError, PipelineStage}`. No feature gates; ships in
the CLI binary.

## 9. Open questions for the next agent

- Wave 3 A02 (session.rs): at session.rs:2403-2408 confirm the
  enrichment is actually populated in the CLI path (does the host
  declare a container name there, or does every production
  `no-decodable-track` degrade to the un-enriched form?).
