# AUDIT: crates/transcode/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 66 / manifest (no code) |
| File verdict | REAL manifest with a STALE capability comment — crate itself unreachable |
| Reachable from shipped entry point? | NO — this manifest declares 4 path deps and is itself declared by nobody (proof: runner.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sed -n verifications exit 0; rg (mp4mux exports, dependents) exit 0 |

## 1. One-paragraph verdict
A disciplined manifest: zero crates.io dependencies (all four production edges are
workspace path deps pointing downward L3→L2/L1/L0, exactly as the header records),
codecs/input correctly quarantined as dev-only (verified: record.rs uses
codecs::PcmDecoder + input::FileAccess only under tests), workspace lints inherited,
no [features] section (so nothing in the crate is feature-gated — relevant to the
release-lane reachability question), publish=false. Two defects: the capability
comment and the package description are STALE — both say the crate ships no muxer
beyond WAV ("MP4 remux is the later 7.E2 card") while the crate now exports
`mp4mux::{Mp4Remuxer, ...}` from card 7.E2 (lib.rs:159,172). And the inherited one:
nothing depends on this crate, so the "Phase-7 surface of MP §16" is a surface with
no building attached.

## 2. What actually works (verified)
— verified by read: `[dependencies]` = codec-api, demux, media-types, plugin-api (all
path deps); `[dev-dependencies]` = codecs, input, testkit. "C5 discipline: std-only,
zero external dependencies" holds — no registry deps at all.
— verified by rg: dev-only usage claim holds (tests/record.rs imports codecs::audio::pcm
and input::FileAccess; src/ has no codecs/input imports — confirmed during the
runner/record audits).
— verified by rg: `rg "transcode" apps/*/Cargo.toml` -> nothing (crate unbuilt by any app).

## 3. Findings
F-001 [S3]
Where: crates/transcode/Cargo.toml:1-3
Code:
```
# crates/transcode — the transcode graph core (card 7.E1, issue #43;
# doc 01 §2.5 registry row `transcode`: layer L3, "recording,
# conversion, stream output" — the Phase-7 surface of MP §16).
```
What is wrong: the manifest is the dependency surface, and that surface has no
consumers — no workspace member declares `transcode`. A registry row pitched as the
product's "recording, conversion, stream output" surface, whose manifest nobody
requires. Confidence: HIGH — Verification: read (exhaustive manifest scan, runner.rs
audit).
F-002 [S4]
Where: crates/transcode/Cargo.toml:8-11
Code:
```
# (WAV-from-PCM remux, raw elementary-stream file sink). Honest
# capability: NO other encoder and NO other muxer ships in v1 (MP4
# remux is the later 7.E2 card); every unsupported conversion is a
# typed `ErrorKind::Unsupported`.
```
What is wrong: stale by one card. 7.E2 landed: lib.rs:159 `pub mod mp4mux;` and
lib.rs:172 `pub use mp4mux::{...Mp4Remuxer...}` ship an MP4 muxer, runner.rs implements
`WorkerPlan::Mp4Remux`, and tests/mp4_remux.rs passes 7/7 in baseline. The manifest's
"honest capability" statement now understates the crate — the one place a reviewer
checks first. Confidence: HIGH — Verification: read (lib.rs exports + baseline).
F-003 [S4]
Where: crates/transcode/Cargo.toml:49
Code: `description = "Lumen transcode graph core: decode-to-WAV pipeline model, job lifecycle with progress/cancel, bounded resources, WAV PCM encode + stream-copy sinks (card 7.E1, issue #43)."`
What is wrong: same staleness — description omits the MP4-remux sink and the
Recorder/Remuxer assemblies (record.rs). Cosmetic (publish=false), but it is the
crate's self-description. Confidence: HIGH — Verification: read.

## 4. Per-function table
N/A (manifest). Sections checked: [package] (name/version/edition 2024/description/
publish=false), [lints] workspace=true, [dependencies] x4 path, [dev-dependencies] x3,
no [features], no [workspace] interference.

## 5. Test honesty
N/A. (Crate test results cited in runner/record audits: 50+6+7+5+5 all ok.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero external dependencies" (C5) | Cargo.toml:40 | REAL | [dependencies] all path deps |
| "NO other encoder and NO other muxer ships in v1 (MP4 remux is the later 7.E2 card)" | Cargo.toml:9-11 | FALSE (stale) | mp4mux ships (lib.rs:159,172; 7.E2 landed) |
| "`codecs` is a DEV-ONLY edge" / "`input` is a DEV-ONLY edge" | Cargo.toml:30-38 | REAL | dev-dependencies section; src/ imports verified clean |
| "layer L3, 'recording, conversion, stream output' — Phase-7 surface of MP §16" | Cargo.toml:2-3 | UNREACHABLE | no dependent manifest |

## 7. Dead / duplicated / unreachable code
No features to gate code off. The crate as a whole is the dead weight (F-001).

## 8. Dependencies & flags
codec-api, demux, media-types, plugin-api (prod, path); codecs, input, testkit (dev).
No features. Edition 2024. Workspace lints (deny unwrap/expect/panic/todo) inherited.

## 9. Open questions for the next agent
- Who owns manifest-comment hygiene? The 7.E2 landing should have updated this header
  and the description. Check whether other crates' manifests carry the same
  one-card-stale pattern (device-discovery's header was accurate in my read).
