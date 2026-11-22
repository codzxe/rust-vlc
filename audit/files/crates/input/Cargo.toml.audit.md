# AUDIT: crates/input/Cargo.toml

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 38 / 1 package manifest |
| File verdict | REAL — every manifest claim verified against the code; std-only, dev-dep discipline, feature gate honest |
| Reachable from shipped entry point? | YES: `input` is a dependency of the CLI (apps/cli/Cargo.toml:54 `input = { path = "../../crates/input" }`) and of demux (crates/demux/Cargo.toml:39) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

A clean manifest whose four concrete claims all verify: (1) std-only —
the only dependencies are workspace-internal `media-types` and
`plugin-api` (no external crates at all); (2) `testkit` is
dev-only (:36-38); (3) the `contract` feature is genuinely test-only
surface — `src/lib.rs:64` gates `pub mod contract` behind
`#[cfg(any(test, feature = "contract"))]` and neither consumer
(cli, demux) enables the feature, so the release binary indeed carries
none of it; (4) the "no unsafe/unwrap/expect/panic" C5 claim holds
across `src/` (Grep: zero hits; the only matches are doc comments).
One hygiene note: the crate is compiled into every CLI build even
though demux's own comment admits the dependency edge is nominal.

## 2. What actually works (verified)

— Dependency closure (Read): `[dependencies]` = media-types,
plugin-api (path-only); `[dev-dependencies]` = testkit. No external
crates, no version pins needed.
— Feature gate verified end-to-end: `contract = []` (:30) →
lib.rs:64 `#[cfg(any(test, feature = "contract"))] pub mod contract;`
→ consumers declare plain `input = { path = … }` without features
(apps/cli/Cargo.toml:54, crates/demux/Cargo.toml:39) → the "release
binaries carry zero of its panicky conformance vocabulary" claim
(:28-29) is true.
— C5 lint claim verified: Grep `unsafe|unwrap\(\)|expect\(|panic!|
todo!|unimplemented!` over `crates/input/src` → only doc-comment
mentions (buffered.rs:234, contract.rs:54).

## 3. Findings

### F-001 [S4] `crates/demux` declares `input` as a regular dependency but uses it only from tests
Where: crates/demux/Cargo.toml:39 vs crates/demux/src (zero `input::` references)

Code (crates/demux/Cargo.toml:3-6, the admission):
```
# the ARCH-2 set; doc 13's "Consumes: input" column is satisfied at the
# trait seam: demuxers pull bytes through `plugin_api::SourceStream`,
# which `input` implements — no compile-time edge needed, mirroring the
# 2.C4 note on doc 13's Consumes lists).
```
What is wrong: demux's `[dependencies]` carries `input` (Cargo.toml:39)
while `crates/demux/src/**` never names it (Grep: only a doc comment
in avi/riff.rs:409); the actual users are `crates/demux/tests/*`
(FileAccess/FileStream in tests/common/mod.rs). That is a
dev-dependency promoted to a release dependency — every `lumen` build
compiles `input` into the demux edge for no production reason (the CLI
needs it anyway via its own edge, so the cost is one redundant graph
edge, not code size).
What correct looks like: move `input` to demux's
`[dev-dependencies]`, or keep it and drop the "no compile-time edge
needed" contradiction in the comment.
Confidence: HIGH — Verification: read + Grep (`input::` in
crates/demux/src → zero).
(Recorded here because the edge is declared for this crate's benefit;
the file to fix is demux's manifest — flagged for the demux wave.)

## 4. Per-function table (manifest keys)

| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` | 14-19 | REAL | — | edition 2024, publish=false |
| `[lints] workspace` | 21-22 | REAL | — | inherits the workspace deny set (W00 clippy gate) |
| `[features] contract` | 24-30 | REAL | — | empty feature, honest gating (verified lib.rs:64) |
| `[dependencies]` | 32-34 | REAL | — | path-only, std-only claim holds |
| `[dev-dependencies]` | 36-38 | REAL | S4 | testkit dev-only as contracted; see F-001 for demux's side |

## 5. Test honesty

N/A (manifest). The crate's tests were audited in the buffered.rs
audit (in-file) and file_stream.rs (next).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std-only, zero external dependencies" | :9 | REAL | dependency list :32-34 |
| "No unsafe, no unwrap/expect/panic (workspace lints)" | :10 | REAL | Grep over src/ → zero hits |
| "Release binaries (no feature) carry zero of its panicky conformance vocabulary" | :28-29 | REAL | cfg gate lib.rs:64; no consumer enables the feature |
| "Network protocols belong to `streaming` … never here" | :6-7 | REAL | no network code in input/src (file.rs, uri.rs, buffered.rs, seekable.rs only) |

## 7. Dead / duplicated / unreachable code

The `contract` module is unreachable in release builds — by design and
correctly gated (the opposite of the usual finding).

## 8. Dependencies & flags

Audited here: this manifest. `[lints] workspace = true` ties the crate
to the root lint table (release workflow builds with the default
feature set — W00 baseline PASS).

## 9. Open questions for the next agent

- Wave 4 (demux): confirm no `src/` code path needs `input` before
  demoting the dependency (F-001).
- Wave 1 (workspace root): the root manifest's `members`/`exclude`
  interplay audited in my playback-engine report (desktop-tauri
  exclusion, Cargo.toml:41/:51) — not repeated here.
