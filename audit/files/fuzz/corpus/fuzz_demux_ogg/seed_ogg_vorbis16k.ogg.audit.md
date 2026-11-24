# AUDIT: fuzz/corpus/fuzz_demux_ogg/seed_ogg_vorbis16k.ogg
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A08) |
| Size / sha256 | 3,796 B / 58046ef1733838a4716108059be5c7d130eb1ea5ec8c6ed334148d026c6a8638 |
| Type | Ogg Vorbis audio, MONO, 16000 Hz, ~40000 bps |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed (campaign unwired); content executed via byte-identical unit-fixture twin |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256; cmp vs unit fixture (IDENTICAL); file(1); sibling cmps; rg consumers; baseline greps |

## 1. One-paragraph verdict
YES — duplicate of the unit fixture, byte-for-byte: cmp says IDENTICAL to
tests/fixtures/generated/valid/ogg_vorbis_16k_mono.ogg (3,796 B, same sha
58046ef1…). The whole fuzz_demux_ogg corpus is assembled from committed fixtures
(seed_ogg_chain.ogg == valid/ogg_chain_two_links.ogg IDENTICAL;
seed_ogg_trunc0.bin == malformed/ogg_vorbis_8k_st__trunc0 IDENTICAL, 12-B page
header), exactly the README:99-110 seed policy. Content is executed green through
the twin (TRUTHS row pins codec/rate/packets/granule/md5); the corpus copy itself
is campaign-only.

## 2. What actually works (verified)
— cmp executed: `fuzz/corpus/fuzz_demux_ogg/seed_ogg_vorbis16k.ogg
tests/fixtures/generated/valid/ogg_vorbis_16k_mono.ogg -> IDENTICAL` (sha above,
both computed). file(1): "Ogg data, Vorbis audio, mono, 16000 Hz, ~40000 bps".
— Twin executed green in baseline cargo-test-workspace.txt:2381-2387
(ogg_fixtures.rs:157 TRUTHS row — every_ogg_fixture_matches_the_ffmpeg_truth :2383
ok; ogg_fixtures_and_derivatives_survive_the_fuzz_harness :2385 ok feeds the
valid fixtures' bytes through demux::ogg::fuzz::fuzz_demux_ogg — the exact
harness the seed targets).
— Registry wiring: fuzz_demux_ogg row (README:59) → demux::ogg::fuzz::fuzz_demux_ogg;
corpus presence gate green (full_gate.txt:57-58).

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_demux_ogg/ vs crates/demux/tests/ogg_fixtures.rs:149-178
Code: `const TRUTHS: [Truth; 3] = [ Truth { fixture: "generated/valid/ogg_vorbis_16k_mono.ogg", ...`
What is wrong: orphan corpus copies — no test reads fuzz/corpus/ (rg 'seed_ogg'
over crates/tests/tools: zero hits); the sweep/fixture suites read the twins by
path, and the only consumer of the copies is the never-run nightly campaign
(fuzz/README.md:165-168). Drift between copy and twin would be caught by nothing
committed (only the registry gate's non-empty-dir check).
What correct looks like: campaign wiring, or a copy==twin hash assertion in a
committed test.
Confidence: HIGH — Verification: read + cmp executed.

F-002 [S4]
Where: the seed's filename vs its content
Code: `fuzz/corpus/fuzz_demux_ogg/seed_ogg_vorbis16k.ogg: Ogg data, Vorbis audio, mono, 16000 Hz, ~40000 bps`
What is wrong: the twin's name says 16k_mono; the seed name drops the channel
count — a reader comparing corpora across targets (fuzz_codec_vorbis's
seed_ogg_vorbis8k_st.ogg IS stereo-named) cannot tell mono from stereo without
running file(1). Cosmetic naming debt only — the bytes are the accurate twin.
Confidence: HIGH — Verification: ran file(1).

## 4. Matrix evidence
README:99-110 corpus policy ("copied from the committed CC0 fixtures"); no
feature-matrix row cites the seed; the ogg demux row's stable evidence is the
twin's TRUTHS pin.

## 5. Open questions for the next agent
- seed_ogg_trunc0.bin is a 12-B "OggS" page-header-only file — when that corpus
  entry is assigned, confirm the twin (malformed/ogg_vorbis_8k_st__trunc0) keeps
  its DERIVATIVES pin (trunc0 = typed demux.ogg.no-streams refusal).
