# AUDIT: crates/input/src/error.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 445 / 21 production fns (18 constructors + mint + 3 I/O folds) + 5 test fns |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI: run.rs open → demux_host.rs:394/419/422 `input::FileAccess::new()` / `input::resolve_file` → input/src/file.rs (open_io:56,59 / not_a_regular_file:61 / seek_out_of_range:91 / seek_io:95 / read_io:103 / capacity_*:138-163) and input/src/uri.rs (remote_host:73 … bad_percent_escape:150) — every constructor in this file has an in-crate caller on the shipped open path. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | rg for `input::` across apps/ and crates (callers), `error::` within crates/input/src (consumer sites); sed verification of quotes. No cargo (adaptation). |

## 1. One-paragraph verdict

A pure error-constructor vocabulary: 18 functions that mint stage-`Open` `EngineError`s with stable dotted codes, plus three I/O folds that preserve the OS error in the `source` chain. No panics, no unwrap, no paths or URIs embedded in messages (verified by an exact-string test), no redefinition of the media-types taxonomy. Every constructor is called from `uri.rs` or `file.rs`, both of which sit on the shipped CLI open path. The only defects are two semantic-stretch mappings in the fold policy that the module doc openly documents. This is one of the few files where the doc's honesty section ("errno has hundreds") matches the code exactly.

## 2. What actually works (verified)

- All 18 constructors mint `PipelineStage::Open` with the exact kind and code they claim — pinned by the 15-case table test (261-349) asserting `stage`, `kind`, and `context.code` per constructor.
- OS-error chain preservation: `open_io`/`read_io`/`seek_io` all end in `.with_source(err)`; test 390-424 asserts `Error::source` is present and carries the OS message.
- No-secrets discipline: constructors take no path/URI text (verified signatures); `display_is_structured_only` (437-444) asserts the exact `Display` string contains only numbers and the code.
- The `FilesystemLoop` non-match is honestly documented (214-216): `io_error_more` unstable on this toolchain; loops fold to the generic arm with the OS error preserved. Verified: no unstable import in the file.

## 3. Findings

### F-001 [S4] Generic I/O failures fold to `AccessDenied::PermissionDenied` — a mislabel for most errno
Where: crates/input/src/error.rs:221-227
Code:
```rust
        _ => mint(
            ErrorKind::Inaccessible {
                reason: AccessDenied::PermissionDenied,
            },
            "input.file.open-io",
        )
```
What is wrong: Every open failure that is not NotFound or PermissionDenied — EIO (dead disk), ENOMEM, EMFILE, EISDIR on some platforms — is labeled `PermissionDenied`. A UI rendering this taxonomy tells the user "access denied" for a hardware failure.
Why it matters: The module doc (13-18) admits this is a "closest taxonomy fit" and promises the chain carries the truth, which it does. The defect is bounded: the reason slot lies, the code (`input.file.open-io`) and the source chain are honest. Hygiene-level taxonomy debt in media-types (no `IoError` reason variant), not hidden dishonesty here.
What correct looks like: An `AccessDenied::IoFailure` (or equivalent) variant in media-types' taxonomy; this fold then maps to it.
Confidence: HIGH — Verification: read.
### F-002 [S4] Seek errors reuse `NotFound` for arithmetic failures
Where: crates/input/src/error.rs:81-109
Code (102-108):
```rust
pub fn seek_overflow() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.overflow",
    )
```
What is wrong: A relative seek that overflows the position range (or computes before start) has nothing to do with "nothing exists at the address" (the module doc's own gloss for `NotFound`, line 14-15). All three seek constructors reuse `NotFound`.
Why it matters: Same class as F-001: the kind slot misleads, the dotted code disambiguates. The distinguishing data (expected/found) that `truncated` carries is not present for seeks.
What correct looks like: A `Corrupt`/`Unsupported`-kind or a seek-specific reason variant with the offending offsets.
Confidence: HIGH — Verification: read.

### F-003 [S4] Module doc contradicts the code on `FilesystemLoop`
Where: crates/input/src/error.rs:14-16 vs 214-216
Code (14-16):
```rust
//! `AccessDenied` has five reasons; errno has hundreds. `NotFound` and
//! `PermissionDenied` map exactly; `FilesystemLoop` resolves to
//! "nothing exists at the address" (`NotFound`).
```
Code (214-216):
```rust
/// `FilesystemLoop` is not matched: `io_error_more` is unstable on this
/// toolchain, and a symlink loop is reported by the OS with its own
/// `io::ErrorKind`, folding into the generic arm below.
```
What is wrong: The module doc promises `FilesystemLoop → NotFound`; the implementation never matches that kind, so a symlink loop folds to the generic `PermissionDenied`-labeled arm. The doc sentence is stale; the function-level comment tells the truth.
Why it matters: A reader of the module doc (or a claim-ledger agent) would record the wrong mapping. Same-file contradiction.
What correct looks like: Delete or correct the doc sentence (and see F-001 for the fold itself).
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 34-36 | REAL | — | Private helper; stage constant. |
| `missing` | 40-47 | REAL | — | NotFound; called by open_io fold + file.rs. |
| `permission` | 50-57 | REAL | — | PermissionDenied; exact fold arm. |
| `not_a_regular_file` | 62-67 | REAL | — | `Unsupported{format: None}` — taxonomy stretch, documented; guards `/dev/zero`-style unbounded reads (file.rs:61). |
| `truncated` | 71-78 | REAL | — | Carries expected/found. |
| `seek_out_of_range` | 81-88 | REAL | S4 | F-002. |
| `seek_before_start` | 92-99 | REAL | S4 | F-002. |
| `seek_overflow` | 102-109 | REAL | S4 | F-002. |
| `remote_host` | 113-118 | REAL | — | Typed refusal: remote file hosts (uri.rs:73). |
| `relative_path` | 123-128 | REAL | — | Typed refusal: no base guessing (uri.rs:86). |
| `suffix_unsupported` | 133-138 | REAL | — | Query/fragment refused, not ignored (uri.rs:83). |
| `bad_percent_escape` | 141-146 | REAL | — | uri.rs:130,150. |
| `nul_in_path` | 149-154 | REAL | — | uri.rs:90. |
| `not_utf8_path` | 158-163 | REAL | — | v0 UTF-8 limitation, documented (ARCH-6). |
| `unc_path` | 171-176 | REAL | — | Mirrors media-types constructor-side refusal (167-170). |
| `capacity_zero` | 179-184 | REAL | — | file.rs:138,163. |
| `capacity_over_budget` | 188-197 | REAL | — | Carries budget/attempted; `usize as u64` — lossless on all supported targets. |
| `read_over_budget` | 201-210 | REAL | — | Same. |
| `open_io` | 217-229 | REAL | S4 | F-001 generic fold; exact arms verified by test. |
| `read_io` | 233-241 | REAL | S4 | Always PermissionDenied-labeled `Inaccessible` (same stretch, undocumented for this fn — module doc covers it). |
| `seek_io` | 244-252 | REAL | S4 | Same. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| constructors_mint_stage_open_kinds_and_codes (261-349) | YES | 15-case exact table over stage+kind+code — any drift fails. |
| limit_constructors_carry_budget_and_attempted (358-385) | YES | Destructures the kind and checks all three fields. |
| open_io_maps_kinds_and_keeps_the_chain (390-424) | YES | Exact arms + fold arm + source-chain presence and content. |
| read_and_seek_io_have_distinct_codes (428-433) | YES | Code distinctness. |
| display_is_structured_only (437-444) | YES | Exact full-string assert — catches any leaked path data. |

No mocks-of-subject (the subject is pure data construction), no ignores, no tautologies. `io::Error::from(io::ErrorKind::NotFound)` constructs a real OS-shaped error without touching the filesystem — appropriate for a pure mapping test.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every error this crate mints is stage `PipelineStage::Open`" (line 8) | module doc | REAL | STAGE const (31) used by `mint` exclusively; test asserts. |
| "the untouched OS error rides the `source` chain" (17-18) | module doc | REAL | `.with_source(err)` on all three folds; test 398-401. |
| "`Display` stays structured data … no secrets" (21-23) | module doc | REAL | Exact-string test 437-444; constructors take no text. |
| "`NotFound` and `PermissionDenied` map exactly" (14) | module doc | REAL | open_io arms 219-220 + test 391-409. |
| "`FilesystemLoop` resolves to `NotFound`" (15) | module doc | FALSE | The code does NOT match FilesystemLoop at all (214-216 admits it); a loop folds to the generic `PermissionDenied` arm, not `NotFound`. The doc sentence describes an aspiration the comment below it contradicts. |

## 7. Dead / duplicated / unreachable code

None. All 18 constructors have in-crate callers (rg list in §AUDIT header); no duplication (each constructor is one `mint` call).

## 8. Dependencies & flags

No feature flags, no `unsafe`, std + media-types only. No unstable APIs (the `io_error_more` absence is documented at 214-216).

## 9. Open questions for the next agent

- W02 (media-types error taxonomy): is there an `AccessDenied` variant that could host honest I/O-failure reasons? F-001/F-002 belong to that crate's design.
- W13 (demux_host.rs): confirm `input::resolve_file` failures surface these codes verbatim on the wire (the CLI error path is your file).
