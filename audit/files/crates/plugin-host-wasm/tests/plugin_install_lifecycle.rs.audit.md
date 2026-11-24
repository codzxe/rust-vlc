# AUDIT: crates/plugin-host-wasm/tests/plugin_install_lifecycle.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 480 / 5 helpers + 4 tests |
| File verdict | REAL (adversarial integration suite, executed) |
| Reachable from shipped entry point? | N/A (test file) — runs in the CI test lane; plugin-host-wasm itself is NOT in the release lane (no workspace dependents; wasm-extensions=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "plugin_install_lifecycle|plugin_malicious_manifest|…" audit/runs/baseline/cargo-test-workspace.txt` -> 4/4 ok (:4990-4996); `rg -n "pub fn inspect" -A 25 src/install.rs` (validation chain shared with install); `rg "InstallConsent|consent" tests/*.rs` (consent coverage survey); `sed -n '110,130p' tests/plugin_examples_conformance.rs` |

## 1. One-paragraph verdict
A genuine, executed, hostile install-lifecycle suite: 14 refusal shapes (ELF native payload, origin forgery, lying/placeholder digest, api-window, refused kind, CRC corruption, deflate entry, both missing entries, non-ZIP, 64 MiB+1 oversize, four signature-bookkeeping disagreements), the no-auto-exec proof via a start-section-trapping guest, and the full user lifecycle (idempotent reinstall, different-bytes-same-version refusal, version coexistence with highest-wins load, fallback after remove, on-load tamper detection, typed NotInstalled, honest Unverified signature posture with consent-posture matching). All four tests pass in the orchestrator baseline. The fixtures are real (zip writer with real CRC, real module_digest, real wasmi spawn; `fixture_module_executes` proves the module runs). Two coverage gaps: the permissions leg of the consent gate has no negative test anywhere, and `inspect`'s validation depth is asserted only positively.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt:4990-4996: `Running tests/plugin_install_lifecycle.rs` -> `fixture_module_executes ... ok`, `plugin_install_remove ... ok`, `plugin_no_auto_exec ... ok`, `plugin_malicious_manifest ... ok` (4/4).
— Helpers verified real: tests/common/mod.rs:427 `zip_stored` (real ZIP: local headers + central dir + EOCD, CRC via the crate's `crc32`), :507 `manifest_toml` (digest computed by the real `module_digest`), :538 `package`, :570 `spawn` (real wasmi engine). The fixture module really executes: `assert_eq!(ok(instance.invoke("core", &[])), vec![42])` (L479).
— The no-auto-exec mechanism is structurally sound: the trapping start section would fault ANY instantiation; install/load return Ok on the same bytes, only spawn faults (L333-339) — a real differential proof, executed in baseline.

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/tests/plugin_install_lifecycle.rs:39-41 and src/install.rs:79-83
Code:
```
fn unsigned_consent() -> InstallConsent {
    InstallConsent::confirm("Acme Tools", &[], SignatureStatus::Unsigned)
}
```
What is wrong: The consent gate has three legs — publisher, permissions, signature (install.rs:79-83 `covers()`). This suite exercises negative cases for publisher (L313-317 "Evil Corp") and signature posture (L443-446), but every fixture manifest here uses EMPTY permissions (`manifest_toml(&wasm, "")`) and every consent passes `&[]`, so the `self.permissions == grants.granted_names()` leg has no negative test. The only other installer test (plugin_examples_conformance.rs:121-126) passes the exact manifest permissions — the positive path. Mutation reasoning: delete the permissions comparison from `covers()` and NO test in the workspace fails. Why it matters: the permission leg is the user-consent anti-escalation guarantee (the user must have seen the exact permission list); a regression that stops comparing permissions — or compares the wrong direction — ships silently. What correct looks like: one test installing a package whose manifest requests `fs-sandboxed-read` with a consent granting `&[]`, asserting `HostError::ConsentRequired`. Confidence: HIGH — Verification: read + rg (consent survey over tests/*.rs).

F-002 [S4]
Where: crates/plugin-host-wasm/tests/plugin_install_lifecycle.rs:303-309
Code:
```
// (a) Inspecting (the "double-open the downloaded file" path)
// validates everything and installs NOTHING: no registry change,
// no execution, no files under the extensions root.
let preview = ok(store.inspect(&pkg));
```
What is wrong: The comment claims inspect "validates everything", but the assertions only check the happy path (id, empty installed list, no dir). No test anywhere inspects a MALICIOUS package and asserts the typed refusal (rg over tests: only two positive inspect call sites — this one and the CLI round-trip asserting "VALID"/"sha256 matches"). The claim happens to be true in code — `inspect` calls the same `validate_package` as `install` (install.rs:172-179, verified by read) — but it is pinned by nothing: a refactor that gives inspect a cheaper, non-validating path breaks the "double-open the downloaded file" security story with zero test failures. What correct looks like: reuse the (a)-(k) refusal corpus against `store.inspect`. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tiny_wasm` | 21-26 | REAL | — | one export, no imports; executed by `fixture_module_executes` |
| `start_trap_wasm` | 31-36 | REAL | — | start section with `unreachable`; the no-auto-exec probe; traps only on instantiation — proven at spawn (L333-339) |
| `unsigned_consent` | 39-41 | REAL | S4 | F-001: empty-permissions consent everywhere |
| `refused` | 44-49 | REAL | — | unwrap_or_else(unreachable!) — loud on an unexpected Ok |
| `plugin_malicious_manifest` | 56-283 | REAL | — | 14 typed refusals + final "nothing installed" assertion (L279-282); oversize case allocates 64 MiB+1 in memory (test-lane acceptable, baseline passed) |
| `digest_of` | 287-289 | REAL | — | real `module_digest` |
| `plugin_no_auto_exec` | 296-340 | REAL | S4 | F-002: inspect claim positive-only; install/load/spawn differential is a real proof |
| `plugin_install_remove` | 346-464 | REAL | — | full lifecycle incl. tamper-on-load (L397-408) and honest Unverified posture (L454-460) |
| `fixture_module_executes` | 471-480 | REAL | — | guards against inert fixtures; invoke -> [42] |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plugin_malicious_manifest | YES | each (a)-(k) asserts a DISTINCT typed variant (BadMagic/OriginForgery/HashMismatch/ApiVersion/KindRefused/CrcMismatch/CompressedEntry/MissingEntry/NotZip/TooLarge/SignatureUnparsable); deleting a validation step fails its arm; final emptiness check catches partial-install leaks |
| plugin_no_auto_exec | YES | start-section trap makes any premature instantiation fault loudly; wrong-publisher consent refusal asserted; install Ok + load Ok + spawn Err is a 3-way differential |
| plugin_install_remove | YES | byte-level tamper (append 0x00) must be caught by load-time digest re-verification; version fallback and NotInstalled arms assert state transitions |
| fixture_module_executes | YES | real wasmi invoke; returns 42 |
Gap: consent permission leg — F-001 (mutation survives).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every forged/corrupt package dies at a named validation step, never at a panic" | L52-53 | REAL | 14 `matches!` on typed PackageError variants; baseline ok |
| "Inspecting … validates everything and installs NOTHING" | L303-305 | REAL-UNPROVEN (code-verified, test-unpinned) | install.rs:172-179 shares `validate_package` (read); no adversarial inspect test (F-002) |
| "digest re-verified on EVERY load" | L395-396 | REAL | tamper test L397-408 executed in baseline (`plugin_install_remove ... ok`) |
| "v1 never claims `verified`" | L459 | REAL | assert on `SignatureStatus::Unverified { .. }` receipt + loaded copy |

## 7. Dead / duplicated / unreachable code
None. All helpers used. `#![allow(dead_code)]` in common/mod.rs covers shared-but-unused-some-suites helpers (acceptable for a shared fixture module).

## 8. Dependencies & flags
Uses only the crate under test + tests/common. No feature gates. No unsafe. Temp dirs self-clean (Drop removes recursively, errors swallowed — fine for tests).

## 9. Open questions for the next agent
- Where does the UI surface the consent dialog (publisher/permissions/signature) and does it construct `InstallConsent` from what was actually rendered? Out of scope here (plugin-host-wasm is not in the shipped lane; the consent UX is Phase-7 work).
