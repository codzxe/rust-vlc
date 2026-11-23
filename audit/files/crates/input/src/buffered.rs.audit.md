# AUDIT: crates/input/src/buffered.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 827 / 17 (incl. 14 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli main→run→host.rs:904 open_source→demux_host.rs:394/422 `input::FileAccess::new()`→file.rs:192 `BufferedReader::new`→FileStream (SourceStream) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (users/contracts) exit 0; sed -n verifications exit 0; baseline citation |

## 1. One-paragraph verdict
A genuinely correct fixed-capacity buffered reader: buffer allocated once and never
grown, two-regime read (window locality vs caller-sized bulk bypass), exact position
tracking, typed truncation on source shrink, typed ceilings on capacity (16 MiB) and
single-read asks (64 MiB). All 14 in-file tests pass in the orchestrator baseline and
they are honest — they instrument the source to count reads served and max ask size,
so the bypass ("one bulk call, never a speculative over-read") is pinned by assertion,
not narration. The one real defect is a contract gap the invariant leans on:
`Seekable::seek`'s doc does not promise the position is unchanged on `Err` (while
`RawRead::read` explicitly does), so a seek that errors after moving would silently
desynchronize `buf_pos` from the source.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L2796-2816 (PACKAGE input):
buffered::tests 14/14 `ok` (bypass counts, read-ahead locality, sticky EOF, typed
truncation pairs, peek honesty, dribble source).
— verified by reachability rg: apps/cli/src/demux_host.rs:394,422 construct
`input::FileAccess::new()` in the shipped `lumen open <file>` path; file.rs:192 builds
the BufferedReader.
— read path arithmetic re-derived by hand: seek window cases (L153-173), bypass loop
(L237-260), read_exact_into truncation pair (L380-391) — all consistent with the
documented position invariant `source.position() == buf_pos + buffered_bytes()`.

## 3. Findings
F-001 [S3]
Where: crates/input/src/buffered.rs:168-172 + crates/input/src/seekable.rs:46-53
Code:
```
        let at = self.source.seek(to)?;
        self.start = 0;
        self.end = 0;
        self.buf_pos = at;
```
What is wrong: on `Err`, `BufferedReader` keeps its window and `buf_pos` unchanged and
assumes the source also did not move — but the `Seekable::seek` contract
(seekable.rs:46-53) never says the position is unchanged on error (unlike `RawRead`,
which states "`Err`: a typed error; the position is unchanged", seekable.rs:64-66). A
source whose seek errors after repositioning (plausible for a network range source)
leaves `source.position() != buf_pos + buffered_bytes()`; the next refill then
attributes bytes to the wrong absolute offset — silent data corruption, and
`position()` (a pure `&self` query, buffered.rs:114-116) lies. What correct looks
like: add "on `Err` the position is unchanged" to the `Seekable::seek` doc (matching
`RawRead`), or re-query `source.position()` after a failed seek and rebuild state.
Confidence: MEDIUM — Verification: read (all in-tree sources honor it; mutation
would need a hostile source — none exists in-tree).
F-002 [S4]
Where: crates/input/src/buffered.rs:238
Code: `        let mut out = vec![0u8; want];`
What is wrong: every bypass read pays a `want`-sized memset (up to 64 MiB) that is
immediately overwritten by the bulk transfer — up to 64 MiB of dead zeroing per large
ask. The comment (L234-236) honestly attributes it to the no-`unsafe` API posture
(`RawRead` needs initialized memory). What correct looks like: spare-capacity +
`set_len` (needs unsafe, workspace forbids) or an `extend`-based RawRead variant.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/input/src/buffered.rs:399-403
Code:
```
    pub fn peek_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let take = out.len().min(self.end - self.start);
```
What is wrong: `peek_into` takes `&mut self` but mutates nothing and cannot fail —
the `&mut` excludes holding `source()`/`position()` handles during a peek, and the `Result` is always `Ok`. API-shape debt only. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| new | 84-101 | REAL | — | capacity 0/over-ceiling typed; snapshots declared_len+buf_pos |
| with_default_capacity | 108-110 | REAL | — | 64 KiB |
| position / buffer_capacity / buffered_bytes | 114-126 | REAL | — | pure window math |
| source_length / source / source_mut | 130-142 | REAL | — | live length query; fault-scripting seams |
| seek | 153-173 | REAL | F-001 | in-window seek no I/O; window-end drop; else delegate |
| read | 198-226 | REAL | — | 0→empty; >64 MiB typed; ≥capacity→bypass; window loop with typed EOF |
| read_bypassing_window | 237-260 | REAL | F-002 | leftover first, direct bulk, one bounded read-ahead refill |
| refill_read_ahead | 266-272 | REAL | — | no confirm_eof (documented: check belongs to data-need reads) |
| drain_into | 276-284 | REAL | — | room-bounded, no realloc |
| read_into | 310-354 | REAL | — | borrow twin; no MAX_READ_BYTES ceiling (caller owns memory, documented) |
| drain_leftover_into | 360-368 | REAL | — | shared first step |
| read_exact_into | 380-391 | REAL | — | typed truncation (expected=ask end, found=stream end) |
| peek_into | 399-403 | REAL | F-003 | never fabricates, never advances |
| confirm_eof | 409-416 | REAL | — | shrink below open-time length → typed (declared, live) |
| 14 tests | 436-827 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| capacity_bounds_are_typed_errors | YES | code + kind + budget/attempted asserted |
| read_requests_over_the_ceiling_are_typed_limit_errors | YES | ceiling refused typed; at-ceiling served |
| positions_and_readahead_are_exact | YES | reads_served/max_read_seen instrument the source (1 refill, 64-byte chunks) |
| large_reads_bypass_the_window_in_one_source_call | YES | 2 source calls not 32; max ask == request; capacity never grows |
| bypass_reads_serve_leftover_read_ahead_first | YES | counts 54-drain + 136-remainder exactly |
| bypass_reads_survive_short_source_reads | YES | custom 40-byte dribble source; byte-exact; caller-sized asks |
| bypass_read_past_end_short_reads_cleanly | YES | short result is data, sticky EOF |
| clean_eof_at_declared_length | YES | repeatable empty reads; seek-to-len legal; past-end typed; failed seek never moves (ScriptSource only — see F-001) |
| shrink_mid_read_surfaces_typed_truncation | YES | (600,300) pair asserted |
| non_zero_start_positions_are_respected | YES | opens at 40, reads 40..50 |
| read_into_reconstructs_and_bypasses_for_payload_asks | YES | 2 calls, max ask 1000, Ok(0) sticky, empty-ask no I/O |
| read_into_serves_small_asks_from_read_ahead | YES | second 10-byte ask: still 1 read |
| read_exact_into_fills_or_fails_typed | YES | (250,200) arithmetic + shrink pair |
| peek_into_never_advances_and_never_fabricates | YES | 0 on empty; exactly-window on big ask; no I/O |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fixed-capacity buffer allocated once ... never grown" | buffered.rs:3-5 | REAL | buf only written via source.read; capacity asserted constant in 2 tests |
| "requests at or above the capacity bypass the window" | buffered.rs:6-8 | REAL | read:205-206 + bypass tests with instrumented counts |
| "A single read request above MAX_READ_BYTES is refused typed" | buffered.rs:11-12 | REAL | read:202-203 + typed-limit test |
| "A source that shrinks ... surfaces Corrupt::Truncated, never a silent EOF" | buffered.rs:12-14 | REAL | confirm_eof:409-416 + shrink tests |
| "source.position() == buf_pos + buffered_bytes() always holds" | buffered.rs:29-30 | REAL-UNPROVEN | holds on all traced paths; unproven for seek-error sources (F-001) |

## 7. Dead / duplicated / unreachable code
None. The window/bypass logic exists twice (Vec twin read:198-260, borrow twin
read_into:310-354) — deliberate twins, documented; drift risk noted by the crate
itself.

## 8. Dependencies & flags
std-only + media-types/plugin-api (SeekOrigin). No features, no unsafe, no env reads.
Ceilings: DEFAULT 64 KiB / MAX_BUFFER 16 MiB / MAX_READ 64 MiB (`[PROPOSAL]`-marked).

## 9. Open questions for the next agent
- Seekable trait owner: add the error-position clause (F-001) and re-check every
  `Seekable` impl for compliance (RangedResource over HTTP is the risky one).
- perf: measure whether the 64 MiB ceiling's memset is ever hit in practice by the
  demux arena (largest single ask in the AVI/MP4 paths).
