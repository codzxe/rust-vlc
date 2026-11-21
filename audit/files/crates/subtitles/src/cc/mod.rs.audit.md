# AUDIT: crates/subtitles/src/cc/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 448 / 14 units (2 enums+Display, 3 structs, CcDecoder×7 methods, CcCueBuilder×2) — no in-file tests (coverage lives in tests/cc_acceptance.rs + submodules) |
| File verdict | UNREACHABLE (real, acceptance-tested state machine; zero shipped or cross-crate callers) |
| Reachable from shipped entry point? | NO — `rg "CcDecoder\|CcCueBuilder\|decode_ga94\|decode_scte20"` → only `crates/subtitles/tests/cc_acceptance.rs` + `cc/fuzz.rs` + in-module. The claimed upstream feed does not exist: `rg -il "ga94\|cc_data\|closed.caption" crates/codecs/src/ apps/` → only the H.264 SEI parser, which discards the bytes (F-001). Crate-level: nothing depends on `subtitles` but `fuzz/Cargo.toml`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -rn "GA94\|cc_data\|user_data" crates/codecs/src/` (sei.rs only, type-5 UUID form + generic Unparsed); `sed -n '148,156p' sei.rs` (Unparsed carries NO payload bytes); `rg "CcDecoder\|CcCueBuilder"` callers census; Read cc_acceptance.rs full (11 tests), cc_data.rs:1-40 (carriage docs), cc/mod.rs full; baseline: `orchrun-test-workspace-nofailfast.log.txt` — cc tests in the 267 (see §2); `sed -n` quote verifies. |

## 1. One-paragraph verdict
The stateful 608/708 front door is real engineering — parity-first drop, XDS skip, channel latch, DTVCC reassembly with a service-1→608 compat route that re-synthesizes parity, capped typed damage, per-call ledger reset — and it is pinned by a genuinely end-to-end acceptance suite (hand-assembled GA94/SCTE-20 vectors, roll-up stack, pop-on double-buffer, PAC colors into bitmap pixels, 200 KB seeded fuzz budget, systematic truncation sweep). But its headline carriage claim is false where it matters: the "H.264 consume seam" the module leans on does not exist — the codecs SEI parser records SEI type-4 as `(payload_type, size)` and throws the payload bytes away, so no byte of a real broadcast's captions can ever reach `parse_ga94_sei`; the only feeder of this decoder is its own test file. A one-nanosecond off-by-one contradicts the documented half-open timing rule, and the ledger drops overflow silently where VobSub records it.

## 2. What actually works (verified)
— Baseline `orchrun-test-workspace-nofailfast.log.txt:5271` (`running 267 tests` → `267 passed; 0 failed`) includes the cc suites; `tests/cc_acceptance.rs` (11 tests) drives `CcDecoder`/`CcCueBuilder` end-to-end: `classic_rollup_scrolls_through_the_full_stack` (RU2 stack "2" over "3", cue spans, open-ended last update), `ga94_sei_carriage_end_to_end`, `pac_colors_reach_the_bitmap` (red pixels + occupied-box anchor 1/32, 1/15), `pop_on_double_buffers_until_end_caption`, `parity_failure_is_typed_and_dropped`, `xds_and_channel_latch` (CC2 latch), `cea708_service1_routes_to_608_and_native_is_recorded`, `scte20_carriage_end_to_end`, `fuzz_lite_survival_seeded_mutations` (>100 KB asserted spent), `systematic_truncation_sweep_survives`.
— Code trace: `decode_608_pair` (:316-365) checks parity on both bytes before any latch/XDS logic, strips bit 7, latches the channel on `0x10..=0x1F` and PAC-shaped `0x40..=0x5F`+`lo>=0x40` via `hi & 0x08` — matches the EIA-608 channel-bit rule; borrow dance around `note()` is correct.

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/cc/mod.rs:5-10 (with crates/codecs/src/video/h264/parser/sei.rs:148-156, crates/subtitles/src/cc/cc_data.rs:14-17)
Code:
```
//! * [`cc_data`] — the ATSC `cc_data()` carriage grammars: the GA94
//!   SEI type-4 form (the H.264 consume seam — the codecs SEI parser
//!   captures type-4 messages intact, read-only), the MPEG-2
//!   picture-`user_data` GA94 form, and SCTE-20 bit-packed carriage.
```
What is wrong: "captures type-4 messages intact" is false at the byte level. The codecs SEI parser's representation for type 4 is `SeiMessage::Unparsed { payload_type: u64, size: u64 }` (sei.rs:148-153) — the GA94 payload BYTES are skipped and discarded; only the type and size survive. `rg` over `crates/codecs/src/` and `apps/` finds zero references to GA94/cc_data/captions beyond that generic skip. cc_data.rs:14-17 doubles down: "The codecs crate's SEI parser captures type-4 messages intact and skipped (… the read-only seam …) — this is the consuming half" — the consuming half has nothing to consume: the producing half drops the bytes.
Why it matters: the honesty table's "GA94 SEI type-4 … carriage | full" (mod.rs:30) is true only for the grammar-in-isolation; the end-to-end H.264 caption path the docs imply ("the H.264 user-data seam", lib.rs:128-130) does not exist anywhere in the tree. Real broadcast H.264 captions reach the SEI layer as opaque bytes and die there.
What correct looks like: either sei.rs gains a `UserDataRegisteredT35(Vec<u8>)` variant retaining bytes (a ~10-line change the doc could then honestly cite), or the docs state plainly: carriage grammars are test-fed only, no codec feed exists.
Confidence: HIGH — Verification: read (sei.rs Unparsed struct; rg census across codecs+apps).

### F-002 [S3]
Where: crates/subtitles/src/cc/mod.rs:438 (doc at :49-56)
Code:
```
            let end = next_pts_ns.map(|next| next.saturating_sub(1));
```
What is wrong: the documented rule is the half-open span `[update_pts, next_update_pts)` (:52-54); with `active_at`'s exclusive end (bitmap.rs:236-241), that rule requires `end = next`. The code sets `end = next - 1`, so the cue goes dark at `next - 1` — one nanosecond earlier than documented. `tests/cc_acceptance.rs:92` enshrines the wrong value (`assert_eq!(c1[0].end, Some(pts[1] - 1))`) while its own comment (:84) calls it "the half-open [pts, next_pts-1] spans" — a third, different notation.
Why it matters: the crate's selling point is sample-exact timing; doc, code, and test here state three inconsistent rules for the same span. Blast radius is 1 ns per transition, but the pattern (test pinning the bug) is the danger.
What correct looks like: `end = next_pts_ns` (half-open), doc unchanged, test asserting `Some(pts[1])`.
Confidence: HIGH — Verification: read (active_at semantics in bitmap.rs:236-241 traced).

### F-003 [S4]
Where: crates/subtitles/src/cc/mod.rs:226-230
Code:
```
    fn note(&mut self, damage: CcDamage) {
        if self.damage.len() < MAX_DAMAGE_EVENTS {
            self.damage.push(damage);
        }
    }
```
What is wrong: damage beyond the 256-event cap is dropped with no record — unlike the VobSub ledger, which appends a `VobSubDamage::Overflow { dropped }` count (vobsub/mod.rs:69-70, 201-203). A heavily damaged stream silently under-reports its damage count.
Why it matters: the damage ledger is the module's honesty mechanism; silent truncation of honesty events contradicts the design language ("capped typed damage ledger", :20-21) and the sibling precedent.
What correct looks like: a `CcDamage::Overflow { dropped }` variant appended once at take-time (needs a dropped counter like vobsub's).
Confidence: HIGH — Verification: read (comparison vobsub/mod.rs:132-140,201-203).

### F-004 [S4]
Where: crates/subtitles/src/cc/mod.rs:376-382
Code:
```
                for pair in block.payload.as_chunks::<2>().0 {
                    let d = [
                        eia608::with_odd_parity(pair[0]),
                        eia608::with_odd_parity(pair[1]),
                    ];
```
What is wrong: `as_chunks::<2>().0` discards a trailing odd byte of a service-1 payload silently — no `CcDamage` variant exists for it, in a module where every other malformation is typed.
Why it matters: a truncated 708-compat packet loses its last 608 byte with zero signal; inconsistent with the typed-everything contract (:58-61 "Never … " list promises damage as data).
What correct looks like: `if !block.payload.as_chunks::<2>().1.is_empty() { self.note(CcDamage::DtvccOddCompatTail { .. }) }`.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/subtitles/src/cc/mod.rs:427-447
Code:
```
            let end = next_pts_ns.map(|next| next.saturating_sub(1));
            cues.push(render::grid_cue(
                &rendered,
                event.pts_ns,
                end,
```
What is wrong: no ordering validation between `event.pts_ns` and `next_pts_ns`. A PTS regression (next < event) produces `end < start`; `CompositedCue::new` does not reject it (bitmap.rs:212-234 has no span-ordering check) and `active_at` then returns false for every `now` — the caption is invisible with zero damage signal. `saturating_sub` also masks the i64::MIN edge.
Why it matters: damaged-stream PTS regressions are a documented reality elsewhere in this repo (baseline ts188-panic-repro; demux wave's PCR-regression lane); this seam turns them into silent caption loss instead of typed damage.
What correct looks like: `if next <= event.pts { note/typed error or clamp end to None }`.
Confidence: HIGH — Verification: read (active_at trace: start>end ⇒ never active).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CcDamage` + `Display` | 78-173 | REAL | — | 11 variants incl. 708 specifics; data + fmt correct |
| `CcCaptionEvent` | 175-185 | REAL | — | pts_ns Option (pre-carriage timestamps) ✓ |
| `CcUpdate` | 187-200 | REAL | — | per-call ledger + records via mem::take ✓ |
| `FieldState` | 202-208 | REAL | — | active-channel latch + 2 channels per field |
| `CcDecoder::new` | 221-224 | REAL | — | Default derive; blank grids |
| `note` | 226-230 | REAL | S4 | F-003 silent overflow |
| `decode` | 236-277 | REAL | — | cc_valid gate, 608/708 split by cc_type (masked 2-bit at cc_data.rs:207,363 — the `_ => {}` arm :257 is unreachable by construction, honest comment); final-state events ✓ |
| `decode_ga94` | 285-292 | REAL | — | parse+decode convenience; grammar errors typed |
| `decode_scte20` | 298-305 | REAL | — | same |
| `reset` | 307-313 | REAL | — | seek/stream-change; no production caller (unreachability) |
| `decode_608_pair` | 315-365 | REAL | — | parity-first, XDS skip on field-2 low bytes, null-padding silent, channel latch `hi&0x08` on control+PAC shapes; flags read back for damage after borrow release |
| `consume_dtvcc_packet` | 368-402 | REAL | S4 | service-1→608 with parity re-synthesis (documented); F-004 odd-tail drop; native records dedup by service, per-call |
| `CcCueBuilder::new/cues` | 411-447 | REAL | S3/S4 | F-002 off-by-one, F-005 ordering; cleared screens skip ✓; z per event ✓ |

## 5. Test honesty (tests live in tests/cc_acceptance.rs — 11 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| classic_rollup_scrolls_through_the_full_stack | YES | exact grid rows per frame + cue spans incl. open-ended |
| ga94_sei_carriage_end_to_end | YES | real GA94 bytes → grid text |
| pac_colors_reach_the_bitmap | YES | exact bitmap dims, red pixels, anchor rationals 1/32 & 1/15 |
| pop_on_double_buffers_until_end_caption | YES | hidden→swap→erase sequence asserted |
| parity_failure_is_typed_and_dropped | YES | damage variant + subsequent-good-pair cursor state |
| xds_and_channel_latch | YES | XdsSkipped + CC2 routing |
| cea708_service1_routes_to_608_and_native_is_recorded | YES | compat route output + service-5 record + DtvContinuationWithoutStart |
| unknown_service_and_truncated_carriage_are_typed | YES | UnknownService708{0}, Truncated, BadMarker{0x7E} |
| scte20_carriage_end_to_end | YES | hand-bit-packed payload → "ok" |
| fuzz_lite_survival_seeded_mutations | YES (as survival) | >100 KB budget asserted; ledger-cap invariant per parseable input |
| systematic_truncation_sweep_survives | YES (as survival) | every truncation of every seed × every surface |
| NOT covered anywhere | — | F-001 (no codec-side feed exists to test), F-002's doc-vs-code rule (test pins the code), F-005 (out-of-order next_pts never fed) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Extract and render ATSC closed captions" status implemented win/linux/macos | feature-matrix.toml `subtitle-cc-608-708` (:1022-1032) | UNREACHABLE (real+tested library; nothing shipped reaches it) | caller census: tests only |
| "FULL EIA-608 channel machine — parity, field/channel latch …" | matrix:1031; mod.rs honesty table :31-33 | REAL (in isolation) | cc_acceptance roll-up/pop-on/latch tests; baseline 267/267 |
| "carriage grammars GA94 SEI type-4 + MPEG-2 picture-user_data + SCTE-20" | matrix:1031; mod.rs:30 | REAL (grammars) / the implied feed FALSE | F-001: sei.rs Unparsed discards bytes; mpeg12 dispatch admitted unwired in cc_data.rs:20-22 |
| "render to CompositedCue over the 704x480 NTSC reference" | matrix:1031 | REAL | pac_colors test: 22×32 cells, anchor 1/15 |
| "26 unit + 11 cc_acceptance tests, all green" | matrix:1031 | REAL (count consistent) | baseline 267/267 incl. cc; 11 tests counted in cc_acceptance.rs |
| "the H.264 consume seam — the codecs SEI parser captures type-4 messages intact, read-only" | mod.rs:7-10; cc_data.rs:14-17 | FALSE | F-001 |
| half-open `[update_pts, next_update_pts)` timing | mod.rs:52-54 | FALSE (code does next−1) | F-002 + cc_acceptance.rs:92 |

## 7. Dead / duplicated / unreachable code
Production-dead twice over (crate unlinked; only test feeder). `CcDecoder::reset` has no caller outside tests. The `_ => {}` cc_type arm (:257) is unreachable via the parsers but reachable via manual `CcDataTriplet` construction (pub field) — silent skip, harmless.

## 8. Dependencies & flags
Internal only (`cc_data`, `cea708`, `eia608`, `render`, `compositing`) + media-types via submodules. Matches boundaries.toml. No unsafe, no features.

## 9. Open questions for the next agent
- A10 (cc_data.rs): confirm `parse_ga94_sei`'s tolerance choices (reserved bit, process_cc_data_flag=0) against CTA-708 — my read of the doc grammar (:29-45) says '0' process flag means "not captions" and should refuse; verify the code agrees.
- A04 (cc/render.rs): does `render_grid` error on anything a `CcGrid` can actually hold (15×32 fixed)? If not, `CcCueBuilder::cues`'s `?` is dead-but-fine (contrast vobsub F-005).
- Whoever owns the codecs wave follow-up: sei.rs `Unparsed` losing payload bytes is the concrete 10-line fix that would make this module's docs true.
