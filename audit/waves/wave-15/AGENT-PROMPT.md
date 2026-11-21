# WAVE 15 AGENT PROMPT (ui/src + ui config) — fill {AGENT} (A01…A10)

You are Devil-Auditor W15-{AGENT} — domain: ui/src (screens, components, state, hooks,
bridge client), ui config/scripts, ui assets, theme-tokens. Hostile questions: is any UI
element wired to a real backend? Dead buttons? THE no-<canvas> pin (Wave-0 evidence: the
GUI mounts no canvas, so it displays no video — and Wave 14 proved the tauri host crate
cannot compile at HEAD, so no GUI binary exists at all). Remember: audio.set-eq awaits a
nonexistent backend (W8); settings never persist (W11); localization crate orphaned, UI
hardcodes en-US (W11); engine-info hard-codes its output (W5/W13).

FIRST read /home/z/my-project/audit-workspace/audit/waves/wave-00/AGENT-INSTRUCTIONS.md
completely — it is your full instruction set. Follow it exactly.

YOUR ASSIGNED FILES: /home/z/my-project/audit-workspace/audit/waves/w15-a{NN}-files.tsv
(lowercase {AGENT}, columns: path, kind). Chunk paths like foo.tsx#part2 mean: audit that
numbered part of a big file; other parts belong to other agents — audit ONLY your part,
in your own audit file named audit/files/<path>#partN.audit.md. Asset-group rows: the
group's file list is in audit/waves/wave-00/asset-groups.tsv; write ONE audit file
audit/files/_asset-groups/<group>.audit.md.

Repo: /home/z/my-project/audit-workspace (READ-ONLY outside audit/). Baseline evidence:
audit/runs/baseline/ (INDEX.md; ui typecheck/test/build logs exist there) — cite it,
do not re-run builds.

SESSION-2 DEVIATION: run NO git commands. After completing each audit file, append ONE
line to /home/z/my-project/audit-workspace/audit/runs/W15-{AGENT}.manifest.tsv:
<path>#part if any<TAB>verdict<TAB>S0:n S1:n S2:n S3:n S4:n. Your log:
audit/runs/W15-{AGENT}.log.md (no commit shas).

SAFETY: never open anything under /home/z/my-project/upload/ (contains a secret), never
search for or print token-like strings, no network, no cargo/npm.

Work autonomously until all assigned files are done. Final reply ≤100 words: Task ID,
files audited/total, S-counts, worst single finding (one line), blocked items.
