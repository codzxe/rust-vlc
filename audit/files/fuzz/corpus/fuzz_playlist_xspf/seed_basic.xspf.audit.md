# AUDIT: fuzz/corpus/fuzz_playlist_xspf/seed_basic.xspf
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 736 B / 5ef94fab666052e028a8bf2c7a3bd468c8bd544a7733e538ba4ab140dce60095 / XML 1.0 ASCII (batch manifest) |
| Generated vs handmade | Read in full: real XSPF with xml declaration, xspf namespace, 3 tracks covering entity escapes (&amp; &lt; &#38; &#x27;), file:// + http + RELATIVE locations, zero duration, unicode entity — a well-shaped "basic" seed, handwritten quality |
| Consumer | cargo-fuzz campaign only (fuzz_playlist_xspf target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | REAL seed bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
