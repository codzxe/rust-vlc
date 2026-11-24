# CLAIM AUDIT: docs/audits/2026-10-22-directives-audit.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07; register authored by ED-AUDIT-1..5) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The wave-1 engineering-directives audit register (8 CRITICAL + 19 MAJOR violations, 4 of 5
scopes VIOLATING). This is the audit's own founding document and its claims hold up:
spot-checked violations are real at their file:line and the registered fix cards
ED-FIX-1..5 landed with code comments citing the audit's finding ids ("ED-FIX-1, audit
C1"). The strongest historical claims (host seek = full reopen; DriftCorrection with zero
consumers; phantom pcm_fuzz_survival.rs) all verify as-then-stated and as-since-fixed.
No fabrication. Line numbers have drifted where fixes landed.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "C1: naive i64 matrix-multiply IDCT... ~50M i64 MACs per 1080p frame" | :33 | REAL at date; fixed | crates/codecs/src/video/mjpeg/idct.rs:24-26 "# Factored form (ED-FIX-1, audit C1)... The naive evaluation is two 8×8×8 matrix multiplies" — the fix carries the audit id |
| 2 | "C8: CLI seek = stop→close→file reopen... Engine-native SessionHandle::seek exists... no host calls it" | :40 | REAL at date; fixed | apps/cli/src/host.rs:1107 `.seek(request, engine_target)` + :30 doc "SessionHandle::seek and the seven-step flow of doc 03 §6 runs" |
| 3 | "M12: DriftCorrection... ZERO consumers — drift-compensation loop does not exist" | :57 | REAL at date; wired since | rg DriftCorrection -> consumers now in crates/audio-engine/{lib,pipeline,resample}.rs + apps/cli/src/pipeline_host.rs |
| 4 | "M14: phantom artifact: 2 files cite pcm_fuzz_survival.rs which does not exist" | :59 | REAL at date; fixed | ls crates/codecs/tests/ -> pcm_fuzz_survival.rs exists today (ED-FIX-5 landed it; adpcm_fuzz_survival.rs sibling too) |
| 5 | "M5: negative biHeight (bottom-up DIB, ubiquitous) → whole-file abort" | :50 | REAL at date | ED-FIX-3 carded; corroborated by W1/W13-era AVI audits (negheight0 fixture in baseline corpus malformed set) |
| 6 | "M13: no benchmark exists anywhere; every performance claim is syntactic-green only" | :58 | REAL at date; evolved | bench.rs (apps/cli, 586 LOC) + docs/perf corpus exist now; PERF-VS-VLC methodology audit (W17-W19 context) scrutinizes exactly this lineage |
| 7 | "M19: mjpa/mjpb: probe hints mjpeg, demuxer rejects (stages disagree)" | :64 | REAL at date | stage-disagreement class corroborated by the W13 four-registries finding (probe/demuxer/registry/error-text disagree) |
| 8 | "clean anchors: #[ignore] inventory exactly 4 (no creep)" | :24 | REAL at date; drifted | baseline loc-patterns now records 66 #[ignore] (AGENT-SPEC §6.4) — the "no creep" guarantee did not hold over 19 waves; the register's snapshot is honest, the drift is real |
| 9 | "audited main @ e072caf... read-only, full-file reads" | :3-5 | UNVERIFIED | commit hash needs git (forbidden); method consistent with evidence shown |
| 10 | "fix cards registered in the worklog ED-AUDIT entry" | :5-6 | REAL-UNPROVEN | worklog.md exists; ED-AUDIT entry not re-located in this audit (archive volume) |

## 3. Contradictions
None. The register's violations and the fix cards' landings agree; every fix carries the
finding id it closes.

## 4. Fabrication / inflation indicators
None. The register names its own clean anchors (credit where due), keeps MINOR items
batched, and sequences fixes by file ownership. The #[ignore] count (4) was true then —
today's 66 is creep the register's "no creep" claim could not see.

## 5. Notes for next agents
- This register is the root document for D-1..D-10 violation lineage; cite the current
  idct.rs:24 "ED-FIX-1, audit C1" header when tracing mjpeg perf fixes.
- The 66-#[ignore] drift vs the "exactly 4" anchor is a W17-consistent theatre datapoint:
  the invariant-scanning guarantees decayed over waves.
