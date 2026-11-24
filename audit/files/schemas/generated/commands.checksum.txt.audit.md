# AUDIT: schemas/generated/commands.checksum.txt
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 7 / 3 digests + 1 count line (generated artifact) |
| File verdict | REAL — 2 of 3 digests independently recomputed by me and matched byte-for-byte |
| Reachable from shipped entry point? | The FILE is a committed build/CI artifact, not runtime code. Its generator chain: `cargo xtask codegen` (tools/xtask/src/cmd/codegen.rs:58) → `cargo run -p commands --bin catalog-gen` → crates/commands/src/bin/catalog_gen.rs → commands::catalog::codegen::generate_outputs/check_against. Drift gate wired in CI: .github/workflows/ci.yml:280 `cargo run -p xtask --locked -- codegen --check`; also enforced by crates/commands/tests/catalog_drift.rs (baseline 2/2 pass). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 fnv1a64 re-implementation over the two committed outputs (schema: `ddd1caf7364833c9` MATCH; ts: `3ad09e5411d5a351` MATCH); rg counts of `CommandSpec {`/`EventSpec {` rows in spec.rs (65/26 after removing the struct definitions at :133/:151 — matches `counts:`); baseline cargo-test-workspace.txt commands block (57 lib incl. `checksum_text_carries_the_counts_and_digests`, `mutated_table_entry_changes_the_checksum`; catalog_drift 2/2 pass). |

## 1. One-paragraph verdict
A genuine, self-consistent generated checksum artifact, not decoration: the two output digests (schema JSON, TypeScript) are FNV-1a-64 over the exact committed bytes — I re-implemented fnv1a64 in python and both reproduce to the last hex digit, which simultaneously proves the hash implementation is standard and that the committed outputs have zero drift from the generation that wrote this file. The third digest (catalog-fnv1a64) folds a canonical text of the Rust catalog DATA (65 command rows + 26 event rows — row counts verified structurally), so id/class/target/args/result changes are caught even when formatting is identical. Drift enforcement exists twice: the crate's own catalog_drift test (baseline pass) and CI's `codegen --check`. The gaps are wiring-adjacent, not correctness: the LOCAL full_gate.sh has no codegen step (CI-only), and the docs/gates/phase1-exit.md numbers (61 commands / 23 events) are stale against the current 65/26.

## 2. What actually works (verified)
- python3 (my own FNV-1a-64, offset basis 0xcbf29ce484222325, prime 0x100000001b3):
  - over schemas/commands.schema.json (139694 bytes) → `ddd1caf7364833c9` == committed `schema-fnv1a64`.
  - over ui/src/generated/commands.ts (14023 bytes) → `3ad09e5411d5a351` == committed `typescript-fnv1a64`.
- Row counts: `rg -c 'CommandSpec \{'` = 66 minus the struct definition (spec.rs:133) = 65 rows; `EventSpec \{` = 27 minus spec.rs:151 = 26 rows — matches `counts: 65 commands / 26 events` (counts come from `catalog_data().len()`/`event_catalog_data().len()`, codegen.rs:138-139).
- Baseline: commands package — 57 lib tests pass incl. `catalog::spec::tests::mutated_table_entry_changes_the_checksum` (mutation sensitivity proven) and `catalog::codegen::tests::checksum_text_carries_the_counts_and_digests`; tests/catalog_drift.rs 2/2 pass (committed == regenerated).

## 3. Findings
F-001 [S4]
Where: scripts/full_gate.sh (absence) vs .github/workflows/ci.yml:280
Code (ci.yml:280, the wired check):
```
        run: cargo run -p xtask --locked -- codegen --check
```
What is wrong: the codegen drift check is CI-only. The local full gate (scripts/full_gate.sh; baseline full_gate.txt sections: fmt, test, arch, fixtures, fuzz-registry, lint-literals, check-identity, sbom, web-budget, gate.sh, packaging) contains NO codegen/checksum step — rg 'codegen|checksum' over full_gate.sh returns nothing. A contributor running only the local gate can commit catalog drift and learn about it only after pushing. Blast radius: small (CI catches it; the crate test lane also fails via catalog_drift).
Confidence: HIGH — Verification: rg over scripts/ + baseline full_gate.txt.

F-002 [S4]
Where: docs/gates/phase1-exit.md vs this file's `counts:` line
What is wrong: phase1-exit.md records the phase-1 exit state as "61 commands / 23 events" (two places, incl. the drift-gate evidence row) while the committed catalog is 65/26. The doc is a historical gate record, so it is not false-at-the-time, but nothing in it flags the counts as a snapshot — a reader cross-checking "the catalog" against the doc gets stale numbers. Doc-hygiene only; the checksum file itself is current.
Confidence: HIGH — Verification: rg in docs/gates/phase1-exit.md + row counts above.

## 4. Per-function table
n.a. (generated data). Artifact anatomy: header comment (@generated, never hand-edit, regen command); `catalog-fnv1a64: debb80f9922a192a` (canonical data text hash — spec.rs:865-890); `schema-fnv1a64` / `typescript-fnv1a64` (output bytes hashes — codegen.rs:137-152); `counts: 65 commands / 26 events`.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| catalog_drift.rs (2 tests, baseline pass) | YES | Regenerates all three outputs and compares to the committed files — any catalog/schema/TS drift fails the test lane. |
| spec::tests::mutated_table_entry_changes_the_checksum | YES | Mutates a table copy, expects a different digest — proves the checksum is input-sensitive, not a constant. |
| codegen::tests::checksum_text_carries_the_counts_and_digests | YES | Pins the file format (header + 3 digest lines + counts). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "@generated, never hand-edit. Regenerate with `cargo xtask codegen`" | checksum.txt:1-3 (self) | REAL | Generator chain verified; catalog_drift + CI --check enforce it. |
| "fnv1a64 over the canonical catalog text and over the generated output bytes" | checksum.txt:2-3 (self) | REAL | Independently recomputed both output digests (exact match); canonical-text digest cross-checked via row counts + mutation test. |
| "61 commands / 23 events" | docs/gates/phase1-exit.md | STALE | Current: 65/26 (verified). |

## 7. Dead / duplicated / unreachable code
None in the file. Note: `catalog-fnv1a64` partially duplicates what `--check`'s byte-comparison already proves (the codegen.rs:134-136 comment itself calls the extra digests "belt-and-braces … eyeballable in a diff") — justified duplication.

## 8. Dependencies & flags
None (data). Generator deps: schemars + serde_json in crates/commands. FNV parameters per published spec with reference vectors (spec.rs:1091-1095: fnv1a64(b"") == 0xcbf29ce484222325 etc.).

## 9. Open questions for the next agent
- Who owns adding `codegen --check` (and `schema-gen --check`, `tokens --check`) to full_gate.sh so the local gate matches CI's step list? (Same class of gap as the missing l10n-audit — see audit.rs.audit.md F-001.)
