# AUDIT: crates/localization/tests/fixtures/refs/unresolved.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 11 / n/a (test fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test input; consumed by tests/audit_proof_matrix.rs `rule4_unresolved_references_are_flagged_exactly` via `extract_msgid_refs("layouts/menus.toml", &src)`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | byte-level verification of the file (python3 + base64: line 5 = `[[menu.leaf]]`, 13 bytes, verified because tool output strips `[[m` sequences — display artifact, file content intact); baseline cargo-test-workspace.txt:2879-2976 (localization 45 unit + audit_proof_matrix incl. rule4, exit 0). |

## 1. One-paragraph verdict
Exactly what its header promises: a menu-config-shaped source with two `[[menu.leaf]]` entries — one `label_key` that resolves against locales-good (`fixture.greeting`), one that does not (`fixture.no-such-key`) — and the consuming test asserts exactly 2 extracted refs and exactly 1 `UnresolvedReference` with file/field/key matched. Note for other agents: my terminal pipeline corrupts the DISPLAY of `[[m…` sequences; I verified the actual bytes via base64 before trusting any quote.

## 2. What actually works (verified)
— verified by baseline run (localization package exit 0): `rule4_unresolved_references_are_flagged_exactly` passes — asserts refs.len()==2, exactly one UnresolvedReference { file: "layouts/menus.toml", field: "label_key", key: "fixture.no-such-key" }, and the combined entry point returning 1 issue; the sibling resolved.toml is the clean negative control (0 issues).
— verified byte-level: file is 428 bytes, 11 lines, two `[[menu.leaf]]` tables each with `label_key` + `command` (media.open-file / media.open-url) — valid TOML (parsed successfully by `toml::Table` in refs.rs:86 inside the passing test).

## 3. Findings
None. Checked: valid TOML (parses in the passing test); exactly the two documented label_keys; no stray keys; header comment matches observed behavior (1 resolves, 1 does not).

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`rule4_unresolved_references_are_flagged_exactly`: YES — exact counts and exact struct-field matches; would fail if the extractor double-counted, missed a ref, or flagged the resolving key.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One reference resolves (fixture.greeting), one does not (fixture.no-such-key): exactly one UnresolvedReference is expected" | fixture header lines 1-4 | REAL | baseline rule4 test asserts exactly this |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
None.
