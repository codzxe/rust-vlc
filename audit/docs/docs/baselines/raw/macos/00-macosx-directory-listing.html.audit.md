# DATA AUDIT: docs/baselines/raw/macos/00-macosx-directory-listing.html (credit: W19-A04, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | raw HTML index of /pub/videolan/vlc/3.0.23/macosx/ — 3 DMGs + 4 sidecars each |
| Live re-fetch | DONE — byte-identical (diff exit 0) |
| Cited by | docs/baselines/vlc-3.0.23-macos.md:28-30 (sizes) and :95-97 (URLs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
AUTHENTIC and CURRENTLY VERIFIED: I fetched https://download.videolan.org/pub/videolan/
vlc/3.0.23/macosx/ live (HTTP 200) and `diff` against this file -> IDENTICAL (exit 0).
Consumers' numbers match the dump exactly: vlc-3.0.23-macos.md:28-30 records
arm64=51,273,389 / intel64=57,544,217 / universal=89,124,160 bytes — all three byte sizes
match the listing rows (`51273389`, `57544217`, `89124160` in lines 5, 10, 15; `sed -n
'5p;10p;15p'` verified). All three DMG URLs in the doc point at the get.videolan.org alias
of this same tree; dump 09 already confirmed that alias serves identical content. Full
sidecar set (.asc/.md5/.sha1/.sha256) present for all three artifacts — internally
consistent with the no-.sha512 finding recorded in dump 09. Zero defects; evidence-hygiene
note only: this dump has NO header (no date/command) unlike its linux siblings — the file
name and sibling numbering (00- vs 09-) carry the provenance instead.
