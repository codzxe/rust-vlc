# DATA AUDIT: docs/baselines/raw/macos/06-web-citations.txt
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A04) |
| Type | Raw web-citation evidence log (59 lines), agent 0.A3, 2026-10-19 |
| Network re-run? | NO (offline sandbox) — cross-file consistency checked instead |
| Chain | 00/01 (listings) · 03 (downloads) · 04 (hashes) · 05 (gpg) all present |

## 1. Blunt verdict
Internally consistent and cross-corroborated at every point I can check
without network: the pinned release date 2026-01-08 matches MASTER-PROMPT.md,
the archive-tree inventory matches the committed directory listing exactly
(3 dmg + 4 sidecar types, no .sha512 — stated in listing line 62), the
macos/→macosx/ 404 correction is recorded verbatim in evidence 01, and the
3.0.24 drift risk is logged in doc 02 §2.5. The web quotes themselves are
offline-unverifiable, and the file says so plainly.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "RELEASE DATE ... 2026-01-08 (matches MASTER-PROMPT §1 'released 2026-01-08')" | :24 | REAL | docs/MASTER-PROMPT.md:16 "(desktop, released 2026-01-08)" |
| 2 | tree lists exactly `vlc-3.0.23-{arm64,intel64,universal}.dmg` + .asc/.md5/.sha1/.sha256; "No .sha512 sidecar, no .pkg, no .zip" | :45-46 | REAL | 01-directory-listing.txt:17-26 full listing; :62 "- No .sha512 sidecars are published in this directory." |
| 3 | "macos/" returns nginx 404 (verbatim body recorded in raw evidence 01)" | :49-51 | REAL | 01:41-44 `<head><title>404 Not Found</title></head>` + `nginx/1.31.4` |
| 4 | "videolan.org front-door ... now serves 3.0.24 ... Baseline stays 3.0.23 (doc 02 §2.5)" | :52-54 | REAL | docs/02-phase0-audit-plan.md:140 "VLC 3.0.24 has since been published ... The baseline stays 3.0.23" + :676 drift row |
| 5 | fetch results: exit=0, byte sizes 44,250 / 42,822 | :7,29 | UNVERIFIED | network claims; not re-runnable offline |
| 6 | D4: signing key EXPIRED yet Good signatures | :57-58 | REAL (cross-ref) | 05-gpg-verification.txt:37-43 verbatim [expired] + Good signature |
| 7 | "Page copies live only in gitignored tmp/pinning/" | :3 | REAL | `rg --files | rg '\.dmg|\.html'` -> no binaries/pages committed; 00-macosx-directory-listing.html is the recorded listing itself |

## 3. Contradictions
None — 5 sibling raw-evidence files tell one coherent story (path
correction, sizes, hashes, signatures, dates).

## 4. Fabrication / inflation indicators
None — the discrepancy log (D1-D4) records its own errors and limits. Findings: none.
