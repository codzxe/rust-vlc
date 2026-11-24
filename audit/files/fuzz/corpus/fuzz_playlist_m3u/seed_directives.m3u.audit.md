# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_directives.m3u
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | text seed, 12 lines, 240 bytes |
| File verdict | REAL (adversarial M3U directive seed) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file read; python3 sha256 (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Good hand-authored adversarial seed: standard directives (`#EXTM3U`,
`#EXTVLCOPT:http-user-agent=Lumen/1.0`, `#EXTGRP:Favourites`), an UNKNOWN directive with
colons in its value (`#EXTCUSTOM-UNKNOWN:some value: with colons`), a title containing a
colon (`#EXTINF:42,Title with: colon`), an absolute URL entry, a second `#EXTVLCOPT`
AFTER the first entry (mid-list directive), and a relative path entry
(`plain/track two.wav` — a space in a path). This exercises exactly the tokenizer edges
(colon-in-value, unknown tags, directive-after-entry ordering) that the M3U parser's
pinned behaviors cover.

## 2. Consumers
fuzz/fuzz_targets/fuzz_playlist_m3u.rs (registry-gated — gate RAN by me, CLEAN).
The playlists crate's own unit tests cover these directive shapes (baseline: playlists
PACKAGE exit_code=0).

## 3. Findings
None. Checked: directive spellings, entry shapes (URL + relative-with-space).

## 4. Open questions
None.
