# AUDIT: tests/fixtures/expectations/valid/avi_mjpeg_2pcm.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | data contract, 82 lines |
| File verdict | REAL (sha/size cross-checked against the fixture; internal arithmetic consistent) |
| Reachable from shipped entry point? | NO (test contract; consumed by first-vertical tests + fixtures --check) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256 cross-check (exit 0); arithmetic check (exit 0); rg consumers (exit 0) |

## 1. One-paragraph verdict
Dense, internally consistent pinned contract for the multi-track parent: sha256
14d3a0e9... (I recomputed the fixture's hash — MATCH) and size 497_160 (MATCH). Track plan
(mjpeg 320x240 @10fps + pcm_s16le 48k stereo + pcm_u8 44.1k mono), duration 2s, seek targets
with tolerance, first_frame luma hash, audio_probe MD5 for stream 1 only — with an explicit
honest omission note for stream 2, 20 per-sample video digests, audio stream digests with
byte counts, and a mid-play track-switch plan (the W2-FIX-MT AC). Arithmetic verified:
48 000 Hz × 2 s × 2 ch × 2 B = 384 000 = audio_streams_bytes[0]; 44 100 × 2 × 1 × 1 = 88 200;
20 samples @ 10 fps = 2 s. All consistent.

## 2. What actually works (verified)
- sha256/size: recomputed vs fixture -> exact match (python3).
- Consumers (rg, exit 0): tests/soak/scenarios*.toml:102-127 (fixture rotation lists),
  tools/bench-vs-vlc/tests/lumen_pcm.rs:80 (digests the FIRST audio track through the real
  path), tests/integration/first-vertical scenarios; the drift gate `cargo xtask fixtures
  --check` byte-compares this TOML (fixtures.rs:2893-2895 "Ledger files ... byte-compared
  by `--check`").
- Honest omission (verified read): `# ... Stream 2 carries no probe section — ... (honest omission, not an oversight).`

## 3. Findings
F-001 [S4] The `[essence_v0]` digests are generator-truth (written by the same code that
authored the fixture bytes) — self-referential for regression purposes (they pin the
generator's determinism, not decode correctness; the decode-truth rows are first_frame/
audio_probe, which the header honestly labels as recorded from Lumen's own decode). Note
only — the header itself discloses this distinction. Confidence: HIGH — read.

## 4. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| sha256 = 14d3a0e9... / size 497_160 | this TOML | REAL | recomputed match |
| "no class-B derivatives (recorded follow-up)" | CORPUS.md:76 | REAL | rg: only valid/2pcm rows exist; no malformed/avi_mjpeg_2pcm__* on disk (ls verified) |
| stream-2 probe omission is honest | comment lines 46-48 | REAL | no [audio_probe] second entry present |

## 5. Open questions
- The switch_plan's post-switch contract (pcm_u8 44.1k mono) — which test executes it?
  (playback-engine scenario family — W-next.)
