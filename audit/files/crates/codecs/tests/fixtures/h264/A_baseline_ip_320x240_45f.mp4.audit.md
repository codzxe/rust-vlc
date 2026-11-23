# AUDIT: crates/codecs/tests/fixtures/h264/A_baseline_ip_320x240_45f.mp4 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL |
| Type / size | ISO Media, MP4 Base Media v1 — 60905 bytes; `ftyp` brands `isom iso2 avc1 mp41` (verified in my byte dump), `free` + `mdat` layout follows |
| Structure (verified) | genuine MP4 container wrapping the baseline H.264 stream (twin of `A_baseline_ip_320x240_45f.h264`, same stem family A–F ladder: baseline/main-cabac/bframes/high-ipb/odd-dims/still) |
| Cited as evidence by | tests/h264_sw_differential.rs:67,360,365 (both ES and MP4 lanes assert on this stem), and the h264 fixture family across parser-corpus/demux suites |
| Commands run | `file` → ISO Media MP4 v1; `od` ftyp dump (isom/iso2/avc1/mp41 brands verified); `./target/debug/deps/h264_sw_differential-9b87180403a77d06` → 17 passed (exit 0, includes lane_a_baseline_mp4 — avcC-framed decode differential); `h264_parser_corpus` → 2 passed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Genuine ffmpeg-muxed MP4 (avc1 brand, the length-prefixed framing lane the h264 nal.rs `AvcConfig`/`AvcNals` code exists for), exercised end-to-end by the passing 17-test differential suite in both container forms. The A–F fixture ladder gives the parser real profile/geometry breadth (baseline → high, odd 127×95, still-image). No findings.
