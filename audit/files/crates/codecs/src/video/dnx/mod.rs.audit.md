# AUDIT: crates/codecs/src/video/dnx/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / functions | 46 / 0 functions (module root: 2 module decls + 4 re-exports, documentation only) |
| File verdict | REAL — every doc claim verified true; the module's own code is declarations, but the subsystem it documents executed 6/6 live here |
| Reachable from shipped entry point? | NO (disclosed): the `"dnxhd"` dispatch arm exists in the shipped binary (apps/cli/src/pipeline_host.rs:326) but NO demuxer/probe mints the `dnxhd` descriptor — the arm's own comment says so (:321-325); mod.rs:35-41 discloses the container/ES wiring as pending the Lead's reconciliation |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/dnx_differential-96c1fdafa06b7fc6` -> 6 passed / 0 failed (exit 0, 6.47s, live ffmpeg-authored lanes); rg (pin, rayon, doc cards, error codes, re-exports, producers census); sed quote verification |

## 1. One-paragraph verdict
A 46-line module root whose value is its documentation, and that
documentation is accurate to the byte. I verified every claim: the
`filmcraft-dnx = { version = "=0.9.0", default-features = false }` pin is
verbatim at crates/codecs/Cargo.toml:158; the lockfile contains no rayon
(rg "name = \"rayon\"" over Cargo.lock: zero hits — the rayon-free claim
holds); the re-triggers citation `filmcraft-dnx-0.9.0.md` EXISTS (this is
the CORRECT record path — unlike the stale 0.1.0 citation W06-A05 flagged
in codecs/Cargo.toml:146); the ±2-LSB ratified tolerance is real — card
6.VTA-DNX-RATIFY exists (worklog archive-2026-10.md:10290), the record
(:272) and docs/formats/video-prores-dnxhd.md:122 both document it, and
tests/dnx_differential.rs asserts `stats.max_delta <= 2` per-plane (:115,
:283) — I ran the whole differential live: 6/6 passed in 6.47s (5 lanes
l1_175m/l2_36m/l3_145m/l4_720p/l5_1080p10_185m + the CBR
truncation-refusal test `truncated_real_frame_refuses_typed`), so the
decoder, the truncation gate, and the tolerance gate are all executed
code. The four re-exports exist in decoder.rs (DnxChroma :111,
DnxColorVolume :142, DnxPicture :162, DnxDecoder :244); error.rs carries
the dotted `codecs.dnx.*` taxonomy (:98-196). The one defect is the
integration state the module honestly discloses: the decode lane ships in
the binary but nothing can feed it.

## 2. What actually works (verified)
— verified by `./target/debug/deps/dnx_differential-96c1fdafa06b7fc6` ->
6 passed / 0 failed, exit 0: `lane_l1_1080p_175m`, `lane_l2_1080p_36m`,
`lane_l3_1080p_145m`, `lane_l4_720p_90m`, `lane_l5_1080p10_185m`,
`truncated_real_frame_refuses_typed` — real decode work through the
admitted engine, live ffmpeg oracle on every lane.

## 3. Findings

F-001 [S3]
Where: crates/codecs/src/video/dnx/mod.rs:35-41 (+ apps/cli/src/pipeline_host.rs:318-326)
Code:
```rust
//! This card is the codecs-crate decode surface. Container /
//! elementary-stream wiring (AVdn/AVdh sample entries, MKF, the
//! CBR ES splitter the raw-`.dnxhd` lanes assume) is the Lead's
//! reconciliation; the packet contract here is "one packet = one
//! complete VC-3 frame".
```
What is wrong: the DnxDecoder is compiled into the shipped lumen-cli
binary and wired into the decode dispatch (pipeline_host.rs:326 `"dnxhd"
=>`), but a census of every `dnxhd` producer finds NONE — no demuxer
sample entry, no probe sniff, no decode-bench lane mints the descriptor.
The arm is dead code in the shipped binary: no user action can reach a
DNxHD decode today. Both this file and the dispatch arm's comment
(:321-325 "No container lane mints the `dnxhd` descriptor yet") disclose
the gap honestly, which is why this is S3 and not an S0 support claim —
no matrix row or engine listing claims working DNxHD playback (the
baseline "landed" list omits dnx). Cost: shipped-binary code with zero
reachable trigger, plus a dispatch arm that invites the next container
card to forget the reconciliation.
What correct looks like: land the mkv V_MS/FOURCC + raw-ES splitter the
comments promise, or gate the arm behind the same feature until a
producer exists.
Confidence: HIGH — Verification: rg census (probe/demux/CLI: only the
arm's own comment matches) + read.

## 4. Per-function table (file <100 LOC, declarations only)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header doc block | 1-41 | REAL | S3 | every factual claim resolves; F-001 is the disclosed integration gap |
| `pub mod decoder;` | 43 | REAL | - | audited separately (dnx/decoder.rs.audit.md) |
| `mod error;` | 44 | REAL | - | private; codes verified (`codecs.dnx.*`) |
| `pub use decoder::{...}` | 46 | REAL | - | all 4 names exist in decoder.rs |

## 5. Test honesty
N/A in-file (no tests here). The referenced suite's honesty verified by
execution: 6/6 live incl. the truncation-refusal test — a broken CBR gate
or a >2-LSB regression fails loudly (asserts at dnx_differential.rs:115,
:283).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`filmcraft-dnx = { version = "=0.9.0", default-features = false }`" | :18-19 | REAL | codecs/Cargo.toml:158 verbatim |
| "keeps the lockfile rayon-free" | :20 | REAL | no rayon package in Cargo.lock |
| "re-triggers docs/dependencies/records/filmcraft-dnx-0.9.0.md" | :22 | REAL | record exists; cites the ratify card (:272) |
| "±2 LSB ... Lead-ratified for VC-3 ... pins exactly that gate on the record's five lanes" | :27-33 | REAL | 6.VTA-DNX-RATIFY (worklog:10290); differential asserts max_delta<=2; 5 lanes ran green live |
| "the CBR truncation gate" | :3, :11-12 | REAL | `truncated_real_frame_refuses_typed` passed live |
| "dotted `codecs.dnx.*` codes" | :14 | REAL | error.rs:98-196 (not_configured, frame.truncated/invalid/unknown_cid/row/engine, output.bit_depth) |
| container wiring is future work | :35-41 | REAL (honest) | pipeline_host.rs:321-325 agrees; zero producers found |

## 7. Dead / duplicated / unreachable code
The entire dnx decode lane is UNREACHABLE from shipped entry points
(F-001) — disclosed, unlike the hevc case (W06-A05 nal.rs F-003). The
codecs-crate differential is the only live consumer.

## 8. Dependencies & flags
Behind `codec-dnx` feature (codecs/Cargo.toml:67), default-on; engine
exact-pinned, rayon-free. No unsafe in this file.

## 9. Open questions for the next agent
- 6.OPW-mkv/ASF cards: when V_MS/FOURCC wiring lands, does the packet
  contract here ("one packet = one complete VC-3 frame") hold for
  interleaved MKV, or does the ES splitter need frame-boundary indexing
  first?
