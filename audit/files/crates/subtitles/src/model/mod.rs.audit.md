# AUDIT: crates/subtitles/src/model/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 0 fns (6 module decls, 5 re-export groups) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — the `subtitles` crate is linked only by `fuzz/` + its own tests (rg over every Cargo.toml). The model tree is the crate's internal vocabulary (14+ consumer files in-crate), so in-crate it is live; no shipped entry point reaches it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg "unwrap\(\)|expect\(|panic!|todo!|unimplemented!|unreachable!" crates/subtitles/src/model/*.rs` (zero hits outside false-positive index patterns — the module doc's "No panics anywhere in this module tree" claim holds statically across all 6 submodules) · `rg "serialize_document|write_document|parse_document" crates/ apps/ tools/ tests/ fuzz/` (callers: mod.rs re-export + tests/timing_edges.rs + tests/roundtrip_matrix.rs ONLY) · orchrun log: model submodule tests green (style :5427-5429, time :5430-5435, position :5421-5426, cue :5419-5420) |

## 1. One-paragraph verdict
A 34-line module manifest: six `pub mod` declarations, five re-export groups, zero logic — nothing to be wrong at the statement level. The audit value here is verification of the module-doc claims that span the tree: "No panics anywhere in this module tree" is statically true (deep-audited in position.rs/cue.rs this session; panic-pattern scan clean in time/style/wire/error), and the advertised vocabulary is real — but a third of the re-exported surface (ResolvedAnchor, Viewport from position; the entire wire trio) has no consumer outside the crate's own test suites. Unreachable from any shipped binary, like the whole crate.

## 2. What actually works (verified)
— Module tree compiles and its tests ran green in the orchestrator repair run: `model::{position,cue,style,time}::tests` all `ok` at orchrun-test-workspace-nofailfast.log.txt:5419-5435; the wire roundtrip AC ran as `tests/roundtrip_matrix.rs` (log:5616).
— "No panics anywhere in this module tree" (mod.rs:16-17): verified by deep audit of position.rs + cue.rs (this agent) and a panic-pattern scan (`unwrap()/expect(/panic!/todo!/unimplemented!/unreachable!`) of time.rs, style.rs, wire.rs, error.rs — zero hits in non-test code (the three bracket-index hits at wire.rs:461, style.rs:68,78 are length-checked comparisons/reads, read in context).

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/model/mod.rs:28-34
Code:
```
pub use position::{
    Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, ResolvedAnchor, VerticalAlign,
    Viewport,
};
pub use style::{Color, TextStyle};
pub use time::CueInterval;
pub use wire::{parse_document, serialize_document, write_document};
```
What is wrong: the crate's advertised public vocabulary re-exports items with zero production consumers: `ResolvedAnchor` + `Viewport` (position.rs F-001: `resolve` is test-only) and the whole `wire` trio — `rg "serialize_document|write_document|parse_document"` over crates/, apps/, tools/, tests/, fuzz/ finds only this re-export line and the two test suites (timing_edges.rs, roundtrip_matrix.rs). The "stable wire form" is a test fixture format with no runtime consumer.
Why it matters: the re-export list is the crate's contract sheet; a consumer deciding to rely on `model::Viewport` or the wire format is relying on scaffolding that nothing in the product exercises — and the surface must be maintained as if it were load-bearing.
What correct looks like: either a production consumer (plugin-api's SubtitleParser bridge was the plan — plugin-api/src/subtitle.rs says "Impl crate: `subtitles` (L2)" yet plugin-api does not even depend on this crate) or a documented "test-vocabulary" marker on the wire/resolve surface.
Confidence: HIGH — Verification: read + rg sweeps.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | — | Tree-spanning claims verified (§2); the "every renderer consumes" phrasing overstates — in-crate renderers consume `Cue`/`Positioning`, nothing outside the crate does. |
| `pub mod` × 6 | 19-24 | REAL | — | cue/error/position/style/time/wire all exist with content. |
| re-export `cue::{…}` | 26 | REAL | — | Live crate-wide (parsers, renders). |
| re-export `error::{…}` | 27 | REAL | — | Used throughout. |
| re-export `position::{…}` | 28-31 | REAL | S4 | Includes dead `ResolvedAnchor`/`Viewport` (F-001). |
| re-export `style::{…}` / `time::…` | 32-33 | REAL | — | Live. |
| re-export `wire::{…}` | 34 | REAL (test-only consumers) | S4 | F-001. |

## 5. Test honesty
No tests in this file. The tree's tests (4 submodule suites + 2 integration suites) all ran green — see §2. Coverage of the re-export surface itself is implicit (compilation + consumers).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No panics anywhere in this module tree: every fallible operation returns Result with a typed error (MP §8)" | mod.rs:16-17 | REAL | Deep audit position/cue + panic-pattern scan of time/style/wire/error: clean (§2). |
| "the shared vocabulary every subtitle format parser (5.L1) projects into and every renderer consumes" | mod.rs:1-2 | SHALLOW | True in-crate (srt/vtt/ass project into it); zero consumers outside the crate — no renderer in any shipped build consumes it (crate unlinked). |
| "wire — the stable serialize/parse roundtrip form" | mod.rs:13 | REAL (test-only) | Roundtrip green (orchrun:5616) but no production caller (F-001). |
| lib.rs:70-72 "no unsafe … no panics on any input — failure is typed" | lib.rs | REAL (model tree) / see dvb/segment.rs F-001 for a crate-wide counterexample (u16 overflow panic path under overflow-checks in dvb) | The claim is scoped to the 5.L2 model in context; the model tree complies. |

## 7. Dead / duplicated / unreachable code
Dead-in-production re-exports: `ResolvedAnchor`, `Viewport`, `parse_document`, `serialize_document`, `write_document` (F-001). Everything else in the module is live in-crate.

## 8. Dependencies & flags
None — pure module manifest. No features, no unsafe, no panics.

## 9. Open questions for the next agent
1. model/wire.rs auditor: is the wire format versioned beyond the `LUMEN-CUES 1` magic (timing_edges.rs:44 shows the header)? If a second consumer ever appears, an unversioned drift would silently break.
2. plugin-api wave: `plugin-api/src/subtitle.rs:2` claims "Impl crate: `subtitles` (L2)" — but plugin-api's Cargo.toml has no `subtitles` dependency. Whoever audits that seam should record the missing edge (the SubtitleParser trait has no real implementation in-tree — only the test double at plugin-api/src/subtitle.rs:106).
