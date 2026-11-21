# AUDIT: crates/codec-api/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 140 / 10 items (6 `pub mod` declarations, 7 headline re-exports, 1 prelude module with 10 re-exports, 1 70-line doc example/doctest) — no function bodies of its own |
| File verdict | REAL (pure facade; the contract it re-exports is executed by the shipped MJPEG lane) |
| Reachable from shipped entry point? | YES — every shipped decoder implements `codec_api::Decoder`; the baseline's one working lane runs through it: `MjpegDecoder` (mjpeg/mod.rs) → `Decoder::decode` → `DecodeOutcome` (60/60 frames, audit/runs/baseline/INDEX.md §contrast); pipeline_host.rs constructs `Box<dyn Decoder>` via this crate's types |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full `Read` (140 lines); `cat crates/codec-api/Cargo.toml` (media-types is the only dependency — matches the doc claim); `rg -n "codec_api"` + `sed -n '237,262p'` on audit/runs/baseline/test-workspace.log.txt (16 unit tests + 8 contract-suite tests ran ok for this crate); `rg -c "Doc-tests"` on BOTH baseline test logs → **zero doc-test sections — no committed run has ever executed a doctest** |

## 1. One-paragraph verdict
A clean facade: module declarations, re-exports, a prelude, and one long doc example — with nothing to hide. The "no codec implementations, names no platform types, carries no selection policy" self-description is verifiably true (module list + the sibling module audits in this wave), and `media-types` really is the crate's only dependency. The contract surface itself is genuinely exercised: the shipped MJPEG lane decodes through these traits, and this crate's own 16 unit + 8 contract-suite tests ran green in baseline. Two hygiene defects only: a `[PROPOSAL …]` planning marker left in the shipped doc header (the same artifact family as mjpeg/decoder.rs:104), and the crate's headline doc example has never been compiled or executed by any committed run — both baseline test logs contain zero `Doc-tests` sections, so the 70-line example that documents the entire contract usage is evidence-free.

## 2. What actually works (verified)
— Facade integrity: every re-exported name (115-121, 130-139) resolves to the six declared modules (107-112); no orphan re-exports, no local types — verified by reading each module path against the module list.
— The contract executes in production: `MjpegDecoder: Decoder` (mjpeg/mod.rs:318-395, my decoder.rs#part2 audit traced the call) → baseline AVI lane `v_frames=60 v_decoded=60` (INDEX.md).
— This crate's own tests ran in baseline: `Running unittests src/lib.rs (…codec_api-33671fb2de609986) … 16 passed` and `tests/contract_suite.rs … 8 tests` (test-workspace.log.txt:237-266) — covering capability predicates, config assembly, stale-generation guards, sink failure chaining, accelerator identity.

## 3. Findings
### F-001 [S4] `[PROPOSAL …]` planning marker left in the shipped crate doc
Where: crates/codec-api/src/lib.rs:7-8
Code:
```rust
//! Layer L1 (doc 01 §2 registry), depends only on `media-types`
//! `[PROPOSAL per doc 01 §2.2(c) — std-only besides]`. This crate is a
```
What is wrong: A literal `[PROPOSAL …]` bracket marker — planning-process residue — ships in the crate-level rustdoc. The factual claim it hedges ("std-only besides media-types") is actually TRUE (Cargo.toml has exactly one dependency), so the marker is pure noise.
Why it matters: Same artifact family as `crates/codecs/src/video/mjpeg/decoder.rs:104` (`[PROPOSAL: the number; …]`) — a recurring pattern that suggests doc copy-paste from planning documents without a final pass; rustdoc renders it verbatim to every reader.
What correct looks like: Delete the marker; state the fact plainly ("depends only on `media-types`; std-only otherwise").
Confidence: HIGH — Verification: read + Cargo.toml comparison.
### F-002 [S4] The crate's headline doc example has never been compiled or run by any committed evidence
Where: crates/codec-api/src/lib.rs:35-105 (the fenced example) vs audit/runs/baseline/test-workspace.log.txt / test-workspace-allfeatures.log.txt
Code:
```rust
//! ```
//! use codec_api::prelude::*;
//! use media_types::packet::SharedBytes;
```
What is wrong: The example is a doctest (the file's only executable content) — but `rg -c "Doc-tests"` over BOTH baseline test logs returns zero sections: the baseline runs (`cargo test --workspace --locked`, release) contain no doc-test phase at all, and neither `cargo build` nor `cargo clippy --all-targets` compiles doctests. So the 70-line NopDecoder example — the first thing every implementer copies — has no committed proof that it compiles, let alone passes its own `assert_eq!(outcome, DecodeOutcome::NeedMoreInput)`.
Why it matters: Doc examples rot silently precisely because most lanes never run them; a contract crate whose canonical usage snippet can drift uncompilable is a documentation regression waiting for its first `--doc` run.
What correct looks like: A committed `cargo test --doc -p codec-api` (or workspace `--doc` lane) in the baseline tree; REQUEST-ORCH-RUN candidate (see log).
Confidence: HIGH — Verification: rg over both logs (zero "Doc-tests" sections — including for every other crate, so this is an infrastructure gap, not this file's defect alone); the example's own correctness is UNVERIFIED from evidence (traced by eye: uses only prelude names that exist; `GenerationId::FIRST.next()?` and `CodecDescriptor::new("mock.nop")?` match the tested APIs in config/decode tests).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate docs (module map, layering claims) | 1-29 | REAL | S4 | F-001 marker; all substantive claims verified (no implementations: module list; no platform types: hw.rs audit is another agent's; only dep: Cargo.toml) |
| doc example (doctest) | 35-105 | REAL-UNPROVEN | S4 | F-002 — no committed execution; traced correct by eye (generational staleness demo) |
| 6 `pub mod` declarations | 107-112 | REAL | — | capability, config, contract, decode, encode, hw — each audited by other wave-6 agents |
| headline re-exports (7 names) | 115-121 | REAL | — | flat-access aliases; all resolve |
| `prelude` module (10 re-exports) | 129-140 | REAL | — | duplicates the headline set + `CodecDescriptor`/`SharedBytes`/`EngineError`/`ErrorKind`/`Frame`/`GenerationId`/`Packet` from media-types — intentional one-stop `use`; the duplication between 115-121 and 130-136 is by-design (prelude must be self-contained) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (doctest example, 35-105) | YES *if it ever runs* — asserts `decode(stale) == NeedMoreInput` after configuring the next generation | it pins the generational-cancellation contract; but no committed run executes it (F-002) |
| this crate's real tests (16 unit + 8 contract-suite) | YES | ran ok in baseline (log 237-266): capability predicates, thread-budget bounds, stale-guard "fires only for past generations", sink-failure chains, accelerator downcast round-trip — the substantive contract rules are tested in the modules, not the facade |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "it contains no codec implementations, names no platform types … carries no selection policy" | lib.rs:9-12 | REAL | module list (107-112) — all six are contract surface; the codec crates depend ON this crate, never the reverse (boundaries.toml, existing audit) |
| "depends only on `media-types` … std-only besides" | lib.rs:7-8 + Cargo.toml | REAL (marker is noise) | Cargo.toml `[dependencies]` has exactly `media-types`; no dev-dependencies |
| "the [`contract`] module is the executable version of these rules" | lib.rs:32-33 | REAL | contract module tests ran ok (baseline log 245-251) |

## 7. Dead / duplicated / unreachable code
- None dead; the re-export duplication (115-121 vs 130-136) is the deliberate prelude pattern.
- The doctest is "dead" only in the evidentiary sense (F-002) — it would execute under any `--doc` lane.

## 8. Dependencies & flags
- `media-types` path dependency only; no features, no optional deps (Cargo.toml verified). No feature gates — the contract is unconditional.
- Wave-6 hostile question: this crate is the seam the reachable H.264 sw lane rides (`H264SwDecoder: Decoder` via `codec_api::prelude::*`, sw.rs:81); it is the most-reached file of my assignment. H.264 reachability trace: mjpeg/decoder.rs#part2.audit.md §8.

## 9. Open questions for the next agent
- contract.rs / decode.rs / hw.rs owners (W06 other agents): my facade audit relies on your module audits for the trait-level contracts; the doctest's semantics (drop_stale_packet returning `Some(NeedMoreInput)` for stale generations) should be pinned by a unit test in decode.rs if it isn't already ("stale_guard_fires_only_for_past_generations" suggests it is).
- Orchestrator: consider a `cargo test --doc -p codec-api` run to close F-002's evidence gap (REQUEST-ORCH-RUN noted in my log).
