# AUDIT: crates/plugin-host-wasm/src/error.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-21 |
| Lines / functions | 413 / 9 (HostError enum 20 variants, PackageError enum 14 variants, Display×2, Error×2, From×2, 1 test) |
| File verdict | UNREACHABLE (real, largely correct taxonomy; crate is a leaf — no shipped entry point depends on plugin-host-wasm) |
| Reachable from shipped entry point? | NO. `HostError`/`PackageError` are used across the crate (instance.rs:66,76-113; install.rs:566-578; package.rs:190; imports.rs; api.rs:97-99; supervisor.rs; both `lumenext` bins) — but `rg -l "plugin-host-wasm" --glob **/Cargo.toml` returns only the crate's own manifest. The shipped `lumen` CLI (apps/cli/Cargo.toml:47-65) does not depend on plugin-host-wasm; nothing else does either. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | full read; per-variant construction census (`rg -c "HostError::$v" src tests` excluding error.rs — 19/20 HostError and 14/14 PackageError variants constructed, `HostCapacity` = 0); `rg "HostCapacity|capacity|active_instances|max_instances|budget" src` (no host accounting anywhere); read instance.rs:30-170 (spawn signature has no host-accounting parameter); `rg PACKAGE_CAP_BYTES` package.rs:36 (Display cross-ref exists); doc 06 §5.4 read (limits table `[PROPOSAL]`, "Concurrent instances … host scheduler", "Total extension heap … host accounting"); baseline orchrun log :4495-4547 (44 plugin_host_wasm unit tests green incl. `error::tests::every_variant_displays_actably`) |

## 1. One-paragraph verdict
This is the typed-error taxonomy for the WASM extension host: 20 `HostError` + 14 `PackageError` variants with genuinely actionable Display strings (each names the rule, the observed values, and the doc section). 19 of 20 HostError variants and all 14 PackageError variants are really constructed somewhere in the crate — the exception is the story: `HostError::HostCapacity` is never constructed by any code path or test, and its promised mechanism (doc 06 §5.4 "4 active instances per host / 256 MiB total extension heap, host accounting") does not exist — `WasmInstance::spawn` (instance.rs:56-63) takes no host-accounting state, so instance count and aggregate heap are unbounded. The lone test is dishonest about its own name (`every_variant_displays_actably` checks 4 of 34 variants with a tautological assertion). All of it is dead weight from the shipped product's perspective: no shipped entry point reaches this crate.

## 2. What actually works (verified)
— 44/44 plugin_host_wasm unit tests green in baseline `orchrun-test-workspace-nofailfast.log.txt:4495-4547`, incl. `error::tests::every_variant_displays_actably ... ok` (:4504).
— Display arms traced: all 20 HostError and 14 PackageError arms produce rule-naming, value-carrying messages (e.g. ApiVersion prints the window `[min, max]`; GuestMemoryOob prints ptr/len/size; TooLarge prints the real cap from `crate::package::PACKAGE_CAP_BYTES`, package.rs:36 — verified to exist).
— Variant-construction census: every HostError variant except `HostCapacity` (F-001) is constructed in src or tests; every PackageError variant is.
— Header claim "never a panic, never a swallowed Result" holds crate-wide by enforcement: workspace lints deny `unwrap_used`/`expect_used`/`panic` (root Cargo.toml:60-62) and baseline clippy is CLEAN with `-D warnings` (INDEX #5, #6).
— `From<io::Error>`/`From<PackageError>` (370-380) and `HostError::source()` → PackageError (359-366) chain correctly.

## 3. Findings
### F-001 [S3]
Where: crates/plugin-host-wasm/src/error.rs:118-124
Code:
```
    /// A host-level resource ceiling was hit while spawning: too many
    /// active instances or the total extension-heap budget is
    /// exhausted (doc 06 §5.4 host accounting).
    HostCapacity {
```
What is wrong: The variant is never constructed anywhere — `rg -c "HostError::HostCapacity" src tests` returns 0 hits outside its definition and Display arm. There is no host-level accounting to produce it: `WasmInstance::spawn` (instance.rs:56-63) receives only per-instance `ResourceLimits`, no registry of active instances, no aggregate-heap ledger; grep for capacity/active_instances/max_instances/budget in src/ finds only the per-instance token bucket in limits.rs.
Why it matters: Doc 06 §5.4 promises "Concurrent instances: 4 active per host (host scheduler)" and "Total extension heap: 256 MiB across instances (host accounting)" with "every breach produces a typed error". The error slot exists; the enforcement does not. A future engine integration that trusts this taxonomy would ship an extension host where a caller can spawn unbounded concurrent instances with unbounded aggregate linear memory — the exact resource-exhaustion class §5.4 exists to prevent. Today's blast radius is zero only because nothing ships this crate (leaf, no dependents).
What correct looks like: Either implement the host scheduler/accounting (a spawn-time check over active instances + Σ memory_bytes producing `HostCapacity`) or delete the variant and mark the two §5.4 rows unimplemented — do not keep a typed error for a ceiling that is not enforced.
Confidence: HIGH — Verification: read (construction census + instance.rs spawn signature + limits.rs). Not executed (no cargo runs allowed, rule 8).
### F-002 [S3]
Where: crates/plugin-host-wasm/src/error.rs:386-412
Code:
```
    fn every_variant_displays_actably() {
        // MP §8: error strings name the rule and the fix path.
        let cases: Vec<HostError> = vec![
            HostError::ApiVersion {
```
(plus, at 407-411:)
```
        for err in cases {
            let text = err.to_string();
            assert!(text.len() > 20, "actionable message: {text}");
            assert!(!text.is_empty());
        }
```
What is wrong: The name says *every* variant; `cases` holds 4 of 20 `HostError` variants (ApiVersion, UndeclaredImport, GuestMemoryOob, ConsentRequired) and 0 of 14 `PackageError` variants. The second assertion (`!text.is_empty()`) is logically dead — anything longer than 20 chars is not empty.
Why it matters: 16/20 HostError and 14/14 PackageError Display arms have zero test coverage. Mutation proof: change the `CpuLimit` arm (251-254) to print `"x"` — the test stays green. The test then functions as false evidence that the whole taxonomy displays actionably (MP §8), which is exactly how it is cited.
What correct looks like: Exhaustively construct one instance of each variant (a `fn sample() -> Vec<HostError>` covering all 20, plus a second loop for all 14 PackageError variants) and assert each message is non-trivial; drop the dead `!is_empty()`.
Confidence: HIGH — Verification: read; mutation reasoned, not run.
### F-003 [S4]
Where: crates/plugin-host-wasm/src/error.rs:359-368
Code:
```
impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HostError::Package(err) => Some(err),
            _ => None,
        }
    }
}

impl std::error::Error for PackageError {}
```
What is wrong: `PackageError` implements `Error` with the default (sourceless) impl while its `Manifest(String)` variant (183) flattens a typed `plugin_api::manifest::ManifestError` into a plain String — the chain stops one level early and the underlying typed error is unrecoverable.
Why it matters: Downstream reporters (`anyhow`-style context walks, log error chains) cannot show why a manifest failed; the typed information exists in plugin-api and is discarded at the boundary (install.rs:566-578 already pays the conversion cost).
What correct looks like: `PackageError::Manifest(Box<plugin_api::manifest::ManifestError>)` + `source()` returning it, mirroring how `HostError::Package` boxes its inner error.
Confidence: HIGH — Verification: read.
### F-004 [S4]
Where: crates/plugin-host-wasm/src/error.rs:13-14
Code:
```
#[derive(Debug)]
pub enum HostError {
```
What is wrong: A public error enum from a library crate without `#[non_exhaustive]`. Adding a variant (which this crate's own doc anticipates — e.g. the §5.4 backfill F-001 requires) is a semver-breaking change for every downstream `match`.
Why it matters: The crate is `publish = false` and currently a leaf, so the blast radius is zero today — but the type is the crate's declared public API surface ("The one error type crossing the host's public API", line 12), designed for the engine integration that doc 06 §3 anticipates.
What correct looks like: `#[derive(Debug)] #[non_exhaustive] pub enum HostError { … }` (and same for `PackageError`), letting future variants land without breaking matches.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HostError` (20 variants) | 14-133 | REAL, 1 dead variant | S3 (F-001) | 19/20 constructed in crate; HostCapacity never (F-001); doc comments cite doc 06 §refs accurately for the rest (spot-checked ApiVersion/§5.5, GuestMemoryOob/§9.4, KindRefused/§5.9 — kind list matches doc's refused set) |
| `PackageError` (14 variants) | 137-211 | REAL | — | All 14 constructed (census); BadMagic carries hex of observed bytes; [PROPOSAL] markers honest (CompressedEntry 163-165) |
| `Display for HostError` | 213-297 | REAL | — | 20 arms, each rule+values+doc-ref; `lumen-allow-literal` escape at 239 documented |
| `Display for PackageError` | 299-357 | REAL | — | 14 arms; TooLarge reads live constant PACKAGE_CAP_BYTES (no drift possible) |
| `Error for HostError` | 359-366 | REAL | S4 (F-003 context) | source→Package only |
| `Error for PackageError` | 368 | SHALLOW | S4 (F-003) | Empty impl; no source; Manifest(String) flattens typed cause |
| `From<io::Error>` | 370-374 | REAL | — | Flattens to Io(String) — same String-flattening pattern as F-003, lower value (io::Error text is usually sufficient); acceptable |
| `From<PackageError>` | 376-380 | REAL | — | Boxes; consistent with `Package(Box<PackageError>)` |
| `every_variant_displays_actably` | 386-412 | SHALLOW | S3 (F-002) | 4/34 variants, dead assertion; green in baseline :4504 |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `every_variant_displays_actably` | Only for 4 of 34 variants, and only if the message drops under 21 chars. | Mutation: `CpuLimit` arm → `"x"` keeps it green; `MemoryLimit` arm → `"oom"` keeps it green. 16 HostError + 14 PackageError arms fully uncovered; `!text.is_empty()` can never fire after `text.len() > 20`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure mode the host can produce is a variant here — never a panic, never a swallowed Result" | header 3-7 | REAL | 19/20 variants constructed; clippy deny + baseline CLEAN (INDEX #5/#6) |
| (inverse) every variant is a failure mode the host can produce | implied by header + F-001 variant doc | FALSE for HostCapacity | 0 constructions; no accounting code (instance.rs spawn, limits.rs, supervisor.rs all lack it) |
| "too many active instances or the total extension-heap budget is exhausted (doc 06 §5.4 host accounting)" | 118-124 | FALSE | Doc 06 §5.4 rows "host scheduler"/"host accounting" have no implementation; numbers marked [PROPOSAL] but the *mechanism* is absent, not just the values |
| MP §8 actionable messages | test comment 388 | REAL (for covered arms) | Display arms traced; but test evidence covers 4/34 (F-002) |

## 7. Dead / duplicated / unreachable code
- `HostError::HostCapacity` + its Display arm (284-286): dead (F-001).
- Whole file unreachable from shipped entry points (leaf crate; only `lumenext`/`lumenext-examples` dev bins and the crate's own lib/tests use it).

## 8. Dependencies & flags
- std only (fmt, path, error, io). No deps, no features. Indirectly depends on `plugin_api::manifest` types at the install.rs boundary (not in this file).
- Crate-level: plugin-host-wasm has no dependents (construction census + Cargo.toml grep) — the "host" in the wave's hostile question is a dev tool, not part of any shipped binary.

## 9. Open questions for the next agent
- instance.rs/limits.rs holder: confirm no spawn-time instance-count check exists anywhere I missed (I traced `spawn` and grepped capacity/budget/active across src — nothing).
- supervisor.rs holder: the escalation/backoff ledger (state.rs ArcLedger) — does it ever limit *concurrent* instances, or only fault/restart policy? If the latter, F-001 stands unchallenged.
- Whoever integrates this crate into an engine entry point (doc 06 §3 card): HostCapacity backfill is a precondition; without it the integration ships unbounded spawning.
