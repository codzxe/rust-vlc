# CLAIM AUDIT: docs/reviews/w24-i-fixes-findings.md (228 lines, W24-R7 review-of-fix-lane)
| Field | Value |
|---|---|
| Auditor | W19-A04 — RECOVERED by W19-R2 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 0 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 (2 timeline drifts, disclosed) |

## 1. Blunt verdict
A review-of-a-fix-lane whose own two NEW findings were subsequently landed with its
finding IDs in the fixes: release-checklist.md:51 now carries the `\|\|` GFM escape
credited "W24-R7 finding NEW-1", and both format records carry dated 2026-11-10 addenda
crediting "W24-R7 finding NEW-2". Its central pitfall premise (bash -n checks only the
first file) I re-executed and confirmed. The verification commands that remain runnable
today reproduce (7 release scripts, 150 keymap bindings, soak-nightly 3 jobs, 5×48 min
scenarios). Commit/branch facts are un-checkable without git.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "one-shot `bash -n good1 bad2 good3` → exit 0 with a syntax error in bad2" (the P2-3 pitfall) | :38, :173 | REAL | EXECUTED this run: `bash -n g1.sh bad2.sh g3.sh` -> exit 0 while bad2.sh alone -> exit 2 ("unexpected EOF while looking for matching `\"'"). Pitfall confirmed. |
| 2 | "`ls scripts/release/*.sh \| wc -l` → 7" (P3-5: 7 release scripts + rc-smoke.sh = 8 sh) | :40, :172 | REAL | Executed: 7 (release-checklist.md:51 states the same 7+1=8 arithmetic). |
| 3 | "`rg -c '= { key = ' keymaps/default.toml` → 150" (keymap bindings) | :108, :178 | REAL | Executed (fixed-string): 150. |
| 4 | P3-7 fix: "`wc -c <` replaces `stat -c %s` in both the test and the PASS echo" | :42 | REAL | scripts/rc-smoke.sh:133-134: `[ "$(wc -c < "$WASM" 2>/dev/null \|\| echo 0)" -gt 1048576 ]` + PASS echo `$(wc -c < "$WASM")`; :131 carries the portability comment. |
| 5 | NEW-1 P2: raw `\|\|` in a GFM table cell truncates the rendered command and empties the Status cell | :48-71 | REAL (at date, FIXED) | docs/release-checklist.md:51 (current): the loop is now `` `for f in …; do bash -n "$f" \|\| exit 1; done` `` with inline note "the `\|\|` is GFM-escaped … — W24-R7 finding NEW-1". Two-character fix landed as recommended. |
| 6 | NEW-2 P3: video-vp8-vp9.md + video-prores-dnxhd.md status front-matter stale vs post-issue-#7 matrix | :73-95 | REAL (at date, FIXED) | video-vp8-vp9.md:167 "## Addendum (2026-11-10) — status front-matter is stale for VP9 … (W24-R7 finding NEW-2)"; video-prores-dnxhd.md:112 same shape for DNxHD — dated addenda landed exactly as suggested. |
| 7 | P2-1 fix: claimed video-codec rows = "av1, h264, hevc, mjpeg, prores-dnxhd, vp9"; h264-software blocked ×4 at date | :36 | REAL (at date) | tomllib parse today: claimed (implemented/partial) desktop video rows = av1, h264, hevc, mjpeg, prores-dnxhd, vp9 — the same 6-row claimed set. Timeline drift: codec-video-h264-software is NOW implemented×3 (matrix row: rust_h264 Lead-admitted 2026-11-15 — postdates this review's 2026-11-10/12 window); vp9/linux now partial. |
| 8 | Target 2: soak-nightly.yml jobs = soak, soak-scenarios, soak-lsan; additive-only vs main | :120-121, :209 | REAL (jobs) / UNVERIFIED (diff) | `.github/workflows/soak-nightly.yml`:100/:164/:234 — the three job ids exist exactly. The additive-only diff claim needs git (not available). |
| 9 | "scenarios-nightly.toml … 5 scenarios × duration_min 48 = 240 min = 4 h" | :149-151 | REAL | Executed: 5 `duration_min` entries, all 48.0 (=240 min); job name "4 h × 5 scenarios" matches. |
| 10 | "[[setting]] counts: unanchored 33 / anchored 32 (schema line 13 is a comment)" | :39, :176-177 | REAL (at date; drifted) | Re-run today: 129 unanchored / 121 anchored — schemas/settings.toml grew ~90 rows in the w26 settings work after this review (the w26 settings-entry-count 45→121 drift). At-date values consistent with the then-schema. |
| 11 | "7 commits on top of origin/w24/e … f00385c, 88c680d, … ; task card said 8" + git ls-remote/ls-tree/merge-base results | :5-13, :43, :192-193 | UNVERIFIED | No-git rule; hashes/ancestry not re-derivable in this sandbox. The honest off-by-one disclosure against its own task card is a credibility marker, not a liability. |
| 12 | rc-smoke fixture claims ("check 4 IS headless-decode and its default fixture IS avi_mjpeg_pcm.avi (exists)") | :37 | REAL | Fixture verified to exist this run (tests/fixtures/generated/valid/avi_mjpeg_pcm.avi, 425,516 B — same file my baseline.md ledger verified); check-4 mapping corroborated by the Wave-0 rc-smoke baseline (headless-decode AVI/MJPEG/PCM PASS). |

## 3. Contradictions
None internal. Two timeline drifts (claims 7, 10) — both are the tree MOVING AFTER the
review (h264-software admission 2026-11-15; settings schema growth), not review errors;
the review itself dated every claim.

## 4. Fabrication / inflation indicators
None. The review DEMOTES its own target's task card (says 8 commits; actual 7 — recorded
as "discrepancy is in the task description, not a lane fault"), re-derives every fix
independently, and its two new findings were accepted and landed with its ID cited.
