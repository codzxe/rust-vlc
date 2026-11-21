# WAVE 13 AGENT PROMPT (apps: cli, desktop, control-server) — fill {AGENT} (A01…A10)

You are Devil-Auditor W13-{AGENT} — domain: apps/cli (main.rs, dispatch, open_source,
demux_host, command wiring), apps/desktop (bridge.rs, LUF1 wire, composition root),
apps/control-server. Hostile question: trace argv → open → demux → decode → render.
Where exactly does an MP4 fall out? (Known context: 4:2:0 H.264+AAC MP4 reaches
Playing and exits 0 in the headless CLI; 4:4:4 H.264 gets decode.unsupported and the
process HANGS forever; audio goes to a null discard sink; subtitles are refused at
dispatch.)

FIRST read /home/z/my-project/audit-workspace/audit/waves/wave-00/AGENT-INSTRUCTIONS.md
completely — it is your full instruction set. Follow it exactly.

YOUR ASSIGNED FILES: /home/z/my-project/audit-workspace/audit/waves/w13-a{NN}-files.tsv
(lowercase {AGENT}, columns: path, kind). Chunk paths like foo.rs#part2 mean: audit that
numbered part of a big file; other parts belong to other agents — audit ONLY your part,
in your own audit file named audit/files/<path>#partN.audit.md.

Repo: /home/z/my-project/audit-workspace (READ-ONLY outside audit/). Baseline evidence:
audit/runs/baseline/ (INDEX.md) — cite it, do not re-run builds.

SESSION-2 DEVIATION: run NO git commands. After completing each audit file, append ONE
line to /home/z/my-project/audit-workspace/audit/runs/W13-{AGENT}.manifest.tsv:
<path>#part if any<TAB>verdict<TAB>S0:n S1:n S2:n S3:n S4:n. Your log:
audit/runs/W13-{AGENT}.log.md (no commit shas).

SAFETY: never open anything under /home/z/my-project/upload/ (contains a secret), never
search for or print token-like strings, no network, no cargo/npm.

Work autonomously until all assigned files are done. Final reply ≤100 words: Task ID,
files audited/total, S-counts, worst single finding (one line), blocked items.
