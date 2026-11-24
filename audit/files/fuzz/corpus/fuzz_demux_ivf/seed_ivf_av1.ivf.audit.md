# AUDIT: fuzz/corpus/fuzz_demux_ivf/seed_ivf_av1.ivf
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A08) |
| Size / sha256 | 1,937 B / fa649131fde4eaa9be22a7ba9ee25dbd9807c41d0a88006ba17488eaffab54a5 |
| Type | IVF container (DKIF header, fourcc AV01) — AV1-in-IVF, 64×48, 30 fps, 30 frames |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed (campaign unwired); content executed via the byte-identical generated fixture twin |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256 + digest-match vs fixtures; header decode; rg consumers; baseline greps |

## 1. One-paragraph verdict
A real AV1-in-IVF seed that is a byte-identical copy of the committed generated
fixture tests/fixtures/generated/ivf/av1_64x48_30fps.ivf (python3 digest match:
IDENTICAL). Header decoded from the bytes: DKIF, version 0, header 32, fourcc
AV01, width 64 (0x40), height 48 (0x30), framerate 30, timescale 1, 30 frames —
the filename of the twin is exact. The twin is heavily consumed and green; the
corpus copy is campaign-only.

## 2. What actually works (verified)
— python3: 1,937 B; sha256 fa649131…54a5; head bytes `DKIF 00 00 20 00 AV01
40 00 30 00 1e 00 00 00 01 00 00 00 1e 00 00 00` (fields as above). Twin match
IDENTICAL (tests/fixtures/generated/ivf/av1_64x48_30fps.ivf, same sha).
— Twin executed green in baseline cargo-test-workspace.txt:2293-2298
(crates/demux/tests/ivf_fixtures.rs:97 fixture row — 5/5 tests: frame_counts_
pts_ladders_and_payload_md5_match_ffmpeg, stream_parameters_match_ffprobe,
truncated_derivatives_are_typed_never_panics, fuzz_lite_over_the_real_ffmpeg_
bytes — the last feeds the real bytes through the ivf fuzz harness, the exact
harness this seed's target wraps). Sibling tests/compat/compat-matrix.json cites
the fixture family.
— Registry wiring: fuzz_demux_ivf row (fuzz/README.md:65) → demux::ivf::fuzz::
fuzz_demux_ivf; corpora-present gate green (full_gate.txt:57-58).
— Provenance: ffmpeg `-f lavfi` generated (ivf_fixtures.rs:15 header documents
the command); corpus copy policy README:99-110.

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_demux_ivf/ vs crates/demux/tests/ivf_fixtures.rs:97
Code: `fixture: "generated/ivf/av1_64x48_30fps.ivf",`
What is wrong: orphan corpus copy — no test reads fuzz/corpus/ (rg zero
in-tree readers); the twins are read by path, and the only consumer of the
copies is the never-run nightly campaign (fuzz/README.md:165-168 "No such run
has happened yet"). Same corpus-orphan family as every seed audited this wave.
Why it matters: corpus copy drift is invisible to every committed gate.
What correct looks like: campaign wiring or a copy==twin hash assertion.
Confidence: HIGH — Verification: read + python3 digest match executed.

## 4. Matrix evidence
No feature-matrix row cites the seed; the IVF demux row's stable evidence is the
twin's ivf_fixtures pins (payload md5 ladder vs ffmpeg). The corpus seed adds
nothing on stable.

## 5. Open questions for the next agent
- seed_ivf_vp9.ivf (sibling): confirm == generated/ivf/vp9_64x48_25fps.ivf when
  that fixture is audited (file 17 of this batch covers the fixture side).
