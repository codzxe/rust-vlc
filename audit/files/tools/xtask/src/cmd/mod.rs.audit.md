# AUDIT: tools/xtask/src/cmd/mod.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 263 / 2 (lookup, names; 4 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask` → `main::dispatch` → `cmd::lookup(name)` → `(sub.run)(&args[1..])`, mod.rs:196-198, 124-125 in main.rs) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read full file; `grep -l '^pub fn run' tools/xtask/src/cmd/*.rs` (17 modules, all registered names resolve to real `run` fns; toml_lite is the only library-only module); `sed -n '70,79p;195,203p'` quote verification; `grep -n NotYetImplemented src/` (no stub path remains). All read-only. |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

mod.rs claims to be the FROZEN subcommand registry — a fixed 17-entry table mapping names to owning cards and real entry functions. What it actually is: a genuine, fully-resolving dispatch table (every `run:` field points at an existing `pub fn run`; I verified all 17 files, no dead commands, no stubs), so as a dispatcher it is REAL and nothing here hard-codes results. What it is NOT: frozen. The header's "later cards must not edit this file" rule was superseded by at least 9 in-file self-justified registrations ("same granted-registry precedent"), so the freeze is decorative and the file's own comment trail documents each violation. That does not fabricate output, but it does mean any claim of "the dispatch table is frozen, trust its stability" is false in practice.

## 2. What actually works (verified) — with the command you ran and its result

- Registry resolution: `SUBCOMMANDS` (72-193) contains 17 entries; `grep -l '^pub fn run' tools/xtask/src/cmd/*.rs` lists exactly 17 modules (all named in the table; toml_lite.rs excluded by design as the shared library). Zero dead commands, zero stub dispatch.
- `lookup` (196-198) is an exact-name linear find; `names()` (201-203) joins for usage messages; both trivial and correct. The uniqueness test (210-216) guards duplicate names.
- Baseline evidence that the dispatch surface executes end to end: `audit/runs/baseline/xtask-gates.log` shows `cargo xtask check-architecture`, `lint-literals`, `check-identity`, `fixtures --check` all dispatching and printing their real per-module output; `xtask-feature-audit.txt` and `xtask-parity-report.txt` show two more dispatches. Seven distinct subcommands have executed evidence in the baseline alone.

## 3. Findings

### F-C01 [S3] — "FROZEN" registry was edited at least 9 times after the freeze, each edit self-justified in the same file
Where: tools/xtask/src/cmd/mod.rs:1-31, specifically 20, 23-31
Code (23-31):
```rust
//! | `cmd::visual`            | `visual`             | 4.G1 (implemented — screenshot-diff harness driver, doc 08 §6; registered under card 4.G1's granted ownership of the registry, same precedent as check-panics/7-e) |
//! | `cmd::feature_audit`     | `feature-audit`      | #20 / card 8.F1 (implemented — feature-matrix auditor; registered under issue #20's granted ownership of tools/xtask/src/**, same precedent as check-panics/7-e and visual/4.G1) |
//! | `cmd::config_validator`  | `config-validator`   | #20 / card 8.F1 (implemented — product.toml vs product.schema.json + cross-file refs; same granted-registry precedent) |
//! | `cmd::parity_report`     | `parity-report`      | #20 / card 8.F1 (implemented — derived parity table from the matrix; same granted-registry precedent) |
//! | `cmd::lint_literals`     | `lint-literals`      | 7.w21a (implemented — the C1 config-plane literal gate, doc 07 §9; registered under task 7.w21a's granted ownership of tools/xtask/src/**, same precedent as check-panics/7-e) |
```
What is wrong: lines 1-9 state the file is FROZEN after card 1.A3 and that "adding a NEW subcommand after the freeze requires Lead-granted temporary ownership of both files (AGENTS.md §3.7)". The file then documents nine post-freeze additions (check-panics, visual, feature-audit, config-validator, parity-report, lint-literals, check-identity, sbom, compat-matrix, web-budget — ten counting all rows citing the precedent), each claiming a grant in prose with no pointer to a Lead decision artifact.
Why it matters: the freeze is the file's central integrity claim and the reason downstream auditors trust one-module-one-owner isolation. If ownership grants are self-declared by the editing agent, the ownership map loses evidentiary value — the audit cannot distinguish "Lead approved" from "agent wrote 'granted' next to its own card".
What correct looks like: each grant cites a dated Lead decision (issue number + commit SHA), or the freeze language is amended to describe the actual (evolving) policy.
Confidence: HIGH (the contradiction is fully visible inside this file)
Verification: read.

### F-C02 [S4] — Two tests assert the identical invariant
Where: tools/xtask/src/cmd/mod.rs:237-262
Code (252-262):
```rust
    #[test]
    fn all_subcommands_are_implemented_invariant() {
        // Graduated from `fixtures_stub_is_the_only_unimplemented` at the
        // 2.A3 merge: the last pre-registered stub went live, so the
        // invariant is now simply "everything registered is implemented".
        ...
        for sub in SUBCOMMANDS {
            assert!(sub.implemented, "`{}` should be implemented", sub.name);
        }
    }
```
What is wrong: `all_wave1_subcommands_are_implemented` (238-249) and `all_subcommands_are_implemented_invariant` (252-262) loop over the same table asserting the same boolean; one of them is pure duplication (the comment even admits the graduation collapsed them).
Why it matters: duplicated invariants rot in tandem and give false coverage impressions.
What correct looks like: one test.
Confidence: HIGH
Verification: read.

### F-C03 [S4] — Ownership map omits `toml_lite`
Where: tools/xtask/src/cmd/mod.rs:11-31 (table) vs :52 (`pub mod toml_lite;`)
Code (52):
```rust
pub mod toml_lite;
```
What is wrong: the "Ownership map" table lists every module→subcommand row but has no row for `cmd::toml_lite`, a module with no subcommand that is shared by feature-audit/config-validator/parity-report (and sbom). Its ownership lives only in a comment inside toml_lite.rs:19-20.
Why it matters: the map claims to be the ownership reference for modules under `cmd/`; an unlisted shared module is invisible to ownership audits.
What correct looks like: a `cmd::toml_lite` row marked "library module (no subcommand) — issue #20".
Confidence: HIGH
Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `lookup` | 196-198 | REAL | — | Exact-name find over the static table. |
| `names` | 201-203 | REAL | — | Usage strings; fine. |
| test: `registry_names_are_unique` | 209-216 | HONEST | — | Would fail on a duplicate registration. |
| test: `registry_contains_all_wave1_subcommands` | 218-235 | HONEST | — | Frozen set asserted. |
| test: `all_wave1_subcommands_are_implemented` | 237-249 | HONEST | S4 | Duplicated by the next test (F-C02). |
| test: `all_subcommands_are_implemented_invariant` | 251-262 | HONEST | S4 | Duplicate invariant (F-C02). |

## 5. Test honesty

All four tests assert real registry properties (uniqueness, presence, implemented flags) and would fail if the table regressed (duplicate name, dropped wave-1 entry, a new stub with `implemented: false`). No fabrication risk: nothing here asserts output content. Note: no test asserts that each `run:` points at a distinct existing function — that is guaranteed by compilation, which the orchestrator serialized, not me.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FROZEN after card 1.A3 … adding a NEW subcommand requires Lead-granted temporary ownership" | mod.rs:3-9, main.rs:3-14 | FALSE-IN-PRACTICE | Ten post-freeze rows cite "granted" precedent in-file (F-C01); no Lead artifact referenced. |
| "every subcommand … is registered NOW and dispatched to its own module" | main.rs:5-9, corroborated by mod.rs | TRUE | 17/17 registered names resolve to real `pub fn run` (grep). |
| "Lead batch-flip at the 1.D4 merge flipped the wave-1 entries" | mod.rs:239-241 comment | UNVERIFIED | Git history not searched for that specific commit; nothing contradicts it. |

## 7. Dead / duplicated / unreachable code

- No dead subcommands: all 17 dispatch entries resolve to live modules.
- Duplicated tests: F-C02.
- `Subcommand::implemented` field: currently constant `true` across the table — the field's `false` state is unreachable (alarm-by-design per comment at 252-262, but still dead state today).

## 8. Dependencies & flags

- std only; depends on `crate::XtaskError` and the 18 `cmd::*` modules. No flags — argument parsing is delegated per-subcommand. `Subcommand::run` is a plain `fn` pointer (`&[String] -> Result<ExitCode, XtaskError>`), so dispatch cannot inject behavior.

## 9. Open questions for the next agent

- The parity-report "TOTAL 0/252 vs windows=66/252" contradiction handed to me resolves as: parity-report prints TWO tables (implemented+verified share, then a "verified-only (MP §1 strict)" table). Per-platform status cells from feature-audit baseline: blocked=129 implemented=193 not_started=503 partial=147 unsupported=36 — zero `verified` cells exist, so the verified-only table is legitimately all zeros; the machine summary line (windows=66/252 linux=61/252 macos=66/252 web=0/252) is the implemented+verified count and 66+61+66+0=193 matches. The numbers are coherent; the hazard is presentation (two unlabeled TOTAL rows). File for parity_report.rs's auditor.
- Where are the actual Lead grant records for the 9-10 post-freeze registrations (F-C01)? If they exist in docs/13-agent-wave-program.md, cite them from the registry.
