# AUDIT: tools/xtask/src/cmd/check_panics.rs
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1093 / 19 (11 prod fns + 8 unit tests + `Must` trait) |
| File verdict | REAL (scanner works, executed) — but it is a DEAD GATE (wired into nothing) that currently FAILS its own tree |
| Reachable from shipped entry point? | NO (dev tool; not in the lumen-cli/desktop release lane). YES from `cargo xtask check-panics` via cmd/mod.rs:108-114 — executed |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | `./target/debug/xtask check-panics` → exit 1, 2 violations; `./target/debug/deps/check_panics-bd5556924084c4c8` → 4/4 pass; rg counts (see below) |

## 1. One-paragraph verdict
The scanner itself is real and good: a hand-written char-level lexer (comments/strings/raw strings/char-vs-lifetime), brace-tracked `#[cfg(test)]` regions, file-level test-mod resolution, path+message-scoped cited invariants, and exit codes 0/1/2. I executed it and it classified 826 panic-family matches across 260 files in the 9 engine crates (779 test-region, 43 contract-module, 3 cited-invariant allowed) and correctly flagged 2 real non-test violations in `crates/demux/src/ts/mod.rs`. But it is a gate in name only: **nothing runs it** — not `scripts/full_gate.sh` (11 sections, no check-panics), not the xtask `gate` subcommand (steps: fmt/check/clippy/test/tree/ui-lint/ui-typecheck/ui-test/tauri/wasm-pack), not any `.github/workflows/*`. The worklog admits the wiring was handed off and never done, while the same entry claims violations "now fails check-panics in CI" — false. Consequence proven by execution: the tree drifted from "green" (2026-10-24, worklog) to FAIL today, and nobody noticed because the only thing that ever runs this code is `cargo test -p xtask` (its unit tests), which does not scan the real tree.

## 2. What actually works (verified)
- `./target/debug/xtask check-panics` (repo root) → `check-panics: 260 files scanned across 9 crates`, `1587 cfg-test regions, 12 file-level test-only mod files`, `FAIL — 2 violation(s)`, exit 1. Counts from the saved output: 779 `allowed (cfg-test region)` + 43 `allowed (contract module)` + 3 `allowed (cited invariant)` + 2 violations = 826.
- `./target/debug/deps/check_panics-bd5556924084c4c8` → 4/4 integration tests pass (seeded violation exit 1; sanctioned zones exit 0; wrong-message citation fails; missing crate exit 2).
- Unit tests: 8/8 pass — baseline `audit/runs/baseline/cargo-test-workspace.txt:6583-6590`.
- All 3 CITED_INVARIANTS sites exist verbatim: `crates/audio-engine/src/pipeline.rs:435`, `crates/codecs/src/audio/pcm/decoder.rs:330`, `crates/playback-engine/src/recovery.rs:545` (`retrying()` at :444, pinning comment :544).
- Contract modules are cfg-gated out of release: `crates/input/src/lib.rs:64` `#[cfg(any(test, feature = "contract"))] pub mod contract;` (same at `crates/media-probe/src/lib.rs:160-161`).
- Documented blind spots verified EMPTY in scope: 0 `::unwrap(`/`::expect(` free calls, 0 spaced `.unwrap (`, 0 `unreachable_unchecked` in the 9 crates' src/ trees (python count).

## 3. Findings
F-001 [S2] The "CI gate" is wired into nothing — dead code with a gate's name.
Where: tools/xtask/src/cmd/check_panics.rs:13-17; tools/xtask/src/cmd/mod.rs:108-114; scripts/full_gate.sh; .github/workflows/*
Code (module doc, verbatim, lines 13-17):
```
//! `unwrap_used` entirely. The 27 non-test `unreachable!` sites the
//! audit counted therefore passed the gate by lint blind spot, not by
//! proof. This check encodes the audit's classification as CI: a
//! grep-based scan over the nine engine crates' `src/` trees that
//! fails on any panic-family form outside the sanctioned zones.
```
What is wrong: "encodes … as CI" is false. Grep of the whole repo for `check-panics`/`check_panics` outside tools/xtask and audit/: zero invocations. `scripts/full_gate.sh` calls xtask for check-architecture, fixtures, lint-literals, check-identity, sbom, web-budget only. The xtask `gate` step table (gate.rs:188-280) has no check-panics step. No workflow file mentions it. The worklog records both the false claim and the dropped handoff:
```
Who hits it first? — anyone adding a panic-family macro outside the sanctioned zones now fails check-panics in CI (previously passed silently through the clippy blind spot).   (docs/worklog/archive-2026-10.md:8155)
(2) Lead: consider wiring check-panics into `xtask gate` — I did NOT touch gate.rs (not my ownership).   (docs/worklog/archive-2026-10.md:8156)
```
Why it matters: an unwired gate provides zero protection while its existence satisfies readers of the docs. What correct looks like: a step in `xtask gate` + scripts/full_gate.sh + a CI lane, or deletion of the "gate"/"CI" language.
Confidence: HIGH — Verification: ran (tool) + read (all wiring sites).

F-002 [S2] The tool FAILS its own tree today (2 violations in shipped-reachable demux code) — direct proof the dead gate let the tree rot.
Where: crates/demux/src/ts/mod.rs:368-372 (flagged by the tool at :369, :372)
Code (verbatim):
```
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
```
What is wrong: non-test `unreachable!` in `TsStream::descriptor()`, reachable from the shipped CLI: apps/cli/src/containers/ts.rs:84 `use demux::ts::{PesConsumer, TsDemuxer};` → descriptor() → these arms. Worklog:8154 records "check-panics green on tree" at F-5 close (2026-10-24); my run today: exit 1. The tool's own policy (CITED_INVARIANTS = "a deliberate, reviewable edit") was bypassed: whoever added these sites never cited them, and no lane ever noticed. The demux sites themselves are the demux owner's problem (cross-ref); here the finding is that a passing-then-failing gate with no executor demonstrates the gate is decorative.
Confidence: HIGH — Verification: ran (exit 1, exact lines in output).

F-003 [S3] Scope: 9 of 39 src trees — the shipped entry-point crates are outside the net.
Where: check_panics.rs:77-87 (SCANNED_CRATES), :45-49 (scope doc)
Code (verbatim):
```
//! - Only the nine engine crates' `src/` trees (the audit's domain);
//!   integration tests under `tests/` are test context by definition.
//!   Extending the scope is a one-line change to [`SCANNED_CRATES`].
```
What is wrong: the census headline "4126 unreachable!" (audit/runs/baseline/loc-patterns.txt:28) is raw substring counting (comments, strings, tests included). Real picture, measured: 4248 raw tree-wide; 1454 raw in the 9 scanned crates' src/; 826 string/comment-stripped matches in scope; only 5 of those are non-test non-contract (3 cited + 2 violations). Meanwhile the ratchet baseline (docs/audit/panic-baseline.json, 2026-11-17) counts 41 shipped panic sites tree-wide — of which apps/desktop (4), apps/cli, streaming, playlists (6), control-protocol (4), plugin-host-wasm, transcode, testkit, tools/xtask (10) are all OUTSIDE this tool's scope. The release lane's own app crates are unscanned by the "panic-surface gate". Documented honestly in the limits section, but the title ("the panic-surface gate") oversells a 9-crate slice.
Confidence: HIGH — Verification: ran (rg counts + tool output + baseline json).

F-004 [S3] Two competing panic gates with contradictory verdicts; the ratchet froze the very violations this tool flags.
Where: scripts/quality/24_panic_budget.sh:1-8, docs/audit/panic-baseline.json
Code (verbatim):
```
# 1,827 shipped `unreachable!` in a media engine are not style debt, they are
# 1,827 "attacker sends bytes -> process aborts" sites.
```
What is wrong: (a) the ratchet's own baseline records grand_total 41, not 1,827 — its header comment is false against its own data (stale from a pre-fix method; its scrub docstring admits an order bug "silently deleted 23 unreachable! from input/src/contract.rs"). (b) The ratchet froze `crates/demux: unreachable 2` in the baseline — exactly the 2 sites check-panics flags — so CI's only live panic gate PASSES what this tool FAILS. Two gates, disjoint scopes, opposite verdicts, no documented authority. (c) The ratchet also counts contract-module sites (input=5, media-probe=1) as "shipped" though they are cfg-gated out of release — overcounting the thing it exists to count. Not this file's code, but this file's raison d'être collides with it.
Confidence: HIGH — Verification: read (script + json) + ran (this tool).

F-005 [S3] Third re-implementation of the same lexer/region-tracker inside the toolchain.
Where: check_panics.rs:431 `fn scan_source`; tools/xtask/src/cmd/lint_literals.rs:828 `pub(crate) fn scan_source`; scripts/quality/24_panic_budget.sh (python scrub/strip)
What is wrong: the comment/string stripper + cfg(test) region tracker is implemented twice in xtask (check_panics, lint_literals — the latter's comments cite "the check_panics brace-tracking model", "the check_panics predicate, verbatim") and a third time, weaker and with a known bug class, in the ratchet script. Three scanners can drift on what "test context" means; today they already disagree on scope and verdicts (F-004). Correct looks like: one shared scanner module with per-gate policies.
Confidence: HIGH — Verification: read (all three).

F-006 [S4] assert-family doc claim is stale and slightly false.
Where: check_panics.rs:55-57
Code (verbatim):
```
//! - `assert!`/`assert_eq!` (the assert family) are NOT scanned — the
//!   audit kept them informational; the two contract modules carry
//!   all 43 non-test uses.
```
What is wrong: measured with a correct lexer (comment/string-stripped, test-region aware, tests.rs mod files excluded): input/contract.rs=41, media-probe/contract.rs=4 (45, not 43), plus 12 more non-test asserts outside the contract modules in `codecs/src/video/h264/hw/videotoolbox/ffi.rs` (6), `vaapi/ffi.rs` (5), `vaapi/library.rs` (1) — all `const _: () = assert!(...)` compile-time layout pins, so not runtime panic surface, but "all 43" is false as written.
Confidence: HIGH — Verification: ran (python count, spot-checked sites).

F-007 [S4] Contract-module allowance is path-based, not cfg-verified.
Where: check_panics.rs:37-38, 92-95, 354
Code (verbatim):
```
//!    and `media-probe/src/contract.rs` (audit §1.8/§1.9; F-2 keeps
//!    them out of release builds — the scan allows them by path
//!    regardless of feature state).
```
What is wrong: if someone deletes the `#[cfg(any(test, feature = "contract"))]` gate from either lib.rs, these files compile into release with 43 panic sites and this tool stays green. The allowance trusts a cfg it never reads. Correct: verify the cfg gate at the `mod` declaration (the scanner already parses attributes).
Confidence: HIGH — Verification: read.

F-008 [S4] Self-scan exemption: the checker's own file is full of the forms it polices, by construction outside its scope.
Where: check_panics.rs:139-146 (FORMS), 21-26 (doc)
What is wrong: census loc-patterns.txt credits this file with 6 `unwrap(`, 2 `todo!`, 2 `unimplemented!` — all string literals/doc text (verified: rg hits at :21,:25,:140,:983,:996,:1014,:1066,:1073 and :144-145). Harmless, but it means raw-substring census numbers (like 4126) must never be quoted as "shipped panic sites"; this tool's stripped matching is the honest count.
Confidence: HIGH — Verification: ran (rg) + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 153-198 | REAL | — | arg parse (--help/--root), exit 0/1 mapping; executed |
| USAGE | 201-212 | REAL | — | matches behavior |
| Match | 219-227 | REAL | — | data |
| Allowance + label | 230-248 | REAL | — | data |
| Outcome + print | 251-304 | REAL | — | report printing; verified in run output |
| rel | 307-312 | REAL | — | path formatting |
| scan_workspace | 315-394 | REAL | — | 2-pass scan/classify; missing crate → error (tested, exit 2) |
| FileScan | 401-416 | REAL | — | data |
| Mode | 419-428 | REAL | — | lexer modes |
| scan_source | 431-660 | REAL | — | the lexer; unit-tested; handles nested block comments, raw strings, lifetimes, string-continuation newlines |
| looks_like_raw_string | 663-678 | REAL | — | guards `br#"` tail-of-identifier case |
| char_literal_len | 682-701 | REAL | — | char vs lifetime; escape bound 12 |
| capture_attribute | 708-753 | REAL | — | bracket counting, verbatim strings |
| is_test_attribute | 761-817 | REAL | — | parsed, not substring; `cfg(not(test))` correctly excluded (unit-tested) |
| resolve_mod_file | 822-846 | REAL | — | Rust 2018 mod resolution; tested |
| collect_rs_files | 850-865 | REAL | — | sorted recursive walk |
| find_workspace_root | 869-892 | REAL | — | walk-up `[workspace]` probe |
| find_matches | 899-954 | REAL | — | token-boundary macro forms; method forms need `.`+`(` |
| tests (8) | 961-1093 | REAL | — | see §5 |
| Must trait (tests) | 1080-1092 | REAL | — | test-only `unreachable!` escape; the workspace's sanctioned pattern |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/test_attribute_shapes | YES | asserts 6 positive and 6 negative attribute shapes |
| tests/stripping_removes_doc_examples_and_strings | YES | doc comment + string containing forms must yield 0 matches |
| tests/cfg_test_regions_cover_their_items | YES | production line must NOT be covered; region must cover the unwrap; history note in-file: it was written wrong and never executed pre-handoff (worklog:8146), then fixed to pin intent |
| tests/file_level_test_mod_marks_the_file | YES | mod-file resolution + whole-file test context |
| tests/pub_cfg_test_mod_declaration_is_recognized | YES | exact region bounds (2,4) |
| tests/inner_cfg_test_whole_file | YES | `#![cfg(test)]` |
| tests/macro_forms_need_token_boundaries | YES | `my_panic!`/`Option::unwrap(x)` must not match |
| tests/raw_strings_and_lifetimes_do_not_confuse_the_scanner | YES | raw string + lifetimes |
| integration: seeded_non_test_site_fails | YES | real binary, scratch workspace, exit 1 + exact site in report (ran: pass) |
| integration: sanctioned_zones_pass_clean | YES | exit 0 on all four zones (ran: pass) |
| integration: cited_invariant_allowance_is_scoped_to_the_message | YES | right path + wrong message must FAIL — the anti-blanket-pass proof (ran: pass) |
| integration: a_missing_scanned_crate_is_an_error_not_a_scope_shrink | YES | exit 2 (ran: pass) |
Gap: NO test covers the tool on the real tree (a smoke assertion "real tree is green" would have caught F-002's drift immediately); no test for `--help`.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "encodes the audit's classification as CI" | module doc L15 | FALSE | no CI lane/gate script invokes it (F-001) |
| "the panic-surface gate" | cmd/mod.rs:111 summary | FALSE-in-effect | unwired; tree currently FAIL (F-001/F-002) |
| "Only the nine engine crates' src/ trees… one-line change to SCANNED_CRATES" | L45-49 | REAL | scope verified by run (260 files, 9 crates) |
| "assert family… two contract modules carry all 43 non-test uses" | L55-57 | FALSE (stale/slightly) | 45 in contract modules + 12 const-layout asserts elsewhere (F-006) |
| "Exit code: 0 no violations | 1 violations | 2 xtask error" | L66 | REAL | all three observed (ran: 0 in fixture test, 1 on tree, 2 in fixture test) |
| "check-panics green on tree" (2026-10-24) | worklog:8154 | STALE→now FALSE | my run: exit 1, 2 violations |

## 7. Dead / duplicated / unreachable code
- The whole command is unreachable from any shipped lane by design (dev tool) — acceptable — but also unreachable from any gate lane, which is NOT by design (F-001).
- Scanner duplicated in lint_literals.rs (F-005) and reimplemented (weaker) in scripts/quality/24_panic_budget.sh.

## 8. Dependencies & flags
- std-only (no external deps; tools/xtask/Cargo.toml has none). No feature flags. `--root` exists solely for the fixture tests (documented, used).
- Relies on the workspace clippy deny set (unwrap_used etc.) for the lint half of the posture; the clippy blind spot motivation (unreachable! not covered by `panic` lint) is plausible per the doc and matches rustc behavior (unverified claim about the throwaway-crate experiment — docs/audits/error-surface-20261023.md §0; UNVERIFIED: did not re-run the experiment).

## 9. Open questions for the next agent
- Who owns the 2 demux violations (ts/mod.rs:369,372) — demux auditor: are they provably dead (can `CodecDescriptor::new` reject "ts-stream-0xNN"?) or a real input-triggered panic path?
- scripts/quality/24_panic_budget.sh (another agent's domain): its header "1,827" vs baseline 41, its comment-scrub order bug class, and its counting of cfg-gated contract sites as "shipped" deserve their own audit row.
- Does any doc besides the worklog claim check-panics runs in CI (docs/audits/error-surface-fixes-20261024.md)? Extract its claims.
