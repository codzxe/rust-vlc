# AUDIT: fuzz/corpus/fuzz_http_headers/cross_format_pl.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (HLS playlist deliberately fed to the HTTP header parser) |
| Size / sha256-prefix | 238 bytes / 3b6dc594bb0767dd (verified locally) |
| File verdict | REAL — deliberate cross-format seed |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256, full text dump |

## 1. Verdict
Real cross-format seed: a well-formed HLS VOD playlist (EXtm3u header,
TARGETDURATION, 5 EXTINF segments, ENDLIST) placed in the
`fuzz_http_headers` corpus — exactly the "cross-format seeds ... format
confusion is fair fuzz material" policy (fuzz/README.md:104-106). The
target is `fuzz_http_headers` (HTTP response-head grammar); the file
exercises header-line parsing against media-playlist text. Presence gated
(baseline full_gate.txt:57-58).

## 2. What actually works (verified)
- Verbatim (first lines of the 238-byte file):
```
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:2
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-PLAYLIST-TYPE:VOD
```
- Directory: fuzz/corpus/fuzz_http_headers also holds ok_200.txt,
  not_found_404.txt, chunked_200.txt, ranged_206.txt, odd_headers.txt —
  the real HTTP-head arms; this file is the confusion arm (os.listdir).

## 3. Findings
None.
