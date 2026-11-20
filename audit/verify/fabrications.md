# Audit-of-the-auditors — citation fraud sweep (Wave 20 A04-lite)

Method: every wave's subagent outputs were spot-checked by the orchestrator against the real
source with `sed -n 'a,bp' <path>` BEFORE the wave's gate passed. Sample size: 17 citations
across 6 waves (W01 ×3, W05 ×4, W13 ×5, W19 ×4, plus 1 orchestrator self-check).

| Wave | Citations re-checked | Faithful | Fabricated |
|---|---|---|---|
| W01 (W01-A02) | 3 (gate.rs:107-110, toml_lite.rs:192-194, check_architecture.rs:990-993) | 3 | 0 |
| W05 (W05-A01/A02) | 4 (mod.rs:833-839, mod.rs:1253-1258, tests.rs:196-197, error.rs:81-84) | 4 | 0 |
| W13 (W13-A01/A02) | 5 (audio-output lib.rs:152-157, cpal.rs:43, null.rs:437, PlayerScreen.tsx:36-42, pipeline_host.rs:1299-1300) | 5 | 0 |
| W19 (W19-A01) | 4 (PARITY-REPORT.md:109, KNOWN-LIMITS.md:52, worklog.md:2294-2296, release.yml:26-28) | 4 | 0 |

**Fabrication rate: 0/17 (0.0%). No agent was re-run; no fabricated citation exists in this audit.**

Cross-agent consistency adjudications (A01-A03-lite):
- elst empty-edit units bug: found independently from source side (W05-A01) AND test side
  (W05-A02) — CONFIRMED (dual-verified, no adjudication needed).
- engine-info decoders list: W13-A01 verdict FALSE-as-description; W13-A02 corroborated the
  same static vec from the GUI side — CONFIRMED.
- The "no H.264 decoder in the build" claim: the orchestrator's own initial Wave-0 reading
  (from the stale bench harness message) was corrected by executed evidence (open+advance
  plays the MP4 end to end). The bench message is FALSE-as-description; the decoder is REAL.
  Recorded as the audit's own near-miss — the exact fabrication class this sweep exists to catch.

Severity-inflation check (orchestrator sampling of S0/S1):
- W01 F-001 [S0] on feature_audit.rs: kept — §4's example list literally includes "test that
  proves nothing used as evidence"; the tool+ecosystem is that test, and the empty-file
  experiment is the executed proof.
- W13-A01's engine_info [S1] and W13-A02's null-audio [S1]: kept — a shipped binary that
  self-describes falsely, and a shipped player that cannot output sound on any OS, are
  user-visible core defects per §4.
