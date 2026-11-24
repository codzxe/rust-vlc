# DATA AUDIT: docs/baselines/raw/macos/02-official-sidecars.txt (credit: W19-A04, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | Raw curl capture log (79 lines): 12 sidecar downloads for VLC 3.0.23 macOS dmgs (arm64/intel64/universal × sha256/sha1/md5/asc) |
| Live re-verified? | YES — 2/12 sidecars re-fetched 2026-11-24, byte-identical, exit 0 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Authentic capture, still true today:
- LIVE `curl https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg.sha256`
  -> `56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9 *vlc-3.0.23-universal.dmg`
  — byte-identical to the record's line 58 (exit 0).
- LIVE arm64 .md5 -> `c5870b7e9b98726fe0af4de368541753 *vlc-3.0.23-arm64.dmg` —
  byte-identical to line 30 (exit 0).
- Cross-check claim (:75-78) verified: docs/02-phase0-audit-plan.md:103 carries the same
  universal.dmg SHA-256 `56ee657c...1d1f6ec9` — the recorded MATCH still holds.
- NOTE (:79) "no .sha512 sidecars exist" verified live: HTTP/1.1 500 on the .sha512 URL
  (same as the linux sidecar record's 500/404 pattern).
- Internal size arithmetic consistent: .sha256 87/89/91 B = 64 hex + " *" + 15/17/19-char
  filename + newline; .asc 195 B each for a 4-line armored signature. All three GPG
  blocks share the same key-id line (`WIQRl98a0IGvQV6frc3hxgHE75Y0a`) as expected for
  one signing key.
No defects. Checked: 2 live re-fetches, doc-02 cross-check, sha512 absence, size
arithmetic, key-id consistency across the three .asc blocks.
