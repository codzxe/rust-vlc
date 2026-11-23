# AUDIT: crates/media-types/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-27 |
| Lines / functions | 29 / n.a. (manifest) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — media-types is a direct dependency of lumen-cli (apps/cli/Cargo.toml) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg over 36 manifests (license/testkit sections); python3 Cargo.lock parse; ls src/ |

## 1. One-paragraph verdict
The manifest is what it claims: zero production dependencies (no `[dependencies]` section at all — std-only, layer L0), a single dev-dependency on the in-tree `testkit` whose dev-only status I verified in the manifests (video-engine/codecs/demux/audio-engine/platform-macos all declare it under `[dev-dependencies]`), `publish = false`, workspace lints inherited. Every crate-level claim in the header comment checks out against the source tree (modules error/ids/packet/time/frame match the "sources, streams, tracks, packets, frames, time, taxonomy" description — SourceUri/StreamId/Track live in ids.rs per the lib.rs module map). One nuance worth recording so nobody cries fraud later: Cargo.lock lists `dependencies = ["testkit"]` for media-types — that is the dev-graph resolution, not a production dep; production builds never link testkit.

## 2. What actually works (verified)
- verified by `rg '^\[dependencies\]'` on the file -> absent; `[dev-dependencies]` carries only `testkit = { path = "../testkit" }`.
- verified by `rg '^testkit'` + section context in 5 crates -> testkit only ever under `[dev-dependencies]`; the claimed enforcement lane (check-architecture) exists (tools/xtask/src/cmd/check_architecture.rs) and its tests passed in baseline (cargo-test-workspace.txt xtask section).
- verified by python3 parse of Cargo.lock -> media-types 0.1.0, deps `["testkit"]` (dev graph — see §3).
- verified by ls src/ -> error.rs, frame.rs, ids.rs, lib.rs, packet.rs, time.rs — matches the description's inventory.

## 3. Findings
F-001 [S4]
Where: Cargo.toml:23 vs Cargo.lock (media-types entry)
Code:
```
# No dependencies. Deliberately boring, extremely stable (doc 01 §2.2).

[dev-dependencies]
```
What is wrong: nothing in the manifest — but `Cargo.lock` shows `dependencies = ["testkit"]` for this package, because the lock records the union of build+dev resolution. A future auditor reading only the lock could misreport "internal dependency present, std-only claim FALSE". The manifest is the truth for the production graph; the lock line is dev-only resolution. Recording this here to pre-empt the false-positive.
Confidence: HIGH — Verification: read + lock parse + 5-manifest sample.

## 4. Per-function table
Not applicable (manifest). Sections: comment header (claims, all verified), [package] (name/version/edition 2024/description/publish=false), [lints] workspace=true, [dev-dependencies] testkit.

## 5. Test honesty
Not applicable (no tests in this file). The dev-dependency exists to serve tests/testkit_wiring.rs — which passed in baseline (2 passed) and proves the boundary it exists to prove.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by design" | Cargo.toml:4 | REAL | no [dependencies]; production graph empty |
| "layer L0, no internal deps" | Cargo.toml:3 | REAL | same; testkit is dev-only (5-manifest sample + comment's enforcement lane) |
| "data types and pure functions only. No I/O, no threading primitives in the public API" | Cargo.toml:8-10 | REAL (consistent) | src modules are type modules; no io/thread imports in frame.rs (file 4 of this agent); full-crate scan out of scope, no contradiction found |
| "no sentinel numbers (MP §8)" | Cargo.toml:10 | REAL (consistent) | frame.rs types use Option/None, not sentinels |
| "any future external dependency must survive the doc-11 audit AND the empty-category check" | Cargo.toml:4-6 | REAL (process claim) | tools/xtask check_architecture + feature_audit exist; wave-00 baseline records xtask feature-audit FAILURE for other reasons (2 missing evidence paths) — unrelated to this crate |
| description "sources, streams, tracks, packets, frames, time, and the stage-tagged error taxonomy" | Cargo.toml:17 | REAL | SourceUri/StreamId/Track in ids.rs, Packet, frame.rs, time.rs, error.rs (lib.rs module map L12-24) |

## 7. Dead / duplicated / unreachable code
None. No features, no bins, no unused keys. No `license`/`readme`/`repository` fields — consistent repo-wide convention (0 of 36 manifests declare license; all `publish = false`).

## 8. Dependencies & flags
Production: none (std only). Dev: testkit (path). Lints inherited from workspace root (deny unwrap/expect/panic/todo etc. — which is why frame.rs is clean). No feature flags anywhere in the crate.

## 9. Open questions for the next agent
- None for this file. Adjacent: docs/dependencies/allowlist.md (referenced at Cargo.toml:6) — verify it exists and lists nothing for this crate (not in my assignment).
