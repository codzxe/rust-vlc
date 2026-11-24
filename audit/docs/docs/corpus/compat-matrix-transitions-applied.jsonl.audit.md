# DATA AUDIT: compat-matrix-transitions-applied.jsonl (6 records)

THE exception to the frozen-matrix story: 6 applied transitions
(2026-11-10) from a real compat-matrix run, each with schema, row,
platform, from->to, reason, next_step, evidence, applied_by:
1. container-asf-wmv linux implemented->partial (open path refuses
   asf probe - matches W4 finding: demuxer real, unwired)
2. codec-audio-aac-lc implemented->partial (142 concealed packets on
   VALID corpus files)
3. codec-audio-aac-lc implemented->partial (1 corpus track REFUSED
   "no decoder in this build" vs 4 decoded)
4. codec-audio-alac implemented->partial (1 refused vs 1 decoded)
5. codec-video-vp9 implemented->partial (1 refused vs 2 decoded)
6. codec-audio-pcm implemented->partial (ALL 6 f32 rows FAIL tolerance
   vs pinned VLC: worst rms 1.22e-1/peak 4.94e-1; 29 integer rows
   bit-exact)
Verdict: REAL — the one place where the status layer was updated by
executed evidence with a full audit trail. Findings: S3:1 — these
downgrades contradict the rows' current statuses in at least 2 cases
(asf row still says implemented per W18-A07's row 007 audit "impl x3")
— i.e., the transitions were applied then partially reverted or the
matrix was re-edited afterwards; drift direction inconsistent.
Cross-ref: W18-A06's corpus-plan audit; W03's "refused" tracks match
transition 3-5's refusal strings (the lying bench registry again).

| Auditor | W19-A07 (orchestrator completion, W19-S2) |
