# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc3
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | binary data (2,803 B) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed by 2 green demux tests (below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix-compare vs parent; file(1); rg consumers; baseline greps |

## 1. One-paragraph verdict
Verified derivative: an EXACT byte-prefix of the valid parent (python3: d ==
parent[:2803], cut fraction 0.8638) — precisely what the truncN convention claims
and what the consumer's comment asserts ("truncations are exact parent prefixes",
ogg_fixtures.rs:68-70). The cut lands at the exact page boundary after the headers
page (2803 B), so the Ogg demuxer sees two whole header pages and no audio page —
the pinned outcome (clean EOS, zero packets). Executed green by two baseline tests.

## 2. What actually works (verified)
— python3: size 2,803 B; sha256 2c255e26a77ac1062c8b6c04ca9b65d5a0cf4910a227d84266e96289b1a0258e;
parent tests/fixtures/generated/valid/ogg_vorbis_8k_st.ogg 3,245 B, sha
91e9db3851e5432393c008f749235dc124782428ad005ce26fb700878549646c; prefix-match TRUE.
file(1): "Ogg data, Vorbis audio, stereo, 8000 Hz, ~27200 bps" (truncation does not
break the magic — expected for a prefix).
— Consumers (executed green in baseline cargo-test-workspace.txt:2381-2387):
crates/demux/tests/ogg_fixtures.rs:563-606 ogg_derivatives_stay_inside_the_typed_
vocabulary — drives this fixture through OggDemuxer::new/start/drain and pins
bytes=0 packets=0 reasons=[] terminal="" damage=0 gdisc=0 for "trunc3"
(DERIVATIVES entry :486-494: "Cut at the exact page boundary after the headers
page (2803 B) ... a clean end with zero packets"); :612-641
ogg_fixtures_and_derivatives_survive_the_fuzz_harness — feeds the bytes through
demux::ogg::fuzz::fuzz_demux_ogg. Baseline :2384/:2385 both `... ok`.
— Generated, not handmade: `cargo xtask fixtures` derivative (op truncate, family
ogg_vorbis_8k_st; parent authored per ogg_fixtures.rs:16 `ffmpeg -f lavfi -i ...`).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml (absence) + tests/fixtures/expectations/malformed/ (absence)
Code: `rg 'ogg' tests/fixtures/generated/MANIFEST.toml -> No matches found`
What is wrong: the entire ogg_vorbis_8k_st derivative family (10 files) is
unregistered in the MANIFEST drift gate (avi/mp4/mkv/webm/wav/aiff families are
pinned there) and has no expectation TOML — no sha256/size pin exists anywhere for
this file. A benign in-place byte drift (same page, demuxer-identical behavior)
would pass every committed test silently.
Why it matters: same unpinned-family defect RR2/RR3 flagged for mp4_h264_aac and
mkv_h264_aac; the family relies solely on behavioral pins.
What correct looks like: MANIFEST.toml rows (path/sha256/size_bytes) for the 10
ogg derivatives, or an explicit family exemption documented in CORPUS.md.
Confidence: HIGH — Verification: read (rg executed, zero hits).

## 4. Matrix evidence
Class-B corpus member cited by tests/fixtures/CORPUS.md's ogg family; recovery
tier: demux-level halves in demux's fixtures (recovery_matrix.rs:6-9 names demux
fixtures_class_b as the demux-level pinning half — for ogg that half is
ogg_fixtures.rs, which is what actually ran).

## 5. Open questions for the next agent
- trunc3's "clean end, zero packets" pin is behaviorally identical to an empty
  stream — is a probe-level classification test (fixtures_matrix covers avi/mp4
  class-B only) ever planned for the ogg family?
