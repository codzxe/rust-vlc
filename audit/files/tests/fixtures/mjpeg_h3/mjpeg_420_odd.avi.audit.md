# AUDIT: tests/fixtures/mjpeg_h3/mjpeg_420_odd.avi
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 17,718 B / c97ae476c7207a6e10b63aadb173cc2885d9fc61b31bc32896b6dce7635c0a21 / RIFF AVI 86×62 10fps MJPEG (odd-dimension 4:2:0 probe; batch manifest) |
| Registration | NOT covered anywhere: `rg mjpeg_h3 tests/fixtures/CORPUS.md` → no hit (CORPUS.md layout lists only generated/ + expectations/); not in MANIFEST.toml; the mjpeg_h3/ directory sits entirely outside the documented corpus |
| Consumers (rg) | crates/codecs/tests/mjpeg_field_differential.rs (odd-dimension chroma field handling) |
| Verdict | SHALLOW provenance, real usage. F-001 [S3]: a fixture directory (mjpeg_h3/) that no ledger, no CORPUS.md layout entry, and no drift gate covers; provenance is undocumented. The consumer test is real (field differential on 86×62 odd dims), so the bytes are exercised.
