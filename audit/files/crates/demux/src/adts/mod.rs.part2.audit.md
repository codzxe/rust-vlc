# AUDIT: crates/demux/src/adts/mod.rs#part2 (lines 635-1268)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 634 (chunk 635-1268 of 1268; part1=1-634 is A09's) / 25 items in range |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.aac` → probe names "adts" (`crates/media-probe/src/signature/es.rs:373`) → `apps/cli/src/containers/mod.rs:52 "adts" => adts::open` → `apps/cli/src/containers/adts.rs:66 AdtsDemuxer::new`, `:68 start`, `:90 seek` → SelectDemuxer → playback-engine `pipeline.rs:1498 demuxer_reanchor` ← `seek.rs:390`. Baseline: demux lib tests 462/0/0 (`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1955`), incl. `adts::tests::truncated_frame_is_the_typed_truncation ... ok` (:1521)) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `grep -n` structure outline of adts/mod.rs → 25 fns in range; `sed -n` on every quoted range (all verified verbatim); `rg -n "self.fail|sticky"` on ogg/mod.rs + wav/mod.rs → both wrap step errors in `fail()` (ogg:1310, wav:875), adts does not; `rg -n "read_prefix_at"` → `avi/riff.rs:307` (seek+alloc per call), `read_prefix_into` `riff.rs:357-359` (seeks absolutely first); `rg -n "adts"` apps/cli/src/demux_host.rs, containers/mod.rs → dispatch at :52; `sed -n '190,245p'` + `566,643p` adts/tests.rs → truncation/seek tests; baseline greps for adts test execution |

## 1. One-paragraph verdict
This chunk is the ADTS/ADIF walk, recovery, seek and the `Demuxer` trait surface — genuinely implemented, arithmetic traced (pts clock, durations, block math, CRC-16/MPEG-2), executed by the 462-passing demux suite in the baseline. Two real defects: the seek scan (`scan_landing_adts`) is the only unbounded scanner in the file — no byte budget, a positional seek + 7-byte read + heap allocation per garbage byte, on the shipped `--start-time` and runtime-seek paths; and the "typed sticky truncation" that three comments and the error-module docs assert is not implemented — `next_event` returns walk errors raw instead of routing them through `self.fail()` the way the Ogg and WAV precedents it cites actually do.

## 2. What actually works (verified)
- ADTS frame walk `step_adts`/`read_frame_head`/`emit_adts_frame` (652-845): header parse → CRC check → overrun check → pool fill → pts on a samples clock → packet; trailing fragments and inter-frame ID3 handled with bounded skips. Traced arithmetic: pts = pre-advance `samples_clock`, advance = `raw.samples()` (1024×blocks), frame_length ≥ header_len enforced at 659-661 and 928.
- CRC-16/MPEG-2 `crc16_header` (979-992): poly 0x1021, init 0xFFFF, MSB-first, no xorout — matches ISO 13818-7 `crc_check` header half; the short-buffer guard at 664 (`full.len() < 9 ||`) short-circuits before `full[7..9]` can panic — verified safe.
- `BitReader` (1070-1125): every read length-checked (`read_bits` n>32 and `bits_left` guards at 1097), `ProgramConfigElement::parse` bounds `comment_len` against `bits_left` (1055-1058) — no panic path found in ADIF parsing.
- `seek`/`reanchor` compute-then-commit: `seek` restores the reader position (1199, 1205); `reanchor` commits only after a successful landing (1244-1252). Baseline-executed: `adts::tests::adts_seek_roundtrip_lands_at_or_before_the_target ... ok` (nofailfast log; grep `adts::tests::` → 42 hits, all ok).
- `resync` (688-749): 4 MB budget (`MAX_RECOVERY_SCAN_BYTES`), 64 KB windowed reads, budget-exhaustion error at 733-735 — bounded, unlike the seek scan (F-001).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/adts/mod.rs:921-935
Code:
```rust
        while at + 7 <= len {
            let head = self.reader.read_prefix_at(7, at)?;
            let Some(raw) = AdtsHeader::parse(&head) else {
                at += 1;
                continue;
            };
            let hlen = raw.header_len();
            if u64::from(raw.frame_length) < hlen || at + u64::from(raw.frame_length) > len {
                at += 1;
                continue;
            }
            frames += 1;
            if frames > MAX_SEEK_SCAN_FRAMES {
                return Err(error::seek_out_of_range());
            }
```
What is wrong: `scan_landing_adts` has no byte budget. `MAX_SEEK_SCAN_FRAMES` (2M) counts only *valid* frames; every non-frame byte costs one `read_prefix_at(7, at)` call, and each such call is a fresh 7-byte `Vec` allocation plus an absolute `seek_to` + `read_into` (`avi/riff.rs:307-312`, `:357-359`). So a garbage stretch of G bytes costs G × (seek + read + malloc). `open_adts` caps its scan at `MAX_OPEN_SCAN_BYTES` (64 KB) and `resync` at `MAX_RECOVERY_SCAN_BYTES` (4 MB) — the seek scan is the one scanner in this file bounded by nothing but the file length.
Why it matters: reachable from the shipped entry point twice over: `lumen open --start-time N x.aac` calls `demuxer.seek(target)` (`apps/cli/src/containers/adts.rs:90`), and the runtime seek flow calls `reanchor` → the same scan (`playback-engine/src/pipeline.rs:1498`, `seek.rs:390`). A truncated/corrupted or crafted .aac (probe only needs a syncword) of N bytes makes one seek an O(N)-syscall crawl — a multi-minute UI freeze on a 100 MB file, and `seek()` can be called repeatedly.
What correct looks like: a byte budget on the scan (like `MAX_OPEN_SCAN_BYTES`), windowed reads (like `resync`'s 64 KB windows) instead of per-byte 7-byte reads, or an incremental index built during the walk.
Confidence: HIGH — Verification: read (call chain grepped; riff.rs read-cost traced; no test covers a garbage-heavy seek input — seek tests use clean 1- and 16-frame files, adts/tests.rs:569-626).

### F-002 [S3]
Where: crates/demux/src/adts/mod.rs:1193 (with 669-672, 684-687, 708-714; crates/demux/src/adts/error.rs:110-113)
Code:
```rust
                Err(err) => return Err(err),
```
context (669-672):
```rust
        if self.walk + u64::from(raw.frame_length) > len {
            // Declared frame overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet instead — the
```
What is wrong: the walk's fatal errors are returned raw from `next_event` — never routed through `self.fail()`, so `self.sticky` is never set. Yet mod.rs calls the truncation "typed sticky truncation" three times (669-672, 685-687, 708-714) and error.rs:110-113 documents it as "sticky, per the Ogg page-truncation precedent". The precedent does the opposite of this file: Ogg wraps step errors in `self.fail(err)` (`ogg/mod.rs:1310`), and so does WAV (`wav/mod.rs:875`). ADTS is the odd one out.
Why it matters: observable drift: (a) `seekability()` (1222-1224) returns `SlowLinear`/`Fast` after a fatal walk error because it only checks `started`/`sticky` — the Ogg precedent returns `None` once sticky (ogg/mod.rs:1354); (b) a re-entrant `start()` returns `Ok(track)` instead of replaying the fatal error (1136-1138 is dead because sticky is never set by the walk); (c) each `next_event` retry re-executes the failing walk — for `recovery_budget_exhausted` that is a fresh 4 MB scan per call; (d) the test comment "Sticky: the error replays" (adts/tests.rs:222-227) passes by re-derivation, not by sticky state — the test cannot distinguish the two.
What correct looks like: `Err(err) => return Err(self.fail(err))` in `next_event`, matching ogg/wav and the three in-file comments; or delete the "sticky" wording everywhere.
Confidence: HIGH — Verification: read (all three demuxers' `next_event` compared; tests read; baseline shows the test passing under the non-sticky implementation, nofailfast:1521).

### F-003 [S4]
Where: crates/demux/src/adts/mod.rs:1207-1210
Code:
```rust
        // 1/1000 (coprime, non-zero): exactly `TimeBase::new`'s Ok
        // value, constructed directly — the DVB-builder precedent. The
        // fallback only exists for the dead track-absent arm (seek is
        // `not_started`-guarded; 8.A1 sweep).
```
What is wrong: `seek` (1198-1205) has no `not_started` guard — only `reanchor` does (1236-1238). The comment's justification is false; the arm is dead for a different reason (verified: `head` is set only inside `start()`'s closure at 1148/1154, `mint_track` unconditionally sets `track` at 646, and `descriptor()` cannot fail for its static ASCII literals, 248-250). A pre-`start` `seek` — which the trait explicitly blesses ("hosts can validate targets before a session exists", plugin-api/src/demux.rs:39-43) — returns `demux.adts.seek.out-of-range` instead of `not_started`, a misleading kind for a not-opened container.
Why it matters: copy-pasted comment (identical text at 1254-1257 where it is true for reanchor) misleads the next maintainer about which method is guarded; the wrong typed code reaches the CLI error surface.
What correct looks like: fix the comment to state the real invariant (head set ⟹ track set), and return `not_started()` for pre-`start` seeks as `reanchor` does.
Confidence: HIGH — Verification: read (guard absence verified at 1198-1205; head/track coupling traced through start/mint_track/descriptor).

### F-004 [S4]
Where: crates/demux/src/adts/mod.rs:1240-1244
Code:
```rust
        let landing = match self.head.as_ref() {
            Some(RawAacFormat::Adif(_)) => self.landing_adif(target_nanos),
            _ => self.scan_landing_adts(target_nanos),
        };
        let (samples, position) = landing?;
        // Commit (D4: every failure above left us untouched).
```
What is wrong: on `Err`, `reanchor` leaves the underlying reader wherever `scan_landing_adts` last read (every `read_prefix_at` seeks absolutely, riff.rs:359) — the read position is not restored, contradicting trait invariant D4's letter ("the demuxer is left at its pre-seek position", plugin-api/src/demux.rs:100-103). `seek` does save/restore (1199, 1205); `reanchor` does not.
Why it matters: the two seek halves disagree about position discipline; benign today only because every walk read is absolute, so the stale position is never observed — a future relative read would corrupt the walk.
What correct looks like: same save/restore as `seek`, or a comment documenting that reads are absolute so D4 holds trivially.
Confidence: HIGH — Verification: read (riff.rs read paths traced; both methods compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `read_frame_head` | 652-679 | REAL | — | CRC short-circuit safe (664); overrun → truncation err; no panic path |
| `resync` | 688-749 | REAL | — | 4 MB budget + 64 KB windows; first-claim-wins truncation (documented ffmpeg divergence, 708-714); layer bits not rechecked mid-scan (documented, 683-684) |
| `fill_frame` | 752-757 | REAL | — | absolute positional read into pool; frame_length ≤ 8200 by MAX_FRAME_BYTES |
| `emit_adts_frame` | 760-806 | REAL | — | pts pre-advance, clock advance `raw.samples()`; duration uses the *reference* header's rate for param-changed frames — flagged with Discontinuity (790-793), honest |
| `step_adts` | 809-845 | REAL | — | ID3 skip, trailing-fragment bounded skip, resync retry; walk not advanced on emit error → idempotent retry |
| `step_adif` | 848-902 | REAL | — | block arithmetic traced; partial final block marked discontinuity; `take as usize` safe for sane block_bytes (≥1, u64 file offsets) |
| `scan_landing_adts` | 907-943 | REAL | S2 | F-001: no byte budget, per-byte seek+read+alloc on garbage |
| `landing_adif` | 946-962 | REAL | — | O(1); past-end target → typed refusal; saturating conversions |
| `target_nanos` | 964-972 | REAL | — | Missing stamp → 0 silently (seek-to-start); acceptable fallback |
| `crc16_header` | 979-992 | REAL | — | CRC-16/MPEG-2 traced; covered by tests (baseline ok) |
| `be16` | 994-996 | REAL | — | trivial, correct |
| `ProgramConfigElement::parse` | 1007-1064 | REAL | — | all reads bounded; comment_len ≤ 255 guarded by bits_left (1055-1058); channels ≤ 93, no overflow |
| `BitReader::new` | 1076-1078 | REAL | — | trivial |
| `BitReader::byte_pos` | 1080-1082 | REAL | — | trivial |
| `BitReader::bits_left` | 1084-1086 | REAL | — | `len*8` no overflow (≤4096-byte window) |
| `BitReader::read_bit` | 1088-1090 | REAL | — | delegates to read_bits |
| `BitReader::read_byte` | 1092-1094 | REAL | — | delegates |
| `BitReader::read_bits` | 1096-1108 | REAL | — | n>32 + bits_left guard before indexing — no panic |
| `BitReader::skip` | 1110-1116 | REAL | — | guarded |
| `BitReader::align_byte` | 1118-1124 | REAL | — | rem≠0 → skip(8-rem), guarded |
| `Demuxer::start` | 1128-1164 | REAL | — | sticky replay of *start* errors works (1136-1138); walk errors never set sticky (F-002 interplay) |
| `Demuxer::next_event` | 1166-1196 | REAL | S3 | F-002: `Err(err) => return Err(err)` — no `fail()` wrap, unlike ogg:1310 / wav:875 |
| `Demuxer::seek` | 1198-1220 | REAL | S4 | F-003: comment claims a guard that is absent; pre-start seek → misleading seek_out_of_range; position restore correct |
| `Demuxer::seekability` | 1222-1233 | REAL | — | ADIF Fast{indexed:false}, ADTS SlowLinear — matches trait semantics; sticky gate neutered by F-002 |
| `Demuxer::reanchor` | 1235-1267 | REAL | S4 | F-004: reader left mid-scan on Err; commit path itself correct (pending cleared, eos reset, Discontinuity queued) |

## 5. Test honesty (tests in adts/tests.rs exercising this chunk)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `truncated_frame_is_the_typed_truncation` (tests.rs:209) | YES for the error itself; NO for stickiness | asserts code `demux.adts.truncated` and a replayed error — but the replay passes by re-derivation (walk unchanged), so deleting the sticky mechanism (which is already absent) changes nothing |
| `truncated_first_frame_is_the_typed_truncation_at_start` (tests.rs:228) | YES | start-error code pinned |
| `adts_seek_roundtrip_lands_at_or_before_the_target` (tests.rs:569) | YES | landing ticks 5*1024 pinned; off-by-one (≤ vs <) would land at 4*1024 and fail |
| `adts_seek_past_the_end_lands_at_the_last_frame` (tests.rs:608) | YES | clamp-to-last pinned; refusal would surface as `must()` panic |
| `adif_seek_is_format_inherent_arithmetic` (tests.rs:534) | YES | seekability variant pinned |
| `committed_mutation_fixtures_are_typed_or_recover_never_panics` (tests.rs:630) | WEAK for the seek scan | fixtures are ≤8 KB — the unbounded byte-creep (F-001) cannot manifest at that size; absence-of-panic only |
| Baseline execution | — | nofailfast:1521 truncated test ok; 42 `adts::` tests, 0 failed |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the truncated-frame path; sticky, per the Ogg page-truncation precedent" | crates/demux/src/adts/error.rs:110-113 | FALSE (as implemented) | `next_event` returns walk errors raw (mod.rs:1193); ogg/mod.rs:1310 wraps in `self.fail`; no sticky state is ever set on this path (F-002) |
| "seek … the query above stays pure so hosts can validate targets before a session exists" | plugin-api/src/demux.rs:39-43 | SHALLOW | pure: yes (position restored); pre-session: returns misleading `seek.out-of-range` instead of `not_started` (F-003) |
| "never hangs … by construction of the demuxer and the budgets above" | crates/demux/src/adts/fuzz.rs:44-46 | REAL for tested input sizes; UNPROVEN at large N | seek scan has no byte budget (F-001); harness inputs are small fixtures |

## 7. Dead / duplicated / unreachable code
- The `unwrap_or(TimeBase { num: 1, den: 1_000 })` fallbacks (1211-1215, 1258-1262): dead (head set ⟹ track set — traced), retained as panic-free residue; justification comment wrong (F-003).
- `read_byte` (1092-1094) used only for the ADIF copyright string (part1's `open_adif`); not dead.
- The sticky replay arm in `start` (1136-1138) is live only for start-errors; walk-errors never reach it (F-002).

## 8. Dependencies & flags
- No feature gates in this chunk; `mod tests` is cfg-gated (109-110 area); `pub mod fuzz`/`test_support` are in-crate modules.
- `SourceReader` comes from `avi/riff.rs` — cross-format reuse inside the demux crate; no layering violation.

## 9. Open questions for the next agent
- A09 (adts part1): does `open_adts`'s 64 KB scan share the per-byte `read_prefix_at(7, at)` cost shape? (Likely yes but bounded — confirm.)
- Whoever owns playback-engine seek: does any caller retry `next_event` after a demux Err, multiplying F-001/F-002 costs? (`pipeline.rs` demux loop error handling.)
- Wave gate: REQUEST-ORCH-RUN not needed; static analysis is conclusive for these findings.
