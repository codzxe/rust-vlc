# AUDIT: crates/plugin-host-wasm/src/api.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 586 / 13 prod (incl. 2 const catalogs) + 6 tests |
| File verdict | REAL (executed) — with one dead subsystem (deprecation window) |
| Reachable from shipped entry point? | NO — no workspace crate depends on plugin-host-wasm (rg over all Cargo.tomls: only its own + a comment in plugin-api); release lane builds lumen-cli+desktop only; the crate's own `lumenext` bin is developer-side |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p plugin-host-wasm --lib api::` -> 6 passed, 0 failed; `rg -n 'deprecation_notice|deprecation_notices_sent' crates/ apps/ tools/` -> only api.rs/state.rs definitions; `rg -l 'plugin-host-wasm' --glob Cargo.toml` -> no dependents; sed verification of quotes |

## 1. One-paragraph verdict
A clean, genuinely-implemented ABI catalog: the 11-entry `lumen:v1.*` import
table with per-module permission gates, a [1,1] version window, typed
guest-status codes, and manifest-derived grants with the allowlist-implies-http
companion rule — all unit-tested and all executed by me (6/6 pass). The import
policy is enforced pre-instantiation in imports.rs:check_import_policy via
`lookup_module`, and unknown modules (e.g. wasi) are refused. The one rotten
beam is the deprecation-window mechanism: `deprecation_notice` has zero
production callers, its state counterpart `deprecation_notices_sent` is
initialized and never touched again, and the function's doc promises
window-awareness its one-line body cannot implement. The whole crate is
unreachable from the shipped release lane.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-host-wasm --lib api::` ->
"test result: ok. 6 passed; 0 failed".
— wiring verified: imports.rs:254 uses `crate::api::lookup_module` in
`check_import_policy` (deny unknown/undeclared imports before instantiation);
instance.rs:76 and install.rs:381,485 call `negotiate_api_version`.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/api.rs:316-317; crates/plugin-host-wasm/src/state.rs:257-258
Code:
```
pub fn deprecation_notice(spec: &ImportSpec) -> Option<&'static str> {
    spec.deprecated.map(|d| d.notice)
}
```
and (state.rs:257-258):
```
    /// First-use deprecation notices already emitted.
    pub deprecation_notices_sent: Vec<&'static str>,
```
What is wrong: The deprecation-window mechanism the module header advertises
("API-version negotiation and the deprecation window mechanism", api.rs:3-4)
is dead code. `rg -n 'deprecation_notice|deprecation_notices_sent'` over
crates/, apps/, tools/ finds only these definitions and api.rs's own unit
test — imports.rs (the only place imports are dispatched) never calls it, and
`deprecation_notices_sent` is never read or pushed after construction
(state.rs:299). Why it matters: when the first real deprecation lands, the
"log-on-first-use notice" contract of doc 06 §5.5 will silently not fire —
nothing is wired to emit it. What correct looks like: `trace_import` (or the
import dispatch path) consulting `deprecation_notice` and recording into
`deprecation_notices_sent`.
Confidence: HIGH — Verification: ran (rg absence proof)

F-002 [S3]
Where: crates/plugin-host-wasm/src/api.rs:311-317
Code:
```
/// The deprecation notice to emit for `spec` on first use within the
/// two-minor window (doc 06 §5.5) — `None` when the import is not
/// deprecated or the use is past the window (at which point the
/// import is simply gone from the catalog, so it can no longer be
/// resolved at all).
pub fn deprecation_notice(spec: &ImportSpec) -> Option<&'static str> {
    spec.deprecated.map(|d| d.notice)
}
```
What is wrong: The doc promises window-conditional behavior ("`None` when
... the use is past the window"); the implementation takes no engine-minor
input and cannot distinguish in-window from past-window use — it returns the
notice whenever `deprecated` is `Some`. The doc describes semantics the code
does not have. (V1_IMPORTS has `deprecated: None` everywhere today, so no
live impact; this is why S3, not S2.)
Confidence: HIGH — Verification: read
What correct looks like: `deprecation_notice(spec, current_minor)` matching
`d.since_minor <= current_minor < d.removal_minor`.

F-003 [S4]
Where: crates/plugin-host-wasm/src/api.rs:501-506 (test helper)
Code:
```
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }
```
What is wrong: house-local `ok` helper duplicates the workspace `testkit::Must`
pattern (present.rs uses `testkit::{Must, assert_err}`); one idiom per crate
would do. Third copy of the same helper in this crate alone (api.rs:502,
fsjail.rs:200, and common/mod.rs:41); belongs in testkit.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| negotiate_api_version | 97-108 | REAL | — | window [1,1]; typed HostError::ApiVersion; called instance.rs:76, install.rs:381,485 |
| Permission::as_str | 126-139 | REAL | — | manifest spellings |
| ValKind::as_str | 148-155 | REAL | — | |
| lookup | 298-303 | REAL | — | linear scan of 11 entries; used by tests + future dispatch |
| lookup_module | 307-309 | REAL | — | used by imports.rs:254 check_import_policy |
| deprecation_notice | 311-317 | STUB/DEAD | S3 | F-001/F-002: no callers, doc contradicts body |
| GuestStatus::code | 353-367 | REAL | — | 0-9 |
| GuestStatus::from_code | 372-391 | REAL | — | unknown -> Err(EArg), no panic |
| Grants::from_permissions | 412-421 | REAL | — | allowlist implies http (doc 06 §2.2); test-pinned |
| Grants::has | 424-433 | REAL | — | Base always true |
| Grants::granted_names | 435-450 | REAL | — | install UI/events spelling |
| kind_extension_legal | 456-458 | REAL | — | thin re-export; install.rs:479 uses it; duplication noted (authoritative rule in plugin_api) |
| V1_IMPORTS / consts | 43-87, 198-292 | REAL | — | 11 entries; test asserts every result is one i32 and permission-per-module |
| 6 #[test] fns | 461-586 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| catalog_shapes_are_doc06_conformant | YES | namespace prefix, one-i32 result, per-module permission asserted for all 11 entries |
| lookup_finds_every_entry | YES | also asserts fs_chmod and wasi modules are absent |
| version_window_is_v1_only_today | YES | 1 ok; 0/2/3/999 typed-refused with found echoed |
| grants_follow_doc06_companion_rule | YES | deny-default + allowlist-implies-http pinned |
| status_codes_round_trip | YES | 0..=9 bijective; 42 -> EArg |
| deprecation_window_mechanism | PARTIAL | exercises only the synthetic Some/None mapping — cannot catch the unwired mechanism (F-001) or the missing window logic (F-002); it tautologically re-tests `Option::map` |
All 6 executed by me: 6 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the deprecation window mechanism" exists | api.rs:3-4 | STUB | F-001: no production caller |
| "`None` when ... the use is past the window" | api.rs:313 | FALSE | body ignores window (F-002) |
| "no i64 host pointers ... handles are small opaque u32 indices" | api.rs:7-16 | REAL-UNPROVEN (here) / REAL (imports.rs) | catalog params are all I32; enforcement audited in imports.rs (another agent's file) |
| module table lists 11 functions across 6 modules | api.rs:23-31 | REAL | V1_IMPORTS has exactly 11 entries matching modules (test) |

## 7. Dead / duplicated / unreachable code
Dead: deprecation_notice (api.rs:316), deprecation_notices_sent (state.rs:258).
Duplicated: kind_extension_legal re-exports plugin_api rule (documented as
double-check). Crate unreachable from shipped lane (no dependents).

## 8. Dependencies & flags
plugin-api (manifest/permissions), settings-schema (crash-policy config), wasmi
2.0.0 pinned default-features=false + std,validate,auto-dispatch (Cargo.toml
records the stack-overflow rationale). No feature gates on api.rs.

## 9. Open questions for the next agent
- imports.rs auditor: does the import dispatch path have any hook where a
  deprecation notice SHOULD fire? If yes, F-001 is confirmed dead-on-arrival
  wiring rather than future scaffolding.
- Who ships `lumenext` (packaging lanes)? If a packaging lane ships it, the
  crate's reachability verdict changes for the binary only.
