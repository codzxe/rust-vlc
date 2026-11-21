# AUDIT: crates/demux/src/es/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 50 / 8 items (module docs + 7 module declarations + test mod) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (children mp3/annexb are constructed on the shipped CLI's `mp3`/`mp2`/`h264`/`hevc` probe lanes — containers/mod.rs:65-70; the wrapper modules h264/hevc/aac themselves are consumed only by fuzz/tests, see their own audits) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `unwrap\(\)|expect\(|panic!|todo!|unimplemented!` over es/*.rs excluding tests/fuzz (only hit: the doc comment itself — house rule holds) · rg `es::h264::` (callers: es/fuzz.rs:71,78 + codecs/tests/h264_sw_differential.rs:134 — no shipped caller) · sed -n verification of quotes |

## 1. One-paragraph verdict
A 50-line registry facade: module docs describing the four raw-ES lanes plus seven `pub mod` declarations. The house-rules block is verified true where I could check it (no unwrap/expect/panic in the family's non-test code — rg confirms; windowed scans, ceiling-checked pools, typed recovery budgets — verified in my mp3.rs and annexb.rs audits), and the error-path inventory matches the typed codes the tests pin (`demux.es.no-sync`, `demux.es.truncated`, `demux.es.not-started`). One inherited falsehood: the doc names "the `first_slice_segment_in_pic_flag` probe" as the HEVC lane's boundary mechanism — the engine reads a different bit (the S0 defect I filed against annexb.rs), so this module's own documentation asserts a probe that does not exist in the code it documents. Also the family-wide `EXTENSIONS` constants (one per child module, each claiming to be "the R3 registry row") are dead — the probe registry carries its own tables and consumes none of them.

## 2. What actually works (verified)
- Module tree compiles into the shipped demux crate; the lanes are executed: baseline `orchrun-test-workspace-nofailfast.log.txt:1678-1690` — the es test block (empty_source_refuses, next_event_before_start, mp3_*, h264_*, hevc_*, aac_lane, fuzz_lite) all ok; the shipped open path (`containers::open` "mp3"/"mp2"/"h264"/"hevc") drives `es::mp3`/`es::annexb` directly.
- House rule "no `unwrap`/`expect`/`panic!` in non-test code": rg over `es/*.rs` excluding tests.rs/fuzz.rs — the only match is the doc line stating the rule. TRUE for the whole family.
- House rule "every scan is windowed, every pool slot is ceiling-checked, recovery budgets are typed refusals": verified in my mp3.rs audit (64 KiB open scan, 4 MiB typed recovery budget, 2×4 KiB pool) and annexb.rs audit (MAX_AU_BYTES refusal, windowed walk); `aac` delegates to the adts demuxer (another agent's file).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/es/mod.rs:10-12
Code:
```
//! - [`hevc`] — HEVC Annex-B (`.265`): the same engine with VPS/SPS/
//!   PPS (types 32/33/34) awareness and the
//!   `first_slice_segment_in_pic_flag` probe.
```
What is wrong: The named probe does not exist in the code this documents. `annexb.rs`'s `starts_new_picture` reads `head[1] & 0x80` over a 2-byte classify window — per H.265 §7.3.1.2 that is `nuh_layer_id`'s MSB (always 0 on base-layer streams), while `first_slice_segment_in_pic_flag` is bit 7 of the byte AFTER the NAL header, which `classify_len(Hevc) = 2` never reads. Full evidence chain and the S0 severity are filed in `audit/files/crates/demux/src/es/annexb.rs.audit.md` F-001.
Why it matters: This facade's documentation is the lane's index — a reader (or the feature matrix) auditing "does the HEVC lane group access units correctly?" gets a confident YES here while the engine is broken for typical files.
What correct looks like: Fix the engine (annexb F-001); this doc line then becomes true as written.
Confidence: HIGH — Verification: read + cross-file trace (annexb.rs:141-162).

### F-002 [S4]
Where: crates/demux/src/es/mod.rs:41-47 (declarations) + the children's `EXTENSIONS` consts (mp3.rs:72, h264.rs:10, hevc.rs:11, aac.rs:13)
Code:
```
pub mod aac;
pub mod annexb;
pub mod error;
pub mod fuzz;
pub mod h264;
pub mod hevc;
pub mod mp3;
```
What is wrong: Every child module exports `pub const EXTENSIONS: &[&str]` documented as "the R3 registry row", and not one is consumed — rg over the repo finds zero references to any of them; media-probe's signature registry (mod.rs:132-145 + the signature files) carries its own extension tables with its own round-trip test.
Why it matters: Four dead constants, each with a doc comment asserting a wiring ("the R3 registry row") that does not exist — the same pattern I filed as mp3.rs F-006; recorded here because mod.rs is the family surface where the pattern is visible as a family decision.
What correct looks like: Either the registry's round-trip suite consumes these tables (a demux→probe dependency the layering forbids — then delete the consts) or the consts move to media-probe as the single source.
Confidence: HIGH — Verification: rg (mp3/h264/hevc/aac EXTENSIONS all unreferenced).

## 4. Per-function table (module-level — this file declares, children implement)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs | 1-39 | REAL | S3 | Lane inventory accurate; HEVC probe claim FALSE (F-001); house-rules block verified TRUE (§2) |
| `pub mod aac` | 41 | REAL | — | Facade over the landed adts demuxer (re-export + probe/open delegation); consumed by es/tests.rs only — its EXTENSIONS dead (F-002) |
| `pub mod annexb` | 42 | REAL | — | The engine; shipped-reachable via containers; carries the family's S0 (annexb.rs F-001) |
| `pub mod error` | 43 | REAL | — | Typed error constructors (`no_sync`/`truncated`/`not_started`/`bad_header`), unit-pinned per doc line 31-39 |
| `pub mod fuzz` | 44 | REAL | — | Fuzz-lite driver; its tests ran ok (baseline log 1678-1679) |
| `pub mod h264` / `pub mod hevc` | 45-46 | REAL | — | 29/30-line wrappers; consumed by fuzz/tests, NOT by the shipped path (containers uses annexb directly) — their own audits |
| `pub mod mp3` | 47 | REAL | — | The raw MPEG-audio lane; shipped-reachable; my full audit filed |
| `#[cfg(test)] mod tests` | 49-50 | REAL | — | 12+ tests, all ok in baseline (log 1678-1690) |

## 5. Test honesty (tests live in es/tests.rs — my mp3.rs and annexb.rs audits carry the full table; the demux doc-test set)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `es::tests::*` (12 tests, log 1678-1690) | YES (11 of 12) | See the per-test tables in my mp3.rs and annexb.rs audits; the one proving-nothing case is `hevc_aus_group_on_first_slice_flag_and_vps` (encodes the spec error — annexb F-001) |
| doctests | N/A here | The es module itself has no doctest; adts/aiff/avi/asf/ogg/ps/ts/wav doctests all ok (log 6362-6378) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `first_slice_segment_in_pic_flag` probe" (HEVC lane) | mod.rs:10-12 | FALSE | annexb.rs F-001: the code reads `nuh_layer_id`'s MSB, never the flag's byte |
| "no `unwrap`/`expect`/`panic!` in non-test code" | mod.rs:22-23 | REAL | rg over es/*.rs (non-test): zero hits |
| "every scan is windowed, every pool slot is ceiling-checked, recovery budgets are typed refusals" | mod.rs:24-25 | REAL | mp3.rs + annexb.rs audits (§2) |
| "raw ES carries no stamps — video packets carry `Stamp::Missing` … and audio lanes use exact sample arithmetic" | mod.rs:26-29 | REAL | annexb mint_track (both Missing, verified); mp3 exact sample clock (corpus-pinned); aac→adts (another agent) |
| "Empty/short sources refuse at construction … `demux.es.no-sync` … `demux.es.truncated` … `demux.es.not-started`" | mod.rs:33-39 | REAL | Tests ok: empty_source_refuses (1681), mp3_no_sync (1687), mp3_truncated (1688), next_event_before_start (1685) |

## 7. Dead / duplicated / unreachable code
- Family-wide dead `EXTENSIONS` consts (F-002).
- The h264/hevc/aac wrapper `open`/`probe` functions are not shipped-path (fuzz/tests only) — not dead, but their "registry surface" purpose is aspirational; the shipped path bypasses them (containers/es.rs constructs the engine directly).

## 8. Dependencies & flags
Std-only per the house rules (verified: no external deps in the es tree). No feature gates, no unsafe. The layering constraint ("layer ordering forbids sharing the crate" with codecs' mp3 tables, line 15) is the documented reason for the table twin — a deliberate duplication.

## 9. Open questions for the next agent
- If annexb F-001 is fixed, this file's line 10-12 needs no edit (it describes the intended probe) — but the hevc.rs wrapper doc (hevc.rs:4-5) repeats the same claim and should be re-checked then.
- Whoever audits `crates/demux/src/es/error.rs` and `fuzz.rs` (not mine): confirm the typed constructors are all unit-pinned as the docs claim.
