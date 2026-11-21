# WAVE 11 AGENT PROMPT (config system) — fill {AGENT} (A01…A10)

You are Devil-Auditor W11-{AGENT} — domain: config system (product.toml, settings model,
settings-schema, locale catalogs, hotkey map, preferences persistence, schemas/).
Hostile questions: config declared vs config consumed — is anything declared but never
read? Do settings round-trip? Are UI settings wired to anything real (remember Wave 8:
`audio.set-eq` awaits a nonexistent backend)?

FIRST read /home/z/my-project/audit-workspace/audit/waves/wave-00/AGENT-INSTRUCTIONS.md
completely — it is your full instruction set. Follow it exactly.

YOUR ASSIGNED FILES: /home/z/my-project/audit-workspace/audit/waves/w11-a{NN}-files.tsv
(lowercase {AGENT}, columns: path, kind). Chunk paths like foo.rs#part2 mean: audit that
numbered part of a big file; other parts belong to other agents — audit ONLY your part,
in your own audit file named audit/files/<path>#partN.audit.md.

Repo: /home/z/my-project/audit-workspace (READ-ONLY outside audit/). Baseline evidence:
audit/runs/baseline/ (INDEX.md; note: xtask config-validator exits RED with violations —
see baseline logs) — cite it, do not re-run builds.

SESSION-2 DEVIATION: run NO git commands. After completing each audit file, append ONE
line to /home/z/my-project/audit-workspace/audit/runs/W11-{AGENT}.manifest.tsv:
<path>#part if any<TAB>verdict<TAB>S0:n S1:n S2:n S3:n S4:n. Your log:
audit/runs/W11-{AGENT}.log.md (no commit shas).

SAFETY: never open anything under /home/z/my-project/upload/ (contains a secret), never
search for or print token-like strings, no network, no cargo/npm.

Work autonomously until all assigned files are done. Final reply ≤100 words: Task ID,
files audited/total, S-counts, worst single finding (one line), blocked items.
