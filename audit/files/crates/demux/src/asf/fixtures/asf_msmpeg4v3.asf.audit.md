# AUDIT: crates/demux/src/asf/fixtures/asf_msmpeg4v3.asf
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 7,129-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (truth entry asf/tests.rs:975-988) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:977 recipe (byte-identical) · python3 audit/analysis/asf_parse.py (walk == truth entry) · ffprobe codec_name (`msmpeg4v3` ✓) |

## 1. One-paragraph verdict
Genuine ffmpeg 7.1.5 output, sha-pinned, byte-exact-reproducible from its committed recipe, and structurally equal to every typed fact in its truth entry (verified by an independent walk). The file's lane is deliberate: a legacy fourcc (MP43/msmpeg4v3) that no shipped decoder supports — the plan (§1.3, asf-selfbuild.md:98) scopes it to "demux + typed `cli.codec_unavailable`", i.e. demux-only by design, so its demux-only differential is honest here (unlike asf_mjpeg's "decodable video" label). No file-specific defects; family findings cross-referenced.

## 2. What actually works (verified)
- sha256 `66201b9205f26a90779a7ee273e6aa4f835b998050a37d0111bda691a63d9746` == pin `66201b9205f26a90` (tests.rs:1490).
- Regeneration from `…testsrc=size=64x48:rate=10:duration=1 -c:v msmpeg4v3 -b:v 100k` (tests.rs:977; plan §1.2:60 verbatim) under ffmpeg 7.1.5 → identical sha256. Confirms the plan's re-pin note ("regeneration deterministic, verified twice", plan §4.4:455) — now verified a third time.
- Independent walk == truth (tests.rs:975-988): HEADER 593 / 5 children · FPO file_size 7129 == real size, 2 packets, preroll 3100, min=max=3200, flags 0x02 · DATA at 593 size 6450 count 2, reserved `01 01` · SIMPLE_INDEX at 7043 size 86 (593+6450=7043; +86=7129 exact) · SP stream 1 video, TSD 51 B → 64x48, data_size/biSize 40, bit_count 24, fourcc `MP43`, size_image 9216, extradata 0.
- ffprobe oracle: `codec_name` = `msmpeg4v3` == tests.rs:987 — the container-level fourcc→codec-name mapping agrees with the oracle.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file. Cross-references (filed and counted in the asf_mjpeg.asf audit):
- F-001 [S3, cross-ref]: this file's video SP EC field carries the real no-error-correction GUID (20FB5700-5B55-11CF-A8FD-00805F5C442B), refuting guid.rs's all-zero `EC_NONE` "observed" claim.
- F-002 [S4, cross-ref]: outside the xtask fixtures drift gate; in-test sha pin compensates.
- F-003 [S4, cross-ref]: a missing committed copy regenerates into src/ via tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 7,129 B | == FPO file_size; plan §4.4:455 row ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk == tests.rs:975-988 |
| sha256 pin | MATCH | tests.rs:1490 |
| Recipe reproducibility | BYTE-EXACT (3rd recorded verification) | ffmpeg 7.1.5 + tests.rs:977 → identical sha |
| Name describes content | YES | msmpeg4v3 codec, .asf extension (ffmpeg's default muxer for this codec) |
| Referenced by code/docs | YES | tests.rs:976, :1490; plan §1.2:60, §4.4:455 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture; legacy-fourcc demux lane by design |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | Per-file typed facts incl. fourcc `MP43` (tests.rs:1240), extradata 0 (:1243), Simple Index offset/size match in top_objects (:1164-1173), seek+resume (index present, :1304-1320). Mutation: corrupt the fourcc read (bmih[16..20]) and :1240 fails. |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | Codec-name + packet-count oracles; hand re-run: `msmpeg4v3` ✓. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "asf_msmpeg4v3.asf 7,129 66201b9205f26a90" + "legacy fourcc … regeneration deterministic, verified twice" | plan §4.4:455 | REAL | sha256sum + my third byte-exact regeneration |
| msmpeg4v3 lane = "demux + typed `cli.codec_unavailable`" (no shipped decoder) | plan §1.3:98 | REAL-UNPROVEN (as to this crate) | Demux side proven (differential); the `cli.codec_unavailable` typed refusal on the ASF lane is unreachable today (module unwired) — codec absent from engine-info's shipped set |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
None beyond the family ones (asf_mjpeg audit §9).
