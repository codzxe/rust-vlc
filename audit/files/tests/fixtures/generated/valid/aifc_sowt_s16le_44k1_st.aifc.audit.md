# AUDIT: tests/fixtures/generated/valid/aifc_sowt_s16le_44k1_st.aifc
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 35352 bytes / n-a (binary fixture; assignment listed 5 #part chunks of one file) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test-only asset; consumed at crates/demux/tests/wav_aiff_fixtures.rs:142 and crates/media-probe/tests/registry_roundtrip.rs:92) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256 batch -> audit/runs/W16-A05.datahashes.tsv; python3 chunk walk; rg consumers |

Data audit. AIFF-C (FORM/FVER/COMM/SSND; `sowt` = little-endian s16), 35352 bytes, sha256 8e9b4f5569adbb5f… (audit/runs/W16-A05.datahashes.tsv row 12). Chunk walk executed: `b'FVER' 4 / b'COMM' 24 / b'SSND' 35288` — SSND payload 35280 bytes = 8820 frames x 2ch x s16 at 44100 Hz, exactly matching the test truth `frames: 8_820` (crates/demux/tests/wav_aiff_fixtures.rs:142-146, `fixture: "generated/valid/aifc_sowt_s16le_44k1_st.aifc", codec: "pcm_s16le", rate: 44_100, channels: 2, frames: 8_820`) and docs/worklog/archive-2026-10.md:10358 ("aifc_sowt_s16le_44k1_st 2ch 44.1k -> 8820 (exact)").

Executed evidence: audit/runs/baseline/cargo-test-workspace.txt:2436+ — `every_aiff_fixture_matches_the_ffmpeg_truth ... ok`; committed decode outcome docs/corpus/compat-matrix-20261110.raw-rows.jsonl:58 — status "decoded", codec pcm_s16le, pcm_bytes 35280, pcm_md5 5d105ceb212ad5d36e4f5608def3d8b7. S4: unlike the malformed corpus, this fixture has NO hash pin in tests/fixtures/generated/MANIFEST.toml (28 [[files]] entries; rg "aifc" -> no match) and no generator path — it is a committed-as-is binary whose provenance rests only on the worklog; a regenerate-and-compare gate cannot catch silent drift for it (the demux truth test does pin behavior).
