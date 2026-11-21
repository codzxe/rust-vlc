# AUDIT: crates/plugin-host-wasm/src/api.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Lines / functions | 586 / 33 items (7 module consts + 10 IMP_ consts + 1 window const + 1 fn + 4 enums + 3 structs + 10-entry catalog + 3 lookup/notice fns + 6 GuestStatus/Grants methods + 1 wrapper + 6 tests + 1 helper) |
| File verdict | UNREACHABLE (real, internally consistent, test-executed; nothing shipped depends on the crate) |
| Reachable from shipped entry point? | NO. plugin-host-wasm has zero dependents (`rg -l "plugin-host-wasm" --glob Cargo.toml` → its own manifest + a scope-fence comment in plugin-api/Cargo.toml:17). In-crate callers are real and traced: `imports.rs:254` (`lookup_module` in `check_import_policy`), `imports.rs:34-35,284-871` (IMP_* trace labels), `state.rs:393,401` (security-event labels), `install.rs:479` (`kind_extension_legal`), `install.rs:485` (`negotiate_api_version`). Not compiled into `lumen` at all. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "IMP_\|deprecation_notice\|lookup\|kind_extension_legal\|since_minor\|removal_minor\|deprecation_notices_sent\|granted_names\|from_code" crates --type rust` (caller census — see findings); `sed -n '240,290p;362,380p;410,430p;553,570p;615,660p;686,700p;817,830p;866,880p' crates/plugin-host-wasm/src/imports.rs` (catalog-vs-linker signature cross-check, 10/10 match); `rg -n "ENGINE_PLUGIN_API_MAJOR" crates/plugin-api/src/lib.rs` (=1 at :125); `sed -n '250,262p;385,405p' crates/plugin-host-wasm/src/state.rs` (dead deprecation slot); baseline greps: `orchrun-test-workspace-nofailfast.log.txt:4495-4611` (plugin_host_wasm 44 unit + 26 integration tests green). |

## 1. One-paragraph verdict
This is the honest core of the plugin ABI: the `lumen:v1.*` module catalog, guest-status code table, permission model, and version-negotiation window are real, mutually consistent, and I cross-checked all ten catalog signatures against the actual wasmi `func_wrap` definitions in imports.rs — 10/10 match (params, results, permission gates). The permission gate itself (`Grants::from_permissions` → `has` → `check_import_policy` via `lookup_module`) is wired and exercised by nine green sandbox-security tests. The rot is at the edges: the entire deprecation-window "mechanism" (doc 06 §5.5, advertised in the module header) is unwired scaffolding — a getter nobody calls, two struct fields nobody reads, a first-use state slot nobody touches, and a test that proves `Option::map` works while its name promises a window; and two public functions (`lookup`, `granted_names`) plus `GuestStatus::from_code` have zero callers in the tree. And the standing domain verdict: none of this is reachable from any shipped binary — the crate that contains it has no dependents; plugins execute only inside this crate's own test suite.

## 2. What actually works (verified)
— Verified by catalog-vs-linker cross-check (sed on imports.rs, all define_* blocks): every `V1_IMPORTS` entry matches the real host function — `api_version` []→[i32] (imports.rs:274-277), `report_manifest` [i32,i32] (280-283), `log_write` [i32,i32,i32] (364-368), `fs_open` [i32,i32,i32] (412-417), `fs_read` [i32,i32,i32,i32] (555-561), `fs_size` [i32,i32] (617-620), `fs_close` [i32] (652-655), `http_get` [i32×5] (688-694), `ui_register_panel` [i32,i32] (819-823), `devices_list` [i32,i32,i32] (867-869). All return a single i32 status.
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4495-4543` → plugin_host_wasm 44 unit tests green, including all 6 `api::tests::*`; `:4557-4611` → integration: `plugin_crash_isolation` 6 ok, `plugin_examples_conformance` 5 ok (incl. `metadata_provider_example_passes_conformance`, `ui_panel_example_passes_conformance`), `plugin_hello_world` 2 ok (incl. `hello_world_extension_runs_and_reports_manifest` — a real guest invoking host imports under limits), `plugin_install_lifecycle` 4 ok, `plugin_sandbox_security` 9 ok (incl. `plugin_permission_escalation`, `plugin_host_pointer_smuggling`). The wasmi host DOES execute plugins end to end — in tests, and nowhere else.
— Verified by reading `Grants::from_permissions` (412-421): the §2.2 companion rule is enforced twice (manifest validation rejects allowlist-without-http; `http: perms.http_fetch || allowlist.is_some()` is defense-in-depth for programmatically built `Permissions`), and `http_hosts` empty-means-blocked semantics are pinned by test 534-540.
— Verified `negotiate_api_version` window `[1,1]`: `ENGINE_PLUGIN_API_MAJOR = 1` (plugin-api/src/lib.rs:125), MIN=1 (api.rs:92); 0/2/3/999 all rejected with the typed error (test 509-518, green).

## 3. Findings

### F-001 [S3]
Where: crates/plugin-host-wasm/src/api.rs:311-318 (with 158-170, and state.rs:258/299)
Code:
```
pub fn deprecation_notice(spec: &ImportSpec) -> Option<&'static str> {
    spec.deprecated.map(|d| d.notice)
}
```
What is wrong: The module header (line 3) advertises "the deprecation window mechanism" as part of this file's contract, and the doc comment on this function describes window-aware behavior ("`None` when … the use is past the window"). The implementation is a field getter: it never reads `Deprecation::since_minor`/`removal_minor` (zero readers tree-wide — rg), never compares against any current engine minor, and has no first-use state. The first-use slot that DOES exist (`state.rs:258 pub deprecation_notices_sent: Vec<&'static str>`, initialized at :299) is never read or written anywhere else — rg finds exactly its declaration and its initialization. Zero production callers of `deprecation_notice`. The test named `deprecation_window_mechanism` (557-585) constructs a synthetic entry and asserts the getter returns the notice — it would pass identically if the window fields were deleted from the struct.
Why it matters: Doc 06 §5.5 promises a two-minor deprecation window with log-on-first-use; a reader of this file believes the machinery exists. It does not — this is a documented seam with no engine behind it. In v1 nothing is deprecated so nothing breaks, but the first real deprecation will discover the mechanism was never built.
What correct looks like: `deprecation_notice(spec, engine_minor)` consulted from the import dispatch path in imports.rs, recording into `deprecation_notices_sent` on first use, with a test that feeds an in-window and a past-window entry and asserts the difference.
Confidence: HIGH — Verification: read + rg caller census (deprecation_notice: 0 non-test callers; since_minor/removal_minor: 0 readers; deprecation_notices_sent: 2 hits = decl + init).

### F-002 [S3]
Where: crates/plugin-host-wasm/src/api.rs:298-302, 435-450, 368-386
Code:
```
/// Look up a catalog entry by `(module, name)`.
pub fn lookup(module: &str, name: &str) -> Option<&'static ImportSpec> {
    V1_IMPORTS
        .iter()
        .find(|spec| spec.module == module && spec.name == name)
}
```
What is wrong: Three public items have zero production callers: (1) `lookup(module, name)` — the actual import-policy check (imports.rs:254 `check_import_policy`) uses only `lookup_module`; `lookup` is called from nothing except its own test. (2) `Grants::granted_names` (435-450) — no callers anywhere in the crate (the install UI / events it cites do not exist). (3) `GuestStatus::from_code` (368-386) — doc says "tests and guest-side bindings"; no guest-side bindings exist in the tree, so its only caller is its test. All three compile clean because they are `pub`.
Why it matters: Dead public API on an ABI surface invites divergence: the day a real guest SDK lands, `lookup`'s (module,name) keying and `from_code`'s code table become contract — untested against any real consumer. `granted_names` duplicates `plugin_api::Permissions::granted` (manifest.rs:755-773) in a second crate — the same list computed two ways, already disagreeing in shape (the plugin-api version emits `network-host-allowlist` too; this one cannot).
What correct looks like: Delete `lookup` and `granted_names` or wire them (policy check could use `lookup` per-function when function-level gating arrives); keep `from_code` only with the guest-SDK crate that consumes it.
Confidence: HIGH — Verification: read + rg caller census per item.

### F-003 [S4]
Where: crates/plugin-host-wasm/src/api.rs:186-188
Code:
```
    /// `(major, minor)` this import appeared in — additive-only
    /// history per §5.5.
    pub since: (u32, u32),
```
What is wrong: `ImportSpec::since` is write-only data: every catalog entry sets `since: (1, 0)` and nothing in the workspace ever reads `spec.since` (rg: zero non-literal readers). The "additive-only history" is recorded but never consulted — no check that a v1.0-served import predates the negotiated api_version, no doc generation from it.
Why it matters: Ten copies of dead provenance data presented as contract machinery; a reader assumes version filtering exists somewhere.
What correct looks like: Either the negotiation path consults `since` (e.g. an api_version 1.0 guest must not see imports with `since.1 > 0`), or the field goes until a consumer lands.
Confidence: HIGH — Verification: read + rg for `.since` readers in plugin-host-wasm (none).

## 4. Per-function table (586 lines, whole file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module consts (`NAMESPACE`..`MOD_DEVICES`) | 47-65 | REAL | — | ABI protocol vocabulary; consumed by imports.rs linker + policy check; `lumen-allow-literal` gate markers present (legit protocol constants, not brand strings). |
| IMP_* trace-label consts | 67-87 | REAL | — | 10/10 used: imports.rs:284,372,420,564,621,656,698,826,871; state.rs:393,401. |
| `ENGINE_PLUGIN_API_MIN` | 89-92 | REAL | — | =1; window [1,1] consistent with ENGINE_PLUGIN_API_MAJOR=1 (plugin-api lib.rs:125). |
| `negotiate_api_version` | 94-108 | REAL | — | Range check → typed HostError::ApiVersion; called at install (install.rs:485); test-pinned for 0/1/2/3/999. |
| `Permission` enum + `as_str` | 110-137 | REAL | — | "base" spelling is host-internal only (no manifest counterpart) — used in UndeclaredImport errors (imports.rs:263). |
| `ValKind` + `as_str` | 139-156 | REAL | — | i32/i64 only; honest v1 ABI. |
| `Deprecation` struct | 158-170 | STUB-ADJACENT | F-001 | Fields since_minor/removal_minor never read anywhere. |
| `ImportSpec` struct | 172-193 | REAL | F-003 | `since` write-only; `deprecated` None in all v1 entries (honestly documented at 189-191). |
| `V1_IMPORTS` catalog | 197-295 | REAL | — | 10 entries; cross-checked against imports.rs linker 10/10 (params/results/permission). |
| `lookup` | 297-302 | REAL (dead) | F-002 | Zero production callers. |
| `lookup_module` | 304-309 | REAL | — | The actual policy path (imports.rs:254). |
| `deprecation_notice` | 311-318 | STUB | F-001 | Getter with delusions of mechanism. |
| `GuestStatus` enum | 320-349 | REAL | — | 10 statuses; E_PERM singled out as escalation (matches state.rs SecurityLedger usage). |
| `GuestStatus::code` | 351-366 | REAL | — | Used at every import site in imports.rs (verified in excerpts). |
| `GuestStatus::from_code` | 368-386 | REAL (dead) | F-002 | Test-only caller; unknown → Err(EArg), never panics. |
| `Grants` struct | 389-406 | REAL | — | Deny-by-default; http_hosts empty = nothing contactable. |
| `Grants::from_permissions` | 408-421 | REAL | — | Companion rule double-enforced (manifest + here); test-pinned. |
| `Grants::has` | 423-432 | REAL | — | Base always true; called from check_import_policy. |
| `Grants::granted_names` | 434-450 | REAL (dead) | F-002 | Zero callers; duplicates plugin_api::Permissions::granted. |
| `kind_extension_legal` | 453-458 | REAL | — | One-line delegation to `PluginKind::extension_allowed_in_v1` (authority stays in plugin-api — documented); caller install.rs:479. |
| tests: `catalog_shapes_are_doc06_conformant` | 465-489 | REAL | — | See §5. |
| tests: `lookup_finds_every_entry` | 491-499 | REAL | — | See §5. |
| tests: `ok` helper | 501-507 | REAL | — | unreachable!-on-Err helper, test-only. |
| tests: `version_window_is_v1_only_today` | 509-518 | REAL | — | See §5. |
| tests: `grants_follow_doc06_companion_rule` | 520-541 | REAL | — | See §5. |
| tests: `status_codes_round_trip` | 543-554 | REAL | — | See §5. |
| tests: `deprecation_window_mechanism` | 556-585 | SHALLOW | F-001 | See §5 — proves Option::map, not a window. |

## 5. Test honesty (all 6 green: orchrun run :4498-4503)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `catalog_shapes_are_doc06_conformant` | YES | Namespace prefix, single-i32 results, per-module permission mapping all asserted (468-488); mutation: switching `fs_read`'s permission to Base fails 482. |
| `lookup_finds_every_entry` | YES (for catalog shape) | Every entry reachable by key; `fs_chmod` and `wasi_snapshot_preview1` correctly absent (497-498). Tests a function production never calls (F-002) — honest about the catalog, useless for reachability. |
| `version_window_is_v1_only_today` | YES | 1 accepted; 0/2/3/999 rejected with `found` pinned (511-517). |
| `grants_follow_doc06_companion_rule` | YES | Deny-default, allowlist⇒http with hosts copied, http-without-allowlist⇒empty hosts (523-540). |
| `status_codes_round_trip` | YES | 0..=9 identity + 42→Err(EArg) (545-553). |
| `deprecation_window_mechanism` | NO | Asserts a getter returns what it was given (576-584). Deleting both window fields from `Deprecation` leaves this test green — it cannot detect the mechanism's absence because there is no mechanism. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "API-version negotiation and the deprecation window mechanism" (file's stated contents) | api.rs:1-4 (module doc) | PARTIAL | Negotiation REAL (97-108 + tests); deprecation mechanism STUB (F-001). |
| Module/permission table: 6 modules, 10 functions, `lumen:v1.media` empty | api.rs:28-43 | REAL | Catalog 198-295 matches imports.rs linker definitions 10/10 (sed cross-check); media absent from both. |
| "Every import is a plain-integer wasm function… no host pointers in v1 ABI" | api.rs:6-17 | REAL | All ten host fns take only i32 params + wasmi Caller (imports.rs excerpts); handle model per §9.4 exercised by `plugin_host_pointer_smuggling` (orchrun :4604). |
| "Runtime denials are typed errors the guest observes … never traps" | api.rs:21-26 | REAL-UNPROVEN here | Status convention verified at import sites I read (EArg/EPerm/EHandle returns); the full sweep is imports.rs auditor's scope. |
| "Mismatches … never UB, never a silent load" (negotiation) | api.rs:94-96 | REAL | Typed error + install-time call (install.rs:485) + test 509-518. |
| Deprecation: "stays functional for two engine minors (log-on-first-use notice)" | api.rs:158-160 (quoting doc 06 §5.5) | PAPER-ONLY | F-001 — no window logic, no first-use state wired. |

## 7. Dead / duplicated / unreachable code
- DEAD: `lookup` (298-302), `Grants::granted_names` (435-450), `GuestStatus::from_code` (368-386) — F-002; `ImportSpec::since` (186-188) — F-003; `Deprecation::{since_minor,removal_minor}` (164-167) — F-001; `state.rs:258 deprecation_notices_sent` (declared+init only).
- DUPLICATED: `granted_names` vs `plugin_api::Permissions::granted` (manifest.rs:755-773) — two spellings of the grant list in two crates, already divergent in coverage (allowlist missing here).
- UNREACHABLE: the entire file from shipped entry points (see header).

## 8. Dependencies & flags
- Depends only on plugin-api (ENGINE_PLUGIN_API_MAJOR, PluginKind, Permissions) + crate::error — no external crates, no feature gates, no unsafe. Clean layering.
- `lumen-allow-literal` markers on ABI constants are the sanctioned lint-literals escape (gate-check log baseline #13 flags a different site, apps/cli — consistent).

## 9. Open questions for the next agent
- imports.rs auditor: confirm the `report_manifest` acceptance path compares api_version via negotiate too (I saw id/kind/version/api_version compared at imports.rs:312-318 — the api_version comparison uses the reported manifest's field; whether it re-negotiates is your call).
- Whoever owns doc 06 §5.5: the deprecation-window promise vs F-001 — doc says the mechanism exists; code has a stub.
- Guest-SDK plan: `from_code` + `lookup` exist for bindings that don't exist — is there a card, or should they be deleted?
