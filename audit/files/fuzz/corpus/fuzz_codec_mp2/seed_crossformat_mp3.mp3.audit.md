# AUDIT: fuzz/corpus/fuzz_codec_mp2/seed_crossformat_mp3.mp3
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A08) |
| Size / sha256 | 6,476 B / f9e387c2b3fdddb69e23725bfe1c938a28c6e300798c3c7232d2737058ab92d7 |
| Type | MP3 with ID3v2.4 tag; MPEG ADTS layer III, v2, 64 kbps, 24 kHz, Stereo |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed (campaign unwired); content executed via the byte-identical unit fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256 + tree-wide digest match; cmp (×3); file(1); rg consumers; baseline greps |

## 1. One-paragraph verdict
A deliberate cross-format seed: real MP3 (layer III, MPEG-2 LSF, 24 kHz) bytes
fed to the MP2 (layer II) decoder target — the README:104-110 policy ("cross-
format seeds ... format confusion is fair fuzz material"; "Formats with no
committed real fixture yet (PS, HEVC, MP2) ride cross-format seeds today" —
MP2 indeed has no real fixture of its own in the corpus). The bytes are a
byte-identical copy of the committed unit fixture
crates/codecs/tests/fixtures/mpeg2_24k.mp3 (found by tree-wide sha match), and
the same bytes also seed fuzz_codec_mp3 and fuzz_demux_mp3stream — 4 identical
copies in tree. The name is accurate: crossformat + mp3.

## 2. What actually works (verified)
— python3 digest-match across the tree: original = crates/codecs/tests/fixtures/
mpeg2_24k.mp3 (6,476 B, sha f9e387c2… IDENTICAL); twins fuzz_codec_mp3/
seed_mp3_mpeg2_24k.mp3 and fuzz_demux_mp3stream/seed_mp3_mpeg2_24k.mp3 (cmp
IDENTICAL). file(1): "Audio file with ID3 version 2.4.0, contains: MPEG ADTS,
layer III, v2, 64 kbps, 24 kHz, Stereo".
— Content executed green via the original in baseline cargo-test-workspace.txt:
mp3_bitexact fixtures_decode_the_recorded_window_exactly (:1261 ok);
mp3_corpus_differential corpus_and_fixture_differential_vs_ffmpeg (:1269 ok);
mp3_fuzz_survival mutated_fixtures_never_panic_and_settle_typed (:1277 ok) —
the original rides the mutation harness.
— Registry wiring: fuzz_codec_mp2 row (README:72) → codecs::audio::mp2::fuzz::
fuzz_mp2_stream; corpora-present gate green (full_gate.txt:57-58, 35 targets).

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_codec_mp2/ vs crates/codecs/tests/mp2_fuzz_survival.rs:13-14
Code: `//! Committed fuzz inputs: the two mp2 fixtures ≈ 56 KB total (the ...`
What is wrong: orphan + unexercised confusion lane — the MP2 harness's own
stable survival test reads only the two REAL .mp2 fixtures (mp2_48k.mp2 /
mp2_lsf_22k.mp2); the cross-format seed (MP3 bytes into the MP2 decoder) is fed
to nothing on stable, and the campaign never ran (fuzz/README.md:165-168). The
seed's entire reason to exist — layer-III-vs-layer-II format confusion — has
zero committed executions.
What correct looks like: one cross-format pass in mp2_fuzz_survival.rs (feed
the mp3 bytes through fuzz_mp2_stream), or campaign wiring.
Confidence: HIGH — Verification: read (mp2_fuzz_survival.rs seed list; rg
zero in-tree readers of the corpus path).

F-002 [S4]
Where: 4 identical copies (original + 3 corpus seeds), no consistency guard
Code: `f9e387c2b3fdddb69e23725bfe1c938a28c6e300798c3c7232d2737058ab92d7 (all four)`
What is wrong: duplication is the corpus policy (self-contained corpora), but
no gate checks copy==original; a partial drift in any copy is invisible until
the campaign lands.
Confidence: HIGH — Verification: python3 digest match (executed).

## 4. Matrix evidence
README:104-110 names cross-format seeds explicitly, MP2 as a format with "no
committed real fixture yet"; the mp2 decoder's stable evidence is mp2_fuzz_
survival (baseline :1241 Running, green) over real .mp2 fixtures — NOT this seed.

## 5. Open questions for the next agent
- The mp2_fuzz_survival.rs:2 comment says "truncated forms of the committed
  fixtures plus the corpus" — verify whether "the corpus" there means the
  LUMEN_CORPUS env trio (D-11) rather than fuzz/corpus/; if the former, F-001
  stands unchanged.
