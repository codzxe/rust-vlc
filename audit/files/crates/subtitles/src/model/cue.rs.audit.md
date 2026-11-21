# AUDIT: crates/subtitles/src/model/cue.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 148 / 4 fns/methods, 3 type defs, 2 tests |
| File verdict | UNREACHABLE (core model type; live vocabulary crate-wide) |
| Reachable from shipped entry point? | NO — crate not linked into `lumen-cli`/apps (only `fuzz/` + own tests). In-crate: `Cue`/`CueId`/`CueText`/`StyledSegment` are the document model used by srt/vtt/ass parsers, model::wire, and the render paths (rg -l: 14 files incl. srt/parser.rs, ass/parser.rs, ass/tags.rs, model wire roundtrip suites). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -l "CueText|Cue::new|CueId" crates/subtitles/src` (14 consumer files) · `rg "total_len|is_plain" crates/subtitles/src` (production callers: NONE — only own tests + one ass/parser.rs fuzz-test assertion at :1413, verified `sed -n 1405,1416p`) · orchrun log: model::cue::tests 2/2 ok (5419-5420) |

## 1. One-paragraph verdict
Forty lines of genuinely clean pure data — a newtyped id, a plain/styled payload enum with an explicit memory rationale, a non-fallible aggregate constructor over typed components — with no panics, no arithmetic, no I/O. Nothing is wrong at the code level; the only defects are that the two `CueText` helper methods have no production caller, and one of the two tests spends most of its assertions on other modules' behavior. Unreachable from any shipped binary like the rest of the crate.

## 2. What actually works (verified)
— 2/2 unit tests ok: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5419-5420.
— `total_len` saturating fold verified by read (usize::saturating_add seed 0 — no overflow possible even on hostile run counts); zero cases pinned by test.
— `Cue::new` const, aggregate of already-validated parts (`CueInterval`, `Percent` carry their own typed validity — cross-checked in model/time.rs and position.rs audits); zero-duration interval documented legal and consistent with `CueInterval::new(s, s)` being Ok (test at :117 uses 0..1_500_000; EndBeforeStart only for end<start, pinned at :132-139).

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/model/cue.rs:52-71
Code:
```
    pub fn total_len(&self) -> usize {
        match self {
            Self::Plain(s) => s.len(),
            Self::Styled(segs) => segs
                .iter()
                .map(|s| s.text.len())
                .fold(0usize, usize::saturating_add),
        }
    }
```
(with `is_plain`, :66-70)
What is wrong: neither helper has a production caller. `total_len`'s only uses are its own test (:126,140) and one fuzz-invariant assertion in ass/parser.rs:1413 (test mod, verified by sed); `is_plain`'s only use is its own test (:125). The render paths match on the variants directly (srt/render.rs:145-146).
Why it matters: public model API that nothing exercises in production — untested-by-use surface that a future refactor can silently break or repurpose (the classic dead-accessor pattern).
What correct looks like: either use them (e.g. wire/render guards on `total_len`) or mark them `#[doc(hidden)]`/remove until a consumer exists.
Confidence: HIGH — Verification: read + rg caller sweep.

### F-002 [S4]
Where: crates/subtitles/src/model/cue.rs:131-147
Code:
```
    fn typed_invalidity_stays_in_components() {
        let err = CueInterval::new(10, 9).err();
```
What is wrong: the test named "typed invalidity stays in components" asserts `CueInterval::new`'s error (model/time.rs behavior) and `TextStyle::new`'s fields (model/style.rs behavior) — other modules' contracts. The only cue.rs-native assertions are `total_len()==0` twice. The name promises a guarantee about `Cue` construction; the body cannot fail on any cue.rs mutation except the two trivial length checks.
Why it matters: test-name/body mismatch inflates perceived coverage of this file (the mutation-resistance of 80% of this test belongs to other files).
What correct looks like: rename/split — keep the CueInterval/TextStyle assertions in their own modules' suites; here assert what `Cue`/`CueText` themselves guarantee (e.g. construction equality roundtrip of all fields).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CueId::new` / `get` | 15-26 | REAL | — | Opaque u64 newtype; any value legal (documented); used by every parser for cue numbering. |
| `StyledSegment` | 30-36 | REAL | — | Pure data; empty-run legal (documented). |
| `CueText::total_len` | 52-64 | REAL (test-only callers) | S4 | Saturating, correct; F-001. |
| `CueText::is_plain` | 66-70 | REAL (test-only callers) | S4 | F-001. |
| `Cue::new` | 87-103 | REAL | — | Const aggregate; non-fallible by construction — claim verified (components typed). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `cue_assembles_typed_parts` (116-128) | YES (weakly) | Field roundtrip via accessors — a swap of id/interval fields or a wrong total_len fails; a broken `Cue::new` that drops positioning would NOT fail (positioning never asserted). |
| `typed_invalidity_stays_in_components` (131-147) | NO (for this file) | 4 of 6 assertions target model/time.rs and model/style.rs (F-002); cue.rs-native part is two empty-payload lengths. |
Both executed ok (orchrun log:5419-5420).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| lib.rs:58-61 "[`model::Cue`] — identity, half-open interval, text payload, Positioning" | lib.rs card 5.L2 scope | REAL | Types exist as described and are the live crate-wide document model (14 consumer files). |
| "a stable wire form ([`model::wire`]) whose roundtrip … is the card's acceptance test" | lib.rs:67-68 | REAL (test-only) | roundtrip_matrix integration suite ran green in the repair run (orchrun log:5616 `Running tests/roundtrip_matrix.rs`); no shipped entry point. |
| Matrix rows subtitle-srt/vtt "implemented" | docs/feature-matrix.toml:892-930 | UNREACHABLE | Crate unlinked from shipped binaries (see file-1 audit, pipeline_host.rs:1238). |

## 7. Dead / duplicated / unreachable code
`CueText::{total_len, is_plain}` — no production callers (F-001). No duplication found.

## 8. Dependencies & flags
Crate-internal only (model::position, model::time, model::style). No features, no unsafe, no panics, const fns throughout — verified by read.

## 9. Open questions for the next agent
1. model/wire.rs auditor: does the wire form ever preserve `is_plain`-ness distinctly (Plain vs Styled-of-one-unstyled-run)? If not, the roundtrip's `equal` may accept a representation change — worth one assertion.
2. srt/vtt parser auditors: confirm duplicate `CueId`s (two cues, same id) are rejected or tolerated somewhere — `CueId` itself has no uniqueness domain (documented), so the invariant must live in the document builders.
