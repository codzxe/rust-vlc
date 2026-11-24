# CLAIM AUDIT: docs/baselines/vlc-3.0.23-linux.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 7 |
| REAL 4 / REAL-UNPROVEN 2 / FALSE 0 / UNVERIFIED 1 | |

## 1. Blunt verdict
The Linux pin record is the best-evidenced class of doc in this repo: every load-bearing claim points at a committed raw dump under docs/baselines/raw/linux/, the dumps exist (01-17 + probe-work), and the numbers I cross-checked (372 plugins, 8 MATCH lines, 32 support libs) are exactly what the dumps contain. One sloppy summary sentence: "every downloaded .deb SHA-256 cross-checked MATCH" — only the 8 oracle .debs were cross-checked; the 32 support-lib .debs (downloaded, hash-computed for the record) never were, and the dump says so honestly. The doc also records its own environment deltas (43/372 unsatisfied deps before support libs; snapshot.debian.org blocked) — no cherry-picking detected.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "A real VLC 3.0.23 ran headless in this sandbox ... `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)`" | §1 | REAL | dump 04 exists, header "raw dump 04: version string (sandbox run)"; exact string present; VLC binary absent now — cannot re-run, but the committed dump is the run's artifact |
| 2 | "All 372 shipped plugin `.so` files load in the extracted tree (raw dump 03)" | §1 | REAL | dump 03:36-37 `## Final plugin dependency scan (all 372 plugin .so files ldd-checked): PLUGINS=372 MISSING=0`; :4 records the pre-fix 43/372 |
| 3 | "8 packages ... all `=3.0.23-0+deb13u1`" | §5 | REAL | dump 02:31-38 — 8 MATCH lines, one per named package |
| 4 | "every downloaded `.deb` SHA-256 cross-checked **MATCH** against Debian's GPG-signed package index (raw dump 02)" | §1 | INFLATED (S4) | 40 .debs were downloaded (8 oracle + 32 support libs); dump 02 carries 8 MATCH lines; the 32 support libs' SHA256s are COMPUTED-only "for the record" (dump 03:42-43). The summary sentence's "every" is false as written; the underlying evidence is honest about the split. |
| 5 | "gpg ... --verify InRelease → Good signature from Debian Archive Automatic Signing Key (12/bookworm) and (13/trixie) (dump 02, verbatim)" | §6.1 | REAL-UNPROVEN | in-repo verbatim log; GPG re-verification impossible offline (no InRelease, no keyring check run by me) |
| 6 | "release page ... exists [HTTP 200 verified 2026-10-19]" | §2 | UNVERIFIED | external network fact; sandbox has no such access now |
| 7 | "Secondary pin candidate ... Flathub ... stable = 3.0.23 ... verified via Flathub API v2 (dump 11)" | §3 | REAL-UNPROVEN | dump 11 exists in-repo; API response committed |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "every downloaded .deb ... cross-checked MATCH" (§1) | dump 03: "These 32 support-lib .debs are NOT part of the pinned oracle definition ... Their SHA256s (computed) for the record" | The dump is the truth; §1's summary sentence overstates (8/40 cross-checked) |

## 4. Fabrication / inflation indicators
One summary-sentence inflation (ledger #4). Otherwise this doc DOWNgrades itself repeatedly (environment deltas quantified, mirror-drift risk §9, unreachable snapshot.debian.org recorded as [VERIFY on next acquisition]) — the opposite profile of a fabricated baseline.

## 5. Matrix rows
Not a matrix chunk. This record is the reference for matrix baseline cells and the bench-vs-vlc harness's pinned oracle (`PINNED_VLC_VERSION = "3.0.23"` — tools/bench-vs-vlc/run.py:85; consistent).
