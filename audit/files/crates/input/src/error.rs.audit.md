# AUDIT: crates/input/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 445 / 20 constructors + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:394,419-422 (`input::FileAccess::new()` / `input::resolve_file`) → crates/input/src/file.rs:56 (`error::open_io`), uri.rs, buffered.rs → this module; exercised by the baseline MP4 smoke (`lumen open` local file) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p input --lib error` → 8 passed/0 failed (own run, 0.00s); rg caller hunts |

## 1. One-paragraph verdict
Clean, honest error-minting module: 20 constructors that produce stage-`Open` `EngineError`s with stable dotted codes, no path/URI text embedded (the `display_is_structured_only` test pins the exact Display string), OS errors preserved in the `source` chain, and a table test that walks the whole vocabulary asserting kind+code+stage. Every constructor is called by a sibling module (file.rs/uri.rs/buffered.rs/seekable.rs) which the shipped CLI reaches through `FileAccess`/`resolve_file`. One real defect: the module doc promises `FilesystemLoop` → `NotFound`, the code explicitly does not implement that mapping (the inline doc admits `io_error_more` is unstable and symlink loops fold into the generic arm) — a doc contradiction on a realistic Linux failure mode, with a second-order wart that the generic fold stamps every unmapped errno (EIO, ENOSPC, ELOOP…) as `PermissionDenied`.

## 2. What actually works (verified)
— `cargo test -p input --lib error` → `test result: ok. 8 passed; 0 failed` (own run; includes `constructors_mint_stage_open_kinds_and_codes` — a 15-entry table — `limit_constructors_carry_budget_and_attempted`, `open_io_maps_kinds_and_keeps_the_chain` asserting the OS string survives in the chain, `read_and_seek_io_have_distinct_codes`, `display_is_structured_only`).

## 3. Findings
F-001 [S3]
Where: crates/input/src/error.rs:13-15 vs 214-227
Code (module doc):
```
//! `AccessDenied` has five reasons; errno has hundreds. `NotFound` and
//! `PermissionDenied` map exactly; `FilesystemLoop` resolves to
//! "nothing exists at the address" (`NotFound`).
```
Code (implementation):
```
/// `FilesystemLoop` is not matched: `io_error_more` is unstable on this
/// toolchain, and a symlink loop is reported by the OS with its own
/// `io::ErrorKind`, folding into the generic arm below.
pub fn open_io(err: io::Error) -> EngineError {
    match err.kind() {
        io::ErrorKind::NotFound => missing().with_source(err),
        io::ErrorKind::PermissionDenied => permission().with_source(err),
```
What is wrong: the module doc claims symlink loops classify as `NotFound`; the code has no `FilesystemLoop` arm — ELOOP falls into the generic arm and is stamped `Inaccessible { reason: PermissionDenied }` with code `input.file.open-io`. The two docs in the same file contradict each other, and the code side is the one that ships. Why it matters: `lumen open` on a symlink loop reports a permission-flavored error on Linux (a common case); the kind slot misleads UI/handlers that branch on taxonomy. What correct looks like: match `io::ErrorKind::FilesystemLoop`→`missing()` when the toolchain allows (or raw-errno match on ELOOP), or delete the module-doc sentence. Confidence: HIGH — Verification: read both blocks; the inline comment is an explicit admission.
(Second-order, same block: the generic arm's `PermissionDenied` reason for EIO/ENOSPC/etc. is documented as "closest taxonomy fit" — dubious for hardware I/O errors, but at least the doc owns the fold; S4-level.)

F-002 [S4]
Where: crates/input/src/error.rs:221-227
Code:
```
        _ => mint(
            ErrorKind::Inaccessible {
                reason: AccessDenied::PermissionDenied,
            },
            "input.file.open-io",
        )
        .with_source(err),
```
What is wrong: every unmapped errno (EIO, ENOSPC, EMFILE, ELOOP, …) becomes `Inaccessible{PermissionDenied}`. The `source` chain carries the truth, so this is classification noise rather than data loss — but "PermissionDenied" in the kind slot for e.g. EMFILE (fd exhaustion, a caller bug) is actively misleading in error surfaces that only show the kind. What correct looks like: a neutral fifth reason (the doc says `AccessDenied` has five — an `Other`/`Unavailable` slot would be the honest fit). Confidence: HIGH — Verification: read; fold pinned by `open_io_maps_kinds_and_keeps_the_chain`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint (private) | 34-36 | REAL | — | stage constant + dotted code |
| missing / permission | 40-57 | REAL | — | used by open_io arms + uri.rs |
| not_a_regular_file | 62-67 | REAL | — | guards unbounded device reads (file.rs:61) |
| truncated | 71-78 | REAL | — | expected/found carried in kind; two call sites in buffered.rs |
| seek_out_of_range / seek_before_start / seek_overflow | 81-109 | REAL | — | seekable.rs:91-95 |
| remote_host / relative_path / suffix_unsupported | 113-138 | REAL | — | uri.rs:73-86 |
| bad_percent_escape / nul_in_path / not_utf8_path | 141-163 | REAL | — | uri.rs:90-150 |
| unc_path | 171-176 | REAL | — | uri.rs:100; mirrors media-types refusal |
| capacity_zero / capacity_over_budget / read_over_budget | 179-210 | REAL | — | buffered.rs:86-203; budget/attempted asserted by test |
| open_io | 217-229 | REAL | S3/S4 | F-001/F-002: no FilesystemLoop arm, generic fold |
| read_io / seek_io | 233-252 | REAL | — | distinct codes, chain preserved |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| constructors_mint_stage_open_kinds_and_codes | YES | 15-row exact kind+code+stage table |
| limit_constructors_carry_budget_and_attempted | YES | exact budget/attempted values in kind |
| open_io_maps_kinds_and_keeps_the_chain | YES | exact mapping + source-chain string check ("os error 999" survives) |
| read_and_seek_io_have_distinct_codes | YES | distinct codes |
| display_is_structured_only | YES | exact full Display string, no caller data |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FilesystemLoop resolves to 'nothing exists at the address' (NotFound)" | error.rs:14-15 | FALSE | F-001: no such arm; inline doc admits |
| "the untouched OS error rides the source chain" | error.rs:16-17 | REAL | open_io test asserts chain string |
| "Constructors take no path or URI text" | error.rs:22-23 | REAL | signatures verified; Display test |
| "Every error this crate mints is stage Open" | error.rs:7-9 | REAL | STAGE const + table test |

## 7. Dead / duplicated / unreachable code
None. All 20 constructors have callers in sibling modules (rg count: each ≥1 non-test call site).

## 8. Dependencies & flags
Depends on media-types taxonomy only (EngineError/ErrorKind/AccessDenied/CorruptDetail/LimitKind/PipelineStage). No feature gates, no unsafe, no I/O.

## 9. Open questions for the next agent
- What are the five `AccessDenied` reasons in media-types? If one is neutral (e.g. `Unavailable`), F-002's fold should use it — check `crates/media-types/src/error.rs`.
