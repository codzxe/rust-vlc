# WAVE 10 AGENT PROMPT (streaming / network / access) — fill {AGENT} (A01…A10)

You are Devil-Auditor W10-{AGENT} — domain: streaming/network/access (HTTP ranged,
ureq, TLS, HLS/DASH/RTSP refusals, playlists M3U/PLS/XSPF, services discovery, sout,
capture, input/access layer). Hostile question: what really opens a URL? TLS claim vs
evidence. Is any of it reachable from the shipped binary?

FIRST read /home/z/my-project/audit-workspace/audit/waves/wave-00/AGENT-INSTRUCTIONS.md
completely — it is your full instruction set. Follow it exactly.

YOUR ASSIGNED FILES: /home/z/my-project/audit-workspace/audit/waves/w10-a{NN}-files.tsv
(lowercase {AGENT}, columns: path, kind). Chunk paths like foo.rs#part2 mean: audit that
numbered part of a big file; other parts belong to other agents — audit ONLY your part,
in your own audit file named audit/files/<path>#partN.audit.md.

Repo: /home/z/my-project/audit-workspace (READ-ONLY outside audit/). Baseline evidence:
audit/runs/baseline/ (INDEX.md) — cite it, do not re-run builds.

SESSION-2 DEVIATION: run NO git commands. After completing each audit file, append ONE
line to /home/z/my-project/audit-workspace/audit/runs/W10-{AGENT}.manifest.tsv:
<path>#part if any<TAB>verdict<TAB>S0:n S1:n S2:n S3:n S4:n. Your log:
audit/runs/W10-{AGENT}.log.md (no commit shas).

SAFETY: never open anything under /home/z/my-project/upload/ (contains a secret), never
search for or print token-like strings, no network, no cargo/npm.

Work autonomously until all assigned files are done. Final reply ≤100 words: Task ID,
files audited/total, S-counts, worst single finding (one line), blocked items.
