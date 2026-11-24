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

## Wave 2 gate (2026-11-24)
- Citations checked: 97, verbatim 87 (89.7%), flagged 10.
- Adjudication: all 10 anchor/format defects (quote text verified present in
  the cited file at or near the range: main.rs:35-39 dropped doc-comment
  line, Cargo.toml:8-12 range shifted ~3, error.rs:210-219/238-249 dedented
  subsets, lib.rs:54 multi-line misanchor, testkit 23-24 whitespace, plus 4
  similar). No misattribution, no invented code. Logged, not re-run.
- Housekeeping: A09 wrote the doc-05 ledger at audit/docs/05-command-layer.md.audit.md
  (path convention violation); orchestrator moved it to
  audit/docs/docs/05-command-layer.md.audit.md. Coverage now 57/57.
- A02's reported accidental catalog-gen write (stray --check file) left NO
  residue: tree purity clean, artifacts byte-identical (verified git status).

## Wave 17 gate (2026-11-25)
- Coverage 100% (244 rows; 12 "trivial" flags are short data/log audits,
  acceptable for data files). A05/A08/A09/A10 hit context/turn limits
  after completing ~75%; recovery batches R1/R2 finished the remaining 35.
- R2 flagged R1's outputs as missing; orchestrator re-verified directly:
  18/18 present (path-prefix confusion in R2's check). Flag dismissed.
- Citation check: mismatches were range-offsets/data-file quirk; no
  fabrications.

## Wave 18 gate (2026-11-25)
- Coverage 100% (32 files). All 10 agents completed. 0 fabrications.
- Inventory chain verified byte-identically regenerable (both parse scripts
  re-run by agents); matrix provenance HOLDS (population honest, statuses
  drift both directions).

## Wave 19 gate (2026-11-25)
- Coverage 100% (674 files): A01/A03/A08 complete; 7 agents hit turn
  limits at ~65%; recovery batches R1-R7 + S1/S2 completed the bulk;
  the final 16 files were audited ORCHESTRATOR-side (W19-S2, logged
  here as a load-adaptation; all data/doc files, sed-verified quotes
  where applicable). 0 fabrications. All 20 matrix row ledgers
  (rows 001-252) exist and are pushed.
