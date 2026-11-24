# CLAIM AUDIT: tools/vta/vp9/README.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL: 5 REAL-UNPROVEN: 2 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 0 UNREACHABLE: 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An accurate, unusually self-critical harness README: every operational claim
checkable against tools/vta/vp9/src/main.rs holds (pipeline, u16LE high-bit-
depth repack, hostile battery, external digests — no MD5 code in the
harness), the comparator arithmetic is correct, and it documents the
measurement-discipline error that forced external digests. The vp9
README+record pair is the source of the VP9 BIT-EXACT claim, whose live
re-verification passed today in the orchestrator baseline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "decodes an .ivf VP9 file through OUR ivf demuxer ... into rusty_vp9 0.1.3" | :3-5 | REAL | tools/vta/vp9/src/main.rs:12-14 (`use demux::ivf::IvfDemuxer`, `rusty_vp9::{DecodedFrame, Vp9Decoder}`), :58, :72; Cargo.toml:13 `rusty_vp9 = "=0.1.3"` |
| 2 | "writes tightly packed I420 rawvideo (8-bit, or u16LE for profile-2 10/12-bit)" | :5-6 | REAL | main.rs:34 "Tightly packed high-bit-depth I420, u16 LE per sample", :117-123 bd!=8 path |
| 3 | "the harness itself only demuxes, decodes, and writes" (digests external) | :22-24 | REAL | rg md5/hash over main.rs -> only 2 comment hits; no digest code anywhere |
| 4 | "A hand-rolled in-harness MD5 was tried first and was WRONG — ... digests stay external on purpose" | :24-26 | REAL (recorded incident) | matches record rusty_vp9-0.1.3.md:187-194 "harness honesty"; NOTE main.rs:5 doc comment still promises "one `frame_md5` line per decoded frame" — stale doc describing the removed behavior (actual output prints `frame i WxH bdN`, main.rs:121/:128) — README right, code comment stale |
| 5 | "frame size = bps·(w·h + 2·⌈w/2⌉·⌈h/2⌉), bps=2 when bd>8; fsz = 115200 # 320x240 I420" | :28-35 | REAL | arithmetic: 320·240 + 2·160·120 = 115200 ✓; formula matches I420 (×2 samples for bd>8) |
| 6 | "ffmpeg -y -c:v libvpx-vp9 ... -f rawvideo -pix_fmt yuv420p" (reference) | :16 | REAL | oracle is external ffmpeg (independent of rusty_vp9); in-tree test re-runs it live (vp9_differential.rs:221-244) and passed in the baseline today (4/4, 21.12 s) |
| 7 | ".webm VP9 lanes: remuxed to .ivf with ffmpeg -c copy ... byte-preserving; verified by whole-stream decode match" | :48-51 | REAL-UNPROVEN | reasoning sound (decode-output equivalence); not re-runnable here (no toolchain) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| README :23 "the harness itself only demuxes, decodes, and writes" | main.rs:5 module doc still says "one `frame_md5` line per decoded frame" | the README/main body — no md5 in output (main.rs:121/:128); stale doc comment in main.rs, flagged for that file's auditor |

## 4. Fabrication / inflation indicators
None. The README advertises its own past measurement bug and the fix.
Oracle independence is genuinely good: reference = external ffmpeg binary;
comparison = external md5sum/python; harness side = our demuxer + the crate
under test — three separate processes.

## 5. Matrix rows
N/A (tool README). The downstream matrix row codec-video-vp9 cites the
record whose evidence this README reproduces; the in-tree re-run passed
today (baseline vp9_differential 4/4).

## 6. What actually works (verified)
— rg + reads over tools/vta/vp9/src/main.rs (230 lines): IvfDemuxer→
push→flush→next_frame loop (:72-83), hostile battery with catch_unwind and
"PANIC — FAIL" output (:157-158), u16LE repack (:34,:117-123);
Cargo.toml standalone [workspace] + exact pin; comparator arithmetic
checked by hand.
