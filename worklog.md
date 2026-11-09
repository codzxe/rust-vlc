# Worklog — Lumen Media Player (multi-agent program)

Append-only shared log. Format and protocol: AGENTS.md §4.
Task statuses: not_started, in_progress, implemented, verified, blocked.
Prior entries (10,945 lines, phases 0–8, Wave 1–7) are archived in
`docs/worklog/archive-2026-10.md` — see `docs/worklog/README.md` for the
archive convention. New entries append below.

---
Task ID: 7.w1b
Agent: W1-B (hygiene sweep)
Task: Issues #26 + #27 — untrack .env, drop stale agent briefs, archive worklog, sweep one-off scripts.

Work Log:
- Verified .env tracked with single var DATABASE_URL (machine-local path); `git rm --cached .env` (kept on disk, now ignored).
- Added .env.example (documented placeholder `file:./db/custom.db`); fixed .gitignore with `!.env.example` negation after `.env.*`.
- Read all 6 .agent-brief-*.md headers (3 carry explicit "Do NOT commit" text; w5/w6/w7 are transient wave briefs); `git rm` all 6.
- Archived worklog.md (10,945 lines, 1.68 MB, phases 0-8) to docs/worklog/archive-2026-10.md via git mv; wrote docs/worklog/README.md (archive convention); created fresh 9-line root worklog.md ending in `---`.
- Re-grepped repo (workflows, tools/, package.jsons, docs) for all 13 resolve/merge candidates: 9 had zero references anywhere (incl. archived worklog) — deleted merge_resolve.py, resolve_w3_pipeline{,2-6}.py, resolve_worklog.py, resolve_worklog_conflict.py. Kept 4 referenced in worklog narrative: resolve_w1_session_flow.py, resolve_w2_scheduler.py, resolve_worklog_append.py, resolve_worklog_p6.py.
- Verified: `git ls-files | grep -E '\.env$|agent-brief'` → empty; scripts/ = 12 files; `git check-ignore` confirms .env ignored / .env.example not.

Stage Summary:
- Commit aed1c4b on w1/26-27-hygiene (21 files, +10964/-11806); pushed to origin (new branch). Repo root clean: no tracked secrets, no stale briefs; worklog live at 1 entry; scripts/ free of zero-ref one-offs.
---
Task ID: w15-dd
Agent: W15-DD
Task: Issue #2 — H.264/HEVC decision records: software-decoder admission audits, ADR-013 VA-API lane draft, R-01/R-02 + feature-matrix refresh.

Work Log:
- Merged main (30e26f2 → clean); re-verified /tmp/w14dd-a throwaway audits with real cargo tree + extracted-tarball reads (rusty_h264 0.17.2, rust_h264 0.6.1, openh264 0.9.9/sys2 cc+nasm-rs, va 0.0.1, libva 0.1.5, libva-sys 0.1.2). Corrected prior window's unsafe claim: rust_h264 = 3 unsafe impl + 26 unsafe fn + 31 blocks (SIMD/threading files only), not "12 in Send/Sync glue".
- Wrote docs/dependencies/records/h264-software-decoder-audits.md (R1–R10 + candidate table; recommendation rust_h264 0.6.1 [PROPOSAL — Lead sign-off]; conformance-gated admission plan: JVT 199/204 target, FFmpeg differential oracle on D-11 corpus, profile-coverage report; openh264 REJECT both modes; va 0.0.1 = Vapory-blockchain false friend).
- Drafted docs/adr/ADR-013-va-api-hw-decoder.md (proposed): VA-API lane design — 5.F2/5.G2 trait seam, AGENTS §3.3 unsafe-boundary discipline (L5 adapter only), SV-28 answered as E2 OS-service exception, CI conformance; options libva 0.1.5 (dlopen) vs libva-sys + own safe layer vs defer; decision pending Lead.
- Updated docs/14-risk-register.md R-01/R-02 (§2.1 status + §2.2 detail): audits complete 2026-11-09, admission pending Lead; HEVC software-candidate audit recorded as owed.
- Updated docs/feature-matrix.toml: codec-video-h264-software/-hevc-software blocked_reason now cite the record (statuses stay blocked — nothing admitted); codec-video-h264/-hevc + platform-hwdecode-h264/-hevc notes cite ADR-013. TOML validated (252 features).

Stage Summary:
- 4 commits on w14/02-h264-hevc, all pushed: e5e8e11 (audit record), 937a777 (ADR-013), fa5e335 (risk register), 619ee3d (matrix). No decoder crates added to the workspace; admission stays Lead-gated [PROPOSAL].
