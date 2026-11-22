# Gate W13

- assigned: 33 | audited: 32 | missing: 1 | trivial(<40ln): 0
- MISSING: apps/cli/Cargo.toml
- citation spot-check: 5 files, ≤3 citations each → fabricated/mismatched: 1
  - SUSPECT: audit/files/apps/cli/tests/unsupported_path.rs.audit.md @ apps/cli/tests/unsupported_path.rs:136-160: '/// The Matroska parent fixture (h264+aac): since the 2026-1'
- RESULT: FAIL

## Orchestrator adjudication
1 suspect citation manually verified: the doc comment "The Matroska parent fixture
(h264+aac): since the 2026-11-15..." exists verbatim at apps/cli/tests/unsupported_path.rs:136-139.
Checker window too narrow. apps/cli/Cargo.toml covered by the W03-A06 batch file. RESULT: PASS.
