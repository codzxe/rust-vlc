# AUDIT: crates/playlists/src/xspf.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 1037 / 28 (parse/parse_detailed/parse_bytes + Scanner(11) + tree builders(6) + helpers(9)) |
| File verdict | REAL — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (only in-tree consumer is `streaming::remote_playlist::open_remote_playlist`, which itself has zero non-test callers; `playlist.load` command is declared/emitted but no executor calls the parser) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 4 |
| Commands run | standalone probe vs prebuilt rlib (see F-001, output pasted); `./target/debug/deps/playlists-c262b72361161a1c` -> 46 passed, exit 0; `./target/debug/deps/xspf_suite-e93da4a812baa463` -> 14 passed, exit 0; sed/rg verifications |

## 1. One-paragraph verdict
A genuinely well-built hand-rolled XML-subset pull parser: bounded input (64 KiB line cap, depth-64 guard), typed damage warnings, entity decoding with verbatim-keep degradation, salvage of unclosed elements, and real deterministic fuzz sweeps — all of which I executed. Its one substantive defect is that `<extension>` containers are NOT opaque: field-named children inside an extension (any namespace prefix) are attributed to the enclosing track and, by first-wins, can shadow the real field — including `<…:location>` hijacking the entry's media location — and a `<track>` inside a playlist-level extension is injected as a real entry with zero warnings. I proved all three with a compiled probe against the shipped rlib. This contradicts the module doc's "extension content … ignored" claim. The module has no in-file tests (all coverage lives in tests/xspf_suite.rs, which passes 14/14).

## 2. What actually works (verified)
— `./target/debug/deps/playlists-c262b72361161a1c` (lib tests incl. `fuzz::tests::hostile_wide_inputs_survive`) -> `46 passed; 0 failed`, exit 0.
— `./target/debug/deps/xspf_suite-e93da4a812baa463` -> `14 passed; 0 failed`, exit 0 (binary newer than source).
— Probe (rustc --edition 2024 -L deps vs `libplaylists-da790bb54352cdf5.rlib`): malformed soup `<playlist><trackList><track><location>a&amp;b</location><?pi ?><!DOCTYPE x [\n<<<>>>&bogus;&#0;&#x110000;</track></wrong></trackList>` -> 1 salvaged entry, location `a&b`, 6 warnings, no panic; `<duration>0</duration>` -> `Some(0.0)` no warning.

## 3. Findings

F-001 [S2]
Where: crates/playlists/src/xspf.rs:403-411 (and 346-367)
Code:
```
                // Everything else (playlist, trackList, unknown/extension
                // elements): an ignored container.
                if !self_closing {
                    stack.push(Frame {
                        name,
                        kind: FrameKind::Other,
                    });
                }
```
What is wrong: `<extension>` frames are pushed as `Other` but their children are still walked by the normal attribution logic: a child whose local name is `location/title/creator/annotation/duration` (L346) becomes a track field, and a child named `track` becomes a real entry. XSPF extension content is application-specific by spec and the module doc (L20-22) claims it is "ignored" — it is not. Empirically verified (compiled probe, output verbatim):
```
C1 title=Some("PWNED-TITLE") warnings=[DuplicateField { line: 1 }]   // <app:title> inside <extension> shadows the real <title>
C2 location="evil://pwned" warnings=[DuplicateField { line: 1 }]     // <app:location> inside <extension> hijacks the media location; real "real.mp3" dropped
C3 entries=["real.mp3", "evil://injected"] warnings=[]               // <track> inside playlist-level <extension> injected as an entry, zero warnings
```
Why it matters: an untrusted XSPF can redirect playback to an attacker-chosen URI via its extension block (the exact block the spec says consumers must not interpret), or add phantom tracks, with at most a `DuplicateField` note. First-wins means the extension value silently shadows the genuine field. What correct looks like: enter an opaque-skip mode on `<extension>` (like `skip_depth` does for nested tracks). Confidence: HIGH — Verification: ran (probe binary under audit/runs/tmp/, compiled against the prebuilt crate rlib). Severity note: currently capped at S2 only because no shipped entry point reaches this parser today (see reachability); the moment `playlist.load` gains an executor this is a security-relevant S1 (location injection from a shared playlist file).

F-002 [S4]
Where: crates/playlists/src/xspf.rs:1012-1032
Code:
```
            match char::from_u32(value) {
                // XML forbids NUL even as a numeric reference.
                Some('\0') | None => None,
                Some(c) => Some(c),
            }
```
What is wrong: only NUL is refused; XML 1.0 also forbids other C0 controls (`&#1;`-`&#8;`, `&#11;`-`&#12;`, `&#14;`-`&#31;`) as character references — they are accepted into field text. Lenient, no panic. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playlists/src/xspf.rs:897-899
Code:
```
            match trimmed.parse::<f64>() {
                // XSPF durations are non-negative milliseconds.
                Ok(ms) if ms.is_finite() && ms >= 0.0 => tb.duration = Some(ms / 1000.0),
```
What is wrong: comment says "non-negative milliseconds" (spec: non-negative integer) but `parse::<f64>()` accepts `"1e3"`, `"0x10"`? (no — f64 rejects 0x), `"1.5"`, `"+5"` — floats and exponents where the spec fixes integers. Lenient acceptance; NaN/inf correctly refused. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/playlists/src/xspf.rs:941-943
Code:
```
fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':' || (c as u32) > 127
}
```
What is wrong: `:` as a name-START character means a bare `<:foo>` parses as an element named ":foo" whose `local_name` becomes "foo" — XML forbids names beginning with `:`. Combined with namespace-blind local-name matching, this widens the F-001 attribution surface. No panic. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/playlists/src/xspf.rs:372-374
Code:
```
                    if matches!(kind, FieldKind::Location | FieldKind::Duration) {
                        warnings.push(XspfWarning::MisplacedElement { line });
                    }
```
What is wrong: a `location`/`duration` inside a playlist-level `<extension>` (legal application content per spec) is flagged `MisplacedElement` — false-positive damage note; the parser cannot distinguish extension content from real misplacement (same root cause as F-001). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse | 268-270 | REAL | — | thin wrapper |
| parse_detailed | 273-475 | REAL | S2 | main loop; extension opacity bug (F-001); salvage-at-EOF correct (innermost-first ordering applies fields before track emission — verified by trace) |
| parse_bytes | 479-482 | REAL | — | UTF-8 gate, one BOM tolerated upstream |
| Scanner::new/take/peek_char/advance_char/skip_ws/rest_starts_with | 516-552 | REAL | — | all `take` ends are char boundaries (verified per call site); line counting correct |
| Scanner::skip_after | 556-566 | REAL | — | EOF-consume + typed warning |
| Scanner::next_token | 568-641 | REAL | — | comment/CDATA/PI/decl/end/start/text dispatch; CDATA raw pass-through correct; `<`+non-name → literal text |
| Scanner::scan_end_tag | 643-675 | REAL | — | resync to `>`; empty name double-noted |
| Scanner::scan_start_tag | 677-753 | REAL | — | both quote styles, attr damage resync, EOF salvage |
| Scanner::resync_attr | 759-766 | REAL | — | documents fuzz-caught spin fix (`<f/creator>`) |
| FieldKind::from_name | 795-805 | REAL | — | 5 fields, namespace-blind by design |
| TrackBuilder::is_set | 824-833 | REAL | — | duplicate tracking incl. empty-set fields |
| close_frame | 838-861 | REAL | — | forced-close warning; field applied before track emit |
| apply_field | 864-904 | REAL | S4 | duration lenience (F-003) |
| track_entry | 907-925 | REAL | — | creator/annotation → directives, deterministic order |
| non_empty / local_name / is_name_start / is_name_char | 929-947 | REAL | S4 | F-004 |
| line_over_cap | 951-970 | REAL | — | byte-exact, 1-based lines |
| decode_entities | 975-1008 | REAL | — | bounded entity scan (≤32 chars), verbatim keep |
| decode_one / strip_hex | 1012-1037 | REAL | S4 | F-002; hex x/X both handled |

## 5. Test honesty
No tests in this file — all coverage is crates/playlists/tests/xspf_suite.rs (audited separately as file 5; 14/14 pass verified). The fuzz-lite sweeps live in crates/playlists/src/fuzz.rs (2/2 pass verified).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Unknown elements anywhere (extension content, foreign namespaces) are ignored" | xspf.rs:20-22 | FALSE | F-001 probe: extension children attributed/injected (C1/C2/C3) |
| "Never panics on any input (fuzz-pinned)" | xspf.rs:36-37 | REAL-UNPROVEN (sweeps pass; nightly libFuzzer campaign not run in sandbox) | fuzz::tests 2/2 + suite fuzz-lite 14/14 pass; full campaign UNVERIFIED (Lead-owned nightly wiring, no cargo-fuzz in sandbox) |
| "prefixed names (`<xspf:track>`) are matched by *local name*" | xspf.rs:8-9 | REAL | code L749/L674 `local_name(&raw_name)`; namespaced fixture passes |
| "unterminated [constructs] … rest of input consumed" | xspf.rs:125-159 | REAL | skip_after/take(src.len()) paths; suite `malformed_markup_degrades_not_dies` |
| "creator/annotation … ride the ordered directive pairs" | xspf.rs:51-53 | REAL | track_entry L911-917 |

## 7. Dead / duplicated / unreachable code
No dead code. Module UNREACHABLE from shipped entry points: `rg parse_xspf` over crates/+apps/ hits only playlists (lib/fuzz/tests) and `streaming/src/remote_playlist.rs:194`; `open_remote_playlist` has zero non-test callers; desktop emits `Command::PlaylistLoad` (apps/desktop/src/dispatch.rs:241) but no executor in control-server/playback-engine calls any playlists parser (rg for executor-side parse calls: 0 hits).

## 8. Dependencies & flags
thiserror (error derives), crate-internal m3u::MAX_LINE_BYTES (=64 KiB, m3u.rs:100) and types::{Entry, HlsSegment, Playlist}. No feature gates. No unsafe.

## 9. Open questions for the next agent
- Does the `playlist.load` executor exist anywhere (control-server)? If it lands, re-grade F-001 to S1 and add a scheme-policy check on injected locations.
- Should `to_m3u_string` round-trip of creator/annotation directives be lossless (xspf.rs:54-58 claims text-preserving)? Verified in xspf_suite (file 5).
- Probe binary + source kept at audit/runs/tmp/xspf_probe.rs for re-runs.
