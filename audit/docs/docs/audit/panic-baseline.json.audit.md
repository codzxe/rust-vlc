# DATA AUDIT: docs/audit/panic-baseline.json (105 loc)

Committed panic census baseline (JSON): generated 2026-11-17T13:04:24Z,
method "brace-matched #[cfg(test)] excluded; tests/benches/examples
skipped", grand_total=41, 14 crates counted. Sample: _template
unwrap:1, audio-engine unreachable:1, codecs unreachable:1...
Cross-check vs Wave-0 census (audit/runs/baseline/loc-patterns.txt:
unreachable 4126 tree-wide, unwrap 19): the baseline counts non-test
code only — 41 total across {unreachable, panic, unwrap, expect, todo}
in shipped code vs 4,126+ tree-wide including tests: CONSISTENT
(different denominators, both honest).
Findings: S4:1 — nothing consumes this file (rg: no CI gate, no xtask
check reads it); the W17 finding that check_panics.rs is a dead gate
extends to its baseline artifact. Verdict: REAL data, UNWIRED evidence.

| Auditor | W19-A02 (orchestrator completion, W19-S2) |
