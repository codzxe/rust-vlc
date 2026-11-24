# AUDIT: fuzz/corpus/fuzz_playlist_xspf/seed_dirty.xspf
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 281 B / c61336fa2fa1cd12… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (hostile text seed, hand-authored in-tree) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: deliberately dirty XSPF: `duration` = `not-a-number`, malformed
entity `&badent;`, unclosed `<title>`, mismatched close tags (`</wrong>`, stray
`<playlist>`), one well-formed track. Hostile coverage for the playlist XSPF
parser (fuzz_playlist_xspf target). Hand-authored (not machine-generated) —
content verified by full read above. Consumer: corpus-dir convention
(fuzz/corpus/fuzz_playlist_xspf/ + fuzz/Cargo.toml target fuzz_playlist_xspf);
no in-repo test reads it by name. In-crate twin harness: crates/... playlist
fuzz module embeds equivalent shapes (see fuzz_playlist_xspf.rs audit).
