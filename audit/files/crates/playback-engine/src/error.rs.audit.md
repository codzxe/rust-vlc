# AUDIT: crates/playback-engine/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 9 items (NoDecodableKnown + from_tracks, SessionError + Display/Error, SessionConfigError + Display/Error, NoDecodableLeaf + Display/Error, config_error) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen` CLI → playback-engine → session.rs:2403-2409 (`NoDecodableKnown::from_tracks` + `config_error` on the zero-wireable-tracks open path — the same lane the baseline HEVC MP4 smoke exercised: open refused, typed error, Error state); SessionError returned by the engine's submit/open API and asserted across tests. Executed: tests/recovery_media.rs `no_decodable_track_names_container_and_codecs` / `..._empty_track_list_...` / `..._without_container_...` all ok (orchrun log 3999-4001); tests/session_flow.rs `nothing_decodable_is_a_typed_config_failure` ok (orchrun log 4105). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `NoDecodableKnown\|config_error\|SessionError::` over src/ + tests/ (callers: session.rs 2403-2409; 25+ test assertion sites across session_flow/seek_flow/model_walk/bounded_alloc/soak/scenarios_b/replacement/source_swap) · read tests/recovery_media.rs:414-455 (enrichment assertions) + session_flow.rs:843-880 · rg `INBOX_CAPACITY` (session.rs:121 = 64 — the doc's cross-reference is real) · rg `with_source` (media-types error.rs:280 — exists) · sed -n quotes verified |

## 1. One-paragraph verdict
A small, disciplined error module that does exactly what its docs claim: two
failure families kept distinct (transport/construction `SessionError` vs
semantic `TransitionRejected` in state.rs — both verified to exist as
described), media failures carried as data, and a genuinely enriched
`no-decodable-track` error whose container name rides `FormatInfo`, whose
empty `sniffed` list is honest-by-comment, and whose codec list rides a
private `source` leaf — every one of those properties asserted by integration
tests that passed in the no-fail-fast baseline. I could not construct a
single defect: every Display arm is covered, every cross-reference
(`INBOX_CAPACITY`, `with_source`, `session.no-decodable-track` code) resolves,
and the three enrichment shapes (named container, container-only, no
container) each have a dedicated passing test.

## 2. What was checked, and how (zero-findings file — full list)
— Every variant of `SessionError` has a Display arm (92-108) and correct
`source()` chaining (110-118: Config/GenerationExhausted chain, the rest
leaf). Read; the soak/scenarios_b host wrappers consume exactly these five
shapes (tests/soak/dispatch.rs:849-866, tests/scenarios_b/host.rs:836-852).
— `NoDecodableKnown::from_tracks` (52-60): maps `Track.codec.to_string()` per
track in demuxer order; verified against the recovery_media fixture
(`data \`subp\`, data \`txts\`` asserted in the source leaf text).
— `config_error` (186-213): code string "session.no-decodable-track" is the
one the session path emits (session.rs:2408-2409) and the tests assert
(recovery_media.rs:431); `FormatInfo { name: Some(container), sniffed: vec![] }`
asserted verbatim (recovery_media.rs:433-441); un-enriched shape (no
container) asserted by `no_decodable_track_without_container_stays_unenriched`
(orchrun log 4001 ok).
— `NoDecodableLeaf` Display (150-173): all four `MediaKind` arms are
exhaustive (compile-checked); empty-list arm "(none reported)" covered by
`no_decodable_track_with_empty_track_list_names_container_only` (orchrun log
4000 ok).
— Doc cross-references: `crate::session::INBOX_CAPACITY` → session.rs:121
(`pub const INBOX_CAPACITY: usize = 64`, used at 1071 `sync_channel`) — real;
`media_types::error::FormatInfo` / `EngineError::with_source` → media-types
error.rs:280 — real; `crate::state::TransitionRejected` and
`crate::event::EngineEventKind::Fatal` exist (state.rs:750, event.rs).
— Module-doc taxonomy claim ("media/pipeline failures are not errors here at
all: they are data carried by the Failed input and the Fatal event"):
verified — the state table's `Failed` rows emit `ReportFatal` (state.rs:411)
and session.rs wraps `EngineError`, not SessionError, on that path.
— Allocation behavior: error-path only; `from_tracks` collects per-track
Strings once at failure — no hot-path cost.
— No panics, unwraps, todo/unimplemented, unsafe, or feature gates in the
file (read line-by-line; hygiene counts in INDEX.md corroborate).

## 3. Findings
None. (Checked per §2; nothing reached even S4. The one design smell —
`SessionError::Invariant(String)` being stringly-typed — is documented as the
rendered-rejection form by choice (84-89), is unreachable except through
caller bugs/table corruption, and both host wrappers re-render it
deliberately; not a defect.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-22 | REAL | — | two-family taxonomy verified against state.rs/session.rs |
| struct NoDecodableKnown | 34-45 | REAL | — | container Option + (kind, codec) pairs; the G1 enrichment payload |
| NoDecodableKnown::from_tracks | 47-61 | REAL | — | probe-truth container (engine never learns it from the demuxer — honest None); test-pinned |
| enum SessionError (5 variants) | 65-90 | REAL | — | Config/GenerationExhausted/SessionClosed/InboxBusy/Invariant; each documented with its trigger; all asserted in tests |
| Display for SessionError | 92-108 | REAL | — | 5/5 arms; actionable text (InboxBusy tells the host to poll) |
| Error for SessionError | 110-118 | REAL | — | source chain for the two wrapping variants |
| enum SessionConfigError | 121-126 | REAL | — | single typed variant |
| Display/Error for SessionConfigError | 128-139 | REAL | — | refusal-to-fabricate message; test-pinned code mapping |
| NoDecodableLeaf (+Display/Error) | 145-174 | REAL | — | private source leaf; container/kind/codec rendering test-pinned; exhaustive MediaKind match |
| config_error | 185-213 | REAL | — | stage Control / kind Unsupported{format} / context code / optional source leaf; all three enrichment shapes test-pinned (baseline ok) |

## 5. Test honesty
No in-file tests. Integration coverage (all ok in the no-fail-fast baseline):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| recovery_media `no_decodable_track_names_container_and_codecs` (414) | YES | asserts context code, exact `ErrorKind::Unsupported{format: Some(FormatInfo{name, sniffed:[]})}`, and the source-leaf text naming container + every codec |
| recovery_media `no_decodable_track_with_empty_track_list_names_container_only` | YES | pins the "(none reported)" arm |
| recovery_media `no_decodable_track_without_container_stays_unenriched` | YES | pins the None-container degradation |
| session_flow `nothing_decodable_is_a_typed_config_failure` (843) | YES | end-to-end: data-only source → Error state → Fatal with `session.no-decodable-track` → wire `PlaybackError` |
| soak/model_walk/bounded_alloc InboxBusy + SessionClosed assertions | YES | the backpressure-by-rejection and stale-handle lanes are exercised under storm loads |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two families, kept distinct on purpose" (SessionError vs TransitionRejected) | error.rs:5-16 | REAL | state.rs:750 TransitionRejected; session.rs applies both distinctly |
| "Media/pipeline failures are not errors here at all: they are data" | error.rs:18-22 | REAL | state.rs Failed rows → ReportFatal effect; EngineError carried on the Fatal event |
| "the failure answers 'which formats would I have needed' instead of a bare refusal" | error.rs:31-33 | REAL | recovery_media enrichment tests (baseline ok) |
| "an empty sniff list is the honest form, never a fabricated one" | error.rs:196-199 | REAL | `sniffed: Vec::new()` asserted verbatim (recovery_media.rs:437-440) |
| "Surfaces only to a host submitting far faster than it polls (INBOX_CAPACITY commands per poll)" | error.rs:80-82 | REAL | session.rs:121/1071 (sync_channel(64)); InboxBusy asserted in soak/model_walk/bounded_alloc |

## 7. Dead / duplicated / unreachable code
None. Every item has a live caller (session.rs executor + hosts); the leaf
type is private-by-design and consumed through the source chain.

## 8. Dependencies & flags
media-types only; no external deps, no features, no unsafe. Ships in the
default CLI build (workspace member; baseline #1).

## 9. Open questions for the next agent
- recovery.rs:307 cites "the NoDecodableKnown chaining pattern" for its own
  error enrichment — the recovery auditor should check that copy keeps the
  same honesty discipline (structured data on the source chain, no
  fabricated sniff lists).
