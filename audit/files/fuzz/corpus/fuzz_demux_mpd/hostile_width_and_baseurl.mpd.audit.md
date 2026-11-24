# AUDIT: fuzz/corpus/fuzz_demux_mpd/hostile_width_and_baseurl.mpd
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 10,241 B, ONE line (no newline), sha256 cba37cbb44bbe7d1…. Content verified by head + python counts: <MPD type="static"> + Period PT8S + AdaptationSet + SegmentTemplate media="s$Number" padded with a ~10 KB identifier token, exactly 1 BaseURL, 1 Representation, 1 width attribute. Hostile shapes exercised: oversize $Number identifier in the media template (the printf-tag substitution path), giant single attribute, BaseURL folding under an oversized sibling. No xmlns declaration (bare <MPD>) — fine for a hostile seed. Consumers: fuzz_demux_mpd target (card 6.C1c); also the matrix row evidence at docs/feature-matrix.toml:1453 names the target. Real hostile input, not filler bytes.
