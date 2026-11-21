# AUDIT: crates/input/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 445 / 25 (1 const, 1 private mint, 20 pub constructors, module docs; 5 tests + 1 helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (chain: `lumen` open → apps/cli/src/demux_host.rs:394 `input::FileAccess::new()` / :419 `input::resolve_file` → crates/input/src/file.rs:56-103 & uri.rs:51-150 → every constructor here; verified call sites listed per function) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `rg -n "error::[a-z_]+" crates/input/src/{file,uri,buffered,seekable}.rs` (all 20 constructors consumed — first per-name count pass was a false negative: `.map_err(error::open_io)` passes the fn as a value, no call parens; re-verified by name) · `rg -n "pub enum AccessDenied" media-types` (5 reasons — fold policy judged against the real taxonomy) · baseline log: input 33/33 unittests ok (:2603-2608), incl. all 5 error::tests |

## 1. One-paragraph verdict
A disciplined, small-surface error factory: every constructor mints a stage-`Open` `EngineError` with a stable dotted code, no constructor accepts path/URI text (redaction by construction, MP §12), the OS error rides the `source` chain untouched, and a five-test table pins the entire vocabulary (stage, kind, code, budget/attempted payloads, chain preservation, Display shape) — all passing in the no-fail-fast baseline. All 20 constructors have live call sites in file.rs/uri.rs/buffered.rs/seekable.rs on the shipped open path. The single defect is documentation drift: the module doc promises `FilesystemLoop` maps to `NotFound`, while `open_io` (which cannot match `FilesystemLoop` on this toolchain — `io_error_more` unstable) folds it into the generic `Inaccessible{PermissionDenied}` arm; the inline comment admits this, so the module doc and the function comment contradict each other.

## 2. What actually works (verified)
— All 5 error::tests passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (input suite :2603-2608, 33/33 ok): `constructors_mint_stage_open_kinds_and_codes` (15-case table), `limit_constructors_carry_budget_and_attempted`, `open_io_maps_kinds_and_keeps_the_chain` (incl. the generic fold + source-chain preservation), `read_and_seek_io_have_distinct_codes`, `display_is_structured_only` (exact string `[open] truncated data: expected 1024 bytes, found 512 (input.file.truncated)`).
— Shipped reachability: `lumen open` reaches this file via demux_host.rs:394/:419 → file.rs:56,59 (`error::open_io`), :61 (`not_a_regular_file`), :91 (`seek_out_of_range`), :95 (`seek_io`), :103 (`read_io`), :138/:141/:163 (`capacity_zero`/`capacity_over_budget`); uri.rs:51 (`open_io`), :73-100 (remote_host/suffix_unsupported/relative_path/nul_in_path/not_utf8_path/unc_path), :130/:150 (`bad_percent_escape`); seekable.rs:92-94 (`seek_before_start`/`seek_overflow`); buffered.rs:386/:413 (`truncated`).
— The truncation AC ("typed error, never a silent EOF, never a panic", :69-70) is enforced at buffered.rs:386/:413 with expected/found payloads; input's truncation integration test file exists (tests/truncation.rs) and the suite passed.

## 3. Findings
### F-001 [S4]
Where: crates/input/src/error.rs:13-15 (doc) vs :217-229 (code)
Code (error.rs:13-15):
```
//! `AccessDenied` has five reasons; errno has hundreds. `NotFound` and
//! `PermissionDenied` map exactly; `FilesystemLoop` resolves to
//! "nothing exists at the address" (`NotFound`). Every other open/read
```
Code (error.rs:217-227):
```
pub fn open_io(err: io::Error) -> EngineError {
    match err.kind() {
        io::ErrorKind::NotFound => missing().with_source(err),
        io::ErrorKind::PermissionDenied => permission().with_source(err),
        _ => mint(
```
What is wrong: the module doc promises `FilesystemLoop` → `NotFound`; the code matches only `NotFound` and `PermissionDenied` and folds `FilesystemLoop` (with every other kind) into the generic arm — `Inaccessible{PermissionDenied}` with code `input.file.open-io`. The function's own comment (:214-216) states the truth ("`FilesystemLoop` is not matched: `io_error_more` is unstable ... folding into the generic arm below"), so the module doc and the function comment disagree with each other.
Why it matters: doc-only impact — a reader of the module docs believes symlink loops are reported as NotFound; a debugger matching on `input.file.missing` will never see them. Behavior itself is defensible (fold + source chain).
What correct looks like: fix the module doc to match the code ("`FilesystemLoop` folds into the generic arm on this toolchain"), or match it via `err.raw_os_error()` when `io_error_more` stabilizes.
Confidence: HIGH — Verification: read both sites; std `io::ErrorKind::FilesystemLoop` requires the unstable `io_error_more` gate (comment's claim consistent with Rust 1.99).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-23 | REAL | S4 | F-001: FilesystemLoop claim contradicts code; "no secrets" claim upheld (no constructor takes text) |
| `const STAGE` | 31 | REAL | — | PipelineStage::Open, asserted by test |
| `mint` | 34-36 | REAL | — | private helper, uniform shape |
| `missing` | 40-47 | REAL | — | called by open_io:219 + test; Inaccessible{NotFound} |
| `permission` | 50-57 | REAL | — | called by open_io:220 + test |
| `not_a_regular_file` | 62-67 | REAL | — | file.rs:61; Unsupported — devices rejected (MP §12 rationale in doc) |
| `truncated` | 71-78 | REAL | — | buffered.rs:386,413; Corrupt{Truncated{expected,found}} — payload tested |
| `seek_out_of_range` | 81-88 | REAL | — | file.rs:91; Inaccessible{NotFound} — closest fit for a position past EOF |
| `seek_before_start` | 92-99 | REAL | — | seekable.rs:92 |
| `seek_overflow` | 102-109 | REAL | — | seekable.rs:94 |
| `remote_host` | 113-118 | REAL | — | uri.rs:73 |
| `relative_path` | 123-128 | REAL | — | uri.rs:86 |
| `suffix_unsupported` | 133-138 | REAL | — | uri.rs:83 |
| `bad_percent_escape` | 141-146 | REAL | — | uri.rs:130,150 |
| `nul_in_path` | 149-154 | REAL | — | uri.rs:90 |
| `not_utf8_path` | 158-163 | REAL | — | uri.rs:92; v0 UTF-8 limitation documented (ARCH-6) |
| `unc_path` | 171-176 | REAL | — | uri.rs:100; mirrors media-types constructor-side refusal (doc) |
| `capacity_zero` | 179-184 | REAL | — | file.rs:138,163 + buffered; tested |
| `capacity_over_budget` | 188-197 | REAL | — | file.rs:141; ResourceLimit{Memory} payload tested |
| `read_over_budget` | 201-210 | REAL | — | buffered.rs (MAX_READ_BYTES); payload tested |
| `open_io` | 217-229 | REAL | S4 | F-001 doc mismatch; generic fold's PermissionDenied reason for arbitrary kinds is the documented "closest fit" — the 5-reason taxonomy (media-types error.rs:100-111) offers no better slot; source chain preserved (tested) |
| `read_io` | 233-241 | REAL | — | file.rs:103; code tested |
| `seek_io` | 244-252 | REAL | — | file.rs:95; code tested |

## 5. Test honesty (5 tests, all passed — baseline :2603-2608)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `constructors_mint_stage_open_kinds_and_codes` | YES | 15-row table asserting stage+kind+code exactly — any drift fails |
| `limit_constructors_carry_budget_and_attempted` | YES | destructures ResourceLimit payloads, asserts all four fields |
| `open_io_maps_kinds_and_keeps_the_chain` | YES | exact mapping for NotFound/PermissionDenied, fold for others, source-chain preserved and string-checked |
| `read_and_seek_io_have_distinct_codes` | YES | distinct codes asserted |
| `display_is_structured_only` | YES | exact full Display string — any added caller data would fail |
| Gap | — | no test pins the FilesystemLoop claim (F-001) — a test asserting the doc's behavior would fail today |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every error this crate mints is stage Open" | error.rs:7-9 | REAL | test asserts STAGE for the whole table |
| "`NotFound` and `PermissionDenied` map exactly" | error.rs:13-14 | REAL | open_io test |
| "`FilesystemLoop` resolves to ... (`NotFound`)" | error.rs:14-15 | FALSE | F-001: folds to generic arm (code :221-227, comment :214-216) |
| "the untouched OS error rides the `source` chain" | error.rs:17-18 | REAL | open_io test asserts source().to_string() contains the OS text |
| "Constructors take no path or URI text" | error.rs:21-23 | REAL | read: no constructor signature takes strings (only u64/usize payloads) |

## 7. Dead / duplicated / unreachable code
- None. All 20 constructors have production call sites (§2 list); first-pass grep false negative documented in the Commands row (fn-as-value `map_err(error::open_io)`).

## 8. Dependencies & flags
- std::io + media-types only. No feature gates. Built and tested in baseline (input 33/33 + integration tests bounded_reads/file_stream/truncation in the same run).

## 9. Open questions for the next agent
- Whoever audits crates/input/src/file.rs + uri.rs: the `not_a_regular_file` check order at file.rs:56-61 — `File::open` before `metadata` — means a directory path fails at open with the OS's EISDIR (generic fold) rather than the typed `input.file.not-a-regular-file`; confirm which error a directory open actually surfaces on each platform.
- media-types wave: `AccessDenied` has 5 reasons but no `StorageFull`/`WouldBlock`-adjacent slot — the generic-fold laziness in `open_io` is taxonomy-bound; a reason audit belongs there.
