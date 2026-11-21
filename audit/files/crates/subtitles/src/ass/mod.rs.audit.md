# AUDIT: crates/subtitles/src/ass/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 52 (module docs + 5 declarations; no code) / 5 declared modules, 0 functions |
| File verdict | UNREACHABLE (claims verified against the declared modules' code — the wave question "do the ASS subset claims match the code?" answers YES at crate level; no shipped entry point reaches any of it) |
| Reachable from shipped entry point? | **NO.** `subtitles::ass::parse` is called only by `crates/subtitles/tests/ass_ffmpeg_ac.rs:26,48,75,111` and the in-crate `ass/fuzz.rs:30` (which `fuzz_sub_ass` is itself only invoked from crate tests). `rg -n "ass::" crates/subtitles/tests/*.rs` → 4 hits, all in-crate. No external crate depends on `subtitles` (see dvb/mod.rs#part1 audit). `lumen engine-info`: no subtitle lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cat crates/subtitles/src/ass/mod.rs` (full read); `rg -n '"b" \| "i"|"fs" =>|"fn" =>|"c" \| "1c"|"alpha" \| "1a"|"pos" =>|"move" =>|_ =>' crates/subtitles/src/ass/tags.rs` (tag-set verification: b/i/u/s:222, fs:242, fn:252, c/1c:261, alpha/1a:274, 2c-4c/2a-4a:286, pos:297, move:311, drop-count default:326); `rg -n 'Script Info|V4\+ Styles|V4 Styles|Format|Style:|Dialogue:|parse_time|centis' crates/subtitles/src/ass/parser.rs` (sections + H:MM:SS.CC verified; parse_time:785, test:1024); `rg -n 'hard_spaces|\\\\N|\\\\h' crates/subtitles/src/ass/parser.rs crates/subtitles/src/ass/tags.rs` (escapes verified: tags.rs:472, parser.rs test :1189-1194); `rg -n "MAX_DAMAGE|damage_cap|dropped_damage" crates/subtitles/src/ass/parser.rs` → no hits (uncapped ledger, F-001); `rg -c "damage.push" crates/subtitles/src/ass/parser.rs` → 13 uncapped push sites; `sed -n '24,26p' crates/subtitles/src/ass/fuzz.rs` (the false "capped" claim); `sed -n '1,110p' docs/formats/sub-ass-ssa.md` + `sed -n '905,916p' docs/feature-matrix.toml` (claims ledger); `rg -c "test ass::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 50 ass tests ok; `rg -n "test result" …` → lib 267/0 (the exact number the matrix evidence cites). |

## 1. One-paragraph verdict
This file is documentation and declarations only, so the audit is the wave's hostile question: do the ASS subset claims match the code? They do — I verified every concrete claim in the module doc against `parser.rs`/`tags.rs`: sections (`[Script Info]`, `[V4+ Styles]`/`[V4 Styles]`, `[Events]`), `H:MM:SS.CC` centisecond times, the twelve G-2 tags (`\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move`) implemented natively in `tags.rs`'s match arms with per-name drop counters for everything else, `\N`/`\n`→newline and `\h`→hard-space, the infallible `parse`, and the typed-damage skip-one-cue model; the "267/0 lib tests" evidence number matches the baseline no-fail-fast run exactly, and 50 `ass::` tests ran green there. The record's "partial" status is honest (a long, specific deferred list). Three defects: the child `fuzz.rs` claims "the damage ledger is capped" while `parser.rs` pushes 13 uncapped `damage.push` sites with no cap constant (same false-bounds class as VTT), the tag list's markdown backticks are unbalanced (mangled rustdoc span), and a stale `[LEAD SEAM]` merge-plan comment says this file "declares ONLY the render-path modules" while declaring `pub mod parser;` eight lines above it. And the standing verdict: no shipped binary can reach any of it.

## 2. What actually works (verified)
— Every subset claim in the module doc, verified tag-by-tag and escape-by-escape against the declared modules' code (commands above), with execution evidence `orchrun-test-workspace-nofailfast.log.txt`: `subtitles` lib `test result: ok. 267 passed` (matches the matrix row's cited "267/0" exactly), 50 `ass::*` tests ok, `ass_ffmpeg_ac` 4 passed.

## 3. Findings

### F-001 [S3] Child module's fuzz contract claims "the damage ledger is capped" — the ASS ledger is uncapped (13 push sites, no cap)
Where: crates/subtitles/src/ass/mod.rs:14-20 (the damage model this module advertises) vs crates/subtitles/src/ass/fuzz.rs:20-23 (the false bound: line 22) vs crates/subtitles/src/ass/parser.rs:261 + 13 push sites
Code (ass/fuzz.rs:20-23, verbatim):
```rust
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
//! and the cue count is bounded by the line count.
```
Code (parser.rs:260-261):
```rust
    /// Typed tolerance events, in encounter order.
    pub damage: Vec<AssDamage>,
```
What is wrong: `AssDocument.damage` is a plain `Vec` pushed at 13 sites (`rg -c "damage.push" parser.rs` → 13) with no cap constant, no dropped-counter (`rg "MAX_DAMAGE|damage_cap|dropped_damage" parser.rs` → no hits) — unlike SRT's 256-event `DamageSink` and DVB's 64-event ledger. A hostile ASS of N bad dialogue lines yields N damage events, linear but unbounded-in-count, so "capped" is false. The `#[non_exhaustive]`-style honesty discipline the crate sells (SRT/DVB ledgers a UI can safely display) is broken by the ASS parser exactly as it is by VTT.
Why it matters: the fuzz harness's "never allocates without a ceiling" contract rests on the false premise; a future consumer dumping the ledger on a corrupt fansub file gets an unbounded report.
What correct looks like: the SRT `DamageSink` pattern (cap + dropped count — `AssStats` is right there to carry the counter).
Confidence: HIGH — Verification: read (all 13 push sites; no cap logic exists).
Note: the fixing owner is the parser.rs/tags.rs agent (F is filed here because mod.rs:14-20 is the damage-model contract this module declares and fuzz.rs is this module's child).
### F-002 [S4] Unbalanced backticks mangle the module doc's tag list
Where: crates/subtitles/src/ass/mod.rs:6-7
Code:
```rust
//! **G-2 override-tag minimum** natively (issue #17: `\b \i \u \s
//! `\fs \fn \c \1c \alpha \1a \pos \move` → styled runs + a
```
What is wrong: the tag list opens one code span at `\b` (line 6) and line 7 begins with a stray backtick before `\fs`; with the closing backtick after `\move` that is three backticks across the paragraph — the first span swallows the line break, and the third opens a second span that runs halfway through line 10 (the next backticks are around `drop (counted)`), rendering a large mangled chunk of the paragraph as literal code in rustdoc.
Why it matters: the module's most important claim — the exact G-2 tag set — is the one place the docs render wrong.
What correct looks like: one span: `` `\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move` `` on a single line (or the tag list on its own line).
Confidence: HIGH — Verification: read; backtick count 3 across the two lines (span-pairing traced).
### F-003 [S4] Stale `[LEAD SEAM]` merge-plan comment contradicts the file it lives in
Where: crates/subtitles/src/ass/mod.rs:39-48
Code:
```rust
// This `mod.rs` is the 5.M2b half: it declares ONLY the render-path
// modules, and the 5.M2a worktree owns `parser.rs` plus its own
// `mod.rs` declaring `pub mod parser;`. At merge the Lead unions the
// declarations (append `pub mod parser;` + any re-exports) — the
```
What is wrong: the comment describes the pre-merge worktree state; the file below it already declares `pub mod parser;` (line 22) and `pub mod tags;` (line 23) above the comment, and `pub mod layout; pub mod render; pub mod fuzz;` (49-52) below it. The record confirms the merge happened ("Cards 5.M2a (parser half) + 5.M2b (render half), merged by the Lead", sub-ass-ssa.md:7-8). The comment now misdescribes its own file and forces every reader to re-derive the merge history.
Why it matters: archaeology comments that contradict the code below them are exactly the "naming that lies" class; a maintainer trimming "the 5.M2a half's" declarations per this comment would delete live modules.
What correct looks like: delete the block (the worklog/archive already records the merge), or reduce it to one line: "5.M2a (parser) + 5.M2b (render) merged here; halves were file-disjoint."
Confidence: HIGH — Verification: read; declarations at 22-23/49-52 vs comment at 41-43.

## 4. Per-item table (file is declarations + docs only; <100 LOC so a full function table does not apply)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module doc — scope/G-2/damage claims | 1-20 | REAL | Every claim verified against tags.rs/parser.rs (§2); markdown defect F-002. |
| `pub mod parser;` | 22 | REAL | Parser exists; claims verified (sections/times/damage; 17 tests). |
| `pub mod tags;` | 23 | REAL | G-2 tag set verified arm-by-arm (10 tests). |
| re-exports (`parse`, `AssDamage`, `AssDocument`, `AssStats`, `AssStyle`, `StyleFlavour`, `PositionOverride`) | 25-26 | REAL | All names exist in the child modules (spot-verified via the greps in §2). |
| stale `[LEAD SEAM]` comment | 27-48 | S4 (F-003) | Contradicts lines 22-23. |
| `pub mod layout; pub mod render; pub mod fuzz;` | 49-52 | REAL | Layout/render/fuzz exist; fuzz.rs carries the false bound claim (F-001). |

## 5. Test honesty (no tests in this file; the declared modules' evidence)
| Evidence claim | Would it fail if the code were broken? | Reason |
|---|---|---|
| "267/0 lib tests incl. per-tag suites" (matrix row evidence) | Yes | Matches the baseline no-fail-fast count exactly (`subtitles` lib `test result: ok. 267 passed; 0 failed`); 50 `ass::*` tests ran ok there, incl. per-tag suites in `tags.rs` (10) and parser (17). |
| "3/3 real-ffmpeg ASS docs exact" | Yes | `ass_ffmpeg_ac.rs` = 3 ffmpeg-generated documents (basic / unicode / six-cue ladder) + 1 header/style check — 4 tests, all asserting exact counts/times/texts/tag counters; 4 passed in baseline. Consistent with the "3/3 docs" phrasing. |
| "9/9 alignment-grid pixel-exact" | Yes (per record) | layout.rs tests pin exact-rational anchors per numpad cell (7 tests in layout + render tests through `place_cue`); ran green in the 267. Not re-counted cell-by-cell by me — REAL-UNPROVEN at the "9/9" granularity, REAL at the suite level. |
| "2000+2048 fuzz-lite runs clean" | Partially | The fuzz harness (`fuzz_sub_ass`) and its committed sweeps exist and pass; the run-count numbers are card-time claims not reproducible from the tree (the sweeps are deterministic-seeded — reproducible in principle, not re-run here: no cargo). |
| Coverage gaps (mutation reasoning) | — | No test anywhere asserts a damage-ledger cap (F-001 passes all 267). |

## 6. Doc claims touching this file (the wave question)
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v1 parses the ASS/SSA syntax faithfully — sections, style lines, dialogue events with `H:MM:SS.CC` times" | mod.rs:4-6; sub-ass-ssa.md:17-22 | REAL | parser.rs: sections + `Format`/`Style`/`Dialogue` in any field order; `parse_time` :785 with the 1 cs = 10 000 µs mapping, test :1024 `0:00:01.00 → 1_000_000`; 50 ass tests green in no-fail-fast baseline. |
| "interprets the G-2 override-tag minimum natively (issue #17: `\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move` → styled runs + a position override" | mod.rs:6-8; matrix row `subtitle-ass-ssa` notes (feature-matrix.toml:915) | REAL | tags.rs match arms: b/i/u/s:222, fs:242, fn:252, c/1c:261, alpha/1a:274, pos:297, move:311; `\move` frozen at start point + counted (record discloses); styled runs + PositionOverride verified. |
| "Everything beyond that minimum … is stripped and *counted* per tag name … never a parse failure" | mod.rs:9-11 | REAL | tags.rs:326 default arm → `AssStats::dropped_tags` BTreeMap (parser.rs:213, `count_dropped` :243-246); record's deferred list is long and specific (karaoke/fr*/t/fad/fade/p/bord/shad/be/blur/fscx/fscy/an/a/r/clip/iclip). |
| "`\N`/`\n` become newlines and `\h` a hard space" | mod.rs:11-12 | REAL | tags.rs:472 (`stats.hard_spaces`), parser.rs:1189-1194 test pins `"hard\Nsoft\nspace\h"` + counter. |
| "The parser is infallible: [`parse`] always returns a document … no panics on any input" | mod.rs:16, :20 | REAL | `parse(&str) -> AssDocument` (no Result); fuzz harness + 267-test suite green; fuzz.rs contract holds for panics (the "capped" part of its bounds claim is the false bit — F-001). |
| "Parse: one pass, output ≤ O(input); the damage ledger is capped" | ass/fuzz.rs:22 (this module's child) | FALSE (the cap clause) | F-001: 13 uncapped `damage.push` sites, no cap constant, no dropped counter. |
| matrix status `partial ×3 desktop / not_started web` | feature-matrix.toml:913 | REAL (honest) + UNREACHABLE-in-product | The "partial" admission with the specific deferred list matches the code exactly (this audit); but no shipped entry point reaches the crate — a user-visible "partial" would require the seam the record's next-steps defer. |
| evidence "…267/0 lib tests…" | feature-matrix.toml:914 | REAL | Matches `orchrun-test-workspace-nofailfast.log.txt` lib count exactly (267/0). |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (Reachability row) — the whole ASS lane, like every other format in this crate.
- The stale merge comment (F-003) is dead weight but not dead code.

## 8. Dependencies & flags
- None in this file. The crate is std+media-types only; no feature gates anywhere in `ass/**` (verified: no `#[cfg(` in the module tree greps). No unsafe. Clippy-clean per baseline #5.

## 9. Open questions for the next agent
- To the `ass/parser.rs` parts owners (A09 part1, A10 part2, A01 part3) and `tags.rs` owner (A02): F-001's cap belongs in your files — 13 push sites, SRT's `DamageSink` is the in-crate precedent to copy; also verify my arm-line citations (222/242/252/261/274/297/311/326) when you audit tags.rs in full.
- Wave-level for the matrix owner: `subtitle-ass-ssa` is the one subtitle row marked `partial` — it is also the only one whose record's honesty (deferred list) fully survives hostile reading. The `implemented ×3` rows (srt/vtt/pgs/dvb/cc/vobsub) all share the UNREACHABLE status without saying so in the row.
- The display artifact strikes again: `parser.rs:269` renders as `#ust_use]` through my tooling — same `[m`-eating artifact hex-verified earlier on pgs/palette.rs:38; the real bytes are `#[must_use]`. NOT a finding.
