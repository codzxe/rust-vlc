# CLAIM AUDIT: docs/reviews/w24-s-chain-findings.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07; doc authored by W24-R12) |
| Date | 2026-11-24 |
| Claims extracted | 11 |
| REAL 6 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 4 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 2026-11-11 gate-doc chain coherence review (f→k→s) claiming 45/45 citations resolved and
"every recountable number exact". In-tree spot checks confirm its numbers byte-exact against
the committed logs, and its headline P2-1 (gate doc never mentions fixes lane w24/g) was TRUE
then and has since been fixed in the doc itself — a "Post-report status (W24-V)" addendum now
names w24/g 11 times. Its git-level claims (rev-parse, byte-diffs, ls-remote) are UNVERIFIABLE
here (git forbidden). No fabrication found in anything re-checkable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "rg -c 'w24/g\|lane G' docs/gates/phase7-exit.md → 0 hits" | :42 | REAL at date; fixed since | current `rg -c` -> 11 mentions; phase7-exit.md:109-112 "Post-report status (W24-V): fixed on `w24/g` `1c48902`" addendum landed |
| 2 | "§3 OF-2: 'not owned by any wave-24 lane'" quote | :47 | REAL | docs/gates/phase7-exit.md:107 verbatim "not owned by any wave-24 lane (settings-schema crate)" |
| 3 | "64/252 windows/linux/macos, web 0/252... blocked=133 implemented=192 not_started=505 partial=142 unsupported=36" | :172-174 | REAL | docs/gates/logs/phase7-exit-parity.log:33,62,71 verbatim |
| 4 | "all 11 phase7-exit-*.log carry headers" | :183 | REAL | ls docs/gates/logs/ -> 11 phase7-exit-* logs; parity log header present (Date/Cmd lines) |
| 5 | "docs/web/browser-matrix-20261110.{md,json} exist on w24/a... nowhere else" | :96-99 | REAL at date | files now merged into the main tree (docs/web/ both present) — P3-3 caveat resolved by later merge |
| 6 | "hw/mod.rs:51-56... MF retraction... only vaapi/ + videotoolbox/" | :169,217-218 | REAL | crates/codecs/src/video/h264/hw/mod.rs:51-56 retraction verbatim; ls hw/ -> vaapi, videotoolbox only |
| 7 | "ci.yml:527 'SIZE BUDGETS: NONE enforced'" | :141,163 | REAL | .github/workflows/ci.yml:542 "# SIZE BUDGETS: NONE enforced. Doc 09 §2.4's 3 MB / 6 MB baselines are" (line drift) |
| 8 | "severity counts P2=1, P3=3, + 4 informational" | :33 | REAL | doc sections: P2-1; P3-1/2/3; O-1..O-4 |
| 9 | "45/45 citations resolved... 26/26 in-repo paths" | :207-211 | REAL-UNPROVEN | spot-checked subset resolves (phase7-exit.md, logs, ci.yml, KNOWN-LIMITS.md:63, hw/, hevc/hw.rs exist); full sweep not repeated |
| 10 | git claims (rev-parse 14 commits, prepend-only byte-diffs, ls-remote empty) | :17-21,226-228 | UNVERIFIED | git forbidden by rulebook |
| 11 | "§2 #2: clippy r2 21.8s warm, r1 cold 1m14s" etc. battery numbers | :234-243 | UNVERIFIED | logs exist; per-line re-tally not repeated here (W17 baseline corroborates gate FAIL state) |

## 3. Contradictions
None. The review flagged the gate doc as understating closure (conservative direction); the
later W24-V addendum fixed exactly that. Cross-checks with wave24-followup-ledger.md (§1.14)
agree on statuses.

## 4. Fabrication / inflation indicators
None. Findings err conservative; the review distinguishes "coherent but incomplete" from
false, and its P3s are hedges, not over-claims.

## 5. Notes for next agents
- The s-chain P2-1 closure is verifiable WITHOUT git: phase7-exit.md:109-112 addendum +
  11 w24/g mentions — use that as the canonical proof.
- PERF/gate context: parity log's 0/252 verified rows line up with W17 gates-theatre and
  W13 registry-disagreement findings.
