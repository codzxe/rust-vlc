# AUDIT: crates/platform-windows/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 200 / 9 (is_available, from_probe_error, from_error, Display::fmt, key, new, set, get, iter) |
| File verdict | REAL (logic executed on this host: 16/16 platform-windows seam tests pass) — but a byte-identical triplicate and inside an orphan crate |
| Reachable from shipped entry point? | NO — `platform-windows` has zero workspace dependents (rg over all Cargo.tomls: only its own manifest + a comment in platform-linux's) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `cargo test -p platform-windows --test seam_behaviour` (exit 0, 16 passed); diff vs platform-linux and platform-macos copies |

## 1. One-paragraph verdict
The capability-negotiation vocabulary (verdict enum, typed reason taxonomy,
error→reason collapse, dotted feature keys, replace-in-place report table) is
real and correct: every branch of `from_error` is exercised by the host-runnable
seam suite (16/16 pass on this Linux box, including
`capability_classifies_and_converts_probe_errors`), `Display` strings are
pinned by test, and unprobed features default to an honest `NotYetImplemented`
rather than assumed availability. The two structural facts sink it anyway: it
is the third byte-identical copy of the same module (only the card-number
comment differs — verified by diff), and the crate around it is wired to
nothing, so no shipped binary can query this report.

## 2. What actually works (verified)
— `cargo test -p platform-windows --test seam_behaviour` -> 16 passed, exit 0
(0.66s compile). The suite asserts `from_probe_error` mappings
(NotCompiledForThisOs→WrongOs, LibraryUnavailable→LibraryNotFound,
SymbolMissing→LibraryNotFound with symbol in detail,
ServiceUnavailable→ServiceNotReachable, fallback collapse with api string
preserved), `Display` output for all four reasons, and report
set-replaces-not-appends semantics.
— `diff crates/platform-linux/src/capability.rs crates/platform-windows/src/capability.rs`
-> only line 2 differs ("card 7.A2" vs "card 7.A1"); macos copy likewise.

## 3. Findings
F-001 [S2]
Where: crates/platform-windows/src/capability.rs:1-200 (crate context)
Code:
```
//! Capability negotiation: what this build can actually do, honestly
//! (card 7.A1; MP §13 "no invented crate capabilities").
```
What is wrong: `platform-windows` is an orphan crate — the third one (with
platform-linux, platform-macos). No Cargo.toml in the tree depends on it; the
release lane (lumen-cli, desktop) never compiles it. This file is the
capability API of an adapter nothing queries. Confidence: HIGH — Verification:
ran (rg sweep over all manifests). Same finding as W08-A07 files 1-3.
F-002 [S3]
Where: crates/platform-windows/src/capability.rs:1-200 (vs platform-linux:1-201, platform-macos)
Code:
```
impl UnavailableReason {
    /// Collapse a probe error into the reason class it proves.
```
What is wrong: byte-identical triplication of a 200-line module across the
three platform crates (diff shows only the card-number comment line differs).
The lib.rs docs record this as deliberate ("mirrored … for the Lead to unify
into a shared L5 crate later — [PROPOSAL]"), but as shipped, any fix must be
applied three times and the copies can silently drift. Why it matters: the
mirroring rationale is already falsified by drift risk in files this audit
touched (platform-macos/macos/dialogs.rs carries api-name slips the linux copy
does not). What correct looks like: one shared crate (the recorded proposal) or
at least a cross-crate golden test asserting the three vocabularies stay
identical. Confidence: HIGH — Verification: ran (diff x2).
F-003 [S4]
Where: crates/platform-windows/src/capability.rs:43-44
Code:
```
    /// The OS integration this crate carries is not for the running
    /// OS (e.g. `platform-linux` on a macOS host).
```
What is wrong: in the WINDOWS crate's doc, the WrongOs example still names
`platform-linux` — copy-paste residue from the linux template. Misleading doc
in the copy that never got adapted. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Capability::is_available | 26-29 | REAL | — | matches! only Available; tested |
| Capability::from_probe_error | 33-36 | REAL | — | delegates from_error; all 5 mappings tested |
| UnavailableReason::from_error | 75-106 | REAL | — | symbol-keeps-detail rule tested; fallback arm stringifies, never drops |
| Display for UnavailableReason | 109-127 | REAL | — | 4 formats pinned by test |
| Feature::key | 144-152 | REAL | — | dotted keys pinned by test (matrix vocabulary) |
| PlatformReport::new | 164-167 | REAL | — | Default empty |
| PlatformReport::set | 169-180 | REAL | — | replace-in-place pinned (rows==1 test) |
| PlatformReport::get | 184-194 | REAL | — | unprobed → NotYetImplemented, pinned |
| PlatformReport::iter | 196-199 | REAL | — | exercised in linux suite's report test |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| capability_classifies_and_converts_probe_errors (tests/seam_behaviour.rs) | YES | exact-eq/match on every mapping incl. fallback |
| platform_report_is_a_table_with_honest_defaults | YES | append bug → rows>1 fails |
| unavailable_reasons_render_honest_strings | YES | contains() on all 4 Display forms |
| feature_keys_are_the_stable_matrix_vocabulary | YES | key rename fails |
(Suite executed: 16/16 pass — see §2.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "computed from live probes … not from compile-time assumptions alone" | header 6-8 | REAL for the crate design (lib.rs discover does live probes) but this file itself only defines vocabulary; the claim's proof lives in lib.rs | seam tests + discover test pass |
| "an honest gap with a recorded follow-up, never a silent stub" | 65-66 | REAL | NotYetImplemented default is what unprobed get() returns (tested) |

## 7. Dead / duplicated / unreachable code
- F-002: the file IS the duplication (x3).
- No dead functions; all 9 are public API of the crate (and exercised by its
  tests), but the crate itself is dead weight in the release graph (F-001).

## 8. Dependencies & flags
Only crate::error (same-crate sibling). No external deps, no unsafe, no
feature gates, no cfg — host-compiles everywhere (proven: built+tested on
Linux).

## 9. Open questions for the next agent
- Is there a Lead decision recorded anywhere (records/, docs/) accepting the
  triple duplication, or is the [PROPOSAL] still open? If open since card 7.Ax
  with no owner, that is process debt worth flagging at the plan level.
- The windows crate's `windows/` integrations module (WASAPI/DXGI) is not in
  my assignment; check whether it compiles on any lane at all (no windows
  target in sandbox — likely same UNVERIFIED status as the macos FFI).
