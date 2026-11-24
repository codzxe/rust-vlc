# AUDIT: fuzz/corpus/fuzz_codec_flac/seed_flac_mono16.flac
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 12,243 B / aa5969e3636b5aa46e885c6060e4928dd1df5ebbe4e7e51521eb417134bb7aa2 |
| File verdict | REAL (valid FLAC seed; metadata parsed) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: valid FLAC — fLaC + STREAMINFO(34) + VORBIS_COMMENT(44) +
PADDING(8192, last) then audio frames; first frame sync fff85908 valid, srate
index 5 = 44,100 Hz, channel index 4 = mono — name `mono16` accurate. Parsed
independently with python.

S4 (duplication): byte-identical (sha256 aa5969e3…) to BOTH
fuzz/corpus/fuzz_demux_flac/seed_flac_mono16.flac AND
crates/codecs/tests/fixtures/flac_mono_s16_44100.flac — three committed copies
(36 KB duplication). Consumers: codec fuzz target + demux fuzz target by
corpus convention; the crates/codecs copy is consumed by name by the codecs
test suite (flac_bitexact.rs family). No drift gate cross-checks the copies
(except the codecs fixture path, which is in its own ledger? — UNVERIFIED).
