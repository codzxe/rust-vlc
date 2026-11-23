# AUDIT: crates/input/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 76 / 0 functions — 6 module decls, 5 re-export groups |
| File verdict | REAL — accurate crate root; honest scope fence |
| Reachable from shipped entry point? | YES — `apps/cli/Cargo.toml:54` → `demux_host.rs:394,419,422` (`FileAccess::new().open`, `resolve_file`) on the file lane of `lumen open` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `cargo test -p input` → 33+5+9 = 47 passed/0 failed (exit 0); rg scheme rejection in uri.rs; rg dependents |

## 1. One-paragraph verdict
The one clean, REACHABLE file in this assignment. The crate root declares the
local-byte-source scope, refuses network schemes typed (verified: uri.rs:58-59
returns `scheme_not_served` for every non-`file` scheme — matching the CLI trace
where http goes to the streaming lane and never silently mis-opens), and keeps
the panicky conformance vocabulary (`contract`) behind `#[cfg(any(test, feature
= "contract"))]` so release binaries carry none of it — the release lane builds
`-p lumen-cli -p desktop` with no features, so the gate holds. All 47 tests
pass (my run). No findings; below is exactly what was checked.

## 2. What actually works (verified)
— verified by `cargo test -p input` → 47 passed; 0 failed (exit 0).
— reachability: rg `input::` in apps/cli/src/demux_host.rs → `FileAccess::new()`,
`resolve_file` on both the path lane and the `file:`-URL lane of the shipped
open path.
— scheme fence: rg in crates/input/src/uri.rs → non-file scheme refused with the
shared `plugin_api::access::scheme_not_served` vocabulary.

## 3. Findings
None. Checked, with method:
1. Module existence — ls src/: buffered.rs, contract.rs, error.rs, file.rs,
   seekable.rs, uri.rs all present; re-exports compile (47-test run builds lib).
2. "Network protocols… rejected here with typed errors, never silently
   mis-opened" (lib.rs:4-7) — verified in uri.rs:58-59 (`scheme != "file"` →
   `scheme_not_served(uri)`), and the CLI open path indeed routes http/https to
   `streaming` (demux_host.rs:427) with every other scheme refused (L428-438).
3. "contract — Test/conformance-only… release binaries carry none of it"
   (lib.rs:29-34) — verified cfg gate at lib.rs:64-65/72-73 and Cargo.toml
   `contract = []` feature; release lane enables no features (baseline
   cargo-build-release-lumen-cli.txt builds without --features).
4. "No registry hook yet" (lib.rs:52-54) — rg for `register` in src/: only the
   doc mention; honest gap, not a stub pretending.
5. Crate deps — plugin_api + media_types (+ std); no I/O beyond std::fs in the
   declared scope (file.rs is another agent's file; the fence claim is the
   lib.rs statement, which holds for uri.rs).
UNVERIFIED (not this file's statements, listed for completeness): buffered.rs's
"allocated once and never grows… hard ceiling… ResourceLimit" claims —
other agent's file; not checked here.

## 4. Per-function table
No functions in this file. Item inventory:
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| crate docs | 1-62 | REAL | scope, decisions, limitations all verified or honest-gapped |
| pub mod buffered/error/file/seekable/uri | 63, 66-69 | REAL | files exist |
| cfg-gated contract mod + re-export | 64-65, 72-73 | REAL | correctly excluded from release |
| re-exports | 71-76 | REAL | compile + tests prove |

## 5. Test honesty
No tests in this file; crate suites 47/47 pass (my run).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Network protocols… rejected here with typed errors" | lib.rs:4-7 | REAL | uri.rs:58-59 + CLI routing demux_host.rs:417-438 |
| "release binaries carry none of [contract]" | lib.rs:29-34 | REAL | cfg gate; release lane featureless |
| "No registry hook yet" | lib.rs:52-54 | REAL | rg: no register fn in src/ |
| "Truncation is a typed error, never a silent EOF" | lib.rs:48-51 | UNVERIFIED | buffered.rs/file.rs behavior — other agent's file |

## 7. Dead / duplicated / unreachable code
None in this file. (`contract` is intentionally gated, not dead.)

## 8. Dependencies & flags
plugin_api, media_types; `contract = []` feature (unused by release). Workspace
lints. testkit dev-only.

## 9. Open questions for the next agent
- buffered.rs: verify the fixed-capacity/no-growth and MAX_READ_BYTES refusal
  claims (the demux arena depends on them).
- file.rs: symlink canonicalization + percent-decoding edge cases (UNC, drive
  letters) vs the mirrored rules in streaming/src/schemes.rs classify_file_uri.
