# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/init.mp4 (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Size / type | 764 bytes; `file` → "ISO Media, MP4 Base Media v5" (verified) |
| File verdict | REAL artifact, METADATA-ONLY consumption (length pinned; bytes never read) |
| Cited as evidence by | docs/feature-matrix.toml:1167; docs/formats/streaming-hls.md:99-100; docs/worklog/archive-2026-10.md:9461 ("init.mp4 = 764 B") |
| Findings | S4: 1 |

## 1. Blunt verdict
A structurally correct fragmented-MP4 init segment: `ftyp` box (28 B, brands iso5/iso6/mp41), `moov` (736 B) containing `mvhd` with timescale 1000 and duration 0 — exactly the shape an `-hls_segment_type fmp4` init section has. Its length (764) is the only thing any test touches: hls_attrs.rs:192-193 calls `fixture_len("fmp4", "init.mp4")` and pins `== 764`, guarding the MAP fixture's identity. The MAP URI ("init.mp4") is asserted on every entry of the parsed playlist; the bytes themselves are never read — no test parses its boxes or serves it over a socket.
F-001 [S4]: byte content never read by any test — the length pin plus the genuine box structure make it honest on-disk truth, but "fMP4 support" evidence must not be read out of this file (the streaming crate's fMP4 handling, where it exists, is tested elsewhere). Consistent with W10-A07's finding for the sibling pl0.m4s/pl4.m4s (inert realism). Confidence HIGH — Verification: od dump (ftyp/moov/mvhd parsed by hand), ls, rg for readers.
