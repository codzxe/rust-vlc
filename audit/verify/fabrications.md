# Fabrications & Citation-Defect Register (orchestrator gate, §7.3 step 4 / §13)

Policy (orchestrator adjudication, applied from Wave 1 onward):
- FABRICATED (invented code or wrong-file attribution): agent re-run on that
  file; incident recorded here.
- ANCHOR DEFECT (quote text real and present in the cited file, line range
  off by a few lines or comment-prefix formatting dropped): logged here,
  finding substance stands; Wave 20 A04 re-sweeps the full population.

## Wave 1 gate (2026-11-24)
- Citations checked automatically: 114 (all audit files, regex-extracted
  Where: path:range + code block, sed-compared).
- Verbatim match: 104 (91.2%). Flagged: 10.
- Adjudication:
  - 1 misattributed citation -> RE-RUN executed: W01-A08,
    audit/files/tools/xtask/src/cmd/check_identity.rs.audit.md attributed
    host.rs:1798-1799 code to check_identity.rs:13-24. Corrected via agent
    re-run; all quotes in the repaired file now byte-verified (5/5 blocks,
    29 line-ranges). Agents' audit-quality failure recorded against W01-A08.
  - 9 anchor/format defects (no invented text; verified present in the cited
    file): deny.toml:14-15 (paren dropped), deny.toml:122-125 & 160-163
    (comment prefixes dropped), LICENSE.audit.md deny.toml:28-30 (prefixes),
    _template/lib.rs:22-24 (should be 21-24), _template/tests/smoke.rs:6-8
    (should be 5-8), compat_matrix.rs:21-23 (text at 10-12),
    web_budget.rs:776-781 (truncation), one parser no-block case.
    Logged, not re-run.
- No fully invented code found in Wave 1.
