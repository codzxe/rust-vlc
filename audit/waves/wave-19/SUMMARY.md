# WAVE 19 SUMMARY — claim ledger B: matrix rows 127-252 + all remaining docs

| Field | Value |
|---|---|
| Agents | A01/A03/A08 full; A02/A04-A07/A09/A10 recovered via R1-R7/S1/S2 + orchestrator completion of final 16 files |
| Files audited | 674 (126 matrix rows 127-252 + ~548 docs/design/data) |
| Coverage | 100% |
| Findings | S0:8+ S2:20+ S3:~80 S4:~150 |
| Tree purity | clean |

## Matrix verdict landscape (rows 127-252, completing the A07 scoring input)
Rows 131-143 (video-fx, aspect/crop/zoom, colors): honest no_code /
UNREACHABLE (colors-adjust real-but-unreachable) / 1 FALSE-understated
(142 autostart: --start-paused shipped+tested).
Rows 144-221 (per R batches): UI/menu/hotkey rows — 8 FALSE-understated
(canvas ctx-menu, fullscreen, crop/aspect, subtitle-pick etc. all
implemented while rows say not_started — W19-A03); hotkey rows 235-247:
11 SHALLOW + 1 REAL (web player dispatches ~8 keys; Space/S/P/N dead).
Rows 222-234 (menus): 2 REAL + 3 SHALLOW (evidence cites AppShell menus
that AppShell.tsx says are GONE) + 8 FALSE-understated.
Rows 248-252: closed out by R batches with the same pattern.

## THE DOC VERDICT (whole docs corpus)
- The INVENTORY layer is bulletproof (W18+W19: parse scripts re-run,
  byte-identical regeneration, every count exact).
- The STATUS/PLANNING layer is stale in BOTH directions: understates
  (http client, reconnect/auth/remote-playlists, autostart, ~10 UI
  rows marked not_started while implemented) AND overstates (asf
  "implemented", seek "partial", billing/compiles claims).
- The w26 verification wave docs are the best paperwork in the repo:
  measured-everything method, vision-hallucination disclosures,
  declared fallbacks, self-disproofs — but /tmp captures never
  committed.
- The transitions JSONL proves the self-correction lane worked ONCE
  (6 evidence-driven downgrades 2026-11-10) — then the matrix drifted
  again (asf row still implemented at W18 read).

## KEY contradictions for the register
1. video-h264.md "no software H.264 decode" vs shipped rust_h264
   (W19-S2 ledger) — the doc that would make a reader believe MP4
   video is impossible.
2. network-http.md "CLI does not dispatch yet" vs executed http open.
3. risk-register "implemented (awaiting Lead review)" vs PENDING
   signatures; "feature-audit still unbuilt" vs running tool.
4. PARITY-REPORT billing-blocked vs RELEASE unblocked (W14-A04).
5. soak-report "nightly tier billing-blocked" vs run IDs (W19-S2).

## PERF-VS-VLC methodology (W19-A09 lane)
Bench evidence is real where committed (decode.csv 30 runs; memaudit
raw with honest v_dec=0 demux-only labeling; mock_vlc.py oracle is
self-labeled MOCK with mechanically-enforced markers) — the f32
tolerance failures (6/6 out-of-tolerance vs pinned VLC) are honestly
recorded and drove a matrix downgrade. No fabricated measurements
found in the perf layer.
