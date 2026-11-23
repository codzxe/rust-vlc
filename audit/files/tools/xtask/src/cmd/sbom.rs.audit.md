# AUDIT: tools/xtask/src/cmd/sbom.rs
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 1104 / 21 prod (incl. nested `dep_entry`, 8 `Json` methods) + 10 tests + `Must` trait |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev/CI tool: `cmd/mod.rs:172` dispatches `sbom` -> `sbom::run`; invoked by `.github/workflows/ci.yml:528` (`cargo run -p xtask --locked -- sbom --check`), `scripts/full_gate.sh:64`, `scripts/quality/07_supply_chain.sh:20`, `scripts/packaging/gen-sboms.sh:44`. Not compiled into lumen-cli/desktop. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `./target/debug/xtask sbom --check` -> exit 1 (DRIFT reproduced); `./target/debug/xtask sbom --out audit/tmp/cyclonedx.regenerated.json` -> exit 0; `diff` committed vs regenerated -> 1 hunk; `./target/debug/deps/xtask-e9d837ce498c05b0 sbom` -> 10 passed / 0 failed; python3 JSON analysis of committed SBOM; `rg`/`sed`/`awk` on Cargo.lock, ci.yml, full_gate.sh |

## 1. One-paragraph verdict
This file is a real, working, deterministic std-only CycloneDX 1.5 SBOM generator with a byte-compare drift gate. I executed both modes and its 10 unit tests: all pass. The baseline failure (`full_gate` step 9 sbom --check FAILED) is **NOT a broken check — the committed artifact is stale**: `docs/security/sbom/cyclonedx.json` still records a `device-discovery@0.1.0 -> plugin-api@0.1.0` dependency edge that the current `Cargo.lock` no longer contains (removed 2026-11-17 per `crates/device-discovery/Cargo.toml:47-53`), i.e. the SBOM was never regenerated after the lockfile changed. That is one stale edge in committed security evidence and a red merge gate. The code itself is honest about its limitations (no licenses, no timestamp, subject spans dev tools) except two doc-vs-code lies: the serial-number fingerprint claims to include dependency edges but does not, and the module doc cites a stale package count (156 vs actual 158).

## 2. What actually works (verified)
- `./target/debug/xtask sbom --check` -> "components: 158 (119 registry, 39 workspace/path) ... result: DRIFT" exit 1 — reproduces baseline full_gate.txt:70-75 exactly.
- `./target/debug/xtask sbom --out audit/tmp/cyclonedx.regenerated.json` -> exit 0, 122701 bytes; deterministic regeneration; `diff` vs committed file shows exactly one line (`"plugin-api@0.1.0",` at committed L3831) — drift isolated to the stale `device-discovery` edge.
- `./target/debug/deps/xtask-e9d837ce498c05b0 sbom` -> `10 passed; 0 failed` (all unit tests in this file).
- python3: committed SBOM has 158 components, 119 purls, 0 duplicate bom-refs, subject `Lumen 0.1.3` — structurally consistent with Cargo.lock (158 packages, 119 crates.io sources, no git/alt-registry sources).

## 3. Findings
F-001 [S2]
Where: docs/security/sbom/cyclonedx.json:3827-3833 (evidence; detected by this file's gate, sbom.rs:203-213)
Code:
```
    {
      "ref": "device-discovery@0.1.0",
      "dependsOn": [
        "media-types@0.1.0",
        "plugin-api@0.1.0",
        "testkit@0.1.0"
      ]
```
What is wrong: The committed SBOM asserts `device-discovery` depends on `plugin-api`; Cargo.lock:191-196 says it depends only on `media-types` + `testkit` (the dep was removed as dead weight, per the NOTE at crates/device-discovery/Cargo.toml:47-53) and the SBOM was never re-committed. Why it matters: committed supply-chain evidence contradicts the resolved dependency graph, and the release merge gate is red (baseline full_gate.txt:69-75). What correct looks like: run `cargo xtask sbom` and commit; the check's diagnosis is correct.
Confidence: HIGH — Verification: ran (`sbom --check` exit 1; `diff` shows exactly this line as the sole difference).

F-002 [S3]
Where: tools/xtask/src/cmd/sbom.rs:641-642 (doc) vs 643-664 (impl)
Code:
```
/// Canonical fingerprint input: subject identity + every package's
/// identity fields + the resolved edges. Order-independent (sorted).
fn fingerprint_input(identity: &ProductIdentity, lock: &LockFile) -> String {
```
What is wrong: The comment claims the fingerprint includes "the resolved edges"; the implementation formats only subject + per-package `name/version/source/checksum` rows — no dependency edges. Why it matters: the `serialNumber` (L702) does not change when only the dependency graph changes, so two materially different SBOMs share a serial; CycloneDX consumers keying on `serialNumber` see "same BOM". The doc is false. Mitigation: the drift gate byte-compares the whole artifact, so gate integrity is unaffected. What correct looks like: include sorted edge rows in `fingerprint_input` (or delete the "resolved edges" claim).
Confidence: HIGH — Verification: read (impl verified by eye against the quoted body; no edges formatted).

F-003 [S4]
Where: tools/xtask/src/cmd/sbom.rs:6-8
Code:
```
//! Emits `docs/security/sbom/cyclonedx.json` — a CycloneDX 1.5 JSON SBOM
//! of the **resolved Cargo.lock graph** (all 156 lockfile packages:
```
What is wrong: The lockfile has 158 packages (my run: "components: 158"; baseline full_gate.txt:72 identical). The doc number is stale. Why it matters: doc-vs-reality drift in the file whose whole purpose is drift detection. What correct looks like: drop the hard-coded count from the doc.
Confidence: HIGH — Verification: ran (`sbom --check` output; `grep -c '^\[\[package\]\]' Cargo.lock` -> 158).

F-004 [S4]
Where: tools/xtask/src/cmd/sbom.rs:188-193
Code:
```
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "missing: {} — run `cargo xtask sbom` and commit the generated SBOM",
                    SBOM_REL
                );
```
What is wrong: The message prints the hard-coded `SBOM_REL` even when `--out` overrode the output path, so the message names a file that was not the one checked; the drift message (L207-212) does the same. Also, arg parsing (L129-143) accepts `--out --check`, silently consuming `--check` as a path. Why it matters: misleading diagnostics in the exact mode CI runs; mis-parsed flags fail later with a confusing error. What correct looks like: print `out_path.display()`; reject flag-shaped option values.
Confidence: HIGH — Verification: read.

F-005 [S3]
Where: tools/xtask/src/cmd/sbom.rs:116-228 and 845-1104 (test module)
Code:
```
/// Entry point for `cargo xtask sbom`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
```
What is wrong: `run()` — argument parsing, `find_root()`, missing-file -> exit 1, drift -> exit 1, byte-compare — has zero test coverage; all 10 unit tests exercise pure helpers (`rg 'run\('` in the test module: none). Why it matters: the exit-code contract (0/1/2, documented at L96-102) is what ci.yml:528 and full_gate.sh:64 branch on; nothing asserts it. A regression in `run()` (e.g. exiting 2 on drift, or 0 on missing file) would pass the whole unit suite. What correct looks like: a test invoking `run(&["--check", "--lock", tmp, "--out", tmp])` against missing/stale/fresh files asserting the three exit codes.
Confidence: HIGH — Verification: read + `rg` (no run( calls in tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 116-228 | REAL | — | Executed both modes; drift logic correct (F-001 is artifact staleness, not code) |
| `find_root` | 234-251 | REAL | — | Walk-up to product.toml; not unit-tested |
| `parse_lock` | 279-363 | REAL | — | Loud failure on unknown per-package keys; root-level unknown sections silently ignored (toml_lite rejects dotted `[[patch.unused]]` headers loudly — toml_lite.rs:178-183) |
| `read_product_identity` | 380-397 | REAL | — | Reads identity.name/display_version/publisher; verified against real product.toml via successful runs |
| `fnv1a_64` | 405-412 | REAL | — | Standard FNV-1a |
| `serial_uuid` | 419-438 | REAL | — | RFC 9562 v8 nibble math checked: version shift 76, variant 10xx shift 60 — correct; not content-complete (F-002) |
| `Json::new` | 456-463 | REAL | — | |
| `Json::open`/`close` | 465-482 | REAL | — | Empty-container handling verified by test |
| `Json::newline`/`separator` | 484-508 | REAL | — | Root-no-separator case handled |
| `Json::key`/`str_value`/`int_value`/`str_array` | 510-533 | REAL | — | |
| `json_escape` | 538-556 | REAL | — | RFC 8259 controls covered by test |
| `package_refs` | 569-602 | REAL | — | Non-crates.io source stays opaque; current lockfile has none (all 119 = crates.io index) |
| `resolve_dep` | 608-639 | REAL | — | Ambiguity/missing -> loud error; descriptor forms "name"/"name version" only (no parenthesized sources in this lockfile) |
| `fingerprint_input` | 643-664 | REAL | — | Doc lie (F-002) |
| `build_sbom` | 667-839 | REAL | — | Deterministic, reorder-stable (verified by test + my two runs) |
| `dep_entry` (nested) | 818-829 | REAL | — | |
| `Must` (test trait) | 849-873 | REAL | — | Uses `unreachable!` (2 of the tree's 4126) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_lock_packages_and_classifies_sources | YES | Asserts counts, sources, deps, dual beta versions |
| rejects_unknown_lockfile_keys_loudly | YES | Asserts three error paths |
| reads_product_identity | YES | Asserts fields + missing-key error |
| dependency_resolution_handles_version_suffixes | YES | Asserts version-suffix match, ambiguous + missing errors |
| sbom_is_deterministic_and_reorder_stable | YES | Two equal builds + swapped-row lockfile must byte-match |
| sbom_carries_expected_cyclonedx_fields | YES (mostly) | 18 needle assertions incl. purl-absence for path package; string-contains only, no schema validation |
| serial_uuid_is_rfc9562_shaped_and_content_sensitive | YES | Version/variant nibbles, determinism, content sensitivity |
| json_escape_covers_rfc8259 | YES | Exact-string asserts |
| json_writer_pretty_prints_and_handles_empty_containers | YES | Exact-output assert incl. `[]` |
| non_crates_io_source_stays_opaque | YES | Asserts no purl for git source |
All 10 executed by me: pass. Gap: `run()`/exit codes untested (F-005).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Wired into CI (cargo-deny job, after the deny check) and scripts/full_gate.sh step 9" | sbom.rs:58-60 | REAL | ci.yml:523-528 (deny at :527, sbom at :528); full_gate.sh:63-67 |
| "all 156 lockfile packages" | sbom.rs:7 | FALSE | 158 (my run + baseline full_gate.txt:72) — F-003 |
| fingerprint "and the resolved edges" | sbom.rs:641 | FALSE | No edges formatted — F-002 |
| "Licenses are NOT included ... G3 cargo-deny owns license policy" | sbom.rs:44-46, 93-94 | REAL | Committed SBOM has no license fields (python3 check); `lumen:license-coverage` property present |
| "serialNumber is a deterministic content-derived UUIDv8 ... same lockfile + product identity -> byte-identical SBOM" | sbom.rs:47-49 | REAL (determinism) / overstated (content-derived) | Two runs byte-identical; but content-derived is incomplete w.r.t. edges (F-002) |
| "Shipped-app vs dev-tool is NOT encoded in Cargo.lock; this SBOM describes the whole resolved graph" | sbom.rs:688-691 | REAL | Honest disclosure; subject dependsOn includes xtask/testkit (dev tools) |

## 7. Dead / duplicated / unreachable code
None. Every function is called; `int_value` used once (L715), fine.

## 8. Dependencies & flags
- Depends only on `super::toml_lite` + `crate::XtaskError` (std-only claim verified: no `use` of external crates, L69-74).
- xtask is a workspace member (in Cargo.lock: 39 path packages incl. xtask), never a dependency of lumen-cli/desktop — no feature-flag gating.

## 9. Open questions for the next agent
- Who removed the device-discovery->plugin-api edge without regenerating the SBOM (2026-11-17 "audit T5") — is the same staleness pattern repeated for other committed generated artifacts (fixtures, schema-gen, tokens)? Check `cargo xtask` gates that PASS against committed artifacts.
- `scope: "required"` is emitted for all 158 components including dev-only tools (xtask, testkit) — disclosed (L30-32) but arguably wrong CycloneDX semantics; does any consumer (gen-sboms.sh / release verification) key on scope?
