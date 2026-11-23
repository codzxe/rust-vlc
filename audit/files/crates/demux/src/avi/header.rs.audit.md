# AUDIT: crates/demux/src/avi/header.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 866 / 24 (types, consts, 12 fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:54 `"avi" => avi::open(stream, opts)` → containers/avi.rs:13 `use demux::avi::AviDemuxer` → demux/src/avi/mod.rs:1149 `parse_hdrl(...)` → this file. lumen-cli release lane builds clean (baseline cargo-build-release-lumen-cli.txt). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | `rg fn read_exact_or_end_at ... riff.rs` + sed of riff.rs:285-296 (exact-length Some contract); `rg parse_hdrl avi/mod.rs` (1 call site); `rg DamageLog avi/mod.rs` (unbounded drain into pending); baseline cargo-test-workspace.txt:1803-1846 (avi::tests incl. header paths all ok) |

## 1. One-paragraph verdict
Real, careful parser: every slice read goes through
`SourceReader::read_exact_or_end_at`, which returns `Some` only for the full
requested length (riff.rs:285-296 — `ReadExact::Complete(bytes)`), so the
`u32le`/`copy_from_slice` indexing in parse_avih/parse_strl cannot panic on
short buffers; u64 offset arithmetic makes `advance` overflow unreachable;
signed biHeight, the extensible-GUID length gate, channel ceiling, and the
physical-truncation-vs-lying-size distinction are all implemented and tested
(avi/tests.rs:586-996, baseline all ok). Two real defects: the damage log
grows without bound (one event per 8 bytes of hostile input, then a second
copy in the pending event queue), and the module's headline robustness claim
("declared list/chunk sizes are hints, not authority") is implemented only for
the hdrl LIST size — a lying avih/JUNK/strl chunk size swallows the movi and
kills the file, which the claim explicitly promises to recover from.

## 2. What actually works (verified)
- read + sed riff.rs:285-296: `read_exact_or_end_at` returns
  `Ok(Some(bytes))` only on `ReadExact::Complete` — exact length — so all
  indexing after `Some(bytes)` in this file is bounds-safe by contract.
- Baseline cargo-test-workspace.txt:1803-1846: `avi::tests::*` header-behavior
  tests pass, incl. `lying_hdrl_size_does_not_swallow_the_movi`,
  `negative_biheight_accepts_bottom_up_dib_with_damage`,
  `short_strh_skips_the_stream_siblings_survive`,
  `wave_format_extensible_resolves_subformat_guids`.
- Offsets cross-checked against the AVI spec by read: avih flags@12,
  streams@24, width@32; strh scale@20, rate@24, start@28, length@32,
  sample_size@44; BITMAPINFOHEADER width@4, height@8; WAVEFORMATEX
  tag@0, channels@2, rate@4, align@12, bits@14 — all correct.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/avi/header.rs:397-400 (and mod.rs:410-414)
Code:
```
    pub fn note(&mut self, skipped: u64, reason: &'static str) {
        self.total = self.total.saturating_add(skipped);
        self.events.push((skipped, reason));
    }
```
What is wrong: no cap on `events`. In `parse_strl`, every unknown chunk
mints one event and the loop advances a minimum of 8 bytes
(`while pos + 8 <= end`, header.rs:619, 669-671), so a hostile `strl`
yields up to `file_size / 8` events (16 bytes each + Vec capacity ≈ 2×);
`mod.rs:410` then moves every event into the `pending` VecDeque as a
`DemuxEvent::Recovered` (~32-48 bytes each) — combined ≈ 4-6× the input size
in RAM. A crafted 500 MB AVI pushes ~3 GB on this 3 GB machine. Why it
matters: memory amplification from attacker-controlled structure with no
ceiling — the same class the module's own ceilings (MAX_STRN_BYTES,
MAX_PCM_CHANNELS) exist to prevent. What correct looks like: a MAX_DAMAGE_
EVENTS cap (e.g. 4096) with a summary event + counter once exceeded.
Confidence: HIGH — Verification: read (path arithmetic); not executed
(cargo-build-budget — no OOM PoC run).

F-002 [S3]
Where: crates/demux/src/avi/header.rs:11-16 (claim) vs 485, 495, 504-506, 551-553
Code:
```
//! Declared list/chunk sizes are **hints, not authority**. The parser
//! consumes what it structurally recognizes, in order:
```
```
fn advance(pos: u64, size: u32) -> u64 {
    pos + 8 + u64::from(size) + riff::pad_for(u64::from(size))
}
```
What is wrong: the claim is implemented only for the hdrl LIST size (the
break rules at 496-511 recover a lying hdrl). For every *recognized* chunk —
`avih` (485), `LIST strl` (495), `JUNK` (505) — the walk advances by the
DECLARED size via `advance`. One flipped bit in a JUNK/strl/avih size inside
hdrl jumps `pos` past the movi; the top-level walk then ends without a movi
and the file is dead, exactly the outcome the doc promises recovery from
("recovers a corrupted hdrl size that would otherwise swallow the movi list").
Only the hdrl-size variant is tested (tests.rs:2529); no test covers a lying
chunk size inside hdrl. Why it matters: header bitrot that ffmpeg survives
kills playback here, and the doc misdescribes the guarantee. What correct
looks like: clamp `advance` to the enclosing `end` and break when the declared
size overruns (or resync to the next recognized form), plus a test.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/avi/header.rs:622-641
Code:
```
            STRF => {
                parts.strf = Some((pos, header.size));
                pos = advance(pos, header.size);
```
What is wrong: a duplicate `strh` overwrites `parts.strh` and a duplicate
`strf` overwrites `parts.strf`, both silently — inconsistent with the
`duplicate-avih` policy at 478-481 which keeps the first and damage-notes the
impostor. A hostile file can make the parsed stream describe the second,
attacker-chosen strh/strf with zero damage signal.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/avi/header.rs:342, 644
Code:
```
pub const MAX_STRN_BYTES: usize = 4096;
```
```
                let want = (u64::from(header.size).min(MAX_STRN_BYTES as u64)) as usize;
```
What is wrong: an `strn` declaring more than 4 KiB is read to the cap and the
over-cap remainder silently ignored — NO damage note, although the constant's
own comment calls over-cap names "corruption". A 100 KB `strn` yields a
4096-byte title and a clean damage log.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/avi/header.rs:740
Code:
```
            if width == 0 || height == 0 || height > i32::MAX as u32 {
```
What is wrong: asymmetric validation — height has an explicit ceiling
(|i32::MIN| rejected), width only a zero check. `biWidth = 0xFFFFFFFF` is
accepted into `StreamFormat::MotionJpeg { width: 4_294_967_295, .. }` and
surfaces as CLI MediaFacts dimensions (apps/cli/src/containers/avi.rs:96).
Current consumers are descriptive (MJPEG frames are self-describing), so no
allocation is driven by it — but the guard's own comment says "Zero or
absurd ... dimensions: skip the stream", and 4 billion px wide is absurd.
Confidence: HIGH — Verification: read + rg of downstream use.

F-006 [S4]
Where: crates/demux/src/avi/header.rs:569-571
Code:
```
    if declared_size < 56 {
        return Err(error::bad_stream_params(chunk_pos));
    }
```
What is wrong: a short-but-present `avih` (declared 40 bytes, physically
available) aborts the WHOLE file via `?` at 483, while the documented
containment rule (module docs 24-32) keeps every degenerate strh/strf to the
stream. avih is file-level so an abort is defensible, but a 55-byte avih
above a perfectly good stream table is refused where ffmpeg reads the prefix;
the asymmetry is undocumented. Blast radius: malformed header only.
Confidence: MEDIUM — Verification: read; ffmpeg comparison not executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AviMainHeader / StreamFormat / PcmSample (types) | 67-208 | REAL | — | closed vocabulary, correct bit widths |
| PcmSample::from_wave_tag | 190-207 | REAL | — | None for impossible combos, tested (tests.rs:926) |
| StreamFormat::codec_descriptor | 217-222 | REAL | — | closed vocab, no bare "pcm" |
| StreamDesc::time_base | 267-269 | REAL | — | typed refusal on zero |
| StreamDesc::track | 278-311 | REAL | — | saturating end stamp, length 0 → Missing |
| StreamDesc::ticks_per_chunk / ticks_for_sample_size | 320-337 | REAL | — | i64::MAX clamp; tested via index.rs |
| DamageLog::note | 397-400 | REAL | S2 | F-001 unbounded events |
| zeroed_main_header | 434-443 | REAL | — | |
| parse_hdrl | 455-533 | REAL | S3 | F-002 sizes-as-authority for avih/strl/JUNK |
| advance | 551-553 | REAL | — | u64, overflow unreachable |
| peek_form | 556-561 | REAL | — | 4-byte exact read, safe indexing |
| parse_avih | 564-583 | REAL | S4 | F-006 whole-file abort on short size |
| parse_strl | 600-861 | REAL | S4 | F-003 dup overwrite, F-004 strn cap; containment otherwise solid |
| peek_size | 864-866 | REAL | — | |

## 5. Test honesty
Tests live in avi/tests.rs (58 tests, baseline ok) + integration tests
idx1_stream_scope.rs / strl_advisory.rs (separate audits). Would they fail if
this file broke? Yes — exact damage reasons, sibling-survival, track counts,
extensible GUID matrix (tests.rs:807-996) are all exact-value asserts.
Gap: no lying avih/JUNK/strl-size-in-hdrl test (F-002); no over-cap strn
test (F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Declared list/chunk sizes are hints, not authority" | header.rs:11 | FALSE in part | advance() at 485/495/505 trusts declared sizes (F-002) |
| "recovers a corrupted hdrl size that would otherwise swallow the movi" | header.rs:16-20 | REAL | tests.rs:2529 passes (baseline) |
| "biHeight is read signed ... accepted with the absolute height and a damage note" | header.rs:33-37 | REAL | code 734-739 + tests.rs:586 |
| extensible "blob too short to carry the GUID" skips stream | header.rs:47-50 | REAL | code 788-791 |
| "every unusable strh/strf parameter set ... is damage-counted" | header.rs:24-28 | REAL | 623-695, 719-839 skip paths |

## 7. Dead / duplicated / unreachable code
None. All public items consumed by avi/mod.rs, index.rs, or CLI containers.

## 8. Dependencies & flags
Depends only on media_types + crate::avi::riff + crate::error (layering clean
per boundaries claim at 122-125). No feature gates. No unsafe.

## 9. Open questions for the next agent
- Does the movi walk (avi/mod.rs step loop) bound `pending` while a single
  `next_event` batch runs? F-001's second copy depends on it.
- Is DamageLog surfaced to the UI unbounded too (Recovered events per frame
  in the transcript)?
