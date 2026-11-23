# AUDIT: crates/subtitles/src/cc/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 448 / 11 (Display, 8 CcDecoder methods, 2 CcCueBuilder methods) |
| File verdict | SHALLOW — REAL state machine, executed, no panics — but the 608 PAC model contradicts the EIA-608 byte layout it cites; real-world captions are mangled (executed proof) |
| Reachable from shipped entry point? | NO — crate unlinked from all apps/engines (see parser.rs F-001 evidence); only fuzz/ + tests |
| Findings | S0: 1 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rustc probe (audit/runs/probe_cc.rs vs prebuilt rlib, exit 0 — outputs quoted below); rg over Cargo.tomls (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
The decoder front door is real, allocation-bounded, panic-free code with a genuinely good
damage taxonomy — I executed it against hand-built carriage vectors and it behaves. But the
EIA-608 interpretation at its core is wrong against the real spec: PACs are matched on first
bytes 0x40–0x5F (text-character range!) instead of the control range 0x10–0x1F, so (a) every
real-world PAC is silently no-oped, and (b) ordinary text pairs whose first char is '@'..'_' and
second ≥ '@' are consumed as PACs. I fed it a real-spec pop-on "HELLO WORLD!" caption and got
"D!" back — 8 of 9 text pairs eaten, cursor never positioned. The module's honesty table claims
"608 PAC … | full" and the feature matrix claims the whole CC feature "implemented" on three
platforms. The suite passes only because every test vector is authored in the decoder's own
dialect — the test comments even acknowledge '"AB" would be a PAC' and dodge it. A second
executed defect: the end-at-next-event rule subtracts 1 ns from a half-open span, creating a
1 ns dead zone that the module's own timing doc forbids.

## 2. What actually works (verified)
— probe_cc (exit 0): repo-dialect PAC (0x45,0x40) + lowercase "hi" → renders; "ab" renders;
  300 bad-parity pairs → ledger capped at exactly 256, decode continues, no panic;
  pop-on double-buffer swap, paint-on, roll-up all exercised via the same probe path and
  baseline suite.
— Baseline: 16 cc unit tests + 11 cc_acceptance tests pass (cargo-test-workspace.txt:5791-5816,
  6022-6034) — all under the repo's own PAC dialect.

## 3. Findings
F-001 [S0]
Where: crates/subtitles/src/cc/mod.rs:341-342 (+ eia608.rs:386-388, tests/cc_acceptance.rs:114-115)
Code: (verbatim, mod.rs)
```
        // The channel latch: codes that carry the channel bit re-latch
        // it; plain text rides the latched channel.
        if (0x10..=0x1F).contains(&hi) || ((0x40..=0x5F).contains(&hi) && lo >= 0x40) {
            self.fields[field].active = usize::from(hi & 0x08 != 0);
        }
```
vs (verbatim, eia608.rs:386-388)
```
        if (0x40..=0x5F).contains(&hi) && lo >= 0x40 {
            return self.pac(hi, lo);
        }
```
What is wrong: real EIA-608 PACs are (first byte 0x10–0x1F, second byte 0x40–0x7F); text bytes
are 0x20–0x7F and may legally form pairs like "HE" (0x48,0x45). This code transposes the two:
PACs are keyed on first byte 0x40–0x5F, so (a) real PACs (e.g. 0x15 0x40, row 5) fall into
`control_code`→`mid_row`→`_ => false` — silently dropped, NO damage; (b) text pairs
(first char '@'..'_', second ≥ '@') are eaten as PACs and also re-latch the active channel on
`hi & 0x08` of a TEXT byte. Executed (probe_cc, real-spec pop-on caption):
RCL + `15 40` (PAC row 5) + pairs `48 45 4C 4C 4F 20 57 4F 52 4C 44 21` ("HELLO WORLD!") + EOC
→ displayed grid contains only "D!" on row 15 — six text pairs destroyed, PAC ignored.
Paint-on "AB" → "AB" vanishes (only "ab" survives). The spec cannot have PACs in the text
range: "HE" would be undecodable from a PAC — first bytes must live in the control range. The
repo's own PAC_ROW table is byte-identical to ffmpeg/ccextractor's row_map (1-based), proving
the row arithmetic was taken from the reference decoders while the byte ranges were not.
Why it matters: the honesty table (lines 30-37) claims "608 PAC (row + attributes + indent),
mid-row, tab, misc codes | full" and the "full EIA-608 channel machine" — false against
real-world caption streams; matrix row subtitle-cc-608-708 claims "implemented" ×3 platforms
(docs/feature-matrix.toml:1030). Tests pass only because vectors are authored in-dialect —
cc_acceptance.rs:114-115 admits it: `"A1" (a text pair: the second byte below 0x40 — "AB"
would be a PAC)` — the workaround is documented, the bug is not. What correct looks like: PAC
dispatch on first byte 0x10–0x1F with second byte 0x40–0x7F (channel bit 0x08 on the first
byte), text on 0x20–0x7F first bytes. Confidence: HIGH — Verification: executed (probe_cc);
UNVERIFIED: no external spec copy in the sandbox (no page-number citation available).
F-002 [S3]
Where: crates/subtitles/src/cc/mod.rs:437-438 vs 51-54
Code: (verbatim)
```
            let end = next_pts_ns.map(|next| next.saturating_sub(1));
```
vs module doc (verbatim)
```
//! [`CcCueBuilder::cues`] gives each update's grids the half-open
//! span `[update_pts, next_update_pts)` — closing one nanosecond
//! before the next event, the DVB end-display-time rule — and leaves
```
What is wrong: compositing's active_at is half-open (`now >= end` ⇒ inactive,
compositing/timing.rs:38-42), so `end = next - 1` yields the span [t, next−1) — one nanosecond
SHORT of the documented [t, next). Executed: cue with next_pts=2_000_000_000 gets
end=Some(1_999_999_999) and `active_at(1_999_999_999) == false` — at every CC update seam
there is a 1 ns window where neither the old nor the new grid is active, violating the
"adjacent cues never gap" invariant the timing module exists to guarantee. The −1 is a DVB
inclusive-end habit applied to an end-exclusive model (double subtraction). Blast radius: 1 ns
per seam — invisible in practice, but the doc claim and the seam property are both wrong.
What correct looks like: `end = next_pts_ns` (no subtraction). Confidence: HIGH —
Verification: executed (probe_cc).
F-003 [S4]
Where: crates/subtitles/src/cc/mod.rs:226-230
Code: (verbatim)
```
    fn note(&mut self, damage: CcDamage) {
        if self.damage.len() < MAX_DAMAGE_EVENTS {
            self.damage.push(damage);
        }
    }
```
What is wrong: events past the 256 cap are discarded with NO overflow count — `CcUpdate` has
no `dropped` field (contrast srt::SrtDocument.dropped_damage_events). The module doc's honesty
table says XDS is "skipped, counted" — counting silently stops at 256. Executed: 300 parity
failures → `damage.len() == 256`, 44 events vanished without trace. Confidence: HIGH —
Verification: executed (probe_cc).
F-004 [S4]
Where: crates/subtitles/src/cc/mod.rs:244-257, 376
Code: (verbatim)
```
                _ => {} // cc_type is 2 bits — unreachable by construction
```
What is wrong: `cc_type` is a `pub u8` on `CcDataTriplet` (cc_data.rs:64) — a public-API caller
can construct triplets with cc_type 4–255, which this arm silently ignores (no damage). In-crate
constructors mask to 2 bits (`t[0] & 0x03`, `head & 0x03`), so the comment holds only for
in-crate paths; the pub field breaks the invariant the comment leans on. Also line 376:
`block.payload.as_chunks::<2>().0` drops an odd trailing byte of a service-1 payload silently
(no damage record). Confidence: HIGH — Verification: read (cc_data.rs:207,363 masking verified).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display for CcDamage | 139-173 | REAL | — | complete, no catch-all lies |
| CcDecoder::new | 222-224 | REAL | — | default |
| CcDecoder::note | 226-230 | REAL | F-003 | cap without overflow count |
| CcDecoder::decode | 237-277 | REAL | F-004 | per-call ledger take; events = final state per changed channel |
| CcDecoder::decode_ga94 | 285-292 | REAL | — | parse-then-decode; grammar errors typed |
| CcDecoder::decode_scte20 | 298-305 | REAL | — | executed via baseline scte20_carriage_end_to_end |
| CcDecoder::reset | 308-313 | REAL | — | full state drop |
| CcDecoder::decode_608_pair | 316-365 | REAL | F-001 | parity drop documented (ffmpeg divergence admitted); latch condition eats text pairs |
| CcDecoder::consume_dtvcc_packet | 369-402 | REAL | F-004 | svc1→608 route re-synthesizes parity (correct); native blocks recorded not applied (honest) |
| CcCueBuilder::new | 417-419 | REAL | — | stateless |
| CcCueBuilder::cues | 427-447 | REAL | F-002 | end−1 on a half-open span; z ordering saturating |

## 5. Test honesty
No #[cfg(test)] in this file; suite = tests/cc_acceptance.rs (11) + eia608.rs tests.
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| classic_rollup_scrolls_through_the_full_stack | PARTIAL | real control codes (14 25/2D) — would catch roll-up bugs; uses in-dialect PAC 44 60 |
| ga94_sei_carriage_end_to_end | YES | carriage grammar |
| pac_colors_reach_the_bitmap | NO for the PAC bug | deliberately avoids "AB" text pairs — author documents the workaround, not the defect |
| pop_on_double_buffers_until_end_caption | PARTIAL | double-buffer real; "H1" chosen with second byte < 0x40 to dodge F-001 |
| parity_failure_is_typed_and_dropped | YES | typed damage + state untouched |
| xds_and_channel_latch | PARTIAL | latch via control code 1C — does not test the text-pair latch corruption |
| cea708_service1_routes_to_608_and_native_is_recorded | YES | 708 reassembly + route |
| unknown_service_and_truncated_carriage_are_typed | YES | typed errors |
| scte20_carriage_end_to_end | YES | bit-packed grammar |
| fuzz_lite_survival_seeded_mutations | YES (for panics) | survival only — explicitly not correctness |
| systematic_truncation_sweep_survives | YES (for panics) | survival only |
Mutation/AC name "card 5.N3 acceptance … per the ATSC A/53 Part 4 / CTA-708 grammars" (header
lines 4-5) — the grammars tested are in-dialect for PAC; no real-world caption vector exists
("No reference corpus exists in the restored sandbox" — their own admission, line 7-8).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "608 PAC (row + attributes + indent), mid-row, tab, misc codes | full" | mod.rs:32 | FALSE on real-world streams | executed: real PAC no-oped, "HELLO WORLD!"→"D!" (F-001) |
| "full EIA-608 channel machine" | mod.rs:11-13 | FALSE on real-world streams | same execution |
| "status implemented (win/linux/macos)" | docs/feature-matrix.toml:1030 | FALSE at product level | crate unlinked; CLI no-ops Frame::Subtitle (see parser.rs F-001) |
| "half-open span [update_pts, next_update_pts)" | mod.rs:52-53 | FALSE | executed: span is [t, next−1) (F-002) |
| "XDS … skipped, counted" | mod.rs:43 | SHALLOW | counting stops at 256, no overflow trace (F-003) |
| "Never … unbounded allocation (the damage ledger and the DTVCC reassembly buffer are capped)" | mod.rs:58-61 | REAL | executed: 300 events → 256; cea708 tests pin reassembly caps |

## 7. Dead / duplicated / unreachable code
File unreachable from shipped entry points (crate unlinked — see parser.rs F-001). No dead
functions. `CcCueBuilder` is stateless (`struct CcCueBuilder;` + Default) — fine.

## 8. Dependencies & flags
In-crate only: cc_data, cea708, eia608, render, fuzz; compositing::CueError/CompositedCue.
Std-only, no unsafe, no features. Parity-drop divergence from ffmpeg documented (mod.rs:81-83) —
honest.

## 9. Open questions for the next agent
- eia608.rs (not assigned to me) owns the other half of F-001 (decode_pair dispatch + the
  "PAC 0x40..=0x5F first bytes" module doc at eia608.rs:9-12) — its auditor should cross-ref
  this finding; the fix must land in both files plus the acceptance vectors.
- Is there ANY committed real-capture CC fixture anywhere (docs/corpus?) that exercises the
  real PAC layout? rg suggests no — worth confirming for the record.
