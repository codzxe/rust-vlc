# AUDIT: fuzz/corpus/fuzz_hls_playlist/pl.m3u8
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 17 / data file (HLS VOD media playlist) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed only by the unwired cargo-fuzz campaign (corpus resolution README:112-116); CONTENT proven via byte-identical twin consumed by 3 green tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256/size; cmp vs both same-named fixtures; rg consumers; baseline greps |

## 1. One-paragraph verdict
Real seed, honestly sourced: an ffmpeg-authored HLS VOD media playlist (VERSION 3,
TARGETDURATION:2, PLAYLIST-TYPE:VOD, 5 EXTINF segments pl0..pl4.ts, ENDLIST) whose
bytes are IDENTICAL to the committed fixture crates/playlists/tests/fixtures/hls/
ts_vod/pl.m3u8 (cmp: identical). The corpus copy itself is dead weight until the
campaign wiring lands; its content is nonetheless fuzz-executed on stable through
the twin (netfuzz_sweep loads the playlists-crate path, not fuzz/corpus/).

## 2. What actually works (verified)
— python3: 238 B, sha256 3b6dc594bb0767dd4472fb3ab4dcde3b11f948c86d3b5cd78e1ffd0207b6a5b5.
— cmp: identical to crates/playlists/tests/fixtures/hls/ts_vod/pl.m3u8 (that dir
also holds the real segments pl0..pl4.ts). DIFFERS from the unrelated
tests/fixtures/hls/ts_vod/pl.m3u8 (223 B, seg000-3, TARGETDURATION:3 — a second
ffmpeg-authored playlist of the same name, see F-002).
— Content executed green in baseline via the twin:
netfuzz_sweep_hls_playlist_survives (:5714 ok, sweep reads
crates/playlists/tests/fixtures/hls/ per netfuzz_sweep.rs:121-122 — the module doc
:25 "the same files are the committed fuzz/corpus/ seeds" is TRUE, byte-proven);
hls::tests::ffmpeg_fixture_ts_vod_types_fully (:4631 ok);
ffmpeg_hls_ts_vod_matches_ffprobe_truth (:4711 ok, hls_vod_e2e.rs).
— Corpus gate presence: full_gate.txt:57-58 (35 targets, corpora present, PASS).

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_hls_playlist/pl.m3u8 vs crates/streaming/tests/netfuzz_sweep.rs:121-122
Code: `let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../playlists/tests/fixtures/hls");`
What is wrong: no stable consumer reads fuzz/corpus/ — the sweep reads the twin
by path; the corpus copy's only consumer is the never-run nightly campaign
(fuzz/README.md:165-168 "No such run has happened yet"). Same corpus-orphan family
flagged for seed_adpcm_yamaha.bin by W16-RR2.
Why it matters: drift between the two copies would never be caught by any
committed test (only the registry gate's non-empty check).
What correct looks like: campaign wiring (the outstanding LEAD-DECISION), or the
sweep asserting hash-equality corpus↔fixture.
Confidence: HIGH — Verification: read + cmp executed.

F-002 [S4]
Where: tests/fixtures/hls/ts_vod/pl.m3u8 (223 B, sha a48a7b58…) vs crates/playlists/tests/fixtures/hls/ts_vod/pl.m3u8 (identical to this seed)
What is wrong: two committed playlists share the path-suffix ts_vod/pl.m3u8 with
DIFFERENT content (4 segs/TARGETDURATION:3/ALLOW-CACHE vs 5 segs/TARGETDURATION:2/
PLAYLIST-TYPE:VOD). Any evidence string citing "ts_vod/pl.m3u8" without its crate
root is ambiguous.
Confidence: HIGH — Verification: cmp + python3 hashes of both.

## 4. Matrix evidence
README:99-110 seed policy ("copied from the committed CC0 fixtures"); registry row
fuzz_hls_playlist (README:89) maps the target whose corpus this seeds. No
feature-matrix row cites this exact path (compat-matrix jsonl cites hls fixtures
generally).

## 5. Open questions for the next agent
- The sibling seeds malformed.m3u8 / multi.m3u8 / single.m3u8 / fmp4_pl.m3u8 are
  the master/fmp4 twins — same orphan-copy pattern, audit when assigned.
