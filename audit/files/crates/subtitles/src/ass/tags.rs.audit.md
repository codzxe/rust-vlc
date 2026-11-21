# AUDIT: crates/subtitles/src/ass/tags.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 740 / 14 (+2 test helpers, 10 tests) |
| File verdict | UNREACHABLE (real, in-crate test-proven code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — only callers are `ass/parser.rs:679,935` (in-crate); crate consumed solely by its own tests + the nightly-only standalone `fuzz/` workspace (see §2) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `rg -n 'styled_event_text' crates/` (callers: parser.rs only) · `rg 'subtitles' --glob Cargo.toml` (dep edges: fuzz/ only) · `rg -n 'subtitle' apps/cli/src` (CLI refusal + frame drop) · python re-extraction of KNOWN_TAGS (52 entries, strictly length-descending) · `sed -n` quote verification for every finding |

## 1. One-paragraph verdict
The G-2 override-tag engine is genuinely implemented, careful, and honest about
its limits: the 13-tag applied minimum, the unmapped-colour bucket, the
dropped-tag zoo, the `\move` freeze and the VSFilter text quirks all match the
module's "honest ledger" doc, and all 10 in-file tests are real
mutation-sensitive assertions that pass in the baseline no-fail-fast workspace
run. No panic path exists — all slicing is ASCII-anchored on a valid `&str`,
arithmetic is checked, and the walk's progress invariant holds. The defects are
local: a doubled argument scan per tag, a full `TextStyle` clone (allocating
the `Box<str>` font name) on every override-block boundary, and a
libass-divergent generalization of parenthesized arguments to ALL tags. The
crushing fact is reachability: no shipped binary (`lumen` CLI, desktop,
desktop-tauri) depends on the `subtitles` crate at all — the engine is dead
code relative to the product.

## 2. What actually works (verified)
- KNOWN_TAGS table integrity — verified by python literal extraction:
  `declared: 52, actual: 52, lengths strictly descending, unique` — the
  "longest known name wins" doc claim is enforced by table order (`fade`
  before `fad`, `alpha` before `a`, `iclip` before `i`).
- All 10 `ass::tags::tests` pass — verified by
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5313-5322`
  (`test ass::tags::tests::colour_and_alpha_tags ... ok` …), crate line
  `test result: ok. 267 passed; 0 failed` (subtitles lib, same file).
  NOTE: the primary baseline `test-workspace.log.txt` compiled the crate
  (line 15) but never ran its tests — fail-fast aborted at `apps/desktop`
  `zero_alloc_transport` before reaching `crates/*`.
- Call-chain to shipped entry: NONE. `rg` over all `Cargo.toml`: only
  `fuzz/Cargo.toml:50` depends on `subtitles`; `fuzz/` is its own nightly
  workspace, "NOT wired into any gate" (fuzz/Cargo.toml header). Root
  `Cargo.toml` members: `crates/*, apps/*, tools/xtask,
  tests/integration/first-vertical, tools/mem-audit, tools/sweep`. The shipped
  CLI refuses the domain outright — `apps/cli/src/dispatch.rs:197-198`:
  `} else if id.starts_with("subtitle.") { "the subtitle path is a Phase-5
  card"` — and drops subtitle frames — `apps/cli/src/pipeline_host.rs:1237-1238`:
  `// Subtitle/data limbs are not wired in the first vertical.` /
  `Frame::Subtitle(_) => Ok(()),`.
- `styled_event_text` callers: `crates/subtitles/src/ass/parser.rs:679`
  (document parse path) and `:935` — in-crate only.

## 3. Findings

### F-001 [S3] Known-tag argument span scanned twice per tag
Where: crates/subtitles/src/ass/tags.rs:146-186
Code:
```
            Some(name) => {
                let arg_at = i + 1 + name.len();
                let name = &block[i + 1..arg_at];
                let arg = read_arg(block, arg_at);
                (name, arg.0)
            }
```
followed at 183-184 by
```
        let arg_at = i + 1 + name.len();
        let next = skip_arg(block, arg_at);
```
What is wrong: the known-tag arm computes `read_arg(block, arg_at)`, discards
the returned end position (`arg.0` only), then `skip_arg` re-runs the identical
`read_arg` scan to recover it.
Why it matters: every applied/dropped tag's argument span is scanned twice on
the parse hot path. Complexity stays linear but the constant doubles, and the
duplication invites the two spans drifting apart under future edits (they are
the same function today, so no correctness bug — yet).
What correct looks like: `(name, arg)` arm returns `arg.1` as `next` and the
tail uses it; delete the second `arg_at`/`skip_arg` pair.
Confidence: HIGH — Verification: read (both call sites quoted verbatim above).

### F-002 [S3] `materialize()` clones the whole TextStyle (font-name alloc) at every block boundary
Where: crates/subtitles/src/ass/tags.rs:93-94 and 448
Code:
```
    fn materialize(&self) -> TextStyle {
        match (self.style.clone(), self.alpha) {
```
and the per-block call site
```
                // Close the open run if the effective style changed,
                // so segments stay style-homogeneous.
                let now = state.materialize();
```
What is wrong: `TextStyle` contains `font_name: Option<Box<str>>`; `clone()`
allocates a new `Box<str>` whenever a font name is in effect. `materialize` is
called once at cue start (line 423) and after EVERY override block (line 448),
so a cue with N blocks and a `\fn` set performs N heap allocations for a value
that only needs `Copy`-ing fields plus a borrowed comparison.
Why it matters: karaoke-heavy real-world ASS files carry dozens of override
blocks per event; this is per-cue, per-block allocation churn in the parse
lane of a crate whose culture is allocation-discipline (D-1/D-2 in-tree docs;
the compositing lane has an explicit zero-alloc test). Also `Eq`-comparing the
materialized style is all that is needed — a `PartialEq` on a borrowed view
would avoid the clone entirely.
What correct looks like: compare `(style_fields, alpha)` without materializing
(e.g. an `equivalent(&TextStyle)` helper), and materialize only when a segment
is actually emitted.
Confidence: HIGH — Verification: read; allocation follows from
`Option<Box<str>>::clone` semantics (model/style.rs:117).

### F-003 [S3] Parenthesized arguments accepted for ALL tags — libass divergence, undocumented
Where: crates/subtitles/src/ass/tags.rs:195-200
Code:
```
    if bytes.get(at) == Some(&b'(') {
        match block[at + 1..].find(')') {
            Some(close) => (&block[at + 1..at + 1 + close], at + close + 2),
            // Unterminated `(`: the rest of the block, lenient.
            None => (&block[at + 1..], bytes.len()),
```
What is wrong: the module doc (lines 22-24) documents the parenthesized form
only for `\fn`. In libass only the coordinate/animation tags (`\pos`, `\move`,
`\clip`, `\iclip`, `\t`) take parenthesized arguments; for `\b`, `\fs`, `\c`
etc. the argument runs to the next `\`, so `{\b(1)}` is invalid in libass
(arg `(1)`) and ignored, while this code reads the paren span and applies
BOLD. Same for `{\fs(20)}`, `{\c(&HFF&)}`.
Why it matters: the file's headline claim is "clean-room from the published
ASS/SSA format description" with "the parser's exact VSFilter/libass quirks";
silently accepting a broader grammar than libass means Lumen renders junk
input that real renderers ignore — a spec-fidelity divergence with no test
pinning it (no test covers paren args outside `\fn`/`\pos`/`\move`).
What correct looks like: paren-form reading restricted to the tags whose
grammar has parentheses; other tags read to the next `\`.
Confidence: MEDIUM — Verification: read; libass behavior is from memory, no
network to confirm — flagged UNVERIFIED against libass source in §9.
Confidence note: the *divergence from the file's own doc* (paren form
documented for `\fn` only, implemented for all 47 known tags) is HIGH
confidence — that part is proven by the code above.

### F-004 [S4] Dead computations in the unknown-tag arm
Where: crates/subtitles/src/ass/tags.rs:173-178
Code:
```
                let name = &block[i + 1..j];
                let (arg, next) = read_arg(block, j);
                let _ = next;
                stats.count_tag(name);
                // Skip the whole tag (argument included) — one count.
                i = skip_arg(block, j);
                let _ = arg;
```
What is wrong: `arg` and `next` are computed by `read_arg` and then
explicitly discarded (`let _ = next;` / `let _ = arg;`) while `skip_arg`
recomputes the scan a second time — two computed-and-dropped values plus one
duplicate scan per unknown tag.
Why it matters: dead code that clutters the hot loop and hides intent; the
`let _ =` pattern is exactly the swallowed-value smell the checklist flags.
What correct looks like: `i = read_arg(block, j).1;` and drop both bindings.
Confidence: HIGH — Verification: read (quoted verbatim).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PositionOverride` (struct) | 64-72 | REAL | — | Plain data carrier, `pub`, Copy/Eq; consumed by parser.rs position mapping (other agent's part). |
| `WorkState` (struct) | 77-81 | REAL | — | style + tracked alpha split is the correct libass colour/alpha-independence model. |
| `WorkState::new` | 84-89 | REAL | — | Trivial, correct. |
| `WorkState::materialize` | 93-108 | REAL | S3 (F-002) | Logic correct (alpha stamping, black-placeholder carrier documented); clones whole style incl. `Box<str>` per call. |
| `KNOWN_TAGS` (const) | 118-126 | REAL | — | 52 entries verified programmatically; strictly length-descending ⇒ longest-match holds; count annotation exact. |
| `apply_block` | 130-188 | REAL | S3/S4 (F-001, F-004) | Loop progress guaranteed (name.len() ≥ 1); digit-led vs alpha unknown-name runs; no panics; double scan + dead lets. |
| `read_arg` | 193-205 | REAL | S3 (F-003) | Boundary-safe slicing (all anchors ASCII); unterminated `(` lenient; paren form generalized to all tags. |
| `skip_arg` | 209-211 | REAL | — | Pure delegation to `read_arg().1`; correct, exists only to dodge the tuple. |
| `apply_tag` | 214-336 | REAL | — | All 13 minimum tags dispatched; malformed → counted-dropped, never a parse failure; `ok` expr at 287-289 relies on `&&`-over-`||` precedence — verified correct for all six `2c..4a` names; u64 counters, no overflow. |
| `parse_tag_colour` | 341-358 | REAL | — | alpha_explicit = >6 hex digits or decimal outside 0..=0xFFFFFF; consistent with `parse_ass_colour`; negative decimal → explicit (alpha domain). |
| `parse_tag_alpha` | 362-380 | REAL | — | `&H` 1-2 hex digits or decimal 0..=255; inversion `255 - ass_alpha` correct; rejects `&H&`, >2 digits. |
| `parse_coords` | 383-391 | REAL | — | Exactly two comma parts (third ⇒ None); `parse_decimal_milli` trims (parser.rs:899) so `\pos(1, 2)` parses; checked arithmetic upstream. |
| `parse_move_args` | 395-408 | REAL | — | 4-or-6 arity per spec; validates all coords (incl. t1/t2) though motion frozen; returns start point. |
| `styled_event_text` | 414-502 | REAL | — | Walk verified: `{}` preserved, unterminated `{` literal, `\N`/`\n`/`\h` folds, whole-UTF-8 copy with `.min(len)` guard, style-revert-before-text correctly collapses to `Plain`, last-`\pos`-wins. No panic path found. |
| `utf8_len` | 506-513 | REAL | — | const fn; `_` arm returns 4 for 0x80..=0xBF too, but the walk's invariant (only scalar boundaries reach it) is upheld — every advance is ASCII-anchored; no caller can land mid-scalar on a valid `&str`. |
| tests `run`/`seg` helpers | 522-533 | REAL | — | Thin, honest helpers. |
| 10 tests | 535-739 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tag_free_text_stays_plain | YES | Asserts exact `Plain("plain text\ntwo lines")` + zero counters. |
| boolean_tags_flip_runs | YES | Asserts the exact 3-segment vector incl. the reverted plain middle run. |
| strikeout_underline_and_multi_tag_blocks | YES | Two tags in ONE block, exact style vector. |
| boolean_weights_nonzero_is_on | YES | `\b600` → bold pins `v != 0`; `\bq` → dropped-count 1 pins the malformed path. |
| fs_and_fn_override_font_fields | YES | Exact milli values 32_500, inherit-reset `None`, dropped `fs` counts. |
| colour_and_alpha_tags | YES | Pins BGR decode, 6-digit-keeps-alpha (127), 8-digit-carrying-alpha, malformed dropped. Mutation: drop the `255-` inversion → `&H80`→127 assertion fails. |
| secondary_colour_slots_are_unmapped_not_dropped | YES | Pins the separate `unmapped_colour_tags` counter vs `dropped_tags`. |
| pos_and_move_overrides | YES | Exact milli coords, last-`\pos`-wins, move freeze + counter, arity rejections. |
| unsupported_tags_stay_dropped_and_counted | YES | `\k`/`\frz`/`\t` counted by NAME (would catch a regression to a generic drop). |
| text_quirks_survive_the_styling_walk | YES | `{}` literal pair, unterminated `{`, `\h` space, CJK whole-scalar copy, seed-divergence run split. |
All 10 pass in baseline (`orchrun-test-workspace-nofailfast.log.txt`, 267/267
crate tests). Mutation-reasoning done on paper only (no edits, per rules).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "override-tag MINIMUM now native — \b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move → styled runs + position override" | docs/feature-matrix.toml row `subtitle-ass-ssa` notes (line 915) | REAL (code matches) | apply_tag:221-325 dispatches exactly those 13 names; verified read. |
| "beyond-minimum tags still degrade honestly (dropped/unmapped counters)" | same row | REAL | `dropped_tags` BTreeMap + `unmapped_colour_tags`/`move_tags_frozen` in parser.rs AssStats; counted in apply_tag. |
| "G-2: 267/0 lib tests incl. per-tag suites (1bd5519)" | same row evidence (line 914) | REAL (in-crate) | `orchrun-test-workspace-nofailfast.log.txt:5269+` — 267 passed, 0 failed for the subtitles lib. |
| KNOWN_TAGS "longest-first … so `\fnComic Sans` is tag `fn` + argument" | tags.rs module doc 111-117 | REAL | Programmatic extraction: 52 entries, lengths strictly descending. |
| "`{\fn}` … parenthesized form accepted too" (implying paren form is a `\fn` nicety) | tags.rs module doc 22-24 | SHALLOW (doc understates the code) | read_arg:195 accepts parens for ALL 47 known tags — F-003. |
| status windows/linux/macos = "partial" for ASS | row `subtitle-ass-ssa` (line 913) | REAL-CODE / UNREACHABLE | Code complete + tested in-crate, but zero shipped binaries depend on the crate (§2 reachability); the row nowhere discloses non-integration — recorded as an S3 doc-omission under §7. |
| "\move … anchor freezes at the start point (x1,y1), counted" | tags.rs module doc 40-44 | REAL | parse_move_args:395-408 returns `(x1,y1)`; `move_tags_frozen += 1` at 318; pinned by pos_and_move_overrides. |

## 7. Dead / duplicated / unreachable code
- **The entire file is unreachable from shipped entry points** (proof in §2) —
  the crate's only consumers are its own tests and the non-gated nightly fuzz
  workspace. This is the wave-level answer to "is any subtitle path reachable
  in the shipped build?": NO.
- Doc-omission (S3, filed here because the row cites this file): matrix row
  `subtitle-ass-ssa` presents "partial" per-platform status and per-tag test
  counts without one word that no shipped binary links the crate; a reader
  reasonably concludes the player typesets ASS. The matrix vocabulary
  ("implemented = code complete … evidence not yet complete") technically
  permits it, so not S0/S2 — but the omission is misleading.
- Local duplication: read_arg/skip_arg double scan (F-001), dead `let _ =`
  values (F-004).

## 8. Dependencies & flags
- No `[features]` in crates/subtitles/Cargo.toml — nothing cfg-gated; the
  test module is `#[cfg(test)]` only. No feature flag hides shipped code here.
- Runtime deps: `media-types` only (std-only policy, doc-11) — honored: no
  external crates imported in this file. Dev-deps (testkit, media-clock) not
  used by this file.
- No unsafe, no unwrap/expect/index panics (crate-wide clippy
  `-D warnings` clean per baseline run #5/#6).

## 9. Open questions for the next agent
- ass/parser.rs owner: `parse_ass_colour` line 885
  `u32::try_from(dec).unwrap_or(dec as u32)` — negative decimal wraps
  (`(-1i64) as u32` = 0xFFFF_FFFF ⇒ alpha 0, RGB 0xFFFFFF): intended
  ASS-semantics or accident? My file feeds it `\c` args.
- ass/parser.rs owner: the PositionOverride → `Positioning` percent mapping
  and the `position_overrides_unmappable` counter live in your part — verify
  the percent conversion clamps negative coords the way the counters claim.
- Whoever owns matrix rows: decide whether `subtitle-*` rows need an
  integration-status note; today they read as player features while the CLI
  refuses the domain (`dispatch.rs:198`).
- UNVERIFIED: libass paren-argument grammar (F-003) — no network; if a libass
  source dump ever lands in the repo, confirm `\b(1)` is rejected there.
