# CLAIM AUDIT: docs/plans/phase8-cards.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A06) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The Phase 8 card-to-issue breakdown (issue #23 close package). Its four "deltas since the
wave program" all verify in-tree (soak-nightly.yml, the VLC diff report, the fuzz target
set, the security review), and its "nothing invented" scope-fidelity claim spot-checks
against doc 13 §11 (8.A1's card text matches). The issues #52-#59 creation claim is
GitHub-side (UNVERIFIABLE). The P8-6 deliverables it plans (parity-statement.md,
known-limits.md) are still absent — the plan is open work, not completed work.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Soak nightly EXISTS — .github/workflows/soak-nightly.yml is live" | :14 | REAL | .github/workflows/soak-nightly.yml exists |
| 2 | "docs/corpus/vlc-diff-report-20261023.md: 29/35 corpus files bit-exact... 6/35 pcm_f32" | :15 | REAL | the report file exists; the 29/35+6/35 split is its recorded content (cross-cited by W19-R runs) |
| 3 | "fuzz/fuzz_targets/ now carries codec targets (aac, adpcm, alac, av1, flac, h264, hevc, mjpeg, mp2, mp3...)" | :16 | REAL | ls fuzz/fuzz_targets/ -> fuzz_codec_{aac,adpcm,alac,av1,flac,h264,...}.rs |
| 4 | "6.H1... produced 1 Medium (same-host https→http scheme downgrade) + 3 Low... docs/investigations/network-security-review.md" | :17 | REAL | docs/investigations/network-security-review.md exists |
| 5 | "all card scope/AC text below is from that section, nothing invented" | :6 | REAL-UNPROVEN | spot-check: doc 13 §11:795 8.A1 card text matches P8-1's scope/AC phrasing; full 16-card sweep not repeated |
| 6 | "issues #52–#59 created 2026-11-09, one per item" | :5,8 | UNVERIFIED | GitHub-side state; no network/git available |
| 7 | "External blockers: Apple account R-35, Windows cert, 3-OS runners, CI billing (owner=user)" | :19-27 | REAL | corroborated by KNOWN-LIMITS R-16/:86, wave24-ledger §5 (same four blockers), risk register |
| 8 | "P8-6: parity tool lockdown... no self-verification path exists in code — AC: attempting a manual status flip fails" | :72 | UNVERIFIED (open plan) | tools/parity-report/** + docs/parity-statement.md do not exist yet — the item is unlanded; W17 baseline (parity-report 0/252 verified) and W18 matrix drifts are the standing context this plan would fix |

## 3. Contradictions
None. The plan's blocker table matches the wave24 followup ledger §5 and KNOWN-LIMITS
records — three independent docs, one blocker story.

## 4. Fabrication / inflation indicators
None. The doc converts a source doc into issues with an explicit fidelity claim that
spot-checks; deltas are labeled as already-merged facts with in-tree verification.

## 5. Notes for next agents
- P8-4/#55 (perf-vs-VLC) is the PERF-VS-VLC methodology audit's subject — the honest
  "including pure-Rust-slower cases" AC is the standard the methodology audit applies.
- P8-6's anti-self-verification AC remains the key unlanded control: today's
  feature-matrix.toml is hand-editable (W18 drift evidence) — cite this plan when
  arguing for the lockdown.
