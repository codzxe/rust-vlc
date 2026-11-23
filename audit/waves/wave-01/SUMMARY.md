# WAVE 01 SUMMARY — workspace root, xtask tooling, gates

| Field | Value |
|---|---|
| Agents | A01–A10, all completed (1 gate re-run: A08 citation repair) |
| Files audited | 47 files (66 chunk rows), 100% coverage, 0 trivial, 0 missing |
| Findings | S0:0 S1:0 S2:12 S3:61 S4:108 (181 total) |
| Citation gate | 114 citations auto-checked, 104 verbatim, 1 misattribution (re-run + repaired), 9 anchor defects logged (audit/verify/fabrications.md) |
| Tree purity | clean — nothing outside audit/ touched |

## Worst findings (with citations in the audit files)
1. [S2, executed] check_architecture.rs: dependencies in table form
   `[dependencies.foo]` are INVISIBLE to all 5 checks — the arch gate exits 0
   PASS on graphs it cannot see (W01-A02). The baseline "1 of 5 failed" is a
   real drift: lumen-cli→streaming edge undeclared in boundaries.toml.
2. [S2, executed] gate.rs row 9 runs `cargo tauri build` in non-tauri
   apps/desktop — the gate can never pass; row 10 wasm permanently SKIP
   (stale constant); gate-CI parity claim FALSE (W01-A06).
3. [S2, executed] compat_matrix.rs: noteless-row note insertion always
   self-rejects (exit 2) — 12/252 rows unwritable through "the ONLY status
   writer"; escape_toml writes spec-invalid TOML with exit 0 "APPLIED"
   (W01-A03).
4. [S2] product.toml:97-101 references 4 nonexistent brand assets — the root
   cause of config-validator's baseline FAILURE (W01-A05/A08).
5. [S2] sbom stale by exactly one edge (device-discovery→plugin-api); lock
   changed 2026-11-17, SBOM never regenerated (W01-A09, reproduced live).
6. [S2] check_panics.rs: real scanner, DEAD GATE — unwired anywhere, and it
   FAILS its own tree (2 demux `unreachable!` sites) while CI's panic
   ratchet freezes those same 2 sites as acceptable — contradictory
   verdicts (W01-A10).
7. [S2] config-validator execution-proven enum hole (`subtitle_mode = 3`
   passes validation) + accent-color check missing (W01-A10).
8. [KEY VERDICT] feature_audit.rs R4 = non-empty evidence string +
   Path::exists() ONLY — an EMPTY FILE passes as evidence, proven by
   execution (W01-A06). The master prompt's suspicion is confirmed at code
   level. Undisclosed missing rules: R1-prefix, R2-P0, R5-ref-existence.
9. [KEY VERDICT] parity_report.rs: the 0/252 is honest AND structurally
   frozen — "verified" is a string-match, evidence is never read, no row
   claims it, and the tooling to earn it is admitted unassigned debt
   (W01-A09).
10. [S3 cluster] toml_lite.rs silently mis-parses `"""x"""` and accepts
    duplicate keys/tables first-wins (W01-A07); .env.example documents a
    dead Prisma DATABASE_URL while 25 real LUMEN_* vars are undocumented
    (W01-A04); tokens.rs `run(_args)` ignores argv — doc 08's documented
    `--check` flag does not exist and typing the documented command MUTATES
    committed outputs (W01-A08).

## Blocked / UNVERIFIED carried forward
- cargo-deny never executed (tool missing, load budget) — deny.toml audited
  statically; 8 slash-form + 14 uncached crate licenses unproven.
- desktop-tauri subtree license/build posture (environment limit).
- Cross-ref for Wave 13: `LUMEN_DEV_INSECURE_TLS=1` documented in docs/11
  SEC-07 does not exist in code (tls_config reads no env — client.rs:91-96)
  — W01-A04 discovery.
- Cross-ref for Waves 4/5: the 2 demux `unreachable!` panic sites that
  check-panics flags.
- Cross-ref for Wave 17: full_gate.sh never runs config-validator; 5 xtask
  subcommands (parity-report, check-identity, sbom, compat-matrix,
  web-budget) have zero test files (W01-A05).
