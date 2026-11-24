# AUDIT: fuzz/corpus/fuzz_codec_hevc/seed_crossformat_mkv.mkv
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 33,443 B / c2f798f0b40d800d… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (valid-seed corpus file; EBML walk exact) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Assignment row 3076. What it is: Matroska container seed (magic 1a 45 df a3).
EBML walk: header element (0x1A45DFA3, size 35) + Segment (0x18538067, size
33,391) sums exactly to 33,443 = file size (python, EOF-exact). Valid container.

Purpose: "crossformat" seed — a *demuxer-level* format deliberately fed to the
codec-level HEVC fuzz target (fuzz/Cargo.toml:237 `fuzz_codec_hevc`), exercising
the codec path's handling of non-Annex-B/non-codec input. No test references it
by name (rg: zero hits). Not a duplicate: fuzz/corpus/fuzz_demux_es/
seed_crossformat_mkv.mkv has a different sha256 (42ece741…) — same name,
different bytes; that naming overlap is mildly confusing but each sits in its
own target's corpus dir.
