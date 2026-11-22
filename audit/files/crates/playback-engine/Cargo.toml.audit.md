# AUDIT: crates/playback-engine/Cargo.toml

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 56 / manifest: 1 package, 6 dependencies, 4 dev-dependencies |
| File verdict | REAL (manifest) with one stale header claim |
| Reachable from shipped entry point? | N/A (build manifest; the crate it builds is the shipped engine) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | read of the manifest + cross-check against crates/playback-engine/src/lib.rs and my per-module audits; rg `unsafe` over src/ |

## 1. One-paragraph verdict

A clean workspace manifest: six internal path dependencies (codec-api, commands, media-clock, media-types, packet-queue, plugin-api), zero crates.io dependencies (the "C5 std-only" claim holds), four test-only dev-dependencies (testkit, demux, codecs, input) with the Lead-flagged dev-edge pattern documented in-line, workspace lints inherited. The one defect: the header comment still describes the 2.C2-era scope — "Seek/sync are deliberately EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1)" — while `src/seek.rs` (card 3.A1) and `src/tracks.rs` (3.A3) are modules of this very crate. The comment contradicts the crate it builds.

## 2. What actually works (verified)

- "zero external dependencies, no unsafe" (header, C5 note): verified — `[dependencies]` are all `path = "../…"`; the only `unsafe` token in src/ is the doc line claiming its absence (my lib.rs audit §2).
- The dev-edge pattern (library stays trait-level, concrete stacks test-only) is real: demux/codecs/input appear only under `[dev-dependencies]`.
- `publish = false`, `edition = "2024"`, workspace lints — consistent with the workspace root.

## 3. Findings

### F-001 [S4] — Header comment claims seek is excluded from this crate; seek.rs is a module of this crate

Where: crates/playback-engine/Cargo.toml:5-7.

Code (verbatim):
```toml
# The minimal state machine (2.C2 scope): the 8 wire-visible playback
# states, two-level session epochs, and the trait-level pipeline wiring
# demux → packet queues → decoders → sinks. Seek/sync are deliberately
# EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1). Stage boundaries are the
```

What is wrong: cards 3.A1 (seek) and 3.A3 (track switching) landed in this crate (`src/seek.rs`, `src/tracks.rs`, both in the module list at lib.rs:103/107 and both audited by me as real, reachable code). The header was never updated after the Phase-3 merges; a reader of the manifest alone would believe the crate has no seek. Same doc-rot family as lib.rs F-001/F-002 (stale counts, stale exclusions).

Confidence: HIGH — Verification: read + module list comparison.

## 4. Per-function table (manifest keys)

| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments (scope, dependency note, C5) | 1-16 | REAL / stale | S4 | F-001; the plugin-api sixth-dependency Lead flag is honestly recorded |
| `[package]` | 18-23 | REAL | — | edition 2024, publish false |
| `[lints] workspace = true` | 25-26 | REAL | — | inherits workspace lints |
| `[dependencies]` ×6 | 28-35 | REAL | — | all path-internal; matches lib.rs's re-export surface |
| `[dev-dependencies]` ×4 | 37-56 | REAL | — | test-only composition edges, documented |

## 5. Test honesty

N/A (manifest). The dev-dependency comments accurately describe what the dev edges are for (track-switch fixture suite, composition proofs).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Seek/sync are deliberately EXCLUDED (Phase 3…)" | Cargo.toml:7 | FALSE | `src/seek.rs` exists, is reachable, and is the shipped seek (my seek.rs audit) |
| "C5 (std-only): zero external dependencies, no unsafe" | Cargo.toml:14-15 | REAL | verified (path deps only; no unsafe blocks) |
| "No concrete codec/demux crate is depended on" | Cargo.toml:8 | REAL | `[dependencies]` has none; demux/codecs are dev-only |

## 7. Dead / duplicated / unreachable code

N/A. (The dead *modules* the crate builds — bookmarks, frame_step, recovery, hotplug hooks — are documented in my per-module audits; the manifest itself has no dead keys.)

## 8. Dependencies & flags

No features declared. All dependencies path-internal and locked via the workspace Cargo.lock (W00 baseline built `--locked` successfully).

## 9. Open questions for the next agent

- boundaries.toml's layer rules (L4 crate depending on L1 plugin-api) — the Lead flag in the header says it's legal; wave-1 owns the registry audit.
