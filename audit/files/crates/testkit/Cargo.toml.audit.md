# AUDIT: crates/testkit/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-21 |
| Lines / functions | 26 / 6 config units (package ×5 fields, lints) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — by design and by enforcement: `cargo xtask check-architecture` rule 5 `testkit-direction` ("testkit only ever in [dev-dependencies]", check_architecture.rs:16,109,1713) ran in baseline INDEX #11 without a testkit violation; all 26 consumer manifests declare it under `[dev-dependencies]` (verified individually). Its code executes only inside test lanes (baseline #3: 1681 workspace tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read crates/testkit/Cargo.toml; loop over all 26 consumer Cargo.tomls classifying the section testkit appears in (all [dev-dependencies]; 4 needed manual re-inspection — streaming:57, packet-queue:31, plugin-host-wasm:71 under [dev-dependencies] header at :59, first-vertical:40 under :39); rg "testkit" tools/xtask/src/cmd/check_architecture.rs (rule implemented at :1713); rg "pub fn/trait/struct" crates/testkit/src/*.rs (surface enumerated); rg "^use" crates/testkit/src (no external crates — std-only confirmed); wc -l (2,093 lines total). |

## 1. One-paragraph verdict
The manifest's central contract — testkit is dev-only, no production crate may depend on it, std-only with zero dependencies — is true, verified, and mechanically enforced. Every one of the 26 consuming manifests wires it under `[dev-dependencies]` (including the four whose comment placement hides the section header from a naive grep: streaming, packet-queue, plugin-host-wasm, first-vertical), the architecture gate implements a dedicated `testkit-direction` check that ran in the baseline, and the crate's sources import nothing outside std/core. The single defect is staleness: the header still describes a v0 crate of "assert + fixture-path helpers, virtual-time trait stub, evidence-recording helper" whose heavy drivers "land with the Phase-2 engine crates", while the crate it fronts is now 2,093 lines including a 1,261-line fault-injection driver suite and the global-alloc counting machinery — the comment undersells both the crate and its enforcement story.

## 2. What actually works (verified)
— Direction contract: all 26 consumers verified `[dev-dependencies]` (media-types/Cargo.toml:25-29 is the documented first consumer, exactly as claimed); `check_testkit_direction` exists (check_architecture.rs:1713) and baseline INDEX #11 (`cargo xtask check-architecture`, exit 1) reports a single unrelated edge violation (lumen-cli→streaming) — the testkit rule did not fire, i.e. it ran clean.
— Std-only claim: no `[dependencies]`/`[dev-dependencies]` sections exist in this manifest at all, and rg over src/*.rs finds no `use` of any external crate — the claim is structurally guaranteed.
— The v0 API named in the header all exists: `Must` trait + assert helpers (lib.rs:44,79,89,100), `fixture_path`/`fixture_bytes` with existence checking (lib.rs:235,245), `VirtualClock` trait + `StepClock` (lib.rs:261,274), oracle/evidence helpers (lib.rs:124,145).
— The crate's alloc-counting tooling demonstrably measures real allocations: baseline INDEX #3's single failure (`apps/desktop/tests/zero_alloc_transport.rs:98`, 608 bytes allocated) is this tooling catching a false zero-alloc claim — the instrument works, the claim it policed did not.

## 3. Findings

### F-001 [S4]
Where: crates/testkit/Cargo.toml:6-13
Code:
```
# the manifest grep recorded in the 1.C3 worklog entry + the dev-dep
# wiring in crates/media-types (the first consumer).
#
# v0 scope (card 1.C3): assert + fixture-path helpers, the virtual-time
# trait stub, the evidence-recording helper. The heavy drivers
# (VirtualClock impl, ScriptSource, sinks, SessionHarness, instruments)
# land with the Phase-2 engine crates they drive (doc 03 §8).
```
What is wrong: Two stale statements. (a) "until then the proof is the manifest grep" — the `check-architecture` gate it calls "card 1.E1 (until then...)" has existed for a long time and enforces the direction rule mechanically (check_architecture.rs:1713, run in baseline #11); the comment still frames enforcement as prospective. (b) The "v0 scope" list and "heavy drivers land with the Phase-2 engine crates" no longer describe the crate: src/faults.rs (1,261 lines) IS a heavy driver suite (FlakySource with scripted faults, DeviceLossSimulator, ClockJumpInjector — the "ScriptSource"-class machinery the comment says lives elsewhere), and src/alloc_count.rs (328 lines) adds the CountingGlobalAlloc instrument. lib.rs:270-285 even ships a VirtualClock *implementation* (StepClock), which the comment classifies as a deferred driver.
Why it matters: The manifest is the crate's contract page; it now misstates both what the crate contains and what enforces its boundaries. A reader checking "is testkit still the thin v0 helper?" gets the wrong answer.
What correct looks like: refresh the header: scope = v0 helpers + fault-injection drivers + alloc counting; enforcement = `cargo xtask check-architecture` rule `testkit-direction` (live, baseline-cited).
Confidence: HIGH — Verification: read (sources and gate code).

## 4. Per-key table (config units)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments (direction contract, v0 scope, std-only) | 1-16 | REAL | S4 | Direction contract TRUE + enforced; v0 scope stale (F-001); std-only TRUE |
| [package] name = "testkit" | 18-19 | REAL | — | Matches all 26 dev-dep path references |
| version = "0.1.0" | 20 | REAL | — | Crate-local |
| edition = "2024" | 21 | REAL | — | Toolchain 1.99.1 supports |
| description | 22 | REAL | — | Says "drivers land with Phase-2 engine crates" — same staleness as F-001, same fix |
| publish = false | 23 | REAL | — | Correct |
| [lints] workspace = true | 25-26 | REAL | — | Inherits deny set; testkit sources comply (no unwrap/expect outside Must helper pattern; lib.rs:72-78 documents the lint-legal panic routing) |

## 5. Test honesty (tests for this crate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crates/testkit/tests/alloc_count_e2e.rs | not read (out of scope for the manifest) | UNVERIFIED — for the testkit source auditor |
| Downstream proof: apps/desktop/tests/zero_alloc_transport.rs (uses testkit CountingGlobalAlloc) | YES (instrument) | Baseline #3: it FAILED with a real measurement (608 bytes) — the counting instrument demonstrably detects allocations rather than rubber-stamping zero |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NO production crate ever depends on testkit — only [dev-dependencies]" | Cargo.toml:3-5 | REAL | 26/26 consumer manifests verified dev-only; check_architecture.rs:1713 rule ran clean in baseline #11 |
| "the dev-dep wiring in crates/media-types (the first consumer)" | Cargo.toml:7-8 | REAL | crates/media-types/Cargo.toml:25-29 |
| "Std-only: zero dependencies of any kind" | Cargo.toml:15-16 | REAL | No dep sections; src imports std/core only |
| "The heavy drivers ... land with the Phase-2 engine crates" | Cargo.toml:11-13 | FALSE (stale) | faults.rs (1,261 lines) + alloc_count.rs (328) are in-crate drivers — F-001 |
| "it is a dev tool, nothing to audit (C5)" | Cargo.toml:15-16 | MISLEADING | Dependency-wise true; but 2,093 lines of test infrastructure whose correctness gates the meaning of every zero-alloc/fault-injection claim in the repo — the devil audit audits it (assigned to the testkit source auditor) |

## 7. Dead / duplicated / unreachable code
- None in the manifest. The crate itself is dev-only by design (not "dead"): its UNREACHABLE-from-production status is the enforced contract, not a defect.

## 8. Dependencies & flags
- Zero dependencies of any kind (no [dependencies], no [dev-dependencies], no features). `[lints] workspace = true`. deny.toml's wildcard-path note (media-types→testkit path edge) is consistent with the dev-dep wiring.

## 9. Open questions for the next agent
- lib.rs:10's own header ("VirtualClock *implementation*, ScriptSource ... land with Phase-2" — read the full sentence when auditing lib.rs) vs the shipped StepClock/FlakySource: same staleness as F-001, confirm from the source side.
- The `Must` trait (lib.rs:44) is the lint-legal unwrap escape hatch for the whole workspace's tests (workspace denies unwrap/expect/panic) — its misuse would quietly reintroduce what the lints ban; worth a look by the testkit source auditor.
- crates/plugin-host-wasm/Cargo.toml:73-83 declares TWO [[bin]] targets (`lumenext`, `lumenext-examples`) — third and fourth workspace bins beyond theme-tokens/sweep/mem-audit; the ci.yml "one crate with a [[bin]]" canary rationale is now triply stale (recorded in my theme-tokens audit F-002).
