# CLAIM AUDIT: docs/release-notes/skeleton-v0.1.3.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 2 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 (one date inconsistency vs feature-matrix meta) |

## 1. Blunt verdict
The skeleton's headline numbers are EXACT and independently recomputed by me
from docs/feature-matrix.toml with stdlib tomllib: 73 format rows
(container 21 + video-codec 16 + audio-codec 19 + subtitle-format 17); claimed
(implemented/partial/verified) on EVERY desktop platform = 38 = 14 + 7 + 9 + 8
exactly as written; web claimed = 0 exactly ("zero rows claimed for the web
build"). The honesty register matches KNOWN-LIMITS.md §4 verbatim in
substance (null audio backend => "no sound on any OS yet"; GUI video display
"not wired" => honest empty surface); the arm64 "best-effort (the job is
continue-on-error)" matches release.yml:876 `continue-on-error: true`; the
unsigned claim matches release.yml's "honest-unsigned path" comments; the
Start-Menu + Desktop shortcut claim matches gen-wxs.sh:240 (w30.9); the MP4
"~1 GB allocation from a 4-byte patch" hardening matches worklog.md:2160-2161
("4-byte-patch->1GB-alloc DoS is now a typed refusal"). The "two programs
ship in every artifact" claim matches the w30.8 payload policy (stub dropped
from zip/AppImage/.app/tar.gz/deb/rpm; MSI member list owned by gen-wxs
twins). Two claims unverifiable from the tree: "fourth tagged release"
(needs git tags) and the "recomputed 2026-11-15" date (the matrix's own meta
says last_updated = "2026-11-09" — the counts are right, the date claim
conflicts with the matrix meta).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "38 of 73 format rows are claimed ... on desktop: 14 containers, 7 video codecs, 9 audio codecs, 8 subtitle formats" | :37-38 | REAL | tomllib count: format rows 73; windows/linux/macos each {container:14, video-codec:7, audio-codec:9, subtitle-format:8} = 38 claimed |
| 2 | "Zero rows are claimed for the web build" | :39 | REAL | tomllib: web claimed (implemented/partial/verified) = 0 across all 252 rows' format subset |
| 3 | "No sound on any OS yet ... the output backend is not enabled" (KNOWN-LIMITS §4) | :57-59 | REAL | docs/KNOWN-LIMITS.md §4: "Audio output: null backend only ... no sound on any OS yet" |
| 4 | "The app shows no video yet ... honest empty region; the CLI renders frames headless" | :60-62 | REAL | KNOWN-LIMITS.md §4 "GUI video display: not wired ... CLI renders frames headless while the GUI displays none" + PlayerScreen.test.tsx no-fake-canvas pin |
| 5 | "Linux arm64 artifacts are best-effort (the job is continue-on-error)" | :142-143 | REAL | release.yml:876 `continue-on-error: true` (job "linux (aarch64 ...; best-effort)") |
| 6 | "Binaries are unsigned. No Windows code-signing certificate, no Apple Developer ID / notarization" | :135-137 | REAL | release.yml:245-248 "the honest-unsigned path is unchanged"; packaging.toml [signing] secret names exist but no certs |
| 7 | "the installer leaves both a Start Menu and a Desktop shortcut (it does not auto-launch)" | :92-93,133-134 | REAL | gen-wxs.sh:240 "Desktop shortcut (w30.9 — the findability fix: a Start-Menu-only ...)" |
| 8 | "a malformed sample table can no longer request a ~1 GB allocation from a 4-byte patch" (MP4 hardening, w30.3) | :96-98 | REAL | worklog.md:2160-2161 "AUD-01 stco/co64/stsc MAX_SAMPLES caps - the 4-byte-patch->1GB-alloc DoS is now a typed refusal" |
| 9 | "Two programs ship in every artifact: `lumen` ... and `lumen-tauri`" | :109-110 | REAL-UNPROVEN | matches the w30.8 payload policy (release.yml:12-16, verify-linux-artifacts stub ban); per-artifact member lists owned by gen-wxs twins (not fully enumerated here) |
| 10 | "recomputed 2026-11-15 after the H.264 software-decode landing, w27" | :34-35 | UNVERIFIED (date) / REAL (substance) | counts verified exact; feature-matrix.toml:38 meta `last_updated = "2026-11-09"` — the recomputation date conflicts with the matrix's own meta; h264-software rows are `implemented` (consistent with w27 landing) |
| 11 | "This is the fourth tagged release" | :20 | UNVERIFIED | needs git tags (agents cannot run git); only skeleton-v0.1.0 and v0.1.3 exist in docs/release-notes/ |
| 12 | "In v0.1.2, no local file could be opened on Windows at all" | :27-28 | REAL-UNPROVEN | corroborated indirectly: KNOWN-LIMITS.md:8 refresh note cites "w30.1–w30.2 the Windows open path — audit findings S-1/S-2/S-3"; no v0.1.2 skeleton in tree to cross-read |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "recomputed 2026-11-15" (:34) | feature-matrix.toml:38 `last_updated = "2026-11-09"` | The 38/73 numbers are correct either way; the date provenance is inconsistent — either the matrix meta is stale or the skeleton's date is wrong. The substance (counts) verified independently |

## 4. Fabrication / inflation indicators
NONE — the opposite. This is the honest-register genre done right: every
scope claim was recomputable and recomputed exactly; the "what still doesn't
work" list leads with the most user-visible gaps (no sound, no video, no
network) and cites the authority docs. The counts being exactly reproducible
from the matrix (not rounded, not inflated) is the tell of a real measurement.

## 5. Process links
- docs/RELEASE.md and docs/release-checklist.md both exist (:150-151 pointers).
- scripts/release/release-notes.sh (in my file list — audited separately) is
  named as the RC-day commit-list generator; its own audit verifies the claim.
