# AUDIT: crates/subtitles/src/vobsub/idx.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| Lines / functions | 304 / 3 prod fns (parse_idx, parse_timestamp, parse_delay) + 3 tests |
| File verdict | REAL-UNPROVEN (genuine tolerant parser, tests pass; one arithmetic-overflow defect; unreachable from shipped entry points) |
| Reachable from shipped entry point? | NO — parse_idx ← vobsub/mod.rs:131 `parse_vobsub` ← vobsub/fuzz.rs + tests/dvb_vobsub_fuzz_survival.rs only; no product crate depends on `subtitles` (see eia608.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p subtitles --lib vobsub` → 15 passed / 0 failed (exit 0, includes the 3 idx tests); sed quote verification |

## 1. One-paragraph verdict
A real, line-oriented tolerant VobSub `.idx` parser: size/palette/id/alt/langidx/delay/timestamp keys, position-preserving partial palettes, typed damage for short palettes / bad sizes / bad timestamps / timestamps before any stream, accumulated per-stream delays applied to all entries, lossy-UTF-8 safety. Its 3 tests (inside the 15 vobsub tests I ran) are behavioral and pass. The one real defect is textbook unchecked arithmetic: the hour field of `timestamp:`/`delay:` is parsed as full-range `i64` and multiplied by 3600 and 1000 with no overflow handling — a two-line hostile `.idx` overflows, which panics in debug builds (contradicting the module's "Never panics" contract) and silently wraps in release. Two smaller tolerance asymmetries: `index:` out of the documented 0..32 domain and unparseable `langidx:` are silently swallowed instead of recorded.

## 2. What actually works (verified)
— verified by `cargo test -p subtitles --lib vobsub` → 15 passed: full two-stream idx parse (palette entries, langidx, titles, filepos hex), tolerance sweep (ShortPalette { got: 6 }, TimestampBeforeStream, invalid-UTF-8 garbage, BadSize, BadTimestamp), positive and negative delay shifts.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/vobsub/idx.rs:197-207 (and the identical shape at 210-222)
Code (verbatim):
```
/// `HH:MM:SS:ms, filepos: hex` → `(ms, filepos)`.
fn parse_timestamp(rest: &str) -> Option<(i64, u64)> {
    let (time, filepos) = rest.trim().split_once(',')?;
    let mut parts = time.trim().split(':');
    let hh: i64 = parts.next()?.trim().parse().ok()?;
    let mm: i64 = parts.next()?.trim().parse().ok()?;
    let ss: i64 = parts.next()?.trim().parse().ok()?;
    let ms: i64 = parts.next()?.trim().parse().ok()?;
    let filepos = filepos.trim().strip_prefix("filepos:")?;
    let pos = u64::from_str_radix(filepos.trim(), 16).ok()?;
    Some(((hh * 3600 + mm * 60 + ss) * 1000 + ms, pos))
}
```
What is wrong: `hh` accepts any `i64` (up to 9.2e18); `hh * 3600` overflows for `hh ≥ ~2.56e15` and `(hh*3600 + mm*60 + ss) * 1000` overflows for `hh ≥ ~2.56e12`. Example hostile line: `timestamp: 3000000000000000:00:00:00, filepos: 0` → `3000000000000000 * 3600 = 1.08e19 > i64::MAX`. In a debug build (and the crate's own test profile) this is an immediate panic — "attempt to multiply with overflow" — directly contradicting the module contract at line 18: "and a `#` comment never parses. Never panics." In release it silently wraps to a negative timestamp, which then flows into `IdxEntry.timestamp_ms` and the delay add at 189. `parse_delay` (221: `sign * ((hh * 3600 + mm * 60 + ss) * 1000 + ms)`) and `delays[ci] += sign` (167) have the same unchecked shapes. Why it matters: this is a hostile-input parser by design (D-4 "tolerant by contract"), and the tolerance is not total. What correct looks like: `checked_mul`/`saturating_mul` chain returning `None` → the existing `BadTimestamp` damage, exactly like every other malformed field. Confidence: HIGH — Verification: read + arithmetic (i64::MAX = 9,223,372,036,854,775,807; 3e15 × 3600 = 1.08e19). Not executed (no test harness may be added outside audit/); overflow behavior is Rust-defined.

F-002 [S4]
Where: crates/subtitles/src/vobsub/idx.rs:53-55 (doc) and 131-144 (parse)
Code (verbatim, 136-141):
```
                    let index = index_part
                        .trim()
                        .strip_prefix("index:")
                        .and_then(|v| v.trim().parse::<u8>().ok())
                        .unwrap_or(0);
                    (lang, index)
```
What is wrong: `IdxStream.index` is documented as "(`index:` value, 0..32) — the `.sub` substream byte is `0x20 + index`" but is never range-checked: `index: 255` is stored as 255 (later becoming substream byte 0x11F — not a byte), and `index: abc` silently becomes 0, selecting substream 0x20 of the wrong stream, with no damage variant (`IdxDamage` has none for it). Same silent-default for `langidx: 300` (→ `None`, line 160) while equally-malformed `size:`/`timestamp:` lines get typed damage. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/subtitles/src/vobsub/idx.rs:272
Code (verbatim):
```
        assert!(!idx.has_palette || idx.palette[0] == 0);
```
What is wrong: Tautological assertion: `palette[0]` is 0 whether the first entry parsed ("000000") or kept its black default — the disjunction can never fail in this fixture. The test name promises tolerance coverage; this line proves nothing. Confidence: HIGH — Verification: read + fixture trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_idx | 90-194 | REAL | S2/S4 | tolerant dispatch correct; palette position-preserving; delay accumulation + end-application correct; F-001 overflow entry, F-002 silent index/langidx defaults |
| parse_timestamp | 197-207 | REAL | S2 | F-001 unchecked i64 math; hex filepos correct |
| parse_delay | 210-222 | REAL | S2 | same unchecked math as F-001; sign handling correct (+/-) |
| tests: full_idx_parse | 233-263 | REAL | — | full happy path, 2 streams, would fail if broken |
| tests: tolerances_never_panic | 266-288 | REAL | S4 | good coverage except F-003 tautology; no overflow input |
| tests: delay_shifts_entries | 291-303 | REAL | — | +/− shifts pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| full_idx_parse | YES | exact palette cells, stream count, titles, timestamps, filepos values |
| tolerances_never_panic | YES (except one line) | 5 malformed-input classes pinned; line 272 is a no-op (F-003) |
| delay_shifts_entries | YES | both signs pinned to exact ms values |
Gap: no test feeds an hour field large enough to overflow (F-001 lives exactly there), no `index:`/`langidx:` garbage, no BOM-prefixed first key line.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tolerant by contract (D-4): unknown lines are skipped … Never panics" | idx.rs:16-18 | FALSE for overflow input | F-001: debug panic on `hh*3600` overflow |
| "partial palettes keep their leading entries" | idx.rs:17 | REAL | ShortPalette { got: 6 } test (run, passed) |
| "timestamps before any `id:` are typed damage" | idx.rs:17-18 | REAL | TimestampBeforeStream test (run, passed) |
| matrix `subtitle-vobsub` "idx/spu parse, 128 KiB cap, grayscale fallback, damage ledger" status implemented | docs/feature-matrix.toml:1038-1040 | REAL-UNPROVEN | 15/15 vobsub tests pass (run) but crate unreachable from shipped binaries (eia608 audit F-001); idx damage ledger honestly documented "uncapped — bounded by the line count" (idx.rs:85) |
| "`index:` value, 0..32" domain | idx.rs:53-55 | NOT ENFORCED | F-002 |

## 7. Dead / duplicated / unreachable code
None in-file. Entire file unreachable from product binaries (header field).

## 8. Dependencies & flags
Std-only, no unsafe, allocations bounded by input lines (damage vec documented uncapped-by-line-count — honest). `String::from_utf8_lossy` for arbitrary bytes ✓.

## 9. Open questions for the next agent
- vobsub/spu.rs and vobsub/mod.rs (parse_vobsub, the 128 KiB cap, grayscale fallback) are unaudited by me — the F-001 wrapped timestamps feed `VobSubDoc` cue intervals there; check whether wrapped negative ms reaches a subtraction that can invert an interval.
- Does the spu demux side validate `0x20 + index` as a byte (F-002's 255 case)?
