# CLAIM AUDIT: docs/formats/container-ivf.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A04) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The IVF container record (5.I3b, 2026-10-24). The matrix row it asked to amend is now
implemented x3 with itself as evidence (the Lead amendment landed); the module set and
the two committed fixtures both verify on disk. Its honesty posture (typed refusals for
out-of-scope fourccs, "walk is truth", keyframe=false honesty) is consistent with the
record's language. No fabrication. Fuzz/md5 claims not re-runnable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matrix row container-ivf still reads not_started with empty evidence" | :3-5 | REAL at date; amended since | feature-matrix.toml:337 now implemented x3; evidence cites "record docs/formats/container-ivf.md, 5.REC-1" |
| 2 | "self_built = crates/demux/src/ivf/** (mod/error/test_support/tests/fuzz)" | :15-16 | REAL | ls crates/demux/src/ivf/ -> mod.rs, error.rs, fuzz.rs, test_support.rs, tests.rs |
| 3 | "the committed ffmpeg-generated fixtures... av1_64x48_30fps.ivf / vp9_64x48_25fps.ivf" | :44-47 | REAL | tests/fixtures/generated/ivf/{av1_64x48_30fps.ivf, vp9_64x48_25fps.ivf} exist at workspace root (ivf_fixtures.rs:97,106 paths) |
| 4 | "fourcc gate AV01 → 'av1', VP90 → 'vp9'; everything else typed unsupported-fourcc" | :21-24 | REAL-UNPROVEN | gate in ivf/mod.rs (module present); branch behavior not re-executed |
| 5 | "VP8-in-IVF: typed v0-scope refusal" | :85-86 | REAL | matrix evidence string carries "VP8 typed v0 refusal" verbatim — cross-source agreement |
| 6 | "payload MD5 ours = ffmpeg -c copy -f md5 (0db6161a…/e62f0aba…)" | :51-52 | UNVERIFIED | md5 differentials not re-run (cargo budget) |
| 7 | "cargo test -p demux 437/0 (322 lib incl. 26 ivf...)" | :96-99 | UNVERIFIED | cargo forbidden; not in baseline granularity |
| 8 | "fuzz-lite ×512 rounds... no panic/hang/unbounded allocation" | :70-73 | UNVERIFIED | not re-runnable |

## 3. Contradictions
None. The row/matrix agree today; the requested amendment landed (same pattern as
mpegps/es in this batch — 5.REC-1's amendment asks were honored).

## 4. Fabrication / inflation indicators
None. Five ffmpeg-behavior divergences recorded as probes with both sides' behavior;
"declared frame count advisory (walk is truth)" and keyframe honesty are stated as
design limits, not hidden.

## 5. Notes for next agents
- Fixture path nuance: the record's "tests/fixtures/generated/ivf/" is WORKSPACE-root
  relative (./tests/fixtures/...), not crates/demux/tests/fixtures/ — cite the full path.
- VP9-in-IVF decode seam remains blocked on rusty_vp9 (5.P1-I) — cross-check
  video-vp8-vp9.md before claiming VP9 playback.
