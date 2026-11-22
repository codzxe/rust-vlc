# AUDIT: crates/input/src/file.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 281 / 12 production items + 1 `#[test]` fn |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | **YES** — every local open in the shipped CLI: `lumen open x.mp4` → `SourceSpec::File` → `FileAccess::new().open(&uri)` (apps/cli/src/demux_host.rs:398-400); `lumen open file:///…` → resolve → `FileAccess::new().open` (demux_host.rs:419-421). W00's AVI vertical (mjpeg+pcm decodes, plays to Ended) reads through exactly this file. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | rg for `FileAccess`/`FileSource` consumers (demux_host.rs:398,421); read of BufferedReader's `length()`/`confirm_eof` call sites (buffered.rs:91, 130-131, 410); sed quote verification |

## 1. One-paragraph verdict

The local-file access stack, and it is a good one: `FileSource` opens via `std::fs`, takes metadata **from the handle** (no open/stat race), refuses non-regular files (directories/FIFOs/devices — closing the `/dev/zero` unbounded-read hole), tracks position in memory, and does a live re-stat for `length()` with the open-time snapshot as fallback. `FileAccess` layers capacity validation (zero/over-ceiling typed) and the read-ahead-budget join (over-ceiling budgets served at the ceiling, observably). `FileStream` wraps it in the bounded `BufferedReader` behind the plugin-api `SourceStream` shape. This is the code that actually reads every local file the player opens — the AVI vertical's bytes flow through it. Two hygiene notes only: a `metadata()` syscall per `seek` (potential per-packet cost for seek-heavy containers) and the in-file test count of one (the real behavioral suites live in `tests/bounded_reads.rs`, my file #17).

## 2. What actually works (verified)

- The open path chain (demux_host.rs:398/419 → `Access::open` 186-197 → `resolve_file` → `FileSource::open` → `BufferedReader::new`), read hop by hop.
- The no-unbounded-read guard (60-62): `!meta.is_file()` refusal — verified reading `FileSource::open`.
- W00 baseline proves the path end-to-end: mjpeg-smoke-bench decodes and advance-test plays the AVI vertical to Ended — both read through `FileStream` (UNVERIFIED by my own run — cited from W00 outputs).

## 3. Findings

### F-001 [S4] — `FileSource::seek` performs a live `metadata()` stat on every seek

Where: crates/input/src/file.rs:87-92.

Code (87-92, verbatim):
```rust
    fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
        if let Some(len) = self.length()
            && to > len
        {
            return Err(error::seek_out_of_range());
        }
```

What is wrong: `self.length()` (80-85) calls `self.file.metadata()` — a syscall — on every seek, to range-check the target. Buffer-window hits never reach this, but any demuxer that seeks per packet (indexed containers doing chunk jumps) pays one stat per seek on top of the seek itself. The open-time snapshot would bound the common case with the live re-stat only on failure. Not measured (no profiling per adaptations) — flagged as a candidate, not a proven cost.

Confidence: MEDIUM — Verification: read; cost UNVERIFIED (no run).

### F-002 [S4] — One in-file test; the module's behavioral proof is delegated

Where: crates/input/src/file.rs:252-279.

What is wrong: the single test pins the budget mapping (a real 4-assertion case incl. the over-ceiling clamp and the zero refusal). `FileSource`'s refusal of non-regular files, the position tracking, and the live/fallback length split have no in-file tests — they are covered by `tests/bounded_reads.rs` (real files) and the crate's unit suite elsewhere. Acceptable layering, thin local net.

Confidence: HIGH — Verification: read + test enumeration.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-22 | REAL-UNPROVEN | — | accurate |
| `FileSource::open` | 55-68 | REAL-UNPROVEN | — | handle metadata (no race); non-regular refusal; fallback_len snapshot |
| `Seekable::position` | 72-74 | REAL-UNPROVEN | — | in-memory tracking |
| `Seekable::length` | 76-85 | REAL-UNPROVEN | — | live stat, open-time fallback |
| `Seekable::seek` | 87-98 | REAL-UNPROVEN | S4 | F-001 per-seek stat; typed out-of-range |
| `RawRead::read` | 102-108 | REAL-UNPROVEN | — | saturating pos advance |
| `FileAccess::new`/`Default` | 124-128, 175-179 | REAL-UNPROVEN | — | DEFAULT_BUFFER_BYTES |
| `FileAccess::with_buffer_capacity` | 136-144 | REAL-UNPROVEN | — | zero/over-ceiling typed |
| `FileAccess::with_read_ahead_budget` | 161-167 | REAL-UNPROVEN | — | over-ceiling clamp observable; tested |
| `FileAccess::buffer_capacity` | 170-172 | REAL-UNPROVEN | — | — |
| `Access::accepts`/`open` | 182-197 | REAL-UNPROVEN | — | scheme gate; resolve → open → bounded reader |
| `FileStream` + `SourceStream` impl | 202-239 | REAL-UNPROVEN | — | delegates; `supports_seek() = true` |
| test (1) | 253-279 | REAL-UNPROVEN | S4 | F-002 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `read_ahead_budget_maps_to_the_window_under_the_ceiling` (253-279) | YES | 4 cases incl. ceiling-verbatim, over-ceiling clamp, zero refusal |

The module's real coverage is `tests/bounded_reads.rs` (my file #17 — real files, capacity bounds, window straddles) plus the contract module's synthetic-source suite (my contract.rs audit).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Regular files only: directories, FIFOs and device nodes are typed rejections at open time" | file.rs:7-10 | REAL-UNPROVEN | `!meta.is_file()` refusal at 60-62; exercised in tests/bounded_reads.rs territory |
| "no open/stat race" | file.rs:17-18 | REAL | metadata from the handle itself (59) |
| "growing files report their current truth; the truncation reference stays the open-time snapshot" | file.rs:20-22 | REAL-UNPROVEN | live `length()` + `declared_len` in BufferedReader::confirm_eof (buffered.rs:409-416) |

## 7. Dead / duplicated / unreachable code

None. Every item is on the shipped local-open path.

## 8. Dependencies & flags

Depends on `std::fs`, `media_types`, `plugin_api` (Access/SourceStream), `crate::buffered`/`error`/`seekable`/`uri`. No feature gates. Uses let-chains (2024 edition) — toolchain pin 1.99.1 covers it.

## 9. Open questions for the next agent

- `BufferedReader` (crates/input/src/buffered.rs) — the actual windowing machinery this file delegates to; its `confirm_eof` truncation contract is the other half of the "never a silent EOF" claim. Another agent's file.
- Does the AVI movi walk seek per packet or read sequentially? Decides whether F-001's stat cost is real. crates/demux/src/avi/riff.rs (wave 4/5).
