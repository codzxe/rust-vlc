# AUDIT: crates/plugin-api/src/manifest.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 1293 / 40 production fns (+8 tests, 1 helper) |
| File verdict | UNREACHABLE from shipped builds (REAL, tested library code) |
| Reachable from shipped entry point? | NO for the manifest model; YES for one constant: `DEFAULT_PRIORITY_BUILTIN` via crates/media-probe/src/signature/mod.rs:28,132-145 (probe registry), media-probe is a dep of lumen-cli (apps/cli/Cargo.toml:56) and web (apps/web/Cargo.toml:63). `PluginManifest::from_toml/validate/to_toml` are called ONLY by crates/plugin-host-wasm (src/install.rs:568, src/imports.rs:312, src/bin/lumenext.rs:142,186) and plugin-api's own tests — plugin-host-wasm is a dependency of NO shipped app. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `cargo test -q -p plugin-api` -> 42+6+2 passed, 0 failed, exit 0; `cargo test -q -p plugin-api manifest` -> 8 passed; sed/rg verifications below |

## 1. One-paragraph verdict
The manifest model is genuine, carefully specified, and its 50 tests pass (I ran them). But it is dead weight in shipped builds: no shipped entry point (lumen-cli, desktop, web) parses or validates a single manifest — the only bytes of this 1293-line file that ship are the constant `100`. The validator also has a real hole: the `for (_name, _value) in [...] {}` loop at L597-607 has an empty body, so misplaced `sample_rates`/`bit_depths` capability keys are never rejected, contradicting the module's own doc claim that "misplaced keys" are caught by `validate`.

## 2. What actually works (verified)
— verified by `cargo test -q -p plugin-api` -> "42 passed / 6 passed (doc06_examples incl. round-trip) / 2 passed", exit 0. Id grammar, prefix/slug rules, priority range/defaults, origin↔prefix, provenance-required, permissions-builtin-forbidden, allowlist⇒http-fetch, unknown-field rejection (incl. typo'd permission) all have failing-capable tests.

## 3. Findings
F-001 [S2]
Where: crates/plugin-api/src/manifest.rs:586-621
Code:
```
        // (membership for the scalar keys is checked below)
        for (_name, _value) in [
            ("max_channels", self.max_channels),
            (
                "sample_rates",
                self.sample_rates.as_deref().map(|v| v.len() as u32),
            ),
```
What is wrong: The loop at 597-607 iterates a 3-tuple array and does NOTHING (`{}` body, both bindings `_`-ignored). It was clearly meant to enforce key-set membership for `max_channels`, `sample_rates`, `bit_depths`. The follow-up loop (608-621) checks only `["max_channels", "latency_class", "hardware", "supports_seek"]`, and `present_string_lists()` (553-578) covers only the `Vec<String>` fields — so `sample_rates` and `bit_depths` are NEVER membership-checked. A `[capabilities]` block for kind `metadata-provider` (or any kind) carrying `sample_rates = [48000]` passes `validate()`. Why it matters: the module doc (L426-428) claims "unknown keys are rejected by `deny_unknown_fields` and misplaced keys by [`validate`]" — false for these two keys; a manifest violating doc 06 §2.2's kind table is accepted. What correct looks like: delete the dead loop; include `sample_rates`/`bit_depths` presence in the 608-621 membership loop. Confidence: HIGH — Verification: read (code-path analysis; no repo test covers it — adding one requires editing source, out of my mandate).

F-002 [S2]
Where: crates/plugin-api/src/manifest.rs:306-310 (callers)
Code:
```
    pub fn from_toml(src: &str) -> Result<PluginManifest, ManifestError> {
        let manifest: PluginManifest = toml::from_str(src)?;
        manifest.validate()?;
        Ok(manifest)
```
What is wrong: Not this function — its reachability. Every production caller of `from_toml`/`validate` lives in crates/plugin-host-wasm (src/install.rs:568, src/imports.rs:312, src/bin/lumenext.rs:142,186), and `rg "plugin_host_wasm::"` shows zero users outside that crate; no `Cargo.toml` of apps/cli, apps/web, apps/desktop lists plugin-host-wasm. The release lane (`cargo build --release --locked -p lumen-cli -p desktop`) never compiles this validation path. Why it matters: doc 06's whole §2 manifest-validation story has no shipped consumer; the two `plugins/extensions/*/manifest.toml` files are parsed only by dev tooling (lumenext) and tests. What correct looks like: wire the extension host into a shipped app or mark doc 06 Phase-2 explicitly. Confidence: HIGH — Verification: read + rg (see log).

F-003 [S3]
Where: crates/plugin-api/src/manifest.rs:597-607
Code:
```
        ] {}
```
What is wrong: Dead code — a loop whose body is `{}` and whose bindings are `_name`/`_value`. Independently of F-001 this is a validator carrying an executing no-op; it also allocates the scrutinee array on every validate call. Why it matters: violates the repo's own lint posture (unused_variables is workspace-deny but `_`-prefixing evades it); misleads readers into thinking membership was checked. What correct looks like: remove it. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/plugin-api/src/manifest.rs:753-773
Code:
```
    /// Grants as a sorted list of permission names (for events and
    /// install UI; doc 06 §5.3 logging).
    pub fn granted(&self) -> Vec<&'static str> {
```
What is wrong: The output is not sorted: order is fs-sandboxed-read, http-fetch, network-host-allowlist, ui-surface, device-query — `device-query` sorts first alphabetically but is pushed last. Doc comment lies about ordering; also `network_host_allowlist.is_some()` reports "network-host-allowlist" granted even when the list is `Some([])`. Why it matters: install-UI ordering claim; minor. What correct looks like: sort or fix the comment. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/plugin-api/src/manifest.rs:159-166
Code:
```
        match self.segments().get(1) {
            // Only meaningful for `ext.` ids; `lumen.` segment 2 is a
            // kind-area, and unprefixed ids have no publisher.
            _ if self.prefix() == Some(EXT_PREFIX) => self.segments().get(1).copied(),
            _ => None,
```
What is wrong: The match scrutinee `self.segments().get(1)` is computed then discarded (both arms ignore it — the guard re-derives everything); `segments()` allocates a Vec twice. Same smell class as F-003. Also L78 typo "contentional" for "contention". Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Serialize/Deserialize PluginKind | 42-60 | REAL | – | string ↔ 16-kind enum; tested |
| PluginId::parse | 102-137 | REAL | – | grammar enforced incl. 64B cap, 2-3 segments, [a-z0-9-] |
| PluginId::as_str/prefix | 140-154 | REAL | – | |
| PluginId::publisher_slug | 159-166 | REAL | S4 | F-005 |
| PluginId::segments | 169-171 | REAL | – | allocates per call |
| Display/Ser/De PluginId | 174-191 | REAL | – | De revalidates via parse |
| SemverField Ser/De/Display | 249-272 | REAL | – | delegates to crate::semver |
| Origin::as_str/Display | 284-298 | REAL | – | kebab-case wire form |
| PluginManifest::from_toml | 306-310 | REAL (UNREACHABLE shipped) | S2 | F-002 |
| PluginManifest::to_toml | 319-324 | REAL (UNREACHABLE shipped) | – | round-trip proven by tests/doc06_examples.rs (ran, pass) |
| PluginManifest::validate | 331-401 | SHALLOW | S2 | F-001 hole; all other cross-field rules verified by tests |
| PluginManifest::effective_priority | 407-415 | REAL | – | tested 100/50 |
| Capabilities::first_listed_key | 505-521 | REAL | – | 16 kinds pinned |
| Capabilities::legal_keys | 524-549 | REAL | – | the table F-001 fails to fully enforce |
| Capabilities::present_string_lists | 553-578 | REAL | – | 16 string keys only |
| Capabilities::validate | 586-715 | SHALLOW | S2/S3 | F-001/F-003; primary-key, dedup, 32B/64-entry, lowercase rules all tested |
| Permissions::is_absent | 749-751 | REAL | – | |
| Permissions::granted | 755-773 | REAL | S4 | F-004 "sorted" false |
| Dependencies::is_absent + wire From/Into | 810-845 | REAL | – | empty table ≡ empty array; tested |
| PackageHash::parse | 893-918 | REAL | – | 64-hex or doc placeholder; `is_full` distinguishes |
| PackageHash::is_full/as_str/Display/Ser/De | 920-949 | REAL | – | |
| Pubkey::parse/as_str/Ser/De/Display | 965-1003 | REAL | – | non-empty check only (grammar deferred to ADR — documented) |
| From\<toml::de::Error\> | 1053-1057 | REAL | – | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| id_grammar (1072) | YES | 5 good + 10 bad ids incl. 64B boundary |
| id_prefixes_and_slugs (1103) | YES | prefix/slug mapping |
| hash_accepts_full_and_doc_placeholder (1111) | YES | 5 bad forms rejected |
| empty_dependencies_table_and_array_agree (1136) | YES | wire-shape equivalence |
| capabilities_primary_key_enforced_per_kind (1164) | YES | 8 asserts incl. dup/upper/oversize/64-entry boundary — but does NOT cover sample_rates/bit_depths placement (F-001 blind spot) |
| cross_field_rules (1193) | YES | 4 cross-field rejections |
| priority_defaults_and_range (1237) | YES | 100/50/1001 |
| unknown_fields_rejected (1273) | YES | incl. typo'd permission `htt-fetch` |
| (external) tests/doc06_examples.rs | YES | ran: 6 passed; proves L17-20 round-trip claim |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Round-trip is value-exact by construction (see the module tests…)" | manifest.rs:17-20 | REAL (pointer imprecise — round-trip tests live in tests/doc06_examples.rs, not "the module tests") | cargo test -p plugin-api -> 6 doc06 tests pass |
| "unknown keys are rejected by deny_unknown_fields and misplaced keys by [validate]" | manifest.rs:426-428 | FALSE for sample_rates/bit_depths | F-001 code path |
| "Grants as a sorted list of permission names" | manifest.rs:753-754 | FALSE (order fixed, unsorted; device-query last) | F-004 read |
| "verified at install and every load" (provenance.hash) | manifest.rs:861-862 | UNVERIFIED here | enforcement lives in plugin-host-wasm (install.rs), itself unshipped — see F-002 |

## 7. Dead / duplicated / unreachable code
- L597-607 dead empty-body loop (F-003).
- Whole manifest parse/validate path unreachable from shipped builds (F-002); only `DEFAULT_PRIORITY_BUILTIN` (L76) ships, via media-probe's probe-signature registry (crates/media-probe/src/signature/mod.rs:132-145).

## 8. Dependencies & flags
Deps: media-types, codec-api (path), serde, toml 1.1, thiserror 2.0; dev: testkit. No feature flags in this crate (crates/plugin-api/Cargo.toml). No unsafe. workspace lints deny unwrap_used — tests use `unreachable!` instead (acceptable).

## 9. Open questions for the next agent
- Does any doc/matrix row claim "extensions load at runtime"? My evidence says the runtime host (plugin-host-wasm) is compiled into NO shipped app. Whoever audits doc 06 §5/§7 or the extensions settings UI should reconcile.
- plugin-host-wasm/src/imports.rs:312 re-parses manifests inside a WASM import — check the error path there (my file 2 audit touches its conformance suite only).
