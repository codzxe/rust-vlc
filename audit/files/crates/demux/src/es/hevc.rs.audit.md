# AUDIT: crates/demux/src/es/hevc.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 30 / 3 items (EXTENSIONS, probe, open) |
| File verdict | REAL (pure delegation; wrapper not on the shipped path) |
| Reachable from shipped entry point? | NO for this wrapper — the CLI routes HEVC via `es::open_annexb(demux::es::annexb::AnnexbCodec::Hevc, …)` (apps/cli/src/containers/mod.rs:63), bypassing hevc::open; hevc::probe/open are exercised only by crates/demux/src/es/fuzz.rs:72,81 (in-crate tests + the real fuzz_demux_es cargo-fuzz target). The delegated engine (annexb) IS shipped-path reachable |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 byte reads; repo-wide EXTENSIONS consumer search; rg symbol verification in annexb.rs; baseline citations :1891-1903 |

## 1. One-paragraph verdict
A 30-line honest delegation: `probe` → `annexb::probe(Hevc, …)`, `open` →
`AnnexbVideoDemuxer::new(Hevc, …)` — both targets verified to exist and be
substantial (annexb.rs:189, :364). No logic of its own to get wrong. The
one defect is `EXTENSIONS`: a dead constant (zero consumers repo-wide) whose
doc claims "the R3 registry row" while the actual routing vocabulary lives
in media-probe's hand-mirrored `AnnexbSignature::extensions` — and that
mirror convention already drifted for the sibling lane (h264.rs claims
"avc"; the probe mirror omits it). Functionality is executed through the
shared engine's test suite (baseline :1891-1903, all ok) and the es fuzz
lane.

## 2. What actually works (verified)
- Delegation targets exist and are real: crates/demux/src/es/annexb.rs:189
  `pub fn probe(codec: AnnexbCodec, header: &[u8]) -> bool`, :364
  `AnnexbVideoDemuxer::new` (read).
- The wrappers execute via the es fuzz harness (fuzz_demux_es drains the
  hevc lane, fuzz.rs:81) — `es::fuzz::tests::empty_and_marker_inputs_survive
  ... ok` and `hostile_wide_inputs_survive ... ok` (baseline :1891-1892);
  the shared engine's unit suite passes (:1893-1903, e.g.
  `es::tests::hevc_aus_group_on_first_slice_flag_and_vps ... ok`).
- The shipped HEVC open path (same engine, different door) is wired:
  containers/mod.rs:63 `"hevc" => es::open_annexb(...Hevc...)`.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/es/hevc.rs:10-11
Code:
```
/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["h265", "265", "hevc"];
```
What is wrong: zero consumers repo-wide (verified by exhaustive
EXTENSIONS search across all crates — only definitions and the asf test's
own-lane use exist). The comment claims this is "the R3 registry row", but
the row that actually routes files is media-probe's
`AnnexbSignature::extensions()` (es.rs:280-282 of the probe crate:
`["264","h264","265","h265","hevc"]`), a hand-maintained mirror with no
compile-time tie. Drift is already visible in the family: h264.rs claims
`"avc"`, the probe mirror does not. Confidence: HIGH — Verification: read
+ repo-wide rg.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EXTENSIONS | 10-11 | DEAD (by convention) | S4 | F-001; every format module has one; only asf's is consumed (by its own test) |
| probe | 13-18 | REAL | — | 1:1 delegation; executed via es fuzz lane |
| open | 20-30 | REAL | — | 1:1 delegation; error contract documented and delegated; executed via es fuzz lane |

## 5. Test honesty
No tests in this file; the shared engine's 10-test suite + the fuzz lanes
cover the behavior (baseline :1891-1903). The wrapper adds no logic, so
nothing is lost — but also nothing here would catch a wrong EXTENSIONS
string or a doc lie (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the `.265` file-access surface over the shared Annex-B engine" | :1-3 | REAL | delegation verified; engine shipped-path wired via containers/mod.rs:63 |
| "VPS/SPS/PPS types 32/33/34-aware, `first_slice_segment_in_pic_flag` probe" | :4-5 | REAL (in annexb) | annexb.rs:185-195 probe doc + the passing `hevc_aus_group_on_first_slice_flag_and_vps` test (baseline :1898) |
| "(the R3 registry row)" | :10 | PARTIAL/FALSE-leaning | F-001: no consumer; the routing row lives in the probe crate's mirror |

## 7. Dead / duplicated / unreachable code
EXTENSIONS (F-001). The wrapper itself is unreachable from the shipped
entry point (the CLI opens the same engine through containers/es.rs's
`open_annexb` directly) — noted, not penalized: it is the registry-surface
convention for the es family and is exercised by the es fuzz lane.

## 8. Dependencies & flags
std-only; super::annexb (in-crate); plugin_api SourceStream type only.
No cfg gates, no unsafe, no unwrap.

## 9. Open questions for the next agent
- crates/demux/src/es/annexb.rs (the engine) is the real audit target for
  HEVC correctness — this wrapper is innocent by construction.
- The "avc" drift (h264.rs vs probe mirror) deserves one line in whichever
  lane audits registry consistency.
