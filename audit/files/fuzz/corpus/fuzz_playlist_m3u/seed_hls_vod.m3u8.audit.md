# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_hls_vod.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | text seed, 17 lines, 238 bytes |
| File verdict | REAL (valid HLS VOD playlist) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file read; python3 sha256 (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Hand-authored HLS VOD seed (README's "hand-authored text ... seeds (CC0, ours)"):
`#EXTM3U`, `#EXT-X-VERSION:3`, `#EXT-X-TARGETDURATION:2`, `#EXT-X-MEDIA-SEQUENCE:0`,
`#EXT-X-PLAYLIST-TYPE:VOD`, five `#EXTINF` segments (2.043356/1.996922/1.996911/1.996922/
1.012433 -> pl0..pl4.ts), `#EXT-X-ENDLIST`. All EXTINF values ≤ TARGETDURATION 2.0? NO —
2.043356 > 2 — a spec-violating EXTINF/TARGETDURATION pair, which is exactly the kind of
dirty truth a playlist fuzzer should chew on (whether deliberate or accident, it is real
fuzz material; the streaming tests/fixtures/hls lane uses its own pl.m3u8, not this seed).
sha256 3b6dc594bb0767dd... (recomputed).

## 2. Consumers
fuzz/fuzz_targets/fuzz_playlist_m3u.rs (registry-gated — gate RAN by me, CLEAN, corpora
present). Campaign unwired.

## 3. Findings
None. Checked: tag spelling (all standard), segment-name shape, ENDLIST terminal.

## 4. Open questions
- The 2.043356 > TARGETDURATION quirk: worth knowing when the HLS interpretation tests
  pin their expectations (W-next: streaming/hls tests).
