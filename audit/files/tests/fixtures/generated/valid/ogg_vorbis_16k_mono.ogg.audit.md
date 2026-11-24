# AUDIT: tests/fixtures/generated/valid/ogg_vorbis_16k_mono.ogg
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 3,796 B / 58046ef1733838a4… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; page walk exact) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |

What it is: Ogg/Vorbis fixture, 3 pages — BOS (ht=2, seq 0, 30 B), header page
(ht=0, seq 1, 3,320 B), data+EOS (ht=4, seq 2, 336 B, granule 6,400) — python
page walk sums exactly to 3,796 = EOF. Stream serial 1,983,438.

Consumer: crates/demux/tests/ogg_fixtures.rs:19-24 (module doc: "`ogg_vorbis_
16k_mono.ogg` — the same shape at 16 kHz mono" and a byte-for-byte invariant vs
the 8k parent family). Generated-vs-handmade: **ffmpeg-authored** —
docs/worklog/archive-2026-10.md:8587 verbatim: "Generated ffmpeg-authored
fixtures (lavfi sine, ffmpeg 7.1.5; ≈23 KiB committed < 200 KiB mandate): ...
ogg_vorbis_16k_mono.ogg (1983438)" — the serial number matches my parse.

F-001 [S2] Where: tests/fixtures/CORPUS.md:12-16 vs this file.
Code (CORPUS.md:13-15): "Every file under `generated/` and `expectations/` is
authored in-tree by the deterministic generator in `tools/xtask/src/cmd/fixtures.rs`,
... no external encoder output — ffmpeg was used only as a local cross-check
oracle during development, never in the build."
What is wrong: this fixture lives under `generated/valid/` and is external
encoder output (ffmpeg 7.1.5 lavfi sine per the worklog; serial 1983438 matches).
docs/inventory/formats.md:262 confirms: "ffmpeg fixture generation commands not
preserved; regeneration recipe approximate". The blanket provenance/license
sentence is false for the whole wav/aiff/ogg second-generation fixture set. Why
it matters: the corpus ledger is the licensing authority for the test corpus; a
false "no external encoder output" claim undermines the clean-room story the
project leans on (AGENTS.md §3.1). What correct looks like: scope the sentence to
the 28 ledger rows and add a separate ffmpeg-authored fixture ledger with
provenance. Confidence: HIGH — Verification: ran (byte parse + worklog +
formats.md). S4: not in the 28-row drift ledger.
