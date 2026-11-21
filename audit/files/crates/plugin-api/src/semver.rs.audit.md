# AUDIT: crates/plugin-api/src/semver.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-21 |
| Lines / functions | 322 / 9 (parse, major, minor, patch, precedence, Display::fmt, valid_identifiers, compare_pre, SemverError enum) + 5 tests |
| File verdict | UNREACHABLE (real, correct, unit-tested parser; no shipped entry point executes it) |
| Reachable from shipped entry point? | NO. Chain that exists: `lumenext`/`lumenext-examples` bins (crates/plugin-host-wasm) → `plugin_api::PluginManifest::from_toml` → `SemverField` deserialize (manifest.rs:258-263) → `Semver::parse`. But `rg -l "plugin-host-wasm" --glob **/Cargo.toml` returns only plugin-host-wasm's own manifest — no crate depends on it; apps/cli (`lumen`) and apps/web link plugin-api for the *traits* only (containers/*.rs `use plugin_api::{Demuxer, …}`) and never touch manifest/semver. `Semver::precedence` has zero callers anywhere outside its own tests. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `wc -l` (322); full read; `rg "Semver|precedence\("` repo-wide (callers: manifest.rs:38,250-262 only → plugin-host-wasm only); `rg -l "plugin-host-wasm" --glob **/Cargo.toml` (leaf crate — no dependents); `sed -n` quote verification (26-33, 180-190); baseline orchrun-test-workspace-nofailfast.log.txt:4461-4465 (5 semver tests green) |

## 1. One-paragraph verdict
A hand-rolled SemVer 2.0.0 parser that is, with one exception, genuinely correct against the spec: build-split before pre-split, three-component core with leading-zero and u64-overflow rejection, §9-vs-§10 identifier rules (leading zeros forbidden in pre-release numeric identifiers only), and a faithful §11 precedence including the worked example — all five tests green in the baseline run. The exception is a real spec bug: numeric pre-release identifiers larger than u64::MAX parse fine (nothing bounds their length) but `compare_pre` maps them through `parse::<u64>().unwrap_or(0)`, so `1.0.0-18446744073709551616` compares as `1.0.0-0` — silent wrong ordering plus a swallowed error. Blast radius today is zero because `precedence` has no production caller (the module's own doc marks it a [PROPOSAL] for future dependency resolution), and the entire file is unreachable from any shipped entry point: only the dev-only `lumenext` tooling parses manifests.

## 2. What actually works (verified)
— 5/5 tests green in baseline `orchrun-test-workspace-nofailfast.log.txt:4461-4465` (doc_examples_parse, full_grammar, precedence_rules, rejects_bad_versions, zero_is_legal_but_only_unprefixed).
— Spec conformance traced by hand: build-before-pre split order (parse:44-58) handles `1.2.3-a+b`; core rejects wrong arity (61-63), non-digits (66-68), leading zeros (69-71), >u64 (72-75); `valid_identifiers` enforces §9 non-empty/charset/numeric-leading-zero for pre and correctly does NOT apply the zero rule to build (§10, `is_pre` flag, line 163); `precedence` implements §11 core→pre cascade with build ignored (107-129); `compare_pre` implements numeric<alphanumeric, field-by-field, smaller-set-loses (171-197). The §11 worked example (8-version chain) is asserted in both directions by `precedence_rules`.
— Round-trip: `Display` (132-143) reconstructs the exact input for `2.3.7-alpha.1.x+build.5.b` (test `full_grammar`, baseline :4462).

## 3. Findings
### F-001 [S3]
Where: crates/plugin-api/src/semver.rs:162-186 (bug at 183-186; enabling gap at 148-168)
Code:
```
                let xn = x.bytes().all(|b| b.is_ascii_digit());
                let yn = y.bytes().all(|b| b.is_ascii_digit());
                let ord = match (xn, yn) {
                    (true, true) => x
                        .parse::<u64>()
                        .unwrap_or(0)
                        .cmp(&y.parse::<u64>().unwrap_or(0)),
```
What is wrong: `valid_identifiers` never bounds numeric identifier length, so `"1.0.0-18446744073709551616"` (2^64) parses successfully; `compare_pre` then parses both sides with `.unwrap_or(0)`. An out-of-range numeric pre-release identifier silently compares as 0.
Why it matters: Violates semver.org §11.4 (numeric identifiers compare numerically): `1.0.0-18446744073709551616` orders *below* `1.0.0-1` instead of above, and *equal* to `1.0.0-0`. It is also a swallowed error: `unwrap_or(0)` converts an impossible-parse into a wrong answer with no diagnostic. Latent today (`precedence` has zero callers outside tests; the module doc line 16-18 itself gates it as a [PROPOSAL] for dependency resolution), but this is the exact function a future registry/dependency resolver will call — a resolver built on it picks the wrong version.
What correct looks like: Compare all-digit identifiers as decimal strings: `(x.len(), x).cmp(&(y.len(), y))` (longer digit-string is larger; equal length compares lexicographically) — no parse, no overflow, spec-exact for arbitrary sizes.
Confidence: HIGH — Verification: read (trace: valid_identifiers 148-168 has no length/overflow check; parse accepts the 20-digit pre; unwrap_or(0) then misorders). Not executed — no cargo runs allowed (rule 8); would need a one-off test.
### F-002 [S4]
Where: crates/plugin-api/src/semver.rs:29-32
Code:
```
    /// Dot-separated pre-release identifiers, with the leading `-`.
    pre: Option<String>,
    /// Dot-separated build identifiers, with the leading `+`.
    build: Option<String>,
```
What is wrong: The stored strings do NOT include the leading separators. `split_once('-')` (line 52) returns the after-separator text (`"alpha.1"`, no hyphen) and that is what line 81 stores; `Display` re-adds the `-`/`+` (136, 139). Both field comments describe the opposite representation.
Why it matters: A maintainer using `semver.pre` directly (private field today, but the comment is the contract for anyone extending this module — e.g. adding an accessor) will concatenate a doubled or missing separator.
What correct looks like: `/// Dot-separated pre-release identifiers (stored WITHOUT the leading `-`; Display re-adds it).` — or store with the separator and fix Display.
Confidence: HIGH — Verification: read (lines 52-54, 77-83, 135-140 traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Semver::parse` | 41-84 | REAL | — | Correct spec order (build→pre→core); rejects arity/digits/leading-zero/overflow; no panics, no unwrap; allocates 1 Vec + up to 2 Strings per parse (fine — manifest-load path, not hot) |
| `Semver::major/minor/patch` | 88-100 | REAL | — | `const fn` accessors; trivial, correct |
| `Semver::precedence` | 107-129 | REAL (spec bug in delegate) | S3 (F-001) | Core cascade correct (108-122); pre/absence rules correct (123-128); delegates to buggy `compare_pre` |
| `Display::fmt` | 132-143 | REAL | — | Round-trip verified by tests; re-adds separators the storage omits (see F-002) |
| `valid_identifiers` | 148-168 | REAL | — | §9/§10 correct incl. pre-only leading-zero rule (163); no length bound on numeric identifiers — the enabler of F-001 |
| `compare_pre` | 171-197 | REAL with spec defect | S3 (F-001) | numeric<alphanumeric correct (187-188); ASCII lexical `x.cmp(y)` correct (189); set-size rule correct (176-178); u64 parse+`unwrap_or(0)` wrong for >u64 numerics |
| `SemverError` (6 variants) | 201-223 | REAL | — | thiserror messages name the rule and the offending text; actionable per MP §8 |
| tests (5) | 229-322 | REAL | — | See §5 |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `doc_examples_parse` (229-238) | Yes | Round-trips 3 doc versions; breaks if parse or Display drifts |
| `full_grammar` (240-248) | Yes | Composite version: accessor values + exact round-trip |
| `rejects_bad_versions` (250-274) | Yes | 18 invalid forms incl. `v`-prefix, spaces, `1.2.3.4`, empty pre/build, `..`, `_`, leading zeros. Gap: no >u64 core overflow case (the `Overflow` arm 72-75 is untested) and no >u64 pre-release case — the latter is why F-001 survives green |
| `precedence_rules` (276-313) | Yes | semver.org §11 worked example both directions + build-metadata-ignored + `assert_ne!` on Eq distinguishing builds. Mutation check: swap the `(true,false) => Less` arm → `1.0.0-alpha.1 < 1.0.0-alpha.beta` assertion fails |
| `zero_is_legal_but_only_unprefixed` (315-321) | Yes | `0.0.0` legal, `0.0.0-00` rejected — pins the zero edge |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "required, semver 2.0 string" validation for manifest `version` | doc 06 §2.2 via header 1-2 | REAL | manifest.rs:258-263 routes `version` through `Semver::parse`; tests green (baseline :4461-4465) |
| "Grammar implemented (semver.org, §1–§9)" | header 7-14 | REAL (with F-001 exception) | Traced parse/valid_identifiers against §9/§10; §11.4 numeric comparison broken beyond u64 |
| "Hand-rolled because the `semver` crate is not in this card's C5 grant" | header 4-5 | REAL | plugin-api Cargo.toml: no semver dep (serde/toml/thiserror + 2 path deps only) |
| "Precedence … exposed for the registry's same-major comparisons [PROPOSAL]" | 16-18 | REAL-UNPROVEN→actually UNUSED | `precedence` has zero non-test callers repo-wide (`rg "precedence\("`); the registry (doc 06 §3) was never built — cross-ref lib.rs F-001 (no plugins/registry exist) |

## 7. Dead / duplicated / unreachable code
- `Semver::precedence` + `compare_pre` (~45 LOC): dead in production — only tests call them (repo-wide grep). Kept deliberately as contract surface ([PROPOSAL] header 16-18); flagging as dead-by-design, not removable noise.
- Whole file unreachable from shipped entry points (see reachability row): `lumen` CLI and apps/web never parse plugin manifests; only `lumenext`* dev bins do.

## 8. Dependencies & flags
- No deps of its own; uses std only + thiserror derive (crate-level dep). No feature gates.
- Compiled into plugin-api which 23 workspace manifests depend on — but the semver/manifest surface is link-dead for all shipped consumers (traits are what they use).

## 9. Open questions for the next agent
- manifest.rs holder: does `SemverField`'s Deserialize surface the `SemverError` text losslessly (manifest.rs:258-263)? I only traced the call site.
- Whoever audits the (unbuilt) registry/dependency-resolution cards: F-001 must be fixed before `precedence` is consumed; a resolver on top of the current `compare_pre` silently downgrades huge-numeric pre-releases.
