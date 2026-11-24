# AUDIT: crates/plugin-host-wasm/src/package.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 525 / 6 fns + 5 tests |
| File verdict | REAL (executed) — UNREACHABLE from shipped entry points (dev-tool lane only) |
| Reachable from shipped entry point? | NO — via `lumenext` bin (src/bin/lumenext.rs:18) + install.rs + tests only; no app depends on the crate (see imports.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p plugin-host-wasm --lib package` → 5 passed, exit 0 |

## 1. One-paragraph verdict
A careful, std-only ZIP reader with real bounds discipline: 64 MiB package cap checked before parsing, 1024-entry cap, stored-method-only, CRC-32 verified per entry, wasm-magic fence, and every slice access guarded. The signature block is parsed and honestly recorded as Unverified — never trusted — matching its own documentation. Defects are small: `stamp_manifest_hash` matches any key *starting* with "hash", duplicate entry names resolve first-match silently, and the signature verifier gap means "signed" packages carry zero cryptographic meaning today.

## 2. What actually works (verified)
— verified by `cargo test -p plugin-host-wasm --lib package` → 5 passed, exit 0 (signature parse/classify incl. 5 refusal shapes; `garbage_is_not_a_zip`; `module_digest` equals the known SHA-256 empty-input constant `e3b0c442…b855`, proving the in-crate sha256 against an external reference value; stamp round-trip + tamper/missing/duplicate refusal).
— verified by earlier run `cargo test -p plugin-host-wasm --test plugin_examples_conformance` → `lumenext_cli_round_trip_packages_and_inspects_both_examples ... ok` (full package→inspect round trip through this reader).
— Baseline: cargo-test-workspace.txt:4897 (`Running unittests src/lib.rs (…plugin_host_wasm…)`), :5021 exit_code=0.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/package.rs:364-366
Code:
```
for line in manifest.lines() {
    let trimmed = line.trim_start();
    if trimmed.starts_with("hash") && trimmed.contains('=') && trimmed.contains("\"sha256:") {
```
What is wrong: the provenance-stamp matcher matches any key that *starts with* "hash" — `hash_algo = "sha256:…"` or `hashmap = "sha256:…"` is silently rewritten to `hash = "<digest>"`, corrupting a different manifest key. Also drops the line's original indentation (pushes `expected` unindented). Why it matters: `lumenext package` (card 7.G1) runs this on developer manifests; a colliding key is silently destroyed. What correct looks like: `trimmed.split_once('=')` and compare the key trimmed == "hash". Confidence: HIGH — Verification: read; mutation reasoning (a `hash_algo = "sha256:x"` line is replaced — not run).

F-002 [S4]
Where: crates/plugin-host-wasm/src/package.rs:286-292
Code:
```
let entry =
    entries
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| PackageError::MissingEntry {
```
What is wrong: duplicate entry names (`manifest.toml` twice) are not rejected; `.find` silently takes the first central-directory hit, while common unzip tooling takes the last — a crafted package can present different content to different tools. Mitigated by the CRC + later manifest validation + provenance-hash equality in install. Confidence: MEDIUM — Verification: read.

F-003 [S4]
Where: crates/plugin-host-wasm/src/package.rs:22-29 (doc) + 155-165 (`status()`)
Code:
```
//! v1 parses and records the block but does **not** verify
//! cryptographic signatures: a real verifier needs a crypto
//! dependency (ed25519) that only a Lead-approved ADR may add.
```
What is wrong: nothing dishonest — the gap is real and loudly documented; recording it because "signature present" still displays as `Unverified { suite: "ed25519-detached-v1" }` in `lumenext inspect` output (bin/lumenext.rs:205-209), which a user can misread as "checked, ed25519". The only integrity actually enforced is the in-crate sha256 provenance equality (install) + per-entry CRC. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/plugin-host-wasm/src/package.rs:141-145
Code:
```
if !value.bytes().all(|b| b.is_ascii_hexdigit()) {
    return Err(PackageError::SignatureUnparsable {
        detail: "`value:` is not hex".to_string(),
```
What is wrong: `value`/`key` lengths are unbounded (up to the 64 MiB package cap) — no max-signature-length check; and empty `key:`/`value:` strings pass (hex check passes vacuously on empty). Bounded overall by PACKAGE_CAP_BYTES, so no DoS; quality gap only. Confidence: HIGH — Verification: read.

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SignatureBlock::parse | 99-152 | REAL | S4 | strict magic/fields; empty-value hex pass (F-004); tests cover 5 refusals |
| SignatureBlock::status | 155-165 | REAL | — | known-suite → Unverified (honest); test-verified |
| read_package | 188-218 | REAL | — | cap-first; wasm magic fence; signature optional |
| read_central_directory | 222-282 | REAL | S4 | backward EOCD scan ≤66k; entry cap; every read bounds-checked; duplicate names (F-002) |
| extract | 285-335 | REAL | S4 | stored-only, size caps, local-header re-read, CRC verify; first-match duplicates |
| module_digest | 338-340 | REAL | — | anchored to the SHA-256 empty-input reference constant by test |
| stamp_manifest_hash | 359-405 | REAL | S3 | placeholder/tamper/dup refusal tested; prefix-match key bug (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| signature_block_parses_and_classifies | YES | 5 malformed inputs asserted to fail (ran: ok) |
| garbage_is_not_a_zip | YES | NotZip on non-ZIP bytes (ran: ok) |
| module_digest_is_the_provenance_form | YES | equality with the external SHA-256 reference constant — a wrong sha256.rs fails this (ran: ok) |
| stamp_writes_the_real_digest_over_the_placeholder | YES | exact-digest + no-ellipsis + round-trip assertions (ran: ok) |
| stamp_refuses_tampered_and_missing_hash_lines | YES | three refusal shapes asserted (ran: ok) — but no test for a colliding `hash*` key (F-001 slips through) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 reads stored (method 0) entries and integrity-checks every entry's CRC-32; compressed entries are refused" | package.rs:5-9 | REAL | extract:293-297 method!=0 → CompressedEntry; :328-333 CRC compare; conformance round-trip test |
| "signature present → trust recorded as Unverified … install demands explicit user consent" | package.rs:25-29 | REAL (recorded) / UNVERIFIED (consent flow) | status():155-165 verified; the consent demand lives in install.rs — not audited by me (next agent) |
| "64 MiB size cap" | package.rs:5 | REAL | PACKAGE_CAP_BYTES:36 checked at :190 |

## 7. Dead / duplicated / unreachable code
None dead in-file. Whole crate unreachable from shipped builds (see imports.rs F-001); this file at least ships inside the `lumenext` dev CLI.

## 8. Dependencies & flags
std-only (crc32/sha256 in-crate). No features. No unsafe.

## 9. Open questions for the next agent
- install.rs: is the "explicit user consent" gate real (a CLI prompt?) or a passed-in flag that dev tooling auto-accepts?
- Who calls `read_package` with attacker-controlled bytes in any shipped lane? (None found — crate unembedded.)
