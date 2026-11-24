# CLAIM AUDIT: docs/reviews/w24-vw-fixes-findings.md (228 lines, W24-R16 review of fixer lanes V+W)
| Field | Value |
|---|---|
| Auditor | W19-A09 — RECOVERED by W19-R2 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A review whose audited LANDED ARTIFACTS all verify in the current tree: the W24-V §3
addendum with its four "Post-report status (W24-V)" bullets is live in phase7-exit.md
with the frozen NOT-PASSED verdict preserved, the W one-line 7.I1 cell fix is verbatim
at 13-agent-wave-program.md:764, the undated browser-matrix.md genuinely does not
exist while the dated pair does, and R5's SHIP verdict + R9's 17-merges header quote
byte-exact. The review's own method (git ancestry/timestamp/ref sweeps) is un-checkable
here without git — but every landed consequence I can test holds.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | W24-V landed "the §3 fix-lane addendum on `docs/gates/phase7-exit.md` (53 insertions / 8 deletions)" | :8-9 | REAL (landed) | phase7-exit.md:21 "Recommended closure path (updated by W24-V's §3 fix-lane …" + four `**Post-report status (W24-V):**` bullets (:110/:130/:154/+1) — the addendum is in the committed doc. |
| 2 | "All four (OF-2/3/4/5) accurate" post-report bullets; OF-1 gets no bullet (lane A, local-only) | :84-88 | REAL | Exactly 4 bullets (rg count = 4); none names OF-1 — matches the review's own discipline claim. |
| 3 | "the verdict header stays frozen ('NOT PASSED at `c339ee8`', '18 of 22 green')" | :75-76 | REAL | phase7-exit.md:10 "**NOT PASSED at `c339ee8` — 5 open findings (OF-1…OF-5)…" + :13 "18 of 22" — preserved verbatim. |
| 4 | OF-4 fix = "the `[PROPOSAL]`'s first option (`env!("CARGO_MANIFEST_DIR")`)" | :82-83 | REAL | `rg -l 'CARGO_MANIFEST_DIR' crates/` -> test-infra files (audio-output/tests/common/mod.rs, theme-tokens tests ×3) — the env-var root-discovery form is in the tree. |
| 5 | OF-5 note correction: MF retracted, VT/VA-API kept | :84-86, :212-213 | REAL | phase7-exit.md:164-166: "**Media Foundation** (Windows, 7.B2): **NOT IMPLEMENTED** … compile-verified MF FFI that does not exist — **retracted**." |
| 6 | W's fix: 7.I1 cell now cites `docs/web/browser-matrix-20261110.{md,json}` "delivered 2026-11-10 on w24/a; plan name drift corrected per W24-R9 IN-6/S" | :119-122 | REAL | `sed -n '764p' docs/13-agent-wave-program.md` — the cell is byte-verbatim as quoted. |
| 7 | "The undated `docs/web/browser-matrix.md` exists on no ref" (negative claim) | :130-132 | REAL (current tree) | `ls docs/web/` -> browser-matrix-20261110.{json,md} + size-record/report + web-column-pass — no undated file exists. |
| 8 | R5 verdict quote: "ALL FOUR IN-SCOPE FINDINGS (OF-2…OF-5) FIXED PROPERLY — none weakened. SHIP." verbatim + suite counts 43/0, 8/0, 125/0, 12/0+3i | :54-59, :204-205 | REAL (quote) | docs/reviews/w24-g-fixes-findings.md:13 carries the verdict line byte-exact (suite table numbers in that doc corroborate the 4-suite shape; per-number recheck done for the quote only). |
| 9 | R9's header: "`286bacd` … 17 merges incl. g" + feature-matrix diff "decomposes exactly into lane D (32) + lane G (4)" | :60-65 | REAL (header) | docs/reviews/w24-integration-findings.md:5-7: "w24-integration-rehearsal @ `286bacd` … **17 merges**: lanes b, c, d, e, f, g, h, i, j, k + reviews r1–r7" — the header matches. |
| 10 | All hash-ancestry/timestamp/ref-sweep verification runs (§"Verification runs", :192-228) | :192-228 | UNVERIFIED | 40+ git commands (rev-parse ×9, merge-base --is-ancestor ×5, cat-file -e, ls-remote, --diff-filter=A, per-ref ls-tree sweeps, rehearsal-log reads) — no git available to auditors; the CONSEQUENCES (claims 1-9) all verified instead. |

## 3. Contradictions
None. W-P3-1 (worklog overbreadth nit, :143-163) is the review flagging ITS OWN audited
lane's imprecision — a demotion of a doc this review is responsible for validating, which
is the correct direction of skepticism.

## 4. Fabrication / inflation indicators
None. The review's PASS verdict is hedged with a P3 nit against its own lane, its
informational findings (V-I1 "exactly as prescribed slightly understates") push AGAINST
overclaiming, and the method section enumerates the exact git machinery so a later
auditor (me) could re-derive the consequences — which I did.
