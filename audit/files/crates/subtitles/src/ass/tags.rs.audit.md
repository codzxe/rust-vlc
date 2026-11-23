# AUDIT: crates/subtitles/src/ass/tags.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 740 / 26 (14 prod incl. 2 structs/const, 12 test items) |
| File verdict | UNREACHABLE (code itself is REAL: engine works, 10/10 tests executed) |
| Reachable from shipped entry point? | NO — subtitles crate is a dep of NO app; only fuzz/Cargo.toml:50 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | rg callers (exit 0); prebuilt libtest run `./target/debug/deps/subtitles-70f89de07410626f ass::tags` -> 10 passed / 0 failed (exit 0); binary mtime 1795457287 > src mtime 1795455358 (current); sed/od quote verification |

## 1. One-paragraph verdict
The G-2 override-tag engine is genuinely implemented and honest: `{\b}{\i}{\u}{\s}{\fs}{\fn}{\c}{\1c}{\alpha}{\1a}{\pos}{\move}` mutate a seeded `WorkState` and emit style-homogeneous `StyledSegment` runs; `\2c..\4a` are validated then counted `unmapped_colour_tags`; everything else (`\k \t \frz \r \an \fad` …) is dropped by name into `dropped_tags` — no silent swallowing. The longest-first `KNOWN_TAGS` table is correctly ordered by descending length (verified by counting all 52 entries), so `\fnComic Sans` is tag `fn` + arg. Tests assert exact segment vectors and counters and would fail under mutation. The fatal problem is not in this file: the entire `subtitles` crate is wired to nothing shipped — `apps/cli/Cargo.toml` (deps list read) has no `subtitles` dependency; the only dependent in the tree is `fuzz/Cargo.toml:50` (`subtitles = { path = "../crates/subtitles" }`). No `lumen open` path can reach this code.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f ass::tags` -> `test result: ok. 10 passed; 0 failed; 0 ignored` (exit 0). Covers: boolean flips, `\fs`/`\fn` overrides incl. empty-`\fn` reset, BGR colour + inverted alpha (6-digit keeps prior alpha, 8-digit sets it), `\pos`/`\move` milli coords + last-wins, `{}` literal preservation, unterminated `{`, `\N`/`\n`/`\h` folding, UTF-8 whole-scalar copy, Plain collapse when nothing diverges.

## 3. Findings
F-001 [S2]
Where: apps/cli/Cargo.toml [dependencies]; fuzz/Cargo.toml:50; this file
Code: (apps/cli/Cargo.toml deps, verbatim list, no `subtitles` entry) / `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50)
What is wrong: `styled_event_text` is reachable only as `ass::parser` (parser.rs:679) -> `ass::parse` -> `subtitles` crate root -> nothing. No shipped crate (lumen-cli, desktop, desktop-tauri, web) depends on `subtitles`; the sole dependent is the fuzz workspace. The feature-matrix rows `subtitle-srt`/`subtitle-vtt` claim `status = { windows = "implemented", linux = "implemented", macos = "implemented", ... }` (docs/feature-matrix.toml:899, 926) for a crate no shipped binary links. Why it matters: a user cannot load any ASS/SRT/VTT subtitle in any shipped build; "implemented" describes library-island code. What correct looks like: playback-engine or apps/cli must depend on `subtitles` and route sidecar/subtitle streams to it. Confidence: HIGH — Verification: ran (rg over all Cargo.tomls: exactly one hit).
F-002 [S3]
Where: crates/subtitles/src/ass/tags.rs:9-14 vs 252-259 and 96-102
Code:
```
//! exactly like ASS's inline overrides. Runs are emitted with the
//! *absolute* style in effect (style ⊕ overrides), so a consumer
//! never re-derives inheritance.
```
vs
```
            // Empty argument: reset to inherit (the style's font).
            if arg.is_empty() {
                state.style.font_name = None;
```
What is wrong: the module's headline invariant ("absolute style, consumer never re-derives inheritance") is false in the `{\fn}`-reset case (emits `font_name = None`, pinned by test at line 601) and in the alpha-with-no-colour case (materialize fabricates a black placeholder colour, lines 96-102). Consumers must re-derive inheritance for exactly those runs. Why it matters: a consumer following the doc renders a default font instead of the style font after `{\fn}`. What correct looks like: resolve `None` to the seed's font name at emit time, or delete the "never re-derives" claim. Confidence: HIGH — Verification: read + ran (test run passes with `None` pinned).
F-003 [S4]
Where: crates/subtitles/src/ass/tags.rs:173-178
Code:
```
                let (arg, next) = read_arg(block, j);
                let _ = next;
                stats.count_tag(name);
                // Skip the whole tag (argument included) — one count.
                i = skip_arg(block, j);
                let _ = arg;
```
What is wrong: `arg` and `next` are computed by `read_arg` then explicitly discarded; `skip_arg` re-runs the same scan. Pure dead computation in the unknown-tag path. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/subtitles/src/ass/tags.rs:118-126
Code:
```
const KNOWN_TAGS: [&str; 52] = [
    // 5
    "alpha", "iclip", "xbord", "ybord", "xshad", "yshad", // 4
```
What is wrong: the trailing `// 4`, `// 3`, `// 2` comments label the FOLLOWING row while `// 5`/`// 1` label the following row on their own line — a reader scanning line-ends reads "yshad, // 4" as "these are 4 chars" (they are 5). Ordering itself is correct (desc length verified). Confidence: HIGH — Verification: read (counted all 52 lengths).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PositionOverride (struct) | 64-72 | REAL | — | plain data, milli-px + from_move |
| WorkState::new | 84-89 | REAL | — | seeds style, alpha None |
| WorkState::materialize | 93-108 | REAL | — | stamps tracked alpha; black placeholder when colourless (see F-002) |
| KNOWN_TAGS const | 118-126 | REAL | S4 | desc-length order verified correct; comment placement F-004 |
| apply_block | 130-188 | REAL | S4 | longest-first scan; unknown-name run counting; dead bindings F-003 |
| read_arg | 193-205 | REAL | — | paren-to-`)` (unterminated lenient), else to next `\`; no panics on empty tail |
| skip_arg | 209-211 | REAL | — | delegates to read_arg |
| apply_tag | 214-336 | REAL | — | booleans (nonzero=on, malformed->dropped), fs>0, fn reset, c/1c alpha-preserving, 2c-4a validated-unmapped, pos/move; `\b` weights collapsed to bool (documented) |
| parse_tag_colour | 341-358 | REAL | — | >6 hex digits = explicit alpha; decimal explicit iff >24-bit; consistent with parse_ass_colour |
| parse_tag_alpha | 362-380 | REAL | — | `&HXX`/decimal, inverted->straight; 1-2 hex digits enforced |
| parse_coords | 383-391 | REAL | — | exactly 2 milli coords |
| parse_move_args | 395-408 | REAL | — | 4 or 6 parts, all validated, returns frozen start |
| styled_event_text | 414-502 | REAL | — | quirks walk (`{}` literal, unterminated `{`, `\N\n\h`), run split on style change, Plain collapse; no boundary panic found (utf8_len only entered on scalar starts of valid &str) |
| utf8_len | 506-513 | REAL | — | 0x80..0xBF would misreport 4 but unreachable from valid UTF-8 &str |
| tests (10) | 515-740 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tag_free_text_stays_plain | YES | exact Plain string + zero counters |
| boolean_tags_flip_runs | YES | exact 3-segment vector + applied=3 + dropped absent |
| strikeout_underline_and_multi_tag_blocks | YES | exact styled vec incl. with_strikeout |
| boolean_weights_nonzero_is_on | YES | `\b600`->bold, `\bq`->dropped count 1 |
| fs_and_fn_override_font_fields | YES | per-segment font size/name + 2 malformed-\fs drops |
| colour_and_alpha_tags | YES | BGR order, alpha 127 from &H80, 8-digit explicit, malformed drop |
| secondary_colour_slots_are_unmapped_not_dropped | YES | unmapped=2, `4c` malformed dropped |
| pos_and_move_overrides | YES | exact milli coords, last-wins, arity errors |
| unsupported_tags_stay_dropped_and_counted | YES | k/frz/t counted, Plain("karaoke"), applied=0 |
| text_quirks_survive_the_styling_walk | YES | `{}`+unterminated `{`+\h+UTF-8 exact; seed-splits on `\b0` |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "override-tag MINIMUM now native — \b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move -> styled runs + position override" | docs/feature-matrix.toml:915 (subtitle-ass-ssa notes) | REAL (library) / UNREACHABLE (product) | test run above; F-001 dependency proof |
| "beyond-minimum tags still degrade honestly (dropped/unmapped counters)" | docs/feature-matrix.toml:915 | REAL | unsupported_tags_stay_dropped_and_counted passes |
| "Runs are emitted with the *absolute* style ... never re-derives inheritance" | tags.rs:12-14 | FALSE (narrow) | F-002: `{\fn}` -> None, test line 601 |
| "\move ... anchor freezes at the start point" (with [VERIFY] marker) | tags.rs:40-44 | REAL (frozen) / [VERIFY] against libass still open | pos_and_move_overrides passes; no libass comparison exists anywhere (UNVERIFIED vs real renderer) |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped builds (F-001). In-file: F-003 dead bindings. `skip_arg` is a trivial wrapper (fine).

## 8. Dependencies & flags
Crate `subtitles` deps: `media-types` only (Cargo.toml read); std-only otherwise. No feature gates on this file. No unsafe.

## 9. Open questions for the next agent
- Who was supposed to wire `subtitles` into playback (matrix notes cite "two-pass sidecar load per oracle evidence")? Check playback-engine for a planned seam.
- parser.rs `apply_position_override` percent-mapping drift (0..=100% vocabulary rejects off-script coords) is the neighboring file's auditor's problem.
