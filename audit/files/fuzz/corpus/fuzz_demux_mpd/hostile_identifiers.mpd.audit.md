# AUDIT: fuzz/corpus/fuzz_demux_mpd/hostile_identifiers.mpd
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 272 B / b000d1c614c5cc97d1d40a8aa039223374d28e1774c7ce84fb9d2c75d2079f0c / ASCII text, no line terminators (batch manifest) |
| Generated vs handmade | Read in full: single-line MPD with duplicate `bandwidth` attribute + unknown `bandwidth-broken` attribute, `$RepresentationID$-$Number$$Time$` template identifier collisions — identifier-hostile by design (matches its name). |
| Consumer | cargo-fuzz campaign only (fuzz_demux_mpd target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002). Sibling hand_*.mpd seeds exist for the non-hostile shapes. |
| Verdict | REAL seed bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
