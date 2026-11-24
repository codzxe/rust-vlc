# DATA AUDIT: docs/baselines/raw/macos/03-dmg-downloads.txt (credit: W19-A08, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | Raw download capture log (44 lines): 3 dmg downloads + sizes + file identification |
| Live re-verified? | YES — 3/3 sizes via HEAD/actual download + UDIF koly trailer byte-identical via range request |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Fully live-reproduced today (2026-11-24):
- arm64: actually downloaded in this audit's sibling check — 51,273,389 bytes ==
  record :12/:28 ✓ (hashes already byte-verified in the 04-hash-crosscheck audit).
- intel64: live `HEAD https://download.videolan.org/.../intel64.dmg` ->
  content-length 57544217 == record 57,544,217 ✓.
- universal: live `HEAD https://get.videolan.org/.../universal.dmg` ->
  Content-Length 89124160 == record 89,124,160 ✓.
- UDIF trailer: live range request for the universal dmg's last 512 bytes returns
  `k   o   l   y  \0  \0  \0 004  \0  \0 002  \0  \0  \0  \0 001` — BYTE-IDENTICAL to
  the record's :43 od dump (koly magic + same trailing header bytes) ✓.
- The internal size cross-check table (:28-30) lists the same three sizes — the
  record is internally consistent AND externally still true.
- The :4-6 note (task-card URL's "macos/" 404s, corrected to "macosx/") is the same
  honest-correction pattern raw evidence 01 documents; the working macosx/ paths are
  the ones I fetched live above.
No defects. Checked: 3/3 sizes live, koly trailer byte-diff, internal table
consistency, URL-path correction note.
