# AUDIT: crates/packet-queue/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 31 / 1 manifest (package, lints, 1 dependency, 1 dev-dependency) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the crate is a build dependency of playback-engine → lumen-cli/desktop (release lane builds clean, baseline cargo-build-release-lumen-cli.txt exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg` cross-checks against boundaries.toml (lines 183-188), workspace Cargo.toml, testkit's direction contract; dependency claims verified against the crate source I audited (queue.rs/stamped.rs). |

## 1. One-paragraph verdict
An honest manifest that exactly matches the architecture boundary file: `[dependencies] media-types` only, `[dev-dependencies] testkit` only, no external crates, `publish = false`, workspace lints inherited — identical to `boundaries.toml`'s `[crates.packet-queue]` row (layer 2, internal_deps=["media-types"], dev_deps=["testkit"], empty external allowlists). The std-only concurrency claim (Mutex + Condvar, "no channels crate, no lock-free crate") is what queue.rs actually implements. The single blemish is the package `description` overselling the deployment: "bounded packet/frame queues … for every pipeline stage boundary" — frames have no `Weigh` impl (my stamped.rs audit, F-001) and the engine wires queues at exactly one boundary (demux→decode).

## 2. What actually works (verified)
- Dependency-direction contract verified both ways: testkit's own manifest states "NO production crate ever depends on testkit"; packet-queue uses it only under `[dev-dependencies]` — honored.
- Std-only claim verified in the crate source I read: queue.rs imports only `std::sync::{Condvar, Mutex}`, `std::time::Instant`, `std::collections::VecDeque` — no external concurrency crates.
- `[lints] workspace = true` — the crate compiles under the workspace's deny-lint set (baseline clippy -D warnings: CLEAN).

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/Cargo.toml:21 (with stamped.rs:80-96, pipeline.rs:39-43)
Code:
```
description = "Lumen bounded, generation-stamped queues: backpressure, wholesale stale-generation drops and watermark signals for every pipeline stage boundary (doc 03 §4.4, MP §8)."
```
What is wrong: "packet/frame queues" and "every pipeline stage boundary" overstate reality. Frames cannot be queued (no `Weigh`/`Generational` impl for `media_types::Frame` — verified in my stamped.rs audit), and the engine constructs these queues only at the demux→decode boundary; the decode→sink stage deliberately has no queue (pipeline.rs module docs own this). The header comment in this same file is honest about it ("the primitive stays generic"); only the crates.io-style description inflates. Why it matters: a consumer reading the description believes frame queues exist. What correct looks like: "packet queues (frame payloads when Weigh lands)". Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
Not applicable (manifest, <100 LOC, no functions). Sections audited: `[package]` (name/version/edition 2024/description/publish=false — all consistent), `[lints]` (workspace inheritance), `[dependencies]` (1), `[dev-dependencies]` (1).

## 5. Test honesty
Not applicable (no tests in a manifest). The crate's tests are in src (17 unit tests across lib/queue/stamped — 6 stamped + 11 others, executed green in my runs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "media-types is the ONLY dependency" | Cargo.toml:9 | REAL | `[dependencies]` has exactly one entry; boundaries.toml:185 `internal_deps = ["media-types"]` matches. |
| "Concurrency is std::sync Mutex + Condvar … no channels crate, no lock-free crate" | 10-12 | REAL | queue.rs:14 `use std::sync::{Condvar, Mutex};` — the entire sync surface. |
| "dev-dependency: testkit (dev-only crate)" | 14-15 | REAL | `[dev-dependencies]` testkit; testkit's manifest direction contract; no production crate imports testkit (rg over crates/*/src shows testkit only in tests). |
| "bounded packet/frame queues between demux, decode and the engines" | 4-5 | PARTLY FALSE | Packet queues real and shipped; frame queues do not exist (F-001). |
| description "…for every pipeline stage boundary" | 21 | FALSE (aspirational) | One boundary wired (demux→decode); decode→sink is queueless by design. |

## 7. Dead / duplicated / unused dependencies
- None. media-types is used (GenerationId, Packet, Nanoseconds, Duration); testkit is used (Must in tests). No unused, no duplicated, no license-problem dependencies.

## 8. Dependencies & flags
- No features declared (no feature-gated code in the crate — verified in the files I audited). Edition 2024. Everything the release lane builds; nothing UNREACHABLE behind a flag.

## 9. Open questions for the next agent
- If `Weigh for Frame` lands (stamped.rs F-001), update this description and the header comment together — they currently disagree with each other about frames.
