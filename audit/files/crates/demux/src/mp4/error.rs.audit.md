# AUDIT: crates/demux/src/mp4/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-21 |
| Lines / functions | 170 / 14 (12 constructors, `mint`, `STAGE`) |
| File verdict | REAL (one dead constructor + half-pinned error contract + one cross-file S2 traced from this audit) |
| Reachable from shipped entry point? | YES — every non-dead constructor has call sites in `crates/demux/src/mp4/mod.rs` (rg counts: truncated 11, bad_box 14, not_started 3, seek_out_of_range 7, no_sample_table 3, scan_budget_exhausted 3, not_mp4/oversized_box/no_moov/no_tracks/box_overruns_parent 1 each); `Mp4Demuxer` is wired: `lumen open file.mp4` → `host.rs:904` → `containers/mod.rs:67 "mp4" => mp4::open` → `containers/mp4.rs` → mod.rs. Exception: `depth_exceeded` — 0 callers (F-001). |
| Findings | S0: 0 S1: 0 S2: 1 (cross-file) S3: 2 S4: 2 |
| Commands run | per-constructor `rg -c "error::<name>" crates/demux/src/mp4/`; `rg -n "depth_exceeded\|MAX_BOX_DEPTH" crates apps --type rust` (0 users each); `rg -o "demux\.mp4\.[a-z.-]+" …tests/fixtures/cli` (code-assertion census); `sed -n '690,710p' mp4/mod.rs` (sniffed bound = 32); `rg -n "sniffed" crates/media-types/src/error.rs` (Display cap 16, lines 311-324); baseline `rg -c "test mp4::tests::" test-workspace.log.txt` → 38/38 ok (lines 1805+); **empirical language trace**: `ffmpeg -f lavfi … /tmp/lang.mp4` + `ffprobe -show_entries stream_tags=language` (`und`) + python hexdump of `mdhd` (`packed_lang=0x55C4`); `sed -n` verification of every quote. |

## 1. One-paragraph verdict
This is a disciplined typed-error module: stable `demux.mp4.*` codes, correct taxonomy kinds per failure class (Corrupt/Unsupported/ResourceLimit), a verified "no secrets" posture (sniffed bytes bounded to 32 at the call site and capped to 16 by the taxonomy's Display), and the wav/ogg constructor pattern faithfully mirrored. It is also NOT the MP4 blocker: the baseline smoke sessions show the demux stage succeeding (2 tracks, duration, `seekable: true`) with failures minted by the decode layer (`decode.unsupported … h264.sw.unsupported`), so these constructors fire only on malformed containers. The defects found: `depth_exceeded` is dead code documenting a depth guard that does not exist anywhere in the walk (its companion constant `MAX_BOX_DEPTH` also has zero users), four of twelve minted codes — including the most-used one (`bad_box`, 14 call sites) — are never asserted by any test while the fuzz harness explicitly swallows errors, `oversized_box` drops its `_kind` argument, and `scan_budget_exhausted` hard-codes `attempted := budget`. Separately, this audit traced the baseline's standing `language: Some("voe")` garbage to a real cross-file defect in `mp4/mod.rs::unpack_language` (F-005, empirically proven).

## 2. What actually works (verified)
— Taxonomy integrity: no local error kinds; only `media_types::{EngineError, ErrorKind, PipelineStage}` + `media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind}` — the "never redefined here" claim (error.rs:3-4) holds (read the whole file; no local enum).
— "No secrets" verified end-to-end: `not_mp4(&head)` at mod.rs:701 receives a ≤32-byte prefix (`read_prefix_at(32, 0)`), and the taxonomy Display caps sniffed hex at 16 bytes — media-types/src/error.rs:311-324 ("sniffed bytes capped at 16 (content hints, never secrets…)").
— Kind/code selection is right per class: truncation → `Corrupt::Truncated` (41-48), structural lie → `Corrupt::InvalidStructure` with offset (53-90), allocation refusal → `ResourceLimit::Memory` (95-104), bounded recovery → `ResourceLimit::RecoveryAttempts` (161-170), lifecycle/seek → `Unsupported` (141-156).
— Eight of twelve codes are asserted in `crates/demux/src/mp4/tests.rs` (census: truncated ×4, seek.out-of-range ×2, not-started ×2, no-tracks ×2, box-overruns-parent ×2, not-mp4/no-sample-table/no-moov ×1; plus `containers/mp4.rs` pins not-started) — and all 38 `mp4::tests` ran green in baseline (test-workspace.log.txt:1805+, 0 failures).
— The wav-mirror claim checked: `wav/error.rs:131-133` has the identical `not_started → Unsupported { format: None }` shape.

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/mp4/error.rs:78-90 (constructor) + crates/demux/src/mp4/mod.rs:31,124-126 (the guard it documents)
Code (error.rs):
```
/// The box-descent depth exceeded the walk's guard (`moov → trak →
/// mdia → minf → stbl → …` is six; anything deeper than the guard is
/// a cycle-shaped lie, not a file).
pub fn depth_exceeded(offset: u64) -> EngineError {
```
and (mod.rs:31):
```
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level.
```
and (mod.rs:124-126):
```
/// Box-descent depth guard (`moov→trak→mdia→minf→stbl` is five;
/// `moov→udta→meta→ilst→©nam→data` is six).
pub const MAX_BOX_DEPTH: u32 = 8;
```
What is wrong: `depth_exceeded` has ZERO callers repo-wide (`rg -n "depth_exceeded" crates apps --type rust` → only its own definition; the only other `depth-exceeded` hits are the unrelated `playlists.mpd.*` codes). Its companion guard `MAX_BOX_DEPTH` likewise has zero users (rg: only the doc mention at mod.rs:31 and the definition at mod.rs:126). The mod.rs doc claims the box walk runs "with a depth guard" — it does not. The actual walk (`iter_boxes`, mod.rs:345-390, plus its fixed call-site nesting at mod.rs:916→934→1008/1051→1072→1099→1127) is bounded lexically — each nesting level is a distinct hard-coded call site — so unbounded recursion cannot happen, but that is structural luck, not the documented guard.
Why it matters: a documented safety property that does not exist is worse than no claim: reviewers and future maintainers will trust `MAX_BOX_DEPTH` to catch "cycle-shaped lies"; anyone adding a generic recursive descent (e.g. for `meta`/`ilst` trees, which the constant's own comment anticipates) will assume a guard that isn't wired. Plus a dead public constructor in the error vocabulary.
What correct looks like: either wire the guard (check depth in `iter_boxes` callers / a recursive descent and call `depth_exceeded`), or delete constructor + constant + the mod.rs:31 sentence.
Confidence: HIGH — Verification: read + rg (0 callers; walk structure traced).

### F-002 [S3]
Where: crates/demux/src/mp4/error.rs:53-61 (bad_box), 95-104 (oversized_box), 161-170 (scan_budget_exhausted) — and the test suites that never pin them
Code (bad_box — the most-used constructor in the module):
```
pub fn bad_box(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mp4.bad-box",
    )
}
```
What is wrong: The minted codes `demux.mp4.bad-box` (14 production call sites), `demux.mp4.oversized-box` (1), `demux.mp4.scan-budget-exhausted` (3) and `demux.mp4.depth-exceeded` (dead) are asserted by ZERO tests repo-wide (`rg "bad-box|oversized-box|scan-budget|depth-exceeded"` over demux tests, CLI tests, containers → no hits). The fuzz harness's own contract explicitly swallows errors: mp4/fuzz.rs:6-8 "malformed input is *expected* to produce typed errors, which are swallowed here — the conformance suite pins the specific error flavors" — but the conformance suite pins only 8 of 12 flavors.
Why it matters: the error vocabulary is the demuxer's public contract (the CLI surfaces `err.context.code` verbatim — see the smoke logs). A regression that makes `bad_box` mint, say, `truncated`, or drops the offset, ships green. `bad_box` is the generic structural-refusal for the most common corruption class (bad 64-bit sizes, non-advancing extents, zero timescale) and it is the one constructor with the most call sites and the least test protection.
What correct looks like: pin each code once in mp4/tests.rs damage cases (the existing damage tests for 64-bit-size-below-16 and zero-timescale already produce bad_box errors — just assert the code string), plus one oversized-moov and one moof-ladder-budget case.
Confidence: HIGH — Verification: read + rg census (documented above).

### F-003 [S4]
Where: crates/demux/src/mp4/error.rs:95
Code:
```
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
```
What is wrong: the `_kind` parameter is accepted and dropped — the minted ResourceLimit error never names which box kind (`moov`? `moof`? `stbl`?) blew the ceiling. The module's own header (error.rs:11-12) advertises "Constructors take byte counts, offsets and box kinds only" — implying kinds are part of the error payload.
Why it matters: when a real file trips the 16 MiB moov ceiling, the operator sees "oversized-box budget=16777216 attempted=…" with no hint of the box; diagnosis requires re-running with a debugger. `ErrorContext` has room for details (the CLI's ts bridge uses `.with_detail`).
What correct looks like: include the kind in the context (`.with_detail("box", kind_str)`) or in the code suffix (`demux.mp4.oversized-moov`), or drop the parameter honestly.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/demux/src/mp4/error.rs:161-170
Code:
```
pub fn scan_budget_exhausted() -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::RecoveryAttempts,
            budget: super::MAX_FRAGMENT_SCAN_BOXES as u64,
            attempted: super::MAX_FRAGMENT_SCAN_BOXES as u64,
        },
        "demux.mp4.scan-budget-exhausted",
    )
}
```
What is wrong: `attempted` is hard-coded equal to `budget` — it is a constant duplicate, not the measured attempt count. The two-field ResourceLimit detail exists to distinguish "budget 65536, attempted 65536" (hit exactly) from "attempted 65537+" (overshoot) and to future-proof a dynamic budget; as written the second field carries zero information.
Why it matters: lazy evidence fields teach consumers to ignore the detail; if the budget ever becomes configurable the "attempted" number silently lies.
What correct looks like: `scan_budget_exhausted(attempted: u64)` fed by the actual counter from the fragment scan.
Confidence: HIGH — Verification: read.

### F-005 [S2] — CROSS-FILE (defect located in crates/demux/src/mp4/mod.rs, reported here because this audit traced it; owner of mod.rs should lift this finding)
Where: crates/demux/src/mp4/mod.rs:534-539 (`unpack_language`)
Code:
```
/// Unpack the mdhd 3×5-bit language code; `None` for "und".
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
What is wrong: The function maps only the properly-packed `und` (0x5193) to `None`. ffmpeg — the reference implementation this repo measures against everywhere — writes `0x55C4` as the mdhd language for every unspecified-language track, and maps it back to `und` on demux. `0x55C4` unpacks to the garbage tag `"voe"`.
Why it matters: every ffmpeg-authored MP4 surfaces `language: Some("voe")` on all its tracks — visible in the repo's own baseline evidence: `mp4-smoke-open-session.txt` and `mp4-smoke-open-session-420.txt` both show `language: Some("voe")` on the h264 and aac tracks (and the baseline INDEX lists this as a standing defect: "TrackInfo `language: Some(\"voe\")` … garbage default (demux wave to trace)" — traced, herewith). User-visible wrong metadata on the single most common MP4 authoring path in existence.
Empirical proof (run by me, 2026-11-21): `ffmpeg -f lavfi -i testsrc … /tmp/lang.mp4` (ffmpeg 7.1.5-0+deb13u1, same 7.1.x line as the baseline corpus) → `ffprobe -show_entries stream_tags=language` reports `und` for the stream; python hexdump of the same file's `mdhd` box: `version=0 packed_lang=0x55C4`. ffprobe says "und"; lumen says "voe".
What correct looks like: `if tag == "und" || packed == 0x55C4 { None }` (mirror ffmpeg's `mov_read_mdhd` unspecified marker) — plus a test with a 0x55C4 mdhd fixture.
Confidence: HIGH — Verification: ran (ffmpeg author + ffprobe + hexdump, outputs above) / read (mod.rs call chain :1117 → :880-888 → TrackInfo).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| STAGE (const) | 18-19 | REAL | — | PipelineStage::Demux, used by every mint |
| mint | 21-23 | REAL | — | kind + ErrorContext::new(code); no details map used (see F-003) |
| not_mp4 | 27-37 | REAL | — | sniffed ≤32 B (mod.rs:701); Display caps 16 (media-types error.rs:311-324); code asserted in tests |
| truncated | 41-48 | REAL | — | 11 call sites; code asserted ×4 |
| bad_box | 53-61 | REAL | S3 | 14 call sites, code NEVER asserted (F-002) |
| box_overruns_parent | 67-76 | REAL | — | offset carries parent_end (documented); code asserted ×2 |
| depth_exceeded | 81-90 | UNREACHABLE (dead) | S3 | 0 callers; guard it documents does not exist (F-001) |
| oversized_box | 95-104 | REAL | S4 | `_kind` ignored (F-003); code never asserted (F-002) |
| no_moov | 107-114 | REAL | — | asserted ×1 |
| no_tracks | 119-126 | REAL | — | asserted ×2 |
| no_sample_table | 130-139 | REAL | — | asserted ×1 |
| not_started | 142-147 | REAL | — | asserted ×2 + CLI containers/mp4.rs ×1; mirrors wav shape |
| seek_out_of_range | 151-156 | REAL | — | 7 call sites; asserted ×2 |
| scan_budget_exhausted | 161-170 | REAL | S4 | attempted := budget constant (F-004); code never asserted (F-002) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mp4/tests.rs damage cases asserting 8 of 12 codes | YES | exact code-string pins (census above) |
| No test exists for bad_box / oversized_box / scan_budget_exhausted / depth_exceeded codes | NO | nothing pins them; fuzz swallows errors by documented contract (mp4/fuzz.rs:6-8) — a wrong-code regression in the most-used constructor ships green |
| 38 mp4::tests in baseline | YES (for what they pin) | all green, test-workspace.log.txt:1805+; they exercise 8 codes + the open paths |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The error taxonomy is media-types' … never redefined here" | error.rs:3-4 | REAL | no local error kinds in this file (full read) |
| "No secrets … Sniffed magic bytes are capped by the taxonomy's own Display" | error.rs:9-13 | REAL | call site bound 32 (mod.rs:701) + Display cap 16 (media-types/src/error.rs:311-324) |
| "mirroring the `wav`/`ogg` constructor pattern" | error.rs:6-7 | REAL | wav/error.rs:131-133 identical not_started shape |
| "walked in memory with a depth guard ([`MAX_BOX_DEPTH`])" | mod.rs:31 | **FALSE** | rg: MAX_BOX_DEPTH 0 users; depth_exceeded 0 callers (F-001) |
| MP4 open failure is demux-typed | baseline INDEX (MP4 smoke) | FALSE (misattribution risk) | smoke sessions: demux succeeds (tracks/duration/seekable), error is `decode.unsupported … h264.sw.unsupported` — decode-stage, not this module (mp4-smoke-open-session.txt; cf. 420 session: full play, EXIT=0) |

## 7. Dead / duplicated / unreachable code
- `depth_exceeded` — dead constructor (F-001); its sibling constant `MAX_BOX_DEPTH` (mod.rs:126) — dead.
- No duplication within the file; each constructor one code, one kind.

## 8. Dependencies & flags
- No feature flags, no `unsafe`, no external crates; depends only on `media_types` (taxonomy types) and `super::MAX_FRAGMENT_SCAN_BOXES` (the single super-dependency, F-004's subject).

## 9. Open questions for the next agent
- mp4/mod.rs owner: lift F-005 (`voe` language) and F-001's dead guard; also check whether `meta`/`ilst` nesting (anticipated by MAX_BOX_DEPTH's comment) is handled anywhere — if a future unit adds it, the guard must land with it.
- mp4/tests owner: pin the four unpinned codes (F-002) — the damage fixtures likely already produce bad_box errors.
- MP4 wave synthesis: this module exonerates the demux error layer for the owner's "MP4 cannot be opened" report — the shipped evidence chain (engine-info hard-coding, decode-bench fall-through, h264.sw 4:2:0-only support) points entirely at the decode/registry layer, per baseline INDEX CORRECTION v2.
