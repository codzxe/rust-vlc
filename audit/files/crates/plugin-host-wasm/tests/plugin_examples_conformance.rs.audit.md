# AUDIT: crates/plugin-host-wasm/tests/plugin_examples_conformance.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 494 / 5 tests + 4 helpers (repo_root, Example{dir,module,manifest_text,lumenext,manifest}, spawn_installed, guest_u32) |
| File verdict | REAL (executed: 5/5 passed) — test-lane only, proves nothing about shipped builds |
| Reachable from shipped entry point? | N/A (test file). The machinery it exercises (ExtensionStore, WasmInstance, wasmi) is compiled into NO shipped app — plugin-host-wasm is a dependency of nothing outside itself. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -q -p plugin-host-wasm --test plugin_examples_conformance` -> "5 passed; 0 failed" exit 0 (0.05s); `rg -l "plugins-examples"` -> only this file (doc path dangling); sed verifications |

## 1. One-paragraph verdict
This is the single most honest test file in the plugin domain: it loads the two committed example extensions through the real wasmi engine, real install/consent/store path, real production `ResourceLimits::for_kind`, real `lumenext package`/`inspect` CLI subprocesses, and asserts capability honesty (the sidecar bytes come back verbatim), typed error paths (`E_PERM` on missing and `../../../etc/passwd` traversal), exact escalation count, and the typed api-version refusal. I ran it: 5/5 pass. The catch is scope: all of this proves the extension system works in the dev/test lane only — no shipped binary ever executes any of it (see manifest.rs audit F-002). One dangling doc reference and two cast/robustness nits.

## 2. What actually works (verified)
— verified by `cargo test -q -p plugin-host-wasm --test plugin_examples_conformance` -> `test result: ok. 5 passed; 0 failed; 0 ignored` (0.05s). Covers: drift gate (regeneration byte-match), metadata conformance (init handshake, honest manifest report `ext.example.metadata`, jailed fs round trip incl. fs_open/fs_read/fs_close trace statuses, verbatim tag bytes, E_PERM×2, Ready-after-errors, escalation_count==2), ui-panel conformance (panel descriptor surface `"side-panel"`, on_command ack log), api-major refusal (HostError::ApiVersion{found:2,min:1,max:1} for both), CLI round trip (package→inspect "VALID"/"sha256 matches"→install receipt id+version).

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/tests/plugin_examples_conformance.rs:435-437
Code:
```
// The developer CLI round trip — the exact commands the walkthrough
// documents (docs/plugins-examples.md), executed for real
```
What is wrong: `docs/plugins-examples.md` does not exist (`rg -l "plugins-examples"` across the repo matches only this file; `ls docs/` has no such file). The real walkthrough is `plugins/extensions/README.md`. Why it matters: readers/agents following the cited path hit a 404; violates the repo's own evidence-discipline (docs must agree with reality). What correct looks like: point at `plugins/extensions/README.md`. Confidence: HIGH — Verification: ran rg + ls.

F-002 [S4]
Where: crates/plugin-host-wasm/tests/plugin_examples_conformance.rs:236-238
Code:
```
        ok(instance.invoke(
            "extract",
            &[i64::from(REQUEST_AT as i32), path.len() as i64]
```
What is wrong: `REQUEST_AT as i32` — u64→i32 narrowing cast that silently wraps past 2^31 (here 8192, harmless). If the guest memory layout ever moves above 2 GiB the test would dispatch a negative pointer without complaint. Same pattern at 278, 290, 301, 365, 382. What correct looks like: `i64::try_from(REQUEST_AT)` or a checked conversion. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/plugin-host-wasm/tests/plugin_examples_conformance.rs:403
Code:
```
        let future = stamped.replace("api_version = 1", "api_version = 2");
```
What is wrong: Blind substring surgery over the whole manifest — if any other field ever contains `api_version = 1` (e.g. a future `[dependencies]` note or comment text) every occurrence is rewritten. Fails loudly today (the stamped manifest parses), but fragile. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| repo_root | 52-56 | REAL | – | canonicalized CARGO_MANIFEST_DIR/../.. |
| Example::dir/module/manifest_text | 64-82 | REAL | – | reads committed artifacts |
| Example::lumenext/manifest | 88-103 | REAL | – | stamps real digest via library fn (same as CLI) |
| spawn_installed | 114-137 | REAL | – | store.install→load→WasmInstance::spawn under `ResourceLimits::for_kind` (production limits, src/limits.rs:102 — `test_tiny` exists and is NOT used here) |
| guest_u32 | 154-157 | REAL | – | LE u32 read |
| committed_example_modules_match_regeneration | 163-182 | REAL | – | subprocess `lumenext-examples --check`, asserts exit + exact output string; passes |
| metadata_provider_example_passes_conformance | 188-314 | REAL | – | ran, passed; asserts verbatim bytes, import trace, 2×E_PERM, Ready, escalation_count==2 |
| ui_panel_example_passes_conformance | 320-387 | REAL | – | ran, passed; descriptor containment asserted honestly (comment admits no JSON parser — S4-free) |
| examples_refuse_future_api_major_with_the_typed_error | 393-432 | REAL | – | ran, passed; typed HostError::ApiVersion window |
| lumenext_cli_round_trip_packages_and_inspects_both_examples | 439-494 | REAL | – | ran, passed; real CLI subprocesses + store install |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_example_modules_match_regeneration | YES | byte-exact drift gate via subprocess exit code + output |
| metadata_provider_example_passes_conformance | YES | exact-length + verbatim-bytes compare; import-trace statuses; exact escalation count 2 (extra or missing events fail) |
| ui_panel_example_passes_conformance | YES | panel count, surface string in descriptor, ack log line exact match |
| examples_refuse_future_api_major_with_the_typed_error | YES | pattern-matches the typed error struct fields (found/min/max) |
| lumenext_cli_round_trip... | YES | asserts CLI exit codes, "VALID", "sha256 matches", receipt id+version |
All 5 executed by me (not just baseline): 5 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the exact commands the walkthrough documents (docs/plugins-examples.md)" | :436 | FALSE (path) | file absent; walkthrough is plugins/extensions/README.md (contains the same commands, verified by read) |
| "spawn it under the production limits" (§7.4) | :112-113, 134 | REAL | `ResourceLimits::for_kind` used; src/limits.rs:102 defines it; test_tiny deliberately not used |
| §7.5 crash injection | doc 06 §7.5 (docs/06-plugin-architecture.md:868) | NOT THIS FILE | covered by tests/plugin_crash_isolation.rs (exists; not in my assignment) |

## 7. Dead / duplicated / unreachable code
None. Guest-layout constants (146-151) duplicate src/bin/lumenext-examples.rs addresses by design — the comment (140-143) declares the mirror and the drift test + conformance asserts would catch divergence.

## 8. Dependencies & flags
Test-only. Uses `CARGO_BIN_EXE_lumenext` / `CARGO_BIN_EXE_lumenext-examples` (auto-built dev bins). Engine: wasmi 2.0.0 (crates/plugin-host-wasm/Cargo.toml:57, default-features=false, features std+validate+auto-dispatch per ADR-002).

## 9. Open questions for the next agent
- The suite proves the host works; the missing link is any shipped app invoking `ExtensionStore`/`WasmInstance`. Whoever audits doc 06 §5 claims of runtime extension loading should treat "loaded at runtime" as test-lane-only until a shipped consumer appears.
- `stamp_manifest_hash` + placeholder-hash acceptance in plugin-api means committed manifests are NOT installable as-is (placeholder `sha256:…(64 hex)`); only the stamped copies install. Consistent here, but worth checking the lumenext bin audit (my file 4).
