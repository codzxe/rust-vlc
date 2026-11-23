# AUDIT: crates/input/src/file.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 280 (239 production + 40 tests) / 12 production fns + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/demux_host.rs` file arm — `input::FileAccess::new().open(&uri)` (local paths) and the `file:` URL arm via `input::resolve_file` → `FileAccess::new().open(&uri)` (demux_host.rs:415-426); FileStream then feeds probe+demux. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read, rg (with_read_ahead_budget/cache_demux_mib/length callers), sed -n, baseline citation |

## 1. One-paragraph verdict
A correct, small, honest local-file access: regular-file-only open with handle-metadata verification (no open/stat race), typed error codes for missing/permission/not-regular, live `size()` with open-time fallback, seek with typed out-of-range, and a thin SourceStream adapter over the bounded reader. It is reachable from the shipped CLI and its integration suite (file_stream.rs, 9 tests in baseline) pins the typed rejections. The one real defect is a wiring lie: `with_read_ahead_budget` is documented as "the `limits.cache_demux_mib` join", but nothing in production ever calls it — the shipped open path uses `FileAccess::new()` (64 KiB default window) unconditionally, and `cache_demux_mib` is parsed and merged by product-config (partial.rs:133, merge.rs:429-430) with no consumer ever reading it: a user-set demux cache knob silently does nothing. Minor notes: `FileSource::length()` performs a `metadata()` stat on every call (called on every seek, every `size()` poll, and every EOF confirmation — fine at demuxer rates, but a polling caller would syscall-loop), and the live-length seek check vs the open-time truncation reference are two different length notions by design (documented at L20-22) — correct but worth a consumer's attention.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE input): `file::tests::read_ahead_budget_maps_to_the_window_under_the_ceiling ... ok` (L2829) + `tests/file_stream.rs ... 9 passed` (typed rejections for missing/directory/device/scheme, contract suite) + `tests/bounded_reads.rs ... 5 passed`.
— verified by read: open path uses handle metadata (`file.metadata()` after open — the no-TOCTOU shape, L56-62); `seek` refuses past live length before touching the OS (L87-92); no unwrap/expect/panic.

## 3. Findings
F-001 [S3]
Where: crates/input/src/file.rs:146-167 (claim) + apps/cli/src/demux_host.rs:424 (actual wiring)
Code:
```
    /// A file access sized from a **read-ahead budget** (card 3.B3,
    /// design A4): the `limits.cache_demux_mib` join. The budget is
    /// honored up to the single-allocation ceiling — a budget at or
    /// below [`MAX_BUFFER_BYTES`] becomes the window verbatim; a
```
What is wrong: the join does not exist. `rg "with_read_ahead_budget"` across the tree finds exactly two callers, both tests: file.rs's own unit test (L253) and the contract-feature-gated `tests/file_stream.rs:211` (stripped from the default-fature baseline). The shipped open path constructs `FileAccess::new()` — the 64 KiB default — with no capacity input. Meanwhile `cache_demux_mib` is declared, schema-validated, and merged by product-config (crates/product-config/src/partial.rs:133, merge.rs:429-430) and never read by anyone. A user raising `limits.cache_demux_mib` gets zero effect and zero error — a silent config no-op, and the file.rs doc comment describes a join that is production-dead. Why it matters: MP §5-style honesty (typed refusal or clamping, never silent) is this crate's stated discipline; this knob is silently ignored. What correct looks like: the demux host reads `config.limits.cache_demux_mib` and constructs via `with_read_ahead_budget`. Confidence: HIGH — Verification: rg over the whole tree (callers listed above).

F-002 [S4]
Where: crates/input/src/file.rs:76-85
Code (verbatim, L76-84):
```
    fn length(&self) -> Option<u64> {
        // Live truth with the open-time value as fallback: a growing
        // file reports its current size; a failing stat keeps the last
        // known length instead of inventing ignorance.
        self.file
            .metadata()
            .map(|meta| meta.len())
            .ok()
            .or(self.fallback_len)
    }
```
What is wrong: every `length()` call is a stat syscall. Call sites: `Seekable::seek` (L88, every seek), `FileStream::size` via `reader.source_length()` (buffered.rs:130-131, every size poll), and `confirm_eof` (buffered.rs:410-411, once per EOF-hitting refill). At demuxer call rates this is noise, but a UI/engine polling `size()` on a timer would hammer stat. A cached-with-TTL or change-notification shape would remove the syscall; not a defect at current call sites. Confidence: HIGH — Verification: read of all callers.

F-003 [S4]
Where: crates/input/src/file.rs:87-92 vs buffered.rs confirm_eof (L408-417)
Code:
```
    fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
        if let Some(len) = self.length()
            && to > len
        {
            return Err(error::seek_out_of_range());
        }
```
What is wrong: two length notions coexist by design — seek bounds against the LIVE length, while the truncation contract compares the live length to the OPEN-TIME snapshot (`declared_len`). Consequence: a file that shrank below its open-time length yields `seek_out_of_range` for targets in `(live, open_time]` but a typed `truncated` from reads that hit EOF — both typed, never silent, but the error a consumer sees depends on whether it seeks or reads into the vanished tail. Documented at L20-22; noted as a consumer-facing inconsistency, not a bug. Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FileSource::open | 55-68 | REAL | — | handle metadata, regular-file check, typed errors |
| Seekable::position | 72-74 | REAL | — | tracked, not polled (Seek::stream_position needs &mut) |
| Seekable::length | 76-85 | REAL | S4 | F-002 stat-per-call |
| Seekable::seek | 87-98 | REAL | S4 | F-003 live-length bound |
| RawRead::read | 102-108 | REAL | — | saturating pos update |
| FileAccess::new / Default | 124-128, 175-179 | REAL | — | 64 KiB default |
| FileAccess::with_buffer_capacity | 136-144 | REAL | — | typed bounds, test-used (bounded_reads.rs) |
| FileAccess::with_read_ahead_budget | 161-167 | REAL | S3 | F-001: the documented config join is production-dead |
| FileAccess::buffer_capacity | 170-172 | REAL | — | observability accessor |
| Access::accepts / open | 182-198 | REAL | — | resolve_file → FileSource → BufferedReader → Box<FileStream> |
| SourceStream::{uri,size,supports_seek} | 208-218 | REAL | — | |
| SourceStream::read / read_into / seek / position | 220-238 | REAL | — | thin adapter; read_into is the no-alloc override |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| file::tests::read_ahead_budget_maps_to_the_window_under_the_ceiling | YES | clamping + zero-refusal pinned — but it tests a constructor no production path calls (F-001) |
| (integration, tests/file_stream.rs, 9 in baseline) | YES | typed rejections per shape; contract asserts when feature on |
| (integration, tests/bounded_reads.rs, 5 in baseline) | YES | capacity bounds + 1-byte-window survival |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "growing files report their current truth; the truncation reference stays the open-time snapshot" | file.rs:20-22 | REAL | L76-85 live stat; buffered.rs declared_len (L91, L410-417) |
| "`with_read_ahead_budget` ... the `limits.cache_demux_mib` join" | file.rs:146-147 | FALSE (join unwired) | F-001: no production caller; product-config merges the knob with no reader |
| "a device like /dev/zero would be an unbounded read" → refused at open | file.rs:7-10 | REAL | L60-62 `!meta.is_file()` → `input.file.not-a-regular-file`; pinned by file_stream.rs device test (baseline) |

## 7. Dead / duplicated / unreachable code
`with_read_ahead_budget` is test-only (F-001). Everything else is live via the CLI open path.

## 8. Dependencies & flags
std only (io/fs/path) + media-types/plugin-api/workspace-internal crate modules. No features. No unsafe.

## 9. Open questions for the next agent
- Who is supposed to own the `cache_demux_mib` → `with_read_ahead_budget` wiring — the demux host (apps/cli) or a later engine card? If a card is named in docs/plans, cite it; otherwise this is an orphaned knob.
- Same question for `cache_network_mib` → `RangedConfig` (ranged.rs audit §9): both knobs are parsed-merged-never-applied — one systemic finding across two crates.
