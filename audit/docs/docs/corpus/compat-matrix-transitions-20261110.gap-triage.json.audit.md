# DATA AUDIT: docs/corpus/compat-matrix-transitions-20261110.gap-triage.json (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 1 transition: codec-audio-pcm/linux implemented -> partial (f32 tolerance gap) |
| Cross-verified against | applied.jsonl, feature-matrix.toml, compat-matrix-20261110.md, vlc-f32-tolerance-20261109.json |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Every load-bearing claim cross-verifies:
1. Transition applied: docs/corpus/compat-matrix-transitions-applied.jsonl's last entry
   cites THIS file as outbox, applied_at 2026-11-10T20:08:26Z, `applied_by: cargo xtask
   compat-matrix apply-transitions`, with reason/next_step/evidence text matching this file
   byte-near-identically.
2. Matrix reflects it: tomllib on docs/feature-matrix.toml -> codec-audio-pcm status =
   { windows: implemented, linux: partial, macos: implemented, web: not_started } ✓.
3. Evidence numbers: compat-matrix-20261110.md:175 records the worst row exactly —
   `0305-avi-av.avi` ... "f32 tolerance **FAIL** (rms 1.22e-1 > 1e-4 or peak 4.94e-1 >
   1e-3, 221164 samples)" ✓ matches this file's "worst row 0305-avi-av.avi measures rms
   1.22e-1 / peak 4.94e-1".
4. Thresholds: vlc-f32-tolerance-20261109.json policy "PASS iff per-row RMS deviation <=
   1e-4 (-80 dB) AND peak deviation <= 1e-3 (-60 dB)" with per-row `rms_max 0.0001 /
   peak_max 0.001` and FAIL verdicts ✓.
5. Honesty markers: the `origin` field openly declares the file is NOT runner-emitted but
   hand-authored by the triage round through the same rule-enforcing writer — a rare,
   explicit provenance disclosure.
S4 (only defect): the file's own `"generated": "2026-11-10T20:15:00Z"` is 7 minutes AFTER
the recorded apply time (20:08:26Z) that cites this file — the stamp describes a
post-apply regeneration, not the applied version; a hand-written round minute (:15:00)
that a runner would not have emitted. Quotes byte-verbatim from Read output.
