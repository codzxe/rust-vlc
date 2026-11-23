# AUDIT: tools/xtask/src/cmd/check_identity.rs
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-23 |
| Lines / functions | 115 / 1 (run) + USAGE const; all logic delegated to lint_literals |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev gate), but wired into the dev lane: scripts/full_gate.sh:57 `cargo xtask check-identity` (full_gate step 8); dispatched via cmd/mod.rs `run: check_identity::run` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | sed on apps/cli/src/host.rs:1760-1802; rg "check-identity" scripts/full_gate.sh; read of lint_literals.rs (Identity::load L188-257, classify L289-329, contains_word L366-385, scan_workspace L577-698); baseline citations (full_gate.txt:64-68) |

## 1. One-paragraph verdict
A thin, honest CLI wrapper: parse `--root`/`--help`, resolve+canonicalize the root, load the identity from product.toml (all-missing-fields-at-once, exit 2 on malformed/missing), delegate to `lint_literals::scan_workspace(Scope::C2)`, print, map to exit 0/1/2 exactly as documented. The gate works and has teeth — it is the source of the standing full_gate identity FAILURE. The rule that fires (task question): **the C2 brand-trio whole-word rule**. `Identity::load` builds the brand list `[("identity.name","Lumen"), ("identity.publisher","Lumen Contributors")]` (short_name "Lumen" deduped away because its value equals name's), and `classify` → `brand_match` → `contains_word("lumen-backpressure-{}-{}s.wav", "lumen")` matches — case-insensitive, "lumen" bounded by string-start and `-`. The site is apps/cli/src/host.rs:1799, the string literal `"lumen-backpressure-{}-{}s.wav",` — a temp filename built by the test helper `long_wav` inside `#[cfg(test)] mod session_backpressure_tests` (module opens at host.rs:1760-1761). It is a violation because check-identity's only test-context exemption is the loopback-URL category (scan_workspace L673-685); identity literals inside `#[cfg(test)]` modules in src files get no exemption, and no `// lumen-allow-literal:` marker sits on the line. So: a true positive by the rule's letter (the product name genuinely does not need to be in a test temp filename), a policy-debatable positive by spirit, and the reason full_gate step 8 is red.

## 2. What actually works (verified)
— verified by citation of audit/runs/baseline/full_gate.txt:64-68: `cargo xtask check-identity` ran (via scripts/full_gate.sh:57) and reported exactly `apps/cli/src/host.rs:1799: identity — identity(identity.name)` — 1 violation, exit != 0, FAIL line printed.
— verified by read (lint_literals.rs:188-257): missing identity fields are reported together as an exit-2 error, matching this file's doc claim L35-37.
— verified by citation of baseline cargo-test-workspace.txt integration tests (tools/xtask/tests/lint_literals.rs): `run_tool("check-identity", ...)` fixtures assert the marker covers C2 (L243-250), C2 catches the MSI GUID that C1 leaves alone with the exact detail `identity.ids.windows.msi_upgrade_code` (L258-282), and C2 re-kinds the accent hex as `identity — identity(brand.accent_color)` (L299-322). Mutation-sensitive, real-binary tests.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/host.rs:1798-1799 (first quote: the unmarked identity literal) + tools/xtask/src/cmd/lint_literals.rs:662-685, quote at L673-675 (second quote: the only test-context exemption; is_test_context computed at L662-667) + tools/xtask/src/cmd/check_identity.rs:13-24 (documented check set — doc reference only, no code quoted from this file)
Code:
```
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
```
```
                if finding.kind == "url"
                    && is_test_context
                    && url_match(&site.text).is_some_and(|(_, local)| local)
```
What is wrong: Not a defect in this file's code — a standing repo state this gate correctly surfaces: the tree carries one unmarked identity literal in a test-only helper, so full_gate step 8 is permanently red (baseline: GATE RESULT FAIL lists identity among the red sections). The scanner KNOWS the site is in a test region (`is_test_context` is computed from `test_regions`/`whole_file_test`/test mod files) but the test-context exemption is granted ONLY to loopback URLs — identity findings inside `#[cfg(test)]` are violations by policy. The two sanctioned fixes (drop the "lumen-" prefix from the temp filename, or add `// lumen-allow-literal: <reason>`) are both unapplied. Why it matters: a permanently red gate step trains developers to ignore the gate; either the literal or the policy must change. What correct looks like: apply the marker at host.rs:1798 (`// lumen-allow-literal: test temp-file prefix, not a user surface`) or rename the file prefix. Confidence: HIGH — Verification: read (host.rs + lint_literals.rs) + baseline citation.

F-002 [S4]
Where: tools/xtask/src/cmd/check_identity.rs:29-33 vs tools/xtask/data/rebrand-allowlist.toml:9-10
Code:
```
//! (config-layer crates, tests/fixtures dirs, comments, loopback-URL
//! test exception, the inline `// lumen-allow-literal: <reason>`
//! marker — printed, never silent); see that module's docs for the
//! rationale, including why the rebrand allowlist's crate-wide
//! `crates/` entry is NOT consumed here.
```
What is wrong: Cross-file doc contradiction (resolves the open question from my rebrand.rs audit): rebrand-allowlist.toml's header claims "the future `check-identity` subcommand consumes this same file" — but check-identity, now shipped, explicitly does NOT consume it (this doc block), and implements its own allowlist policy in lint_literals (CONFIG_LAYER_CRATES, tests/fixtures dirs, marker). The rebrand allowlist is therefore consumed by no command for behavior: rebrand counts it (inert), check-identity refuses it. What correct looks like: fix the allowlist header to say it is currently documentation-only for the rebrand plane. Confidence: HIGH — Verification: read (both files + lint_literals.rs:486 CONFIG_LAYER_CRATES).

F-003 [S4]
Where: tools/xtask/src/cmd/check_identity.rs:4-9
Code:
```
//! The reverse assertion of the rebrand system (doc 07 §8.3 step 6):
//! in the un-rebranded tree, no engine/app crate hard-codes an
//! identity string where `product-config` should be used. This is why
//! `cargo xtask rebrand` CAN work — every identity-bearing surface
//! either reads `product.toml` (the config layer), is generated from
//! it, or carries an audited allowlist entry.
```
What is wrong: The header asserts an invariant ("no engine/app crate hard-codes an identity string") that this gate's own current run disproves (1 violation, full_gate red). Read as a target-state description it is fine; read as a description of the tree it is false today. What correct looks like: "...must not hard-code ... (currently 1 known violation, full_gate red)". Confidence: HIGH — Verification: read + baseline.

F-004 [S4]
Where: tools/xtask/src/cmd/lint_literals.rs:287-288 (quoted doc comment; the double identity push it mis-describes is L289-306 — affects check-identity's report)
Code:
```
/// Classify one literal's text under `scope`. Returns at most one
/// finding per kind, in kind order (identity, url, abs-path, hex-color).
```
What is wrong: The comment says "at most one finding per kind", but `classify` can push TWO `kind: "identity"` findings for one literal — one from `brand_match` (L291-296) and one from `technical_match` (L298-306) — e.g. a literal containing both "Lumen" and "dev.lumen" is reported twice at the same line. Harmless double-report, contradicting the documented contract of the shared machinery this command delegates to. What correct looks like: dedup identity findings per site or fix the comment. Confidence: MEDIUM — Verification: read (logic traced; no such literal exists in the current tree, so unobserved in output).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| USAGE const | 51-64 | REAL | — | matches module docs incl. exit-code contract |
| run | 67-115 | REAL | — | --help/-h, --root w/ value check, unknown-arg rejection; canonicalize; Identity::load (exit 2 on bad product.toml); scan_workspace(C2); print; 0/1/2 exit mapping verified against full_gate baseline |

## 5. Test honesty
No tests in this file. Coverage lives in tools/xtask/tests/lint_literals.rs (integration, real binary, scratch fixtures): marker covers C2, C2 > C1 on GUID and accent, category totals printed. All run in the baseline workspace test (cited). Would they fail if classify/scan broke? YES — exact exit codes and exact detail substrings asserted.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| identity set = name/short_name/publisher (whole-word, case-insensitive) + ids/GUID (exact substring) + accent hex (case-insensitive substring) | header L19-24 | REAL | lint_literals.rs:228-251 (brand/technical lists), classify L289-306, contains_word L366-385 |
| "A missing or malformed product.toml ... is exit 2" | header L35-37 | REAL | Identity::load L188-220 → XtaskError::Internal → exit 2 |
| "Exit code: 0 no violations \| 1 violations found \| 2 xtask error" | L41 | REAL | run L110-114 + full_gate baseline (FAIL path) |
| "no engine/app crate hard-codes an identity string" (un-rebranded tree) | header L4-6 | FALSE today | 1 standing violation (F-001/F-003) |
| rebrand allowlist consumed by check-identity | rebrand-allowlist.toml:9-10 | FALSE | this file L28-33 says NOT consumed (F-002) |

## 7. Dead / duplicated / unreachable code
None in this file. (Cross-ref: the rebrand allowlist itself is dead weight for both its consumers — see rebrand.rs audit F-002.)

## 8. Dependencies & flags
Depends on lint_literals::{Identity, Scope, find_root, scan_workspace} and toml_lite (via lint_literals). No features. Wired into full_gate step 8 (scripts/full_gate.sh:56-60).

## 9. Open questions for the next agent
- Why does the identity policy exempt loopback URLs in test context but not identity strings in test context? If intentional (identity literals are exactly what rebrand must catch), document it in this file's header; if not, host.rs:1799 is a false-positive class waiting to recur with every new test that names a temp file after the product.

## CORRECTION (gate re-run)
Wave-1 gate citation defect repaired in place by W01-A08 re-run. Every quote and line range below was re-verified with sed -n 'A,Bp' against the real tree; all match verbatim.

1. F-001 Where (THE flagged defect): OLD `tools/xtask/src/cmd/check_identity.rs:13-24 (documented check set) + apps/cli/src/host.rs:1798-1799 + tools/xtask/src/cmd/lint_literals.rs:662-685` — the first code quote (`let path = std::env::temp_dir().join(format!("lumen-backpressure-{}-{}s.wav",`) was misattributed to check_identity.rs:13-24; that code does NOT exist in check_identity.rs (grep: 0 hits). NEW: `apps/cli/src/host.rs:1798-1799 (first quote: the unmarked identity literal) + tools/xtask/src/cmd/lint_literals.rs:662-685, quote at L673-675 (second quote: the only test-context exemption; is_test_context computed at L662-667) + tools/xtask/src/cmd/check_identity.rs:13-24 (documented check set — doc reference only, no code quoted from this file)`. Both quotes verified byte-for-byte: host.rs:1798-1799 and lint_literals.rs:673-675. Finding substance unchanged.
2. F-003 quote truncation (found during re-verification): OLD quote stopped at `should be used.` — the real line 6 continues (`... should be used. This is why`) and lines 7-9 were omitted. NEW: full verbatim quote of check_identity.rs:4-9. Where stays 4-9 (now exactly matches the quote).
3. F-002 Where: OLD `check_identity.rs:28-33` → NEW `check_identity.rs:29-33` (the exact range of the quoted doc block; verified verbatim). rebrand-allowlist.toml:9-10 verified unchanged.
4. F-004 Where: OLD `lint_literals.rs:287-306 (affects check-identity's report)` → NEW `lint_literals.rs:287-288 (quoted doc comment; the double identity push it mis-describes is L289-306 — affects check-identity's report)`. Quote verified verbatim at 287-288; the L291-296 / L298-306 references in the prose verified.
5. Section 2 test-evidence ranges (tools/xtask/tests/lint_literals.rs): OLD `L258-277` → NEW `L258-282` (the exact-detail assert `identity.ids.windows.msi_upgrade_code` sits at L278-282, outside the old range); OLD `L299-317` → NEW `L299-322` (the re-kind assert `identity — identity(brand.accent_color)` sits at L316-322, outside the old range). L243-250 verified as-is.
6. Section 1: `module opens at host.rs:1760` → `host.rs:1760-1761` (#[cfg(test)] at 1760, `mod session_backpressure_tests` at 1761).
7. Section 6 row 4 Source: `header L4-7` → `header L4-6` (exact lines of the quoted claim).
8. Findings header row: unchanged — S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 (same 4 findings, no severity or count change; citations only).

Re-verification results: 5/5 code quotes match sed output verbatim (host.rs:1798-1799; lint_literals.rs:673-675; check_identity.rs:29-33; check_identity.rs:4-9; lint_literals.rs:287-288); 29/29 additional line-range citations re-checked against the tree — all PASS.
