# Wave 18 SUMMARY — Claim ledger A: feature-matrix.toml (252 rows)

Auditor: W18-A01 (orchestrator, script `scripts/matrix_ledger.py` + executed-evidence map).
Method: all 252 rows extracted (tomllib); verdict per §5 scale; REAL only where the orchestrator executed the feature end to end on this host; full per-row table in `audit/waves/wave-18/rows-verdicts.tsv`; claim ledger in `audit/docs/feature-matrix.toml.audit.md`.

## Verdict distribution (252 rows)
| Verdict | Rows | Meaning |
|---|---|---|
| REAL | **4** (1.6%) | executed end to end by this audit (linux): container-mp4-mov-3gp, codec-video-h264, codec-video-h264-software, codec-audio-aac-lc |
| REAL-UNPROVEN | 11 | the executed workspace suite (1001 passed) includes real conformance/differential lanes over them (mjpeg, pcm, flac, mp3, vorbis, opus, adpcm, avi, wav, flac-native, mp3-raw) |
| SHALLOW | 52 | implemented cells whose only backing is path-existence evidence (the empty-file experiment standard) |
| FALSE | 1 | config-advanced-folding: implemented cells citing a nonexistent evidence path |
| PARTIAL | 45 | as claimed |
| STUB/PAPER as claimed | 43 + PAPER-ONLY 96 | honest absence claims, default-true at sweep level, not re-derived file-by-file this run |

## Key executed facts behind the verdicts
- The MP4 litmus row set is REAL: `lumen open+advance` on a real H.264+AAC MP4 → `Playing → Ended`, `time 3.041814s (100.0%)`, exit 0 (`audit/runs/baseline/mp4-smoke-open-advance.txt`); codec-video-h264-software's evidence string cites the missing path `crates/codecs/src/video/h264/sw` (feature-audit R4, executed exit 1) — REAL feature, FALSE evidence string.
- verified-only parity: 0/252 (executed parity-report) — no row anywhere claims `verified`; v0.1.0–v0.1.2 shipped on implemented-tier claims.
- The matrix's own meta: last_updated 2026-11-09; PARITY-REPORT still quotes 64/252 symmetric (stale: live is 66/61/66, asymmetric).

## Anti-laziness gate
Row counts cross-checked: 252 asserted by tomllib parse = feature-audit's own census (executed). Verdicts only from executed evidence or explicit sweep-level caveats; no row was marked REAL without a pasted run. PASS.

## Blocked items
- windows/macos/web platform cells: executed on linux only; per-cell cross-OS verdicts UNVERIFIED (no Windows/macOS host in this sandbox).
