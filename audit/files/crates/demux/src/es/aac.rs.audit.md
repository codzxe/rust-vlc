# AUDIT: crates/demux/src/es/aac.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 35 / 4 (1 const, 1 type re-export, 2 fns) |
| File verdict | UNREACHABLE (real delegation, zero shipped callers — confirms prior finding "es/aac lane shipped-dead") |
| Reachable from shipped entry point? | NO — raw AAC opens via the ADTS lane: media-probe mints `adts` (FormatId asserted in signature/es.rs:373, registry_roundtrip.rs:126-128 pairs extension `aac` with AdtsSignature) → apps/cli/src/containers/mod.rs:52 `"adts" => adts::open` → containers/adts.rs:66 `AdtsDemuxer::new`. `demux::es::aac` has no caller outside the demux crate's own tests (rg over workspace: es/tests.rs:290,295; es/fuzz.rs:73,84; tests/es_corpus.rs:235 — nothing in apps/, tools/, or other crates). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `cargo test -p demux --lib aac_lane_is_the_landed_adts_demuxer` (exit 0, 1/1 ok); rg sweeps for `es::aac|RawAacDemuxer|aac::EXTENSIONS|aac::open|aac::probe` and `EXTENSIONS` consumers (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
This is a 35-line facade that renames `crate::adts` (the REAL, fully
audited demuxer — files 1-2 of this run). The delegation is honest and the
one test that drives it passes (3 packets, probe true), but the facade is
dead weight in the shipped product: no probe id, no CLI arm, no external
caller reaches it — a `.aac` file probes as `adts` and opens through
containers/adts.rs, never through this module. Its `EXTENSIONS` const has
zero readers anywhere; the doc's "the R3 registry row" names a registry
that carries its own literal instead. Nothing here is false about
behavior — the docs explicitly say the extension "is the landed ADTS/ADIF
demuxer ... consumed, not duplicated" — the module simply has no reason to
exist in the shipped graph.

## 2. What actually works (verified)
— `cargo test -p demux --lib aac_lane_is_the_landed_adts_demuxer` → ok (opens via `super::aac::open`, drains 3 packets, `frames_demuxed()==3`, `probe(&[0xFF,0xF1])` true).
— Delegation targets verified live: `adts::probe` and `AdtsDemuxer::new` are the exact entry points the CLI's adts arm uses (containers/adts.rs:61-66); the doc's "# Errors" list (empty/unseekable/unknown-length; no syncword/ADIF magic) matches the adts constructor contract verified in this run's file 1-2 audit (32 adts tests passed).
— `adts::EXTENSIONS = &["aac", "adts", "adif"]` (adts/mod.rs:135) is the claimed superset — TRUE.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/es/aac.rs:30-34 (whole module's shipped use)
Code: (verbatim)
```
pub fn open(
    stream: Box<dyn plugin_api::SourceStream>,
) -> Result<RawAacDemuxer, media_types::EngineError> {
    RawAacDemuxer::new(stream)
}
```
What is wrong: `es::aac::{open, probe, RawAacDemuxer}` have no production caller anywhere in the workspace — the only references are the demux crate's own unit test, fuzz lane, and corpus test. The shipped path for `.aac` is probe id `adts` → containers/mod.rs:52 → containers/adts.rs, which calls `AdtsDemuxer::new` directly. Why it matters: a second public entry point for the same demuxer that nothing exercises in production — any future behavior fork between the two names would be invisible to the shipped lanes; the module survives only because `pub` items in a library crate never trigger dead-code lints. What correct looks like: delete the module (the adts lane already owns `.aac`), or wire an `"aac"` probe id to it if a distinct raw-AAC identity is ever wanted. Confidence: HIGH — Verification: ran (rg sweep over workspace; test executed). Confirms the prior finding "CLI routes AAC via ADTS, es/aac lane shipped-dead" with the full chain.

F-002 [S3]
Where: crates/demux/src/es/aac.rs:11-13
Code: (verbatim)
```
/// File extensions this demuxer claims (the R3 registry row; the
/// landed implementation claims the superset `aac`/`adts`/`adif`).
pub const EXTENSIONS: &[&str] = &["aac"];
```
What is wrong: zero consumers. rg for `EXTENSIONS` across the workspace: every other demux's table is mirrored or tested somewhere (e.g. asf/tests.rs:92 asserts `EXTENSIONS == &["asf","wmv","wma"]`; registry_roundtrip.rs keeps media-probe's own mirror), but `es::aac::EXTENSIONS` is read by nothing — not even a test. The R3 round-trip suite that the doc points at carries its own literal `"aac"` paired with AdtsSignature (media-probe/tests/registry_roundtrip.rs:250), not this const. Why it matters: the doc's "the R3 registry row" implies a registration that does not exist; the const duplicates one entry of `adts::EXTENSIONS` with no reader. What correct looks like: remove it or make the round-trip suite consume it. Confidence: HIGH — Verification: ran (rg sweep).

## 4. Per-function table (file < 100 LOC; full inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EXTENSIONS const | 13 | UNREACHABLE | S3 | F-002: zero readers |
| RawAacDemuxer re-export | 16 | UNREACHABLE | — | Pure alias of adts::AdtsDemuxer; only es/tests.rs uses the alias path via `aac::open` |
| probe | 20-22 | REAL (delegation) | — | Forwards to crate::adts::probe; executed via unit test |
| open | 30-34 | REAL (delegation) | S3 | F-001: no shipped caller; executed via unit test only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| es::tests::aac_lane_is_the_landed_adts_demuxer (es/tests.rs:284-296) | YES for the delegation | Would fail if `open`/`probe` stopped delegating (packets.len()==3, frames_demuxed()==3, probe true) — but it cannot catch any shipped-path regression because the shipped path never calls this module |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the `.aac` extension is the landed ADTS/ADIF demuxer ... consumed, not duplicated" | aac.rs:1-2 | REAL | adts/mod.rs:135 claims `aac`; CLI "adts" arm opens it (containers/mod.rs:52) |
| "this module is the registry surface" | aac.rs:6 | PAPER-ONLY | No registry consumes it (F-001/F-002); the real surface is media-probe's AdtsSignature |
| "the R3 registry row" (EXTENSIONS) | aac.rs:11 | FALSE (as implication) | Round-trip suite uses its own literal (registry_roundtrip.rs:250); zero readers of the const |
| "# Errors: empty/unseekable/unknown-length; no ADTS syncword or ADIF magic" | aac.rs:26-29 | REAL | Matches adts constructor contract (verified in files 1-2 audit, 32 tests) |
| Matrix row container-adts-aac "implemented" | docs/feature-matrix.toml:280-281 | REAL (but via adts, not this file) | Evidence path is `crates/demux/src/adts/**`; nothing cites es/aac |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-001); EXTENSIONS is fully dead (F-002).
- Duplication: a second public name (`es::aac::open`) for `adts::AdtsDemuxer::new`.

## 8. Dependencies & flags
- plugin_api (SourceStream), media-types (EngineError), crate::adts. No features, no unsafe, no allocation of its own.

## 9. Open questions for the next agent
- If the es lane is ever given its own probe ids (an "aac" id distinct from "adts"), this module becomes the natural arm — until then it is a liability (two names, one behavior, zero shipped coverage of one name).
- tests/es_corpus.rs:235 exercises this module in the corpus lane; confirm whether that lane runs anywhere reproducible (same machine-local-corpus question as mkv file 8 of this run).
