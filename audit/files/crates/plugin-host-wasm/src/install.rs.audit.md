# AUDIT: crates/plugin-host-wasm/src/install.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Lines / functions | 637 / 18 (16 fns/methods/impls + 2 tests) |
| File verdict | UNREACHABLE (real, internally consistent code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `ExtensionStore` is referenced ONLY by `crates/plugin-host-wasm/src/lib.rs` (re-export), the crate's own integration tests (`tests/plugin_install_lifecycle.rs`, `tests/plugin_examples_conformance.rs`) and `plugins/extensions/README.md`. Grep over all `*.rs` for `plugin_host_wasm` finds no consumer outside the crate itself; no `apps/*` Cargo.toml depends on `plugin-host-wasm`. Shipped bins are `lumen`/`desktop`/`lumen-desktop-tauri` (scripts/packaging/gate.sh:85), none of which link this crate. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 1 |
| Commands run | Grep `ExtensionStore\|InstallConsent\|declared_permissions` (→ only lib.rs re-export, tests, README); Grep `plugin_host_wasm` in `*.rs` (→ only crate's own bins/tests); Grep `plugin-host-wasm` in `**/Cargo.toml` (→ only its own manifest); Grep `\.load\(|\.install\(` in crate (→ only tests); Read of api.rs (Grants), package.rs (SignatureBlock), error.rs, tests/plugin_install_lifecycle.rs; sed -n verification of every quote below; grep baseline test logs (see F-verification). |

## 1. One-paragraph verdict
This is genuinely written, defensive, well-tested-looking code — typed refusals for every malformed-package case, exact-match consent at install, digest re-verification at load, path-component fencing. But it is a store with no shoppers: nothing in the repository installs, removes, or loads an extension except the crate's own integration tests; the shipped `lumenext` CLI (audited separately) deliberately refuses to install or execute. Worse for the evidence story, the two baseline `cargo test --workspace` runs both ABORTED (at `desktop` and `first-vertical` respectively) before ever reaching `plugin-host-wasm`, so not one of the store's lifecycle tests has committed run evidence. Inside the file there are two real defects: the documented "re-validated end to end … kind" claim at load is not implemented (kind legality is checked at install only, and the on-disk manifest is not digest-covered), and the same-version idempotence guard compares only the module bytes, so the manifest — the thing carrying permissions, publisher and provenance — can be silently replaced on a "same bytes" reinstall.

## 2. What actually works (verified)
— Nothing verified by execution. The in-tree integration suite `tests/plugin_install_lifecycle.rs` exercises install/refuse/idempotence/coexistence/tamper-refusal with exact-variant asserts (would fail under mutation of the checks they target — e.g. removing the `consent.covers` call at install.rs:208 fails `plugin_no_auto_exec` (b)); but `audit/runs/baseline/test-workspace.log.txt` and `test-workspace-allfeatures.log.txt` contain **zero** `Running` lines for any `plugin-host-wasm` test binary (both runs errored out at earlier crates: `error: test failed, to rerun pass -p desktop --test zero_alloc_transport` / `-p first-vertical --test evidence`), so these tests have NO baseline evidence. README's pasted `cargo test -p plugin-host-wasm … 5 passed` output is a doc claim, not citable run evidence. Compilation is proven: `audit/runs/baseline/build-release-workspace.log.txt:231` "Compiling plugin-host-wasm v0.1.0".

## 3. Findings

### F-001 [S2]
Where: crates/plugin-host-wasm/src/install.rs:319-323 and 377-381
Code:
```rust
    /// Load an extension for spawning: the highest installed version,
    /// re-validated end to end (manifest, origin, kind, api window,
    /// **digest** — a tampered on-disk copy refuses to load, doc 06
```
and
```rust
        // Re-validate exactly like install (structural rules come
        // free: the files were extracted from a package once; the
        // manifest, origin, kind, window and hash are re-checked).
```
What is wrong: `load()` (lines 328–415) re-checks manifest schema, origin, api window, provenance existence, module digest and signature-scheme agreement — it never calls `api::kind_extension_legal` (the only caller in the crate is install.rs:479, verified by Grep `kind_extension_legal`). The doc comments claim twice that kind is re-validated at load.
Why it matters: the module digest covers only `plugin.wasm`; `manifest.toml` on disk is unprotected (and self-attested anyway). A manifest edited after install — corruption or a same-wasm repackaged file — can change `kind` and still load. Kind picks the sandbox ceiling class (`ResourceLimits::for_kind`, limits.rs:102-108: ui-panel/audio-filter/video-filter 64 MiB vs 32 MiB) and the v1 kind fence ("byte-source, codec and output kinds stay builtin", error.rs:267-272) is claimed as a load-time property it does not have. This is drift between a documented security invariant and the code, in the exact function whose doc string sells the invariant.
What correct looks like: `if !api::kind_extension_legal(manifest.kind) { return Err(HostError::KindRefused { .. }) }` inside `load()`, or delete "kind" from both comments.
Confidence: HIGH — Verification: read + Grep caller map (kind_extension_legal: 1 call site, install.rs:479).

### F-002 [S2]
Where: crates/plugin-host-wasm/src/install.rs:213-233
Code:
```rust
        if dir.exists() {
            // Same version already installed: idempotent only when the
            // bytes are identical; a silent overwrite would break the
            // provenance story (the recorded digest belongs to what
            // the user consented to).
            let existing = std::fs::read(dir.join(ENTRY_MODULE)).map_err(HostError::from)?;
            if existing != package.module_wasm {
```
What is wrong: the idempotence guard compares ONLY the module bytes; when they match, lines 230–241 unconditionally overwrite `manifest.toml` and the `signature` file. The manifest carries `[permissions]`, `[provenance]` (publisher, source, license) and capabilities — exactly the facts the comment says must not silently change.
Why it matters: a package with identical `plugin.wasm` but a different manifest (extra `fs-sandboxed-read`/`http-fetch` permissions, different publisher) installs over an existing same-version extension without the "remove it first" refusal. The consent check at :208 compares against the NEW package's facts, so a UI deriving consent from `inspect()` re-shows them — but the store's own written invariant ("idempotent only when the bytes are identical") is enforced for the half that matters least: the module is digest-locked anyway, the manifest is not. Permission-creep pinned to an unchanged version number is the realistic abuse.
What correct looks like: compare the on-disk `manifest.toml` (and `signature`) too, and refuse on any difference with the same `HostError::Layout` "remove it first" error.
Confidence: HIGH on the code fact (read), MEDIUM on exploitability (depends on embedding UI behavior — no embedding app exists).

### F-003 [S3]
Where: crates/plugin-host-wasm/src/install.rs:259-267
Code:
```rust
    pub fn remove(
        &self,
        id: &str,
        version: Option<&str>,
        _consent: &InstallConsent,
    ) -> Result<Removed, HostError> {
        // _consent: the parameter is mandatory so no call site can be
        // written without deciding the consent story; the current
        // body only needs it to exist (doc 06 §5.7: user-initiated).
```
What is wrong: `remove()` ignores the consent value entirely — any `InstallConsent`, for any publisher/permission set, unlocks deletion of any installed extension. The module doc (install.rs:17-26) sells "Install and remove take an `InstallConsent`", and error.rs:287-291's `ConsentRequired` text says "install/remove requires explicit user consent".
Why it matters: the type signature promises a consent gate that does not exist; the error variant named for it can never be returned by `remove`. Blast radius is limited (removal only deletes store-owned directories, and no production caller exists), so S3 not S2.
What correct looks like: verify `_consent` names the extension being removed (id at minimum), or rename the parameter and drop the consent language from the docs.
Confidence: HIGH — Verification: read; grep confirms no other remove implementation.

### F-004 [S3]
Where: crates/plugin-host-wasm/src/install.rs:368-376 and 394-408
Code:
```rust
        let signature_block = std::fs::read_to_string(highest.path.join(ENTRY_SIGNATURE))
            .ok()
            .map(|t| crate::package::SignatureBlock::parse(&t))
            .transpose()
            .map_err(HostError::from)?;
```
and
```rust
        if let Some(declared) = provenance.signature.as_ref() {
            let recorded = signature_block.as_ref().map(|b| b.suite.as_str());
            if recorded != Some(declared.scheme.as_str()) {
```
What is wrong: install-time validation (`validate_package`, :511-556) refuses BOTH mismatch directions and checks suite AND key (tests k1–k4). Load-time validation checks only the declared→recorded direction, only the suite (not the key), and a `signature` file that exists but cannot be read (`.ok()`) is silently downgraded to "no block" → `SignatureStatus::Unsigned`. A manifest that declares no signature but ships with a stray signature file loads with an `Unverified` posture install would have refused (Some-block/None-declared).
Why it matters: install and load disagree on what a valid installed layout is; the loaded `LoadedExtension.signature` can misreport the posture. In v1 nothing is actually verified, so the security impact is nil today — this is validation drift, S3.
What correct looks like: extract the shared signature cross-check into one function used by both `validate_package` and `load`, and treat an unreadable-but-present signature file as a `Layout` error.
Confidence: HIGH — Verification: read of both paths + package.rs `SignatureBlock::parse`/`status`.

### F-005 [S3]
Where: crates/plugin-host-wasm/src/install.rs:596-601
Code:
```rust
/// The permissions a manifest asks for (install UI helper; the
/// [`Grants::from_permissions`] derivation is the enforced one).
#[must_use]
pub fn declared_permissions(manifest: &PluginManifest) -> plugin_api::Permissions {
    manifest.permissions.clone()
}
```
What is wrong: dead code. Grep across the repository finds zero callers (`declared_permissions` appears only at its definition). It is also a trivial clone wrapper whose doc immediately disclaims it ("the `Grants::from_permissions` derivation is the enforced one").
Why it matters: unused public API on a crate nothing depends on; a "helper" for a UI that does not exist.
What correct looks like: delete it, or wire it into `InstallPreview` so the consent dialog data is derived in one place.
Confidence: HIGH — Verification: Grep `declared_permissions` over repo → 1 hit (definition).

### F-006 [S4]
Where: crates/plugin-host-wasm/src/install.rs:213-233
Code:
```rust
        if dir.exists() {
            …
        } else {
            std::fs::create_dir_all(&dir)
                .map_err(|e| HostError::Io(format!("cannot create {}: {e}", dir.display())))?;
        }
        std::fs::write(dir.join(ENTRY_MANIFEST), &package.manifest_toml)
```
What is wrong: check-then-write with no locking; two concurrent `install()` calls for the same id+version can interleave file writes and leave a torn directory (manifest from A, module from B).
Why it matters: limited blast radius — `load()`'s digest re-verification refuses a torn state with a typed error, so the failure mode is a stuck extension requiring manual removal, not a security hole.
What correct looks like: write to a temp dir + atomic rename, or a lock file under the version dir.
Confidence: HIGH on the race existing (read); LOW likelihood in practice (single-user store, no production callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `InstallConsent::confirm` | 66-76 | REAL | — | Plain constructor; `permissions: &[&'static str]` order-sensitive vs `granted_names()` (fs,http,ui,devices order) — a UI listing permissions in another order gets refused (fails safe). |
| `InstallConsent::covers` | 79-83 | REAL | — | Exact match of publisher/permissions/signature; tested (consent_must_match_exactly). |
| `ExtensionStore::open` | 147-156 | REAL | — | create_dir_all + typed Io error. |
| `ExtensionStore::root` | 159-161 | REAL | — | Trivial accessor, correct. |
| `ExtensionStore::inspect` | 172-179 | REAL | — | Pure validation, no writes; provenance of "installs nothing" rests on `validate_package` not touching disk (verified: it only reads `pkg`). |
| `ExtensionStore::install` | 188-249 | REAL (defects) | S2/S4 | Consent exact-match enforced (:208); idempotence guard module-bytes-only (F-002); TOCTOU (F-006); never executes wasm (no wasmi symbol referenced). |
| `ExtensionStore::remove` | 259-278 | REAL (defect) | S3 | `_consent` ignored (F-003); `remove_dir_all` scoped under store root via `version_dir`/`extension_dir` + `safe_component`. |
| `ExtensionStore::installed` | 284-317 | REAL | — | NotFound→empty is correct; unreadable dir→typed Io; non-dir entries skipped. |
| `ExtensionStore::load` | 328-415 | REAL (defects) | S2/S3 | Highest-version selection via `Semver::precedence` correct (parses, skips noise); kind NOT re-checked (F-001); signature cross-check one-directional + `.ok()` swallow (F-004). |
| `ExtensionStore::extension_dir` | 417-420 | REAL | — | `safe_component` gate. |
| `ExtensionStore::version_dir` | 422-426 | REAL | — | Double `safe_component` gate; ids also grammar-locked by plugin-api (`^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*){1,2}$`, plugin-api/src/manifest.rs:87). |
| `validate_package` (free fn) | 471-562 | REAL | — | Six-step chain implemented as documented; every refusal typed; matches tests (a)–(k4). |
| `parse_extension_manifest` (free fn) | 566-573 | REAL | — | Origin double-check (also inside validate_package :475) — harmless duplication. |
| `From<ManifestError> for PackageError` | 575-579 | REAL | — | Stringifies; structure lost (S4-grade, not counted). |
| `safe_component` (free fn) | 584-594 | REAL | — | Empty/`.`/`..`/`/`/`\`/NUL; tested. Windows reserved names (CON/NUL) not blocked, but plugin-api id grammar makes them unreachable for `id`; `version` is a Semver string. |
| `declared_permissions` (free fn) | 598-601 | DEAD | S3 | F-005. |
| test `safe_component_rejects_traversal` | 607-614 | REAL | — | Would fail if the filter list were weakened. |
| test `consent_must_match_exactly` | 616-636 | REAL | — | Covers wrong publisher/permissions/posture; would fail if `covers` were loosened. |

## 5. Test honesty (integration suite covering this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `plugin_malicious_manifest` (lifecycle.rs:56) | YES | Exact-variant asserts (a)–(k4) on 14 refusal paths; weakening any check flips a `matches!` to false. |
| `plugin_no_auto_exec` (lifecycle.rs:297) | YES | Trapping start-section module: if install/load ever instantiated, the test errors; consent mismatch assert pins `ConsentRequired`. |
| `plugin_install_remove` (lifecycle.rs:347) | YES | Idempotence, version coexistence, fallback, tamper refusal (`InstalledHashMismatch`), NotInstalled — each pins an exact variant. |
| `fixture_module_executes` (lifecycle.rs:472) | YES | Proves fixtures are runnable wasm (invoke returns 42). |
| **Baseline evidence for any of the above** | NO RUN | Neither baseline workspace test log reaches plugin-host-wasm (both abort earlier). Counts as UNVERIFIED execution. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "WASM sandboxed extensions … status all platforms not_started, evidence ''" | docs/feature-matrix.toml:2321-2331 (plugin-wasm-extensions) | TRUE (honest) | Nothing ships/wires the host (caller grep); consistent with not_started. |
| "Addon discovery and storage (extension store) … not_started" | docs/feature-matrix.toml:2334-2344 (plugin-addon-store) | TRUE (honest) | `ExtensionStore` has no production caller. |
| "installs it into a real `ExtensionStore` … spawns it under the production `ResourceLimits::for_kind` ceilings" | plugins/extensions/README.md:138-146 | REAL-UNPROVEN | Test code exists (plugin_examples_conformance.rs) but no baseline run output covers it; README's pasted output is unverifiable doc text. |
| "re-validated end to end (manifest, origin, kind, api window, digest)" (load) | install.rs:319-321 doc comment | FALSE | F-001: kind never re-checked at load. |
| "no code path exists that installs without a consent value naming the package" | install.rs:22-24 | TRUE for install | `consent.covers` exact-match at :208; note remove is NOT covered (F-003). |
| "tested" (double-opening never installs; trapping start) | install.rs:26, 34 | REAL-UNPROVEN | Tests exist (plugin_no_auto_exec) but were never run in any citable baseline (see §2). |

## 7. Dead / duplicated / unreachable code
- `declared_permissions` — dead (F-005).
- The whole `ExtensionStore` surface is production-unreachable (see header): only tests call it.
- Origin check duplicated: `parse_extension_manifest` (:569) and `validate_package` (:475) both refuse non-extension origin — harmless belt-and-braces.
- `HostError::ConsentRequired` unreachable from `remove` (F-003).

## 8. Dependencies & flags
- Depends only on plugin-api (manifest/semver), crate::api/package/error — all in-crate or plugin-api; no feature flags gate this file. Crate pins `wasmi 2.0.0` with `default-features=false, features=["std","validate","auto-dispatch"]` (Cargo.toml:57) — this file never touches wasmi (correct: install must not execute).
- `[lints] workspace = true`; no `unsafe`, no `unwrap/expect` outside `#[cfg(test)]` (the two tests use `assert!` only). Consistent with the crate's no-unsafe claim.

## 9. Open questions for the next agent
- Does `WasmInstance::spawn` (instance.rs) re-derive limits from `manifest.kind`, compounding F-001? (Audited by me in instance.rs — see that audit.)
- `supervisor.rs` (A08's file): does any code path ever construct `HostError::HostCapacity` / enforce `HOST_MAX_ACTIVE_INSTANCES`? (Grep says no — recorded in my limits.rs audit.)
- Orchestrator: REQUEST-ORCH-RUN `cargo test -p plugin-host-wasm --locked` — the crate's 5 integration suites have zero run evidence in `audit/runs/baseline/`.
