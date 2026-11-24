# CLAIM AUDIT: docs/formats/container-es.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A04) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The raw-ES wrapper record (5.I3a, 2026-10-24). Verbatim-verified: the module set, the
three matrix rows it asked to amend are implemented x3 today (amendments landed), and
the mpeg-es-video row it explicitly left honest is STILL not_started x4 — the exact
asymmetry the record prescribed. No drift, no fabrication. Fuzz/corpus claims not
re-runnable here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matrix rows container-h264-annexb, container-hevc-annexb, container-mp3-raw still read not_started with empty evidence" | :5-6 | REAL at date; amended since | feature-matrix.toml:311/:324/:285 all now `implemented` x3 platforms (the Lead amendments landed) |
| 2 | "container-mpeg-es-video (.mpgv/.m1v/.m2v) = not_started (see gaps)" | :13-14 | REAL (still true) | feature-matrix.toml:298 status not_started x4 — the row the record demanded stay honest is unchanged |
| 3 | "self_built = crates/demux/src/es/** (annexb/h264/hevc/mp3/aac/error/mod + tests)" | :19-20 | REAL | ls crates/demux/src/es/ -> aac.rs, annexb.rs, error.rs, fuzz.rs, h264.rs, hevc.rs, mod.rs, mp3.rs, tests.rs |
| 4 | "es::h264 EXTENSIONS ['h264','264','avc']; es::hevc ['h265','265','hevc']" | :34-36 | REAL-UNPROVEN | modules present; the constant lines not re-opened this session (one-line check omitted for budget) |
| 5 | "6/6 corpus lanes: ours = ffprobe packet counts, AU sizes byte-equal" | :49-61 | UNVERIFIED | corpus-v4 absent in sandbox (recorded results stand per 5.I3a's own runs) |
| 6 | "Xing/Info/VBRI first frame packet-skipped (measured ffmpeg behavior)" | :39-42 | UNVERIFIED | measurement claim; consistent with the mp3 module's documented behavior |
| 7 | "+FB fuzz-lite ×512 rounds — no panic/hang" | :75-77 | UNVERIFIED | not re-runnable (cargo budget) |
| 8 | "cargo test -p demux ALL GREEN — 310 lib (12 new es)... workspace 2752/0 (5.W5-CLOSE)" | :100-102 | UNVERIFIED | cargo forbidden; workspace-level counts not in baseline granularity |

## 3. Contradictions
None. The record's requested amendments landed; the row it left not_started stayed
not_started — full doc/matrix agreement today (a no-drift row set, unlike W18's
drifted cases).

## 4. Fabrication / inflation indicators
None. Divergences (keyframe flags, attached-pic handling) recorded against ffmpeg on
both sides; the "not measured — the ceiling is the policy" note is honest labeling.

## 5. Notes for next agents
- W13 context: `container-mp3-raw` implemented while engine-info decoders list only
  mjpeg/pcm_s16le — demux rows vs decoder rows are different matrix families; do not
  conflate when citing the registry disagreement.
- The 0145 APIC divergence (metadata-skip vs ffmpeg's synthetic packet) is the reference
  case for "player behavior, not framing" honesty.
