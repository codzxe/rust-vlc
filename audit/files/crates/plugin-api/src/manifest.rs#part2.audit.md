# AUDIT: crates/plugin-api/src/manifest.rs#part2
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 586-1293 (nominal 647-1293 per wave-00 assignments.tsv `646/647` split; extended BACK to 586 = start of `Capabilities::validate`, the function enclosing nominal line 647, so the whole function is audited once — part1 (W12-A08, nominal 1-646) should stop at 585 = end of `present_string_lists`. Includes the `#[cfg(test)] mod tests` at 1059-1293.) |
| Lines / functions | 708 / 41 items (16 production fns/impls + 11 type/struct defs + 1 enum + 2 From impls + 1 test helper + 9 tests + serde impls) |
| File verdict | UNREACHABLE (real, tested validation code; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO. Production callers of the manifest model exist ONLY inside plugin-host-wasm: `crates/plugin-host-wasm/src/imports.rs:312` (`PluginManifest::from_toml` in the `lumen:v1` manifest import), `install.rs:568` (`parse_extension_manifest`), and dev bins `lumenext.rs:142,186`. No crate depends on plugin-host-wasm (`rg -l "plugin-host-wasm" --glob Cargo.toml` → only its own manifest + a scope-fence COMMENT in `crates/plugin-api/Cargo.toml:17`); nothing shipped depends on it. The shipped `lumen` CLI depends on plugin-api but uses only the demux traits — `rg "Manifest\|PluginKind\|PackageHash\|Pubkey" apps/cli/src apps/web/src` → 0 hits; `rg "plugin_api::" apps/cli/src apps/web/src` → only `Demuxer/SourceStream/DemuxEvent/SeekSupport/SeekOrigin`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `rg -n "PluginManifest::from_toml" crates apps` (production callers: imports.rs:312, install.rs:568, lumenext.rs:142/186 only); `rg -l "plugin-host-wasm" --glob Cargo.toml` (no dependents); `rg -n "plugin_api::" apps/cli/src apps/web/src` (demux traits only); `rg -n "Manifest\|PluginKind\|PackageHash\|Pubkey" apps/cli/src apps/web/src` (0); `rg -n "ManifestError::Version\|::Version\("` (0 constructors — F-003); `rg -n "\.granted\(\)\|is_full\(\)"` (lumenext.rs:201, install.rs:494, tests); `rg -n "digest\|sha256\|verify" plugin-host-wasm/src/{instance,supervisor,state}.rs` (0 — no load-time hash check); `rg -n "config_schema\|Dependencies" plugin-host-wasm/src` (no consumer — F-002); `rg -n "manifest\|plugin" audit/runs/baseline/{test-workspace,orchrun-test-workspace-nofailfast}.log.txt` (plugin tests absent from headline run #3 — fail-fast stopped at `desktop`; green only in orchrun repair run :4427-4473, doc06_examples :4475-4486); `sed -n '596,621p;753,755p;1016,1021p'` (quote verification); Read tool for quotes containing `…` (sed eats multibyte ellipsis in this sandbox — known sandbox quirk, W07-A01 precedent). |

## 1. One-paragraph verdict
The second half of the manifest model is genuine, careful validation code — the §2.2 cross-field rules, the permission/provenance/dependency wire types, and the sha256 `PackageHash` newtype are all real and executed green by 8 in-file tests plus the 4-fixture doc 06 round-trip suite (orchrun no-fail-fast run :4447-4455, :4475-4486). But it is theater for a shipped product: nothing in the shipped `lumen` CLI (or the unbuilt tauri shell) ever parses a manifest — the only production consumers live in plugin-host-wasm, a crate nothing depends on. And the validator has a real hole: the kind-key-set rule (§2.2 "no key outside the kind's row") is enforced for 19 of 21 keys, but `sample_rates` and `bit_depths` escape it because their membership check is an EMPTY-BODIED `for` loop (597-607) — the loop computes lengths, discards them, and checks nothing. A `demuxer` manifest carrying `sample_rates` validates clean. Plus two dead/paper artifacts: `ManifestError::Version` can never be constructed, and the `[dependencies]` "not loaded without its deps" promise has zero enforcement anywhere in the tree.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4427-4473` → `plugin_api` 42 tests passed including all 8 `manifest::tests::*` (`id_grammar`, `cross_field_rules`, `unknown_fields_rejected`, …); `:4475-4486` → `doc06_examples` 6 passed including `all_four_reparse_from_their_own_output` (the parse→serialize→parse→serialize stability proof for the four doc 06 §2.3 fixtures).
— Verified by reading `crates/plugin-host-wasm/src/install.rs:494-508`: the placeholder-hash escape hatch in `PackageHash::parse` (908-914) IS gated — install requires `provenance.hash.is_full()` and exact string match against `module_digest(&package.module_wasm)`, so a doc-placeholder hash cannot pass install.
— Verified by tracing `Capabilities::validate`'s list rules (644-713): non-empty, per-entry non-empty, duplicate rejection via HashSet insert, 32-byte entry cap, 64-total cap, lowercase charset — each rule has a matching assertion in `capabilities_primary_key_enforced_per_kind` (1176-1189).
— Caveat on evidence provenance: the HEADLINE baseline run #3 (`test-workspace.log.txt`, 1681 passed) contains ZERO plugin-api tests — `cargo test` fail-fasted at `desktop`'s `zero_alloc_transport` failure before reaching `plugin-api` alphabetically. Plugin-api/plugin-host-wasm greenness exists only in the orchestrator's `--no-fail-fast` repair run. INDEX.md #3's totals must not be read as covering this crate.

## 3. Findings

### F-001 [S2]
Where: crates/plugin-api/src/manifest.rs:597-607 (with 608-621, 688-713)
Code:
```
        // (membership for the scalar keys is checked below)
        for (_name, _value) in [
            ("max_channels", self.max_channels),
            (
                "sample_rates",
                self.sample_rates.as_deref().map(|v| v.len() as u32),
            ),
            (
                "bit_depths",
                self.bit_depths.as_deref().map(|v| v.len() as u32),
            ),
        ] {}
```
What is wrong: This `for` loop has an EMPTY body — it iterates a constructed array of `(name, Option<u32>)` pairs, binds both to underscore names, and does nothing. It is the only place `sample_rates`/`bit_depths` appear in the kind-membership logic; the "below" loop (608-621) checks only `["max_channels", "latency_class", "hardware", "supports_seek"]`, and the numeric-list loop (688-713) checks only non-empty/dup/count. Result: a manifest with `kind = "demuxer"` and `[capabilities] formats = ["x"], sample_rates = [44100]` passes `validate()` even though `legal_keys(Demuxer)` = `["formats", "extensions", "mime_types"]` — I traced every branch: `present_string_lists` never yields `sample_rates` (numeric), the dead loop does nothing, no other check names it. The struct doc at 426-428 claims "misplaced keys [are rejected] by [`validate`]" and doc 06 §2.2:211 lists `sample_rates`/`bit_depths` as decoder/encoder-only keys. Clippy `-D warnings` passes this (empty loop bodies are not a default lint; underscore bindings suppress unused-variable) — baseline clippy clean does not cover it.
Why it matters: `validate` is the single choke point every manifest passes through (install.rs:568, imports.rs:312 both call it as "the plugin-api validator — same rules as install time"). Two of the 21 capability keys escape the §2.2 table, so a non-conforming manifest validates clean — the exact class of drift this module exists to reject, and the module's own doc (426-428) asserts the false guarantee.
What correct looks like: delete the dead loop and add `"sample_rates"`/`"bit_depths"` presence checks to the 608-621 loop (they are `Option<Vec<u32>>` — same `is_some()` shape), plus a test: `caps.formats = Some(["x"]); caps.sample_rates = Some(vec![44100]); assert!(caps.validate(PluginKind::Demuxer).is_err());` — that test fails today.
Confidence: HIGH — Verification: read + full branch trace of `validate`; no test in 1059-1293 feeds a misplaced numeric list (mutation: adding one would expose the gap).

### F-002 [S3]
Where: crates/plugin-api/src/manifest.rs:796-805
Code:
```
/// Plugin-to-plugin dependencies (doc 06 §2.2 — plugin ids, *not*
/// crate dependencies). All listed ids must be present and enabled or
/// the depending plugin is not loaded with a `plugins.failed` event;
/// cycles are rejected at registry build (the registry's job, not
/// this type's).
```
What is wrong: The enforcement promised here is PAPER-ONLY tree-wide. `rg "Dependencies|dependencies"` over `crates/plugin-host-wasm/src` finds no consumer: no registry, no cycle rejection, no load gating. The integration suite itself documents the gap as an ignored test: `orchrun-test-workspace-nofailfast.log.txt:3881` — `test gap_plugins_failed ... ignored, gap: plugins.failed not wired — handoff H-9 (plugin registry, doc 06)`. Additionally the type accepts duplicate ids (`dependencies = ["ext.a", "ext.a"]` parses fine) while every capability list rejects duplicates — inconsistent grammar for the same "list of ids" concept.
Why it matters: A manifest section is parsed, validated for shape, and then ignored by every loader in the tree; anyone reading the manifest model believes dependency gating exists. The scope fence ("the registry's job") is honest, but the registry does not exist and no shipped path loads plugins at all, so the section enforces nothing anywhere.
What correct looks like: Either the registry lands (Phase 2 per doc 06 §3) or the type's docs state "parsed, not yet enforced — no registry exists in v0"; add duplicate-id rejection to `Dependencies` deserialization to match the capability-list rules.
Confidence: HIGH — Verification: read + rg over all consumers (none) + ignored-gap test line cited.

### F-003 [S3]
Where: crates/plugin-api/src/manifest.rs:1018-1020
Code:
```
    /// `version` is not semver 2.0.
    #[error("manifest version: {0}")]
    Version(#[from] SemverError),
```
What is wrong: `ManifestError::Version` is never constructed anywhere in the workspace (`rg "ManifestError::Version|::Version\("` over crates+apps → 0 hits). A bad `version` string fails inside `SemverField::deserialize` (258-266), which maps `SemverError` to `serde::de::Error::custom` → surfaces as `toml::de::Error` → `From<toml::de::Error>` (1053-1057) → `ManifestError::Toml`. The documented error shape ("manifest version: …") can therefore never occur; callers matching on `Version` handle a phantom arm.
Why it matters: Dead public API that misleads error handlers; the `#[from]` implies a `?`-conversion path that no code takes. This is the "typed refusals" facade the checklist warns about — an error variant that exists for documentation, not for runtime.
What correct looks like: Remove the variant (and the `SemverError` import) or route `SemverField` failures into `ManifestError::Version` via a custom `Deserialize` on `PluginManifest`.
Confidence: HIGH — Verification: read + rg (zero construction sites).

### F-004 [S4]
Where: crates/plugin-api/src/manifest.rs:753-759
Code:
```
    /// Grants as a sorted list of permission names (for events and
    /// install UI; doc 06 §5.3 logging).
    pub fn granted(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.fs_sandboxed_read {
            out.push("fs-sandboxed-read");
```
What is wrong: The doc promises a "sorted list"; the code pushes in fixed field order (`fs-sandboxed-read`, `http-fetch`, `network-host-allowlist`, `ui-surface`, `device-query`) — `device-query` sorts first alphabetically but is emitted last. The output is deterministic but NOT sorted.
Why it matters: A doc-claim/code mismatch in a public accessor whose stated purpose is stable event/UI output; any consumer relying on "sorted" (e.g. for diffing grants across versions) gets wrong results.
What correct looks like: `out.sort_unstable()` before return, or delete "sorted" from the doc.
Confidence: HIGH — Verification: read; emission order enumerated from the full body (755-773).

### F-005 [S4]
Where: crates/plugin-api/src/manifest.rs:786-790, 854-871
Code:
```
pub struct ConfigSchema {
    /// Builtin: `settings/<area>/<id>.toml` checked into the repo.
    /// Extension: the full inline TOML fragment.
    pub settings: String,
}
```
What is wrong: `ConfigSchema::settings` is accepted with zero validation — empty string, path traversal (`../../etc`), or a non-TOML fragment all validate. Same for `Provenance::publisher`/`source`/`license` (856-870): bare `String`s, no URL shape on `source`, and `license` is documented as "SPDX identifier" (869-870) but never checked against any SPDX list. `validate()` (331-400, part1) never touches `config_schema` or these fields.
Why it matters: The module's headline promise is "validated field-by-field" (module doc line 2); four fields get no validation at all. Blast radius is small today (unreachable from shipped code, and the registry that would consume these does not exist), hence S4.
What correct looks like: non-empty checks in `validate()`; `settings` grammar (repo path for builtin vs parseable inline TOML for extension — `toml::from_str` on the fragment); `license` matched against a vendored SPDX id list.
Confidence: HIGH — Verification: read + traced `validate()` — no reference to `config_schema`, `publisher`, `source`, `license`.

### F-006 [S4]
Where: crates/plugin-api/src/manifest.rs:17-20
Code:
```
//! - [`PluginManifest::to_toml`] — serialize back. Round-trip is
//!   value-exact by construction (see the module tests and the four
//!   doc 06 §2.3 example fixtures, which must parse AND round-trip
//!   per the card's acceptance criteria).
```
What is wrong: "see the module tests" — the in-file `mod tests` (1059-1293) contains no test that calls `to_toml`. Round-trip is actually proven by the SEPARATE integration file `crates/plugin-api/tests/doc06_examples.rs` (`all_four_reparse_from_their_own_output`, green in orchrun run :4475-4486).
Why it matters: Citation drift: the pointer sends a reader to tests that do not contain the claimed coverage.
What correct looks like: Point at `tests/doc06_examples.rs`, or add one in-file `to_toml`/`from_toml` stability test.
Confidence: HIGH — Verification: read of all 9 module tests (none call `to_toml`); doc06_examples.rs:134-147 shows the double round-trip harness.

## 4. Per-function table (part2: lines 586-1293)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Capabilities::validate` | 586-715 | REAL (with hole) | F-001 S2 | Key-set membership for 19 keys, primary-key presence, list rules, numeric rules — all traced correct EXCEPT `sample_rates`/`bit_depths` membership (dead loop 597-607). No panics: HashSet-based dedup, no indexing; `format!` on untrusted strings bounded by 32 B/64 entries. |
| `Permissions` (struct) | 725-745 | REAL | — | serde kebab-case wire (`http-fetch` etc. verified by test 1289-1291 typo rejection); deny_unknown_fields. |
| `Permissions::is_absent` | 747-751 | REAL | — | `*self == default()` — correct deny-by-default ground state; called from part1's `validate` (383) and serde skip. |
| `Permissions::granted` | 753-773 | REAL | F-004 S4 | Field-order emission, NOT sorted despite doc; only production caller is dev bin lumenext.rs:201. |
| `ConfigSchema` (struct) | 784-790 | REAL | F-005 S4 | Unvalidated `settings: String`. |
| `Dependencies` (struct) + `is_absent` | 806-815 | REAL | F-002 S3 | Parsed-then-ignored tree-wide; duplicates accepted. |
| `DependenciesWire` (enum) | 818-825 | REAL | — | Untagged array-or-empty-table; honest bridge for the §2.2/§2.3 shape conflict. |
| `Depth0` (struct) | 828-830 | REAL | — | Empty deny_unknown_fields struct — any key in `[dependencies]` table rejected. |
| `From<DependenciesWire> for Dependencies` | 832-839 | REAL | — | Table → empty vec; correct normalization. |
| `From<Dependencies> for DependenciesWire` | 841-845 | REAL | — | Always Array — serializes empty as `[]` but PluginManifest skips absent (240-241), so empty table round-trips to omission (value-exact, documented 802-805). |
| `Provenance` (struct) | 854-871 | REAL | F-005 S4 | hash/signature via typed newtypes; publisher/source/license bare strings. |
| `PackageHash` (struct) | 881-885 | REAL | — | raw + full flag; invariant maintained only by `parse`. |
| `PackageHash::parse` | 893-918 | REAL | — | 64-ascii-hex (either case) → full; doc placeholder (`…` + "(64 hex)") → non-full; everything else typed-rejected. Placeholder risk gated at install (install.rs:494 — verified). |
| `PackageHash::is_full` | 922-924 | REAL | — | const; production caller install.rs:494. |
| `PackageHash::as_str` | 927-929 | REAL | — | Verbatim raw. |
| `Display`/`Serialize`/`Deserialize for PackageHash` | 932-949 | REAL | — | Round-trips raw text exactly; Deserialize routes through parse (validation on every load from text). |
| `Signature` (struct) | 952-959 | REAL | — | scheme: String unvalidated (ed25519 is [PROPOSAL]); no verification code exists anywhere (rg ed25519 in host → none) — v1-optional per doc 06:266, honest. |
| `Pubkey::parse` / `as_str` | 971-983 | REAL | — | Non-empty only; grammar deferred to pending ADR (documented). |
| `Serialize`/`Deserialize`/`Display for Pubkey` | 986-1003 | REAL | — | Trivial pass-through with parse-on-deserialize. |
| `ManifestError` (enum) | 1011-1051 | REAL | F-003 S3 | `Version` variant dead; all other variants constructed (verified by rg for the reachable ones; messages actionable). |
| `From<toml::de::Error>` | 1053-1057 | REAL | — | Stringifies — loses span info; acceptable for v1, message preserved. |
| tests: `id` helper | 1064-1069 | REAL | — | unreachable! on invalid fixture ids — fine for fixtures. |
| tests: `id_grammar` | 1071-1100 | REAL | — | See §5. |
| tests: `id_prefixes_and_slugs` | 1102-1108 | REAL | — | See §5. |
| tests: `hash_accepts_full_and_doc_placeholder` | 1110-1133 | REAL | — | See §5. |
| tests: `empty_dependencies_table_and_array_agree` | 1135-1161 | REAL | — | See §5. |
| tests: `capabilities_primary_key_enforced_per_kind` | 1163-1190 | REAL (gap) | F-001 | Never feeds a misplaced NUMERIC list — the one hole. |
| tests: `cross_field_rules` | 1192-1234 | REAL | — | 4 of 6 origin/provenance/permission rules; untested arms: `lumen.`-prefix+extension-origin, builtin+provenance-present. |
| tests: `priority_defaults_and_range` | 1236-1270 | REAL | — | Covers defaults (100/50) and the 1001 boundary. |
| tests: `unknown_fields_rejected` | 1272-1292 | REAL | — | Root field, sub-table, typo'd permission — good deny_unknown_fields sweep. |

## 5. Test honesty (module tests, 1059-1293; all green in orchrun run :4447-4455)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `id_grammar` | YES | 5 good/11 bad ids incl. exact-64B boundary (1097-1099); mutation: relaxing the 64-byte cap or charset fails an assert. (Exercises part1's `PluginId::parse` — listed because the test lives in my range.) |
| `id_prefixes_and_slugs` | YES | prefix()/publisher_slug() outputs pinned exactly. |
| `hash_accepts_full_and_doc_placeholder` | YES | Full/placeholder/bad (5 shapes incl. wrong prefix `sha512:`) — mutation: accepting any string fails `sha256:xyz` assert (1131). |
| `empty_dependencies_table_and_array_agree` | YES | Untagged-enum both arms + `is_absent`; mutation: dropping the Table arm fails 1149-1153. |
| `capabilities_primary_key_enforced_per_kind` | YES for the rules it feeds | legality/primary/dup/upper/33B/65-entry/64-entry all asserted (1169-1189); mutation: removing the legality check fails 1171. NO coverage for misplaced numeric lists — which is why F-001 survives green. |
| `cross_field_rules` | YES (fed arms) | Each `matches!` pins a specific variant; mutation: deleting the provenance-required rule fails 1196-1202. Two unfed arms (see table above). |
| `priority_defaults_and_range` | YES | effective_priority 100/50 pinned; `Priority(1001)` variant pinned (1266). |
| `unknown_fields_rejected` | YES | Three insertion points for unknown keys; mutation: removing deny_unknown_fields fails all three asserts. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "misplaced keys [rejected] by validate" | manifest.rs:426-428 (module doc) | FALSE (for 2 keys) | F-001: `sample_rates`/`bit_depths` escape; trace in §3. |
| "Round-trip is value-exact by construction (see the module tests…)" | manifest.rs:17-20 | REAL but miscited | Round-trip proven in tests/doc06_examples.rs (orchrun :4475-4486), not module tests (F-006). |
| "Grants as a sorted list" | manifest.rs:753-754 | FALSE | F-004: fixed field order, `device-query` emitted last. |
| "`hash` … verified at install and at every load" | doc 06 §2.2:265, quoted in manifest.rs:861-862 | REAL (install) / PAPER-ONLY (every load) | install.rs:494-508 real (is_full + exact match); `rg digest|sha256|verify` in plugin-host-wasm instance/supervisor/state → 0 — no load-time re-verification exists. |
| decoder/encoder key set includes `sample_rates`, `bit_depths` (implying kind-scoped) | doc 06 §2.2:211 | PARTIALLY FALSE | Enforcement claim broken by F-001 — the key set is not enforced for these two keys. |
| "All listed ids must be present and enabled or the depending plugin is not loaded with a `plugins.failed` event" | manifest.rs:796-798 | PAPER-ONLY | No registry/consumer anywhere; `gap_plugins_failed` ignored test (orchrun :3881). |
| "signature … optional in v1, required if `product.toml` demands signed extensions" | manifest.rs:864-866 / doc 06:266 | REAL-UNPROVEN | Parsed as a type; no verification code exists (ed25519 is a proposal); no product.toml enforcement found in host (rg) — deferred, documented as such. |
| "empty table and an absent table are the same value (empty list)" | manifest.rs:802-805 | REAL | Test 1135-1161 green; `is_absent` normalization verified. |

## 7. Dead / duplicated / unreachable code
- DEAD: the empty `for` loop body 597-607 (F-001) and `ManifestError::Version` (F-003).
- UNREACHABLE (shipped): the entire manifest model — see header reachability row. Compiled into `lumen` (plugin-api is a CLI dep) but never called by it.
- Duplication: none material in my range; `PackageHash`/`Pubkey`/`PluginId` follow the same newtype pattern deliberately.

## 8. Dependencies & flags
- Crate deps (plugin-api/Cargo.toml): serde, toml, thiserror + path deps media-types, codec-api — matches the allowlist comment; no feature gates touch manifest.rs; `[lints] workspace = true` inherited.
- `toml` 1.1: untagged enum support and deny_unknown_fields on empty struct both exercised (tests green).

## 9. Open questions for the next agent
- W12-A08 (manifest.rs#part1): my part boundary starts at 586 — please end part1 at 585 to avoid double-auditing `Capabilities::validate`. The `PluginManifest::validate` cross-field rules (331-400) are yours; note they never touch `config_schema` (my F-005).
- Whoever owns plugin-host-wasm/src/install.rs + supervisor: the "verified at every load" half of the hash claim (06:265) — I found install-time verification only; confirm no load-time re-hash exists on your side and file if confirmed.
- Doc-06 owner: §2.2:211's decoder/encoder row and the §2.2 "no key outside the kind's row" rule vs F-001.
- Registry (Phase 2, doc 06 §3): dependencies enforcement (F-002) — is there a card, or is the promise floating?
