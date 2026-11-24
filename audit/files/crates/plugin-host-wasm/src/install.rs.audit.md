# AUDIT: crates/plugin-host-wasm/src/install.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 637 / 16 |
| File verdict | REAL — well-tested install/load/tamper-refusal store; but UNREACHABLE from the shipped release lane |
| Reachable from shipped entry point? | NO — `rg -n "plugin" crates/lumen-cli/Cargo.toml apps/desktop/src-tauri/Cargo.toml` → no matches (exit 2); neither lumen-cli nor desktop depends on plugin-host-wasm. Reached only by the `lumenext` dev bin (`crates/plugin-host-wasm/src/bin/lumenext.rs`) and the test suites. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 3 |
| Commands run | rg callers/deps (exit 0/2); sed quote verification (exit 0); baseline citation |

## 1. One-paragraph verdict
Real, defensively-written install lifecycle: packages arrive as caller-provided bytes (no network fetch, no auto-download), pass a full validation chain (ZIP structure+caps+CRC+wasm magic, manifest schema, origin=extension fence, v1 kind fence, api window, full `sha256:<64hex>` provenance match against the module, signature-block/manifest cross-consistency), require an exact-match consent, install per-user under `<root>/extensions/<id>/<version>/`, never execute at install time, and re-verify the module digest at every load — all proven by the passing baseline lifecycle tests. Signatures are NEVER cryptographically verified (v1 records `Unverified`/`UnknownSuite` honestly; `SignatureStatus` has no `Verified` variant — package.rs:79-86). Hostile focus answers: sha256 YES (enforced install+load, hand-rolled but NIST-vector-tested), signature verification NO (bookkeeping only, admitted), install origin = caller's bytes, per-user root. Defects: two comments claim `load` re-checks `kind` — it does not (the one `kind_extension_legal` call is in `validate_package`, line 479); `remove` ignores its consent parameter entirely while the module doc sells consent enforcement for both operations; consent never binds extension id/version/digest despite the doc's "cannot slip in another extension" absolutism; load's signature cross-check is one-way versus install's.

## 2. What actually works (verified)
— baseline `audit/runs/baseline/cargo-test-workspace.txt:4884-5022`: plugin-host-wasm exit_code=0; `plugin_install_remove`, `plugin_no_auto_exec`, `plugin_malicious_manifest`, `fixture_module_executes` all pass — exercising consent refusal, idempotency, different-bytes refusal, version coexistence + highest-pick, module tamper refusal (`InstalledHashMismatch`), NotInstalled paths, and no-exec-at-install (trapping start section faults only at spawn).
— In-file unit tests `safe_component_rejects_traversal`, `consent_must_match_exactly` pass in the same baseline (44 unit tests ok).
— rg: `kind_extension_legal` occurs exactly once in this file (line 479, `validate_package`); `SignatureStatus` enum has no `Verified` variant (package.rs:79-86); `status()` (package.rs:155-162) returns only `Unverified`/`UnknownSuite`.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/install.rs:319-322 and 377-379
Code:
```
    /// Load an extension for spawning: the highest installed version,
    /// re-validated end to end (manifest, origin, kind, api window,
    /// **digest** — a tampered on-disk copy refuses to load, doc 06
```
and
```
        // Re-validate exactly like install (structural rules come
        // free: the files were extracted from a package once; the
        // manifest, origin, kind, window and hash are re-checked).
```
What is wrong: Both comments claim `kind` is re-checked at load. It is not: `load` (328-415) calls `parse_extension_manifest` (origin only) and `api::negotiate_api_version`, but `api::kind_extension_legal` appears nowhere in the load path (rg: single hit at line 479 inside `validate_package`), and `WasmInstance::spawn` does not check kind either (instance.rs:91 merely records `kind: manifest.kind`). Why it matters: the doc 06 §5.9 v1 kind fence is install-only; a post-install edit of the unprotected manifest.toml (the provenance hash covers only plugin.wasm) loads and spawns an extension-illegal kind while the comments claim tamper-refusal. The doc is false about what the code does. What correct looks like: call `api::kind_extension_legal(manifest.kind)` in `load` (fail-closed) or delete "kind" from both comments. Confidence: HIGH — Verification: read + rg.
F-002 [S3]
Where: crates/plugin-host-wasm/src/install.rs:17-21 vs 259-267
Code:
```
//! Install and remove take an [`InstallConsent`] that records *what
//! the user saw and confirmed*: publisher, permission list, signature
//! status. The store refuses an install whose package does not match
//! the consent exactly
```
and
```
    pub fn remove(
        &self,
        id: &str,
        version: Option<&str>,
        _consent: &InstallConsent,
    ) -> Result<Removed, HostError> {
```
What is wrong: `remove` ignores `_consent` completely (the parameter body comment at 265-267 admits it "only needs to exist"). The module doc's "Consent is a value, not a flag" section presents install and remove as equally consent-enforced; only install enforces. Any code path can remove any installed extension with a fabricated consent. Why it matters: the doc overstates the guarantee; an embedder that skips the confirmation dialog for removal gets no backstop (contrast: install's mismatch refusal is real and tested). The lifecycle test cannot test remove-consent refusal because no such refusal exists. What correct looks like: validate the consent against the installed manifest's publisher/signature before deleting, or state plainly that remove consent is unenforced in v1. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: crates/plugin-host-wasm/src/install.rs:394-397 (load) vs 524-532 (validate_package)
Code:
```
        if let Some(declared) = provenance.signature.as_ref() {
            let recorded = signature_block.as_ref().map(|b| b.suite.as_str());
            if recorded != Some(declared.scheme.as_str()) {
```
What is wrong: install's `validate_package` rejects BOTH `(None, Some)` (block without manifest declaration) and `(Some, None)` mismatches; `load` only checks the declared→recorded direction. A manifest.toml with `[provenance].signature` stripped (post-install edit — manifest is not hash-bound) plus the orphaned signature file loads without error, reporting the block's status, a package state install would refuse. Contradicts the "Re-validate exactly like install" comment. Why it matters: one-sided re-validation; the tamper story is asymmetric. What correct looks like: mirror the full `(block, declared)` match from `validate_package` in `load`. Confidence: HIGH — Verification: read.
F-004 [S3]
Where: crates/plugin-host-wasm/src/install.rs:54-61, 79-83
Code:
```
pub struct InstallConsent {
    /// Publisher the user was shown (manifest `[provenance]`).
    pub(crate) publisher: String,
    /// Permission names the user was shown (plain language list).
    pub(crate) permissions: Vec<&'static str>,
    /// Signature status the user was shown.
    pub(crate) signature: SignatureStatus,
}
```
What is wrong: Consent binds publisher+permissions+signature posture but NOT the extension id, version, or module digest. Two packages from the same publisher with identical permission sets and identical signature posture satisfy each other's consent — the module doc's "a UI cannot present one extension and slip in another" (line 22-23) is false in that (realistic: same vendor ships two extensions) case, and the consent never binds the bytes the user evaluated. What correct looks like: include id (and version or digest) in `InstallConsent::covers`. Confidence: HIGH — Verification: read; `covers` (79-83) compares exactly three fields.
F-005 [S4]
Where: crates/plugin-host-wasm/src/install.rs:230-241
Code:
```
        std::fs::write(dir.join(ENTRY_MANIFEST), &package.manifest_toml)
```
What is wrong: non-atomic three-file write (no temp+rename); a crash mid-install leaves a partial version dir. Fail-closed on read (torn module → hash mismatch; torn manifest → parse error), so this is hygiene, not corruption risk. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/plugin-host-wasm/src/install.rs:337-350
Code:
```
            let better = match &best {
                None => true,
                Some((current, _)) => parsed.precedence(current) == std::cmp::Ordering::Greater,
            };
```
What is wrong: `Semver::precedence` ignores build metadata (semver.rs:102-107, "build metadata ignored"), so `1.0.0` vs `1.0.0+b1` tie and the winner is whichever `read_dir` (OS order) yields first — nondeterministic pick among equal-precedence versions. What correct looks like: tie-break on the raw string. Confidence: MEDIUM — Verification: read of semver.rs; no execution.
F-007 [S4]
Where: crates/plugin-host-wasm/src/install.rs:218
Code:
```
            let existing = std::fs::read(dir.join(ENTRY_MODULE)).map_err(HostError::from)?;
```
What is wrong: if the version dir exists but the module file is missing (partially deleted install), install fails with a raw Io error and no recovery hint, unlike the different-bytes branch (220-224) which says "remove it first". Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| InstallConsent::confirm | 63-76 | REAL | — | pub constructor; any code can fabricate (design accepts trusted embedder) |
| InstallConsent::covers | 79-83 | REAL | S3(F-004) | 3-field exact match; no id/version/digest |
| InstallReceipt/InstalledVersion/InstalledExtension/LoadedExtension/InstallPreview/Removed | 86-133,429-446 | REAL | — | plain data carriers |
| ExtensionStore::open | 147-156 | REAL | — | create_dir_all, typed Io error |
| ExtensionStore::root | 159-161 | REAL | — | getter |
| ExtensionStore::inspect | 172-179 | REAL | — | validate-only, installs/executes nothing (tested: plugin_no_auto_exec) |
| ExtensionStore::install | 188-249 | REAL | S4(F-005,7) | full chain + consent + traversal-safe dirs; non-atomic writes |
| ExtensionStore::remove | 259-278 | REAL | S3(F-002) | consent ignored (`_consent`) |
| ExtensionStore::installed | 284-317 | REAL | — | NotFound → empty; unsorted (documented) |
| ExtensionStore::load | 328-415 | REAL | S3(F-001,3), S4(F-006) | highest-version pick; origin+window+hash+one-way sig check; kind NOT re-checked |
| extension_dir / version_dir | 417-426 | REAL | — | both go through safe_component |
| validate_package | 471-562 | REAL | — | the 6-step chain is real and matches its doc; sha256 enforced |
| parse_extension_manifest | 566-573 | REAL | — | plugin-api validator + origin fence |
| From<ManifestError> | 575-579 | REAL | — | stringly-typed but typed at HostError layer |
| safe_component | 584-594 | REAL | — | rejects "", ".", "..", separators, NUL; unit-tested |
| declared_permissions | 599-601 | REAL | — | trivial clone; UI helper |
| tests (2) | 607-637 | REAL | — | honest, would fail on regression |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| safe_component_rejects_traversal | YES | direct input/output on the guard |
| consent_must_match_exactly | YES | covers() asserted on wrong publisher/perms/posture |
| tests/plugin_install_lifecycle.rs::plugin_install_remove (baseline) | YES | wrong-consent install asserted refused; tampered module asserted `InstalledHashMismatch`; highest-version pick asserted; remove→NotInstalled asserted. Would NOT catch F-001/F-003 (no on-disk manifest-tamper fixture) or F-002 (remove-consent never asserted) |
| plugin_no_auto_exec (baseline) | YES | trapping start section must NOT fire at install/load, only at spawn |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 never claims `verified`" (receipt) | install.rs:96-98 | REAL | package.rs:79-86 enum has no Verified; status() 155-162 |
| "a tampered on-disk copy refuses to load" | install.rs:321-322 | SHALLOW | true for module bytes (hash re-check, tested); manifest.toml is NOT integrity-bound (hash covers module only), and kind/sig-direction checks are missing — tampered manifests load |
| "kind … re-checked" at load | install.rs:320, 378 | FALSE | F-001: no kind check in load path |
| "no code path exists that installs without a consent value naming the package" | install.rs:23-24 | SHALLOW | install requires consent, but consent names publisher/perms, not the package (F-004), and remove needs none (F-002) |
| "Merely reading or opening a .lumenext … installs and executes nothing — tested" | install.rs:24-26 | REAL | plugin_no_auto_exec in baseline |
| structural fence "ELF/Mach-O/PE payloads die here" | install.rs:458-459 | REAL-UNPROVEN | read_package enforces wasm magic (claimed; package.rs is another agent's file; baseline plugin_malicious_manifest passes) |

## 7. Dead / duplicated / unreachable code
No dead code. Whole module unreachable from the release lane (lumen-cli/desktop lack the dependency — this is the known Phase-7-not-wired state, cf. baseline line 4234 gap note "plugins.crashed not wired — handoff H-10").

## 8. Dependencies & flags
plugin-api (manifest/Origin/Semver), crate::api (Grants, negotiate_api_version, kind_extension_legal), crate::package (read_package, module_digest, SignatureStatus/Block), crate::error. No external deps. No feature gates.

## 9. Open questions for the next agent
- Does any UI/desktop plan wire ExtensionStore, and will consent then bind id/version (F-004)?
- package.rs reader: does it enforce max module/package sizes ("caps" per 458) — asserted in the chain doc, verification belongs to the package.rs auditor.
