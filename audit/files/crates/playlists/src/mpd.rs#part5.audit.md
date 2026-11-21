# AUDIT: crates/playlists/src/mpd.rs#part5 (lines 2281-2854)
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 574 (chunk boundary: part5 begins mid-test — `hand_static_fixture_full_surface` starts at 2278 in part4/A01's range; my range is its body from 2281 through EOF 2854) / 14 tests |
| File verdict | REAL (the tests) — but the parser they pin is UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO for the code under test. Proof: `rg -n "mpd|parse_mpd|MpdDocument" crates/streaming/src/dash/` → sched.rs:287 consumes `Mpd`, abr.rs:330 `use playlists::parse_mpd_bytes` is inside `#[cfg(test)]` (sed -n 320,345p verified); `rg "dash::" apps/ crates/playback-engine/ crates/input/` → zero; `rg -n "remote_playlist\|hls::\|parse_mpd" apps/cli/src/ apps/desktop-tauri/ apps/desktop/` → zero. The playlists crate is compiled into `lumen` only as a transitive dep of streaming (apps/cli/Cargo.toml → streaming → playlists), and the only in-streaming consumer of the MPD model is the dead dash session layer. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "mod tests\|#\[test\]" mpd.rs` (14 tests mapped, boundary at 2278) · ls tests/fixtures/dash/ (all 4 fixtures exist: ffmpeg_av_list 2125B, ffmpeg_av_template 2288B, hand_dynamic 1490B, hand_static_cenc 1985B) · `rg -n "MAX_MPD_BYTES\|MAX_LINE_BYTES\|MAX_MPD_DEPTH"` (64 depth / 8 MiB / 64 KiB — match test comments) · sed -n 542,640p (MpdWarning variants: BadEntity/StrayText carry only `line`) · sed -n 2548,2552p + 2563,2569p (quote verification) · reachability greps above · baseline #3 (workspace tests 1681 passed / 1 failed — failure is apps/desktop zero_alloc, not playlists) |

## 1. One-paragraph verdict
Part5 is the second half of mpd.rs's test module: 14 tests that hammer
the REAL public parser (`parse_mpd` :652, `parse_mpd_bytes` :685,
`parse_xs_duration` :1916 — not shims) with hand-authored ISO/IEC
23009-1 fixtures, ffmpeg output, refusal tables, a billion-laughs bomb,
dirty-world degradation, and a deterministic mutation loop. The tests are
genuinely hostile and nearly all of them would fail under concrete
mutations of the parser (table below); fixtures exist on disk; the
bounded-input constants match the assertions. Two defects: the tested
surface is dead code in every shipped binary (the DASH session layer that
consumes the MPD model is itself referenced by nothing), and the entity-
bomb test's comment promises a "verbatim-keep check" that does not exist
and cannot exist — the warning model carries no text, so the comment
overstates the coverage the test provides.

## 2. What actually works (verified)
— All 14 tests call the real parser functions (rg for their definitions:
parse_mpd 652, parse_mpd_bytes 685, parse_xs_duration 1916) with
fixtures that exist (ls verified, 4 files).
— Constants asserted by tests match the parser's: MAX_MPD_DEPTH=64
(mpd.rs:126, depth test nests 70 and asserts `depth > MAX_MPD_DEPTH`),
MAX_MPD_BYTES=8 MiB (mpd.rs:132, oversize case uses `MAX_MPD_BYTES + 1`),
MAX_LINE_BYTES re-used from m3u (mpd.rs:118).
— Tests ran and passed: baseline #3 `cargo test --workspace` (INDEX.md —
1681 passed / 1 failed; the one failure is apps/desktop
zero_alloc_transport, unrelated to playlists).
— Error-code pinning: `error_code_table` (2764-2797) asserts both
`code()` and `to_string().contains(code)` for all seven refusal codes.
— Fuzz driver is itself panic-safe under empty buffers (arm 3 uses
`buf.len().max(1)` and `drain(at..end)` with end clamped; arm 1 guards
empty — traced line by line at 2815-2848).

## 3. Findings

### F-001 [S4]
Where: crates/playlists/src/mpd.rs:2548-2552 (comment) vs 2563-2569 (actual assertions)
Code:
```rust
        // run costs at most its own byte size — asserted by simply
        // completing, plus the verbatim-keep check.
```
and the actual assertions:
```rust
        assert_eq!(parsed.mpd.periods.len(), 1);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::BadEntity { .. }))
        );
```
What is wrong: The comment promises a "verbatim-keep check" for the
unexpanded `&lol3;` references. No such check exists — the test asserts
only `periods.len() == 1` and that SOME BadEntity warning fired.
Moreover the check is structurally impossible as described:
`MpdWarning::BadEntity` carries only `line` (mpd.rs:598-602, verified by
sed) and character data inside `<Period>` is DROPPED with a StrayText
warning ("stray character data (dropped)", mpd.rs:625-630) — the verbatim
text is never observable from the parsed model.
Why it matters: The bomb test is the crate's no-entity-expansion
evidence; a reader trusting the comment believes verbatim retention is
pinned. In reality the only protection against expansion is "the test
completes" (and the BadEntity-presence assertion, which would fail only
if entities became *known*). A regression that expanded entities but
still dropped the text would pass everything except timing.
What correct looks like: Either assert verbatim retention where it IS
observable (an unknown entity inside a modeled attribute such as
BaseURL, whose value is carried in the model) or delete the
"verbatim-keep check" claim from the comment.
Confidence: HIGH — Verification: read (comment + assertions + warning
enum fields all verified by sed; no other assertion in the test touches
entity text).

### F-002 [S3]
Where: crates/playlists/src/mpd.rs:2281-2854 (the entire tested surface), provenance: the parser at 652-685
Code:
```rust
        let parsed = match parse_mpd(text) {
            Ok(p) => p,
            Err(e) => unreachable!("hand static fixture must parse: {e}"),
        };
```
What is wrong: Nothing in the tests — everything in their value: the
parser they exhaustively pin cannot be reached from any shipped entry
point. The only consumer of the MPD model is streaming's dash layer
(sched.rs:287 `mpd: &Mpd`), which no app or engine crate references
(`rg "dash::" apps/ crates/playback-engine/ crates/input/` → zero);
abr.rs's `parse_mpd_bytes` import is `#[cfg(test)]`-gated (verified).
The playlists crate ships inside `lumen` only as streaming's compile-time
dependency, with zero runtime callers of `parse_mpd`.
Why it matters: 574 lines of test code (plus the ~2,200-line parser in
parts 1-4) are maintained and CI-executed to protect a code path no user
can trigger; any doc claiming DASH playback/MPD parsing works in the
shipped binary is false on reachability alone. This mirrors the
product-level "not_started" honesty of the transcode crate (W01-A03
F-002) — but unlike transcode, here the dead code is linked INTO the
shipped binary.
What correct looks like: Wire the streaming client/session (the layer
that would own dash) into an entry point, or cut playlists from
streaming's live dependency surface until the session layer ships.
Confidence: HIGH — Verification: read (grep call chains above; apps/cli
demux_host.rs uses only streaming::{fetch,ranged,schemes,mockhttp} —
verified by rg, no playlists path).

## 4. Per-function table (test functions in 2281-2854)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `hand_static_fixture_full_surface` (body from 2281; test starts 2278 in part4) | 2278-2358 | REAL | — | Pins duration 120.0, base_urls, ContentProtection (scheme/value/default_kid), SegmentTemplate inheritance (rep-level None, set-level Some), SegmentBase ranges, Role descriptor. Strong. |
| `hand_dynamic_fixture_types` | 2364-2408 | REAL | — | Pins Dynamic, minimumUpdatePeriod 2.0, timeShiftBufferDepth 30.0, AST/PublishTime verbatim, timeline t/d/r incl. `r: Some(-1)` (live-edge marker). Strong. |
| `duration_table` | 2412-2439 | REAL | — | 10 accept cases (incl. negative, whitespace-padded, day+time) + 13 refusals (Y/M/W, missing P, empty, fractional days, trailing junk). Would fail on any parse_xs_duration regression. |
| `not_mpd_refuses` | 2442-2457 | REAL | — | 5 non-MPD inputs → NotMpd + code pinned. |
| `bad_value_refuses` | 2460-2508 | REAL | — | 8 junk-attr inputs, each pinning the exact `attr` name in BadValue. |
| `truncated_refuses` | 2513-2527 | REAL | — | 5 cut points incl. `len()-3` (one byte short of `</MPD>`-ish); Truncated + code pinned. |
| `depth_exceeded_refuses` | 2530-2542 | REAL | — | 70 nested vs MAX_MPD_DEPTH=64; asserts `depth > MAX_MPD_DEPTH`. |
| `bounded_input_battery` | 2546-2592 | REAL | S4 | Entity bomb, oversize (8 MiB+1 → TooLarge + code), over-cap line (→ LineTooLong, code NOT asserted — cosmetic inconsistency), non-UTF-8 (→ InvalidUtf8 + code). F-001 comment defect. |
| `dirty_world_degrades_to_warnings` | 2598-2697 | REAL | — | Junk facts → None + BadAttrValue; missing bandwidth → warning; duplicate SegmentTemplate → first wins + DuplicateSegmentDescription; unknown element/attr tolerated; missing-end-tag salvage; stray end tag noted. Pins real degradation semantics. |
| `empty_root_records_no_periods` | 2701-2713 | REAL | — | Empty root parses, zero periods recorded as NoPeriods warning. |
| `namespace_tolerant_local_names` | 2717-2731 | REAL | — | `dash:`-prefixed root/children + xml:lang matched by local name. |
| `base_url_and_entity_decoding` | 2735-2760 | REAL | — | `&amp;` decoded once in BaseURL text; `&amp;amp;` → `x&amp;y` (single-pass decode pinned — this is the REAL verbatim/no-re-expansion check, unlike F-001's phantom). |
| `error_code_table` | 2764-2797 | REAL | — | All 7 refusal codes pinned incl. Display string. |
| `fuzz_lite_mutations_never_panic` | 2802-2853 | REAL | — | 600 deterministic splitmix64 mutations (truncate/flip/splice/drain) of one fixture; asserts only no-panic — stated purpose, honest, weak by design (result discarded, single fixture, fixed seed). |

## 5. Test honesty (this chunk IS tests; one concrete mutation each)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hand_static/dynamic fixtures | YES | Mutate SegmentTemplate inheritance to copy-to-rep → `r0.segment_template.is_none()` assert (2341) fails. |
| duration_table | YES | Mutate parse_xs_duration to accept "P1Y" → refusal loop (2433-2438) fails. |
| not_mpd_refuses | YES | Mutate root check to accept any element → NotMpd match fails. |
| bad_value_refuses | YES | Mutate timescale parse to default-on-error → BadValue match fails (attr name pinned). |
| truncated_refuses | YES | Mutate EOF tolerance → one of the 5 cuts parses Ok → let-else unreachable! panics. |
| depth_exceeded_refuses | YES | Mutate MAX_MPD_DEPTH to 100 → 70-deep input parses → unreachable! panic. |
| bounded_input_battery | PARTIALLY | Bomb: would fail if entities became known (BadEntity absent) or parse crashed; would NOT fail on silent text-drop changes (F-001). Size/line/UTF-8 cases: YES. |
| dirty_world | YES | Mutate first-wins to last-wins → duration Some(9999) vs asserted Some(2000) fails (2649). |
| empty_root / namespace / base_url | YES | Each pins specific model output (e.g. single-pass decode → `["x&amp;y"]` at 2759). |
| error_code_table | YES | Any code-string change fails. |
| fuzz_lite | NO (beyond panics) | Only no-panic is asserted; a parser that always returns Err instantly still passes. Stated in-comment; not a hidden lie. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Total input over MAX_MPD_BYTES (8 MiB)… Any line over 64 KiB… Nesting deeper than 64" | mpd.rs module doc 57-61 | REAL | Constants at 126/132 + m3u::MAX_LINE_BYTES import at 118; tests assert all three. |
| "asserted by simply completing, plus the verbatim-keep check" | mpd.rs:2551-2552 | FALSE (comment) | F-001: no verbatim-keep check; unobservable from the warning model. |
| MPD parsing capability in shipped binary | (no doc claims it — matrix DASH rows audited by wave-matrix agents) | UNREACHABLE | F-002 call-chain proof. |

## 7. Dead / duplicated / unreachable code
The tested parser surface is unreachable from shipped entry points
(F-002). Within the chunk: no dead test code; every test runs in the
default `cargo test` profile (no #[ignore], no feature gates — verified
by reading; none of the 14 tests carry attributes beyond #[test]).

## 8. Dependencies & flags
Test module uses only crate-internal items + `include_str!` fixtures
under crates/playlists/tests/fixtures/dash/ (all present). `lumen-allow-literal`
comments on example-host URLs (2292, 2600, 2737, 2748) are consistent
with the repo's lint-literals gate (baseline #13 shows the gate is real
and enforced elsewhere). No unsafe, no I/O, no threads.

## 9. Open questions for the next agent
- A01-A03 (mpd.rs parts 1-4): the parser's BadEntity keeps references
  "verbatim" per the enum doc (mpd.rs:598-602) — in TEXT position the
  text is dropped (StrayText); confirm attribute-position verbatim keep
  is what base_url_and_entity_decoding exercises, and consider adding the
  attribute-position unknown-entity case F-001 asks for.
- Whoever audits streaming/src/client.rs: if the session layer ever gains
  an app caller, F-002's reachability verdict for the whole MPD stack
  flips — re-run the chain then.
