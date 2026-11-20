# AUDIT: tools/xtask/src/cmd/sbom.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 1104 / 20 (11 free fns + 9 Json methods; 10 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask sbom` → `cmd/mod.rs:172-177` registry → `sbom::run`; also wired into `scripts/full_gate.sh` step at line 64: `if cargo xtask sbom --check > /tmp/sbom.log 2>&1`) |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | Read full file (1104 lines); python3 cross-check of Cargo.lock ↔ docs/security/sbom/cyclonedx.json: lockfile `[[package]]` rows=158, crates.io source rows=119, sbom components=158, (name,version) sets EQUAL, subject dependsOn=39 (=path-source rows 39), metadata subject "Lumen 0.1.3"; `grep -c '^\[\[package\]\]' Cargo.lock` = 158; `grep -n 'xtask sbom' scripts/full_gate.sh` (line 64); `git log --oneline -- docs/security/sbom/cyclonedx.json` (regen commits: 53a106fb "SBOM regen (158 components, subject Lumen 0.1.3, no drift)", 98cec32e same shape at 0.1.2); `sed -n` quote verification. All read-only; no cargo build/test run by me. |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

sbom.rs claims to emit a CycloneDX 1.5 SBOM of the resolved Cargo.lock graph with the product identity from product.toml, deterministic bytes, honest omissions (no licenses, no fabricated fields), and a byte-exact `--check` drift gate. It does exactly that, and I verified it end-to-end statically: the committed artifact at docs/security/sbom/cyclonedx.json has 158 components whose (name, version) set is EXACTLY the 158 `[[package]]` rows of the current Cargo.lock — including the 119 registry-source rows and 39 path-source workspace rows that appear as the subject's 39 `dependsOn` edges — with the real `Lumen 0.1.3` identity from product.toml. This is real enumeration of lockfile data, NOT a static list: a static list could not track the 0.1.2→0.1.3 regenerations recorded in git history. The defects are small: the module header still says "all 156 lockfile packages" (stale prose — the file itself prints live counts at runtime), and every component carries `scope: "required"` including dev-only/test-only crates, which overstates the shipped runtime graph even though the header admits the SBOM "describes the whole resolved graph".

## 2. What actually works (verified) — with the command you ran and its result

- **Real dependency enumeration (the core hostile question)**: python3 comparing Cargo.lock vs the committed JSON produced: `lockfile packages: 158`, `crates.io source rows: 119`, `sbom components: 158`, `sets equal: True`, `subject dependsOn count: 39`. The subject's dependency edge count matches the path-source (workspace member) row count exactly — the classification logic in `package_refs` (569-602) is doing real work on `source` strings, and `fingerprint_input`/`build_sbom` (643-664, 667-839) sort and resolve real edges.
- **Drift gate is byte-exact and real**: `--check` regenerates in memory and byte-compares (lines 185-213: missing file → exit 1, differing bytes → exit 1 with remediation text, identical → "no drift — N bytes verified"). Git history corroborates the regen workflow (53a106fb, 98cec32e: "SBOM regen (…), no drift").
- **Honest omissions verified in the artifact**: no license fields, no authors, no CPE, no timestamp in the JSON (checked schema keys: `$schema`, bomFormat, specVersion, serialNumber, version, metadata, components, dependencies only); the `lumen:license-coverage` property states cargo-deny owns licenses.
- **Deterministic serial**: `serial_uuid` (419-438) is a documented FNV-1a-128 content hash shaped as RFC 9562 UUIDv8 — non-cryptographic by design and honestly labeled (414-418); the unit test (1039-1064) asserts version nibble 8, variant 10xx, and content sensitivity.
- **CI wiring exists**: `grep -n 'xtask' scripts/full_gate.sh` → `cargo xtask sbom --check` at line 64 (plus check-architecture/lint-literals/check-identity/fixtures/web-budget steps), matching the header claim "wired into … scripts/full_gate.sh step 9" (line 58-60) — the claim is TRUE for full_gate.sh; the separate "(cargo-deny job)" CI wiring is UNVERIFIED by me (ci.yml not read).

## 3. Findings

### F-S01 [S3] — Stale hard-coded count in the header: "all 156 lockfile packages"
Where: tools/xtask/src/cmd/sbom.rs:6-8
Code:
```rust
//! Emits `docs/security/sbom/cyclonedx.json` — a CycloneDX 1.5 JSON SBOM
//! of the **resolved Cargo.lock graph** (all 156 lockfile packages:
//! registry dependencies and workspace members alike), with the product
```
What is wrong: the current Cargo.lock has 158 `[[package]]` rows and the committed SBOM has 158 components (both counted by me). The "156" is a frozen-in-prose number from an older lockfile.
Why it matters: this is exactly the class of hard-coded number a hostile audit hunts for — here it lives in documentation rather than in the computation (the runtime "components:" line at 176-182 is computed from the parsed lockfile, and `parse_lock` rejects unknown keys loudly at 337-343), so the OUTPUT cannot go stale — but the DOC did. A reader trusting the header's "156" would wrongly conclude the committed 158-component artifact is out of spec.
What correct looks like: drop the count from prose ("all lockfile packages") — the artifact is self-describing and `--check` is the guard.
Confidence: HIGH
Verification: ran (counted 158/158 and set equality via python3; grep of the doc line).

### F-S02 [S3] — `scope: "required"` stamped on every lockfile package, including dev-only and target-gated ones
Where: tools/xtask/src/cmd/sbom.rs:763-775 (component emission), header 30-32
Code (774-775):
```rust
        json.key("scope");
        json.str_value("required");
```
What is wrong: CycloneDX `scope` is defined relative to the subject's runtime: `required` = needed at runtime. Cargo.lock includes dev-dependencies, build-dependencies and target-gated crates (e.g. `assert_matches`, windows-only crates on Linux trees) that will never execute in the shipped binary. All 158 get "required". The header admits the posture ("every lockfile row is in the resolved build graph", 30-32, and "Shipped-app vs dev-tool is NOT encoded in Cargo.lock", 688-691) — but the ARTIFACT itself, once separated from this file's commentary, asserts that test-only crates are runtime-required components of Lumen.
Why it matters: a downstream consumer (SBOM scanner, auditor, vulnerability matcher) will flag advisories against dev-only crates as production exposure — noise at best, wrong risk ranking at worst. "Honest" lives in the generator's comments, not in the bytes it emits.
What correct looks like: omit `scope` entirely (CycloneDX allows omission) or derive required/optional via `cargo metadata`'s dependency kinds — the latter breaks the C5 std-only posture, so omission is the honest fix.
Confidence: HIGH
Verification: read + artifact inspection (all components carry `"scope": "required"`; 158 occurrences of the scope key pattern alongside 158 bom-refs minus the subject).

### F-S03 [S4] — Subject dependency semantics: "subject depends on every path-source package" conflates workspace members with shipped code
Where: tools/xtask/src/cmd/sbom.rs:688-699
Code (688-697):
```rust
    // The subject depends on every path-source package (the workspace
    // member set). Shipped-app vs dev-tool is NOT encoded in Cargo.lock;
    // this SBOM describes the whole resolved graph, stated in the module
    // docs and the artifact's README.
    let mut subject_deps: Vec<usize> = sorted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.source.is_none())
        .map(|(idx, _)| idx)
        .collect();
```
What is wrong: `tools/xtask` itself and every dev tool member are declared direct dependencies of the Lumen application subject (verified: subject `dependsOn` = 39 = all path rows, including xtask/mem-audit/sweep/testkit tooling). Honest comment, overstated bytes.
Why it matters: same consumer-facing overstatement as F-S02, one level up (the subject edge).
What correct looks like: prune tool members from the subject edge and record the pruning in a property, or emit `dependsOn` only for the actually-shipped member set.
Confidence: HIGH (structure verified in the artifact), MEDIUM (whether this matters to Lumen's consumers)
Verification: ran (subject dependsOn count = 39 = path rows, python3).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 116-228 | REAL | — | Flag parsing (--check/--root/--lock/--out), root discovery, parse, build, write/compare; exit codes 0/1/2 honored. |
| `find_root` | 234-251 | REAL | — | Walks up to product.toml. |
| `parse_lock` | 279-363 | REAL | — | Real [[package]] extraction; unknown key → loud error (337-343); `version` root entry required. |
| `read_product_identity` | 380-397 | REAL | — | identity.name/display_version/publisher with type errors. |
| `fnv1a_64` | 405-412 | REAL | — | Standard FNV-1a. |
| `serial_uuid` | 419-438 | REAL | — | Deterministic UUIDv8 shaping; honestly non-cryptographic. |
| `Json::new`/`open`/`close`/`newline`/`separator`/`key`/`str_value`/`int_value`/`str_array` | 456-533 | REAL | — | Deterministic pretty printer; tested (1076-1090). |
| `json_escape` | 538-556 | REAL | — | RFC 8259 controls; tested. |
| `package_refs` | 569-602 | REAL | — | purl/distribution only for the exact crates.io registry constant; git/other sources stay opaque (tested 1093-1103). |
| `resolve_dep` | 608-639 | REAL | — | Version-suffix disambiguation; ambiguous/unresolvable → loud error (629-638). |
| `fingerprint_input` | 643-664 | REAL | — | Sorted, order-independent content fingerprint. |
| `build_sbom` | 667-839 | REAL | S3 | Real graph build; scope/subject overstatement (F-S02/F-S03). |
| tests (10) | 845-1104 | HONEST | — | Real mini-lock parsing, determinism, reorder-stability, CycloneDX field presence, opaque non-crates.io handling. Would fail if the reader or emitter regressed. |

## 5. Test honesty

Yes. The ten tests parse a realistic MINI_LOCK (multi-version packages, path+registry sources, dependency arrays), assert ambiguity errors, byte-determinism, reorder-stability (rows swapped → identical output), the presence of the exact CycloneDX fields, and that path packages do NOT claim a purl. If `parse_lock` silently dropped packages, `resolve_dep` invented edges, or the emitter lost fields, these fail. What they cannot catch: drift between the real Cargo.lock dialect and MINI_LOCK (mitigated by the loud unknown-key rejection) and the scope-semantic issue (F-S02), which is a spec judgment, not a code bug.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all 156 lockfile packages" | sbom.rs:7 | FALSE (stale) | 158 counted in Cargo.lock and in the artifact (F-S01). |
| "Emits … a CycloneDX 1.5 JSON SBOM of the resolved Cargo.lock graph" | sbom.rs:5-9 | TRUE | Set-equality cross-check (158/158, exact (name,version) match). |
| "byte-stability across lockfile re-orders" | sbom.rs:28-29 | TRUE (by test) | Test `sbom_is_deterministic_and_reorder_stable` (977-997); UNVERIFIED by execution (cargo test serialized). |
| "Wired into CI (cargo-deny job …) and scripts/full_gate.sh step 9" | sbom.rs:58-60 | HALF-VERIFIED | full_gate.sh line 64 runs `cargo xtask sbom --check` (verified); the CI cargo-deny-job wiring UNVERIFIED (ci.yml not in scope of my read). |
| "Licenses are NOT included (Cargo.lock carries none; G3 cargo-deny owns license policy)" | sbom.rs:45-46, 93-94 | TRUE | Artifact contains no license fields; property records the delegation. |
| "serialNumber is a deterministic content-derived UUIDv8" | sbom.rs:47-49 | TRUE | Artifact serial `urn:uuid:57dc9149-8ff3-89ca-8883-92b89fc20cec` — version nibble 8, variant 8; test 1039-1064 pins the shape. |

## 7. Dead / duplicated / unreachable code

- None found. All 20 functions are reachable from `run`. The `Must` test trait duplicates the one in toml_lite.rs (cargo-clone between test modules — acceptable, test-only).
- `Json::int_value` is used only for `"version": 1` — near-trivial but real.

## 8. Dependencies & flags

- std only (C5 posture held: no toml/serde_json/cargo-cyclonedx, no `cargo metadata` subprocess, no network — verified by full read: only `std::fs`, `std::fmt`, `std::path`, `std::process::ExitCode`).
- Flags: `--check`, `--root <dir>`, `--lock <path>`, `--out <path>`, `--help`. Bad flag → usage error exit 2 (144-148). Defaults: `<root>/Cargo.lock`, `<root>/docs/security/sbom/cyclonedx.json`.
- Depends on `super::toml_lite` (audited separately — its F-T01 duplicate-header gap applies to `read_product_identity` if product.toml ever grows a duplicated `[identity]`).

## 9. Open questions for the next agent

- Does the CI "cargo-deny job" actually invoke `cargo xtask sbom --check` (header claim at line 58)? Read `.github/workflows/ci.yml` for the wiring — if absent, only full_gate.sh enforces drift locally.
- F-S02/F-S03: is there a Lead decision on record accepting "whole resolved graph, all required" as the SBOM scope? If yes, the property set should record that decision in the artifact itself, not only in generator comments.
- If Cargo.lock moves to format v5 (root `version = 5` with changed keys), `parse_lock`'s loud unknown-key rejection fires — the failure mode is honest, but confirm the CI failure would be attributed to the reader rather than ignored.
