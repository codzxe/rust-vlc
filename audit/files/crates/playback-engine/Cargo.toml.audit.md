# AUDIT: crates/playback-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 56 / 1 package manifest (6 prod deps, 4 dev-deps, lints, package meta) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — this manifest builds the crate the release lane ships (`cargo build --release --locked -p lumen-cli` links playback-engine; baseline cargo-build-release-lumen-cli.txt exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg usage verification of every declared dependency in src/ and tests/ (counts below); rg unsafe in src/ -> 0 blocks |

## 1. One-paragraph verdict
A clean, honest manifest: six workspace-path production dependencies — every one verified in use in src/ — zero external crates, zero unsafe blocks (the C5 claim is true), testkit correctly dev-only (its five src-file appearances are all `use testkit::Must;` inside `#[cfg(test)]` modules), and the three concrete-stack dev-deps (demux/codecs/input) confined to test files as the Lead-flag comment records. The two defects are stale header prose: the crate is described as the "minimal state machine" with "seek/sync deliberately EXCLUDED (Phase 3)" while it now ships seek.rs, tracks.rs, the events policy, frame_step, hotplug, bookmarks, chapters, snapshot and recovery — and the dev-dep note names one consuming test file when three exist.

## 2. What actually works (verified)
— verified by rg, every dependency is consumed:
  - codec-api: src/pipeline.rs:102 (`use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget}`) — 1 src file.
  - commands: 12 src files; media-clock: 4; media-types: 11; packet-queue: 1; plugin-api: 5.
  - testkit: 52 test files + 5 src files (all unit-test modules: bookmarks.rs:116, events.rs:1651, snapshot.rs:173, session.rs:2871, pipeline.rs:2739).
  - demux/codecs/input: 3 test files each (tests/track_switch_fixture.rs, tests/soak/dispatch.rs, tests/scenarios_b/host.rs).
— verified the C5 claim: rg `unsafe` in src/ -> only the comment at lib.rs:88 ("no unsafe"); zero unsafe blocks. No non-path dependencies in [dependencies].
— verified shipped: the release lane builds this crate (baseline cargo-build-release-lumen-cli.txt, exit 0).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/Cargo.toml:5-8
Code:
```
# The minimal state machine (2.C2 scope): the 8 wire-visible playback
# states, two-level session epochs, and the trait-level pipeline wiring
# demux → packet queues → decoders → sinks. Seek/sync are deliberately
# EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1). Stage boundaries are the
```
What is wrong: The header describes the crate as of card 2.C2 and asserts seek/sync are excluded. They shipped long ago: `pub use seek::{SeekFailure, SeekTarget}` (lib.rs:135, card 3.A1), tracks.rs + SwitchTrack mint (3.A3, verified in my table.rs audit), the events/time-tick policy (3.B4, verified in my events_contract.rs audit), plus frame_step, hotplug, bookmarks, chapters, snapshot, recovery — 16 src modules, not a minimal state machine.
Why it matters: The manifest is the first file a reviewer reads; it misstates the crate's scope in the direction of smallness. The "minimal" framing also under-sells what the C5 std-only constraint is actually protecting (a much larger surface than 2.C2).
What correct looks like: Header updated to the current module list, or stripped to the layer-registry facts that remain true.
Confidence: HIGH — Verification: read + rg module list vs header.

F-002 [S4]
Where: crates/playback-engine/Cargo.toml:46-54
Code:
```
# The card 3.A3 real-fixture suite (tests/track_switch_fixture.rs)
# composes the engine with the concrete demux/codec/input stacks
# TEST-ONLY — the same dev-edge pattern demux uses for input (card
```
What is wrong: The note names one consumer (track_switch_fixture.rs); the dev-deps are consumed by three test files (also tests/soak/dispatch.rs and tests/scenarios_b/host.rs — rg-verified). Same understated-blast-radius class as the tests/common header (my file #11, F-001).
Why it matters: Removing or breaking one of the unnamed consumers' deps would surprise anyone trusting this note as the consumer list.
What correct looks like: Name all three (or say "the real-fixture suites").
Confidence: HIGH — Verification: rg.

## 4. Per-section table (file < 100 LOC; table not mandatory, included for completeness)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header comment | 1-22 | STALE | S4 | F-001, F-002. The plugin-api sixth-dep Lead-flag note (13-18) and the C5 note (20-22) remain accurate. |
| [package] | 24-29 | REAL | — | name/version/edition 2024/description/publish=false — consistent with workspace. |
| [lints] | 31-32 | REAL | — | workspace = true (the 121-allow tree lives elsewhere, not laundered here). |
| [dependencies] | 34-40 | REAL | — | All 6 used in src/ (see §2); all path deps; no version duplication risk (Cargo.lock audited separately). |
| [dev-dependencies] | 42-56 | REAL | S4 | All 4 used; testkit dev-only contract intact; consumer list incomplete (F-002). |

## 5. Test honesty
N/A (manifest). The manifest's own claims were verified as above.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "plugin-api is the sixth [dep] … the demux stage boundary IS plugin_api::Demuxer" | lines 13-18 | REAL | plugin-api used in 5 src files; Demuxer is the stage seam (pipeline wiring, pipeline.rs). |
| "C5 (std-only): zero external dependencies, no unsafe. Concurrency is std::sync primitives only" | lines 20-22 | REAL | rg: zero unsafe blocks in src/; zero non-path deps; no async runtime in [dependencies]. |
| "testkit is dev-only by contract (MP §3.7 / ARCH-7)" | lines 43-44 | REAL | All 5 src-file uses are inside #[cfg(test)] modules (line numbers in §2). |
| "Seek/sync are deliberately EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1)" | lines 7-8 | FALSE (stale) | F-001: seek.rs/tracks.rs/events policy all shipped and exported at the crate root. |
| "No concrete codec/demux crate is depended on" | lines 10-11 | REAL | [dependencies] has none; demux/codecs/input are dev-only (verified confined to tests/). |

## 7. Dead / duplicated / unreachable code
- No unused dependencies (every declared dep consumed — the opposite problem of dead deps).
- No duplicated dependencies; versions are path-only, resolved by the workspace.

## 8. Dependencies & flags
- Six production workspace deps (codec-api, commands, media-clock, media-types, packet-queue, plugin-api), four dev workspace deps (testkit, demux, codecs, input). No features defined or gated — everything ships unconditionally in the release lane (frame_step.rs's unreachability is a wiring gap, not a feature gate — see my frame_step audit).

## 9. Open questions for the next agent
- None for this file. The workspace-level dependency graph (Cargo.toml/Cargo.lock at root) is audited by other agents; my crate-level verification found nothing to escalate.
