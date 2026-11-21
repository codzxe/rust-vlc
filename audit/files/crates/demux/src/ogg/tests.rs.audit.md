# AUDIT: crates/demux/src/ogg/tests.rs (lines 1-575, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 575 / 21 items (2 helpers + 19 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (cfg(test)); code under test reachable: `lumen open x.ogg` → probe "ogg" → `apps/cli/src/containers/mod.rs` ogg arm → OggDemuxer (mod.rs); baseline executed all 19 tests ok (`orchrun-test-workspace-nofailfast.log.txt`: 19 `ogg::tests::`, 0 failed — incl. :1816, :1823, :1825) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full Read of tests.rs + ogg/test_support.rs (builder verified: lacing runs, CRC patch offset, LE layouts); `grep -rn "not_ogg" --include="*.rs" .` → defined+unit-tested in error.rs:24/174/197, named in tests.rs:480, **minted by no demuxer code**; `grep -n "pub fn" ogg/mod.rs` → accessor list (damage_total :528 is the only damage surface); `sed -n '828,847p'` mod.rs → orphan-run note(0,…) path; `sed -n '1170,1185p'` mod.rs → no-streams site; `grep -rn "opus_head_packet"` → definition only, zero callers; baseline greps; `sed -n` verification of every quote below |

## 1. One-paragraph verdict
A genuinely strong demuxer test battery: lacing assembly, mux vs chain, CRC-drop recovery, granule honesty (last-packet-owns-granule, −1 holes, regressions), typed truncation with sticky replay, seek landing semantics pinned to the 6.FIX behavior — every test builds byte-realistic pages via a verified builder and asserts observable outputs; all 19 executed and passed in the baseline. The defects are in what the names promise versus what the body can even observe: the "not_ogg" typed path named by one test does not exist anywhere in the demuxer (the error constructor is dead code), the "damage noted" and "zero-copy views" claims of two other names are unasserted — and in the damage case unobservable through the public API — and one test hand-rolls an OpusHead that duplicates a helper which is thereby left dead.

## 2. What actually works (verified)
- Builder `build_page` (test_support.rs:37-81): lacing runs (255-run keeps packet open), zero-length packets, CRC patched at the computed offset — verified byte-arithmetic; `vorbis_id_packet` LE24 rate layout correct (86-94).
- Packet assembly across pages (`continued_packet_assembles_across_pages`, 179-208): a 600-byte packet split 255/345 reassembles byte-exactly — would fail on any lacing or offset bug.
- Granule honesty trio: `only_the_last_packet_on_a_page_owns_the_granule` (stamps `[None, Some(512), Some(1024)]`, 88), `minus_one_data_page_punches_a_visible_hole` (stamps `[Some(256), None, Some(1024), Some(1280)]`, 364), `granule_regression_is_an_honest_discontinuity` (325) — mutation-proof pins of the never-guess policy.
- `truncated_page_is_a_typed_sticky_error` (263-285): pins code `demux.ogg.truncated-page`, one packet already flowed, and a replayed error; ogg's `next_event` genuinely wraps in `self.fail` (mod.rs:1310), so sticky state exists here (unlike adts — see my adts/mod.rs#part2 audit F-002).
- `seek_and_reanchor_land_on_the_last_page_before_the_target` (492-550): pins landing-report = previous granule (768), 5 post-landing packets, exactly-one discontinuity-marked packet `[true,false,false,false,false]` — the strongest test in the file; regression of the 6.FIX-ogg-reanchor behavior would fail it.
- `unterminated_lacing_run_is_a_typed_overrun` (401-425): ~1031 pages of pure-255 runs → typed `demux.ogg.packet-overrun`, tolerant of the error firing in either `start`'s tail probe or the walk (both converge on the sticky code — traced both paths).
- `mux_two_serials_two_tracks_interleaved` / `chained_links_boundary_discontinuity_and_fresh_tracks`: interleave order and per-link fresh track ids pinned (109-121, 167-168).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/ogg/tests.rs:480-489
Code:
```rust
fn not_started_and_not_ogg_are_typed() {
    let mut ogg =
        OggDemuxer::new(mem_stream(vec![1u8; 64]).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
    match ogg.next_event() {
        Err(err) => assert_eq!(err.context.code, "demux.ogg.not-started"),
```
What is wrong: the name promises two typed paths; the body tests only `not-started` (plus `seekability()==None`). The non-Ogg bytes (`vec![1u8; 64]`) are never passed to `start()` — and they could not produce a "not-ogg" error if they were: repo-wide grep shows `error::not_ogg` (error.rs:24-33, code `demux.ogg.not-ogg`) is minted by no demuxer code — its only callers are its own constructor unit test (error.rs:174, 197). A capture-less stream falls through to `error::no_streams()` (mod.rs:1176-1179).
Why it matters: the test name documents a format-detection taxonomy the code does not implement; a reader believes "not ogg → typed not-ogg" is covered when the shipped demuxer answers `no-streams` for a non-Ogg file routed to it. The dead constructor is error.rs's defect (A04's file), but this test is where the false coverage claim lives.
What correct looks like: either drop "not_ogg" from the name, or make `start()` on capture-less bytes mint `not_ogg` and assert it here.
Confidence: HIGH — Verification: read + repo-wide grep (`not_ogg` → 5 hits, all definitions/tests/names, zero demuxer call sites).

### F-002 [S4]
Where: crates/demux/src/ogg/tests.rs:388-397
Code:
```rust
    assert_eq!(
        pkts[0].1, b"\x00a1",
        "the orphan run is dropped, the real packet survives"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DemuxEvent::Recovered { .. })),
        "nothing was skipped"
    );
```
What is wrong: the test name says the continued-flag lie is "damage noted", but the body asserts the opposite observable — no `Recovered` event — and asserts nothing about damage. The internal note exists (`mod.rs:836-839`: `self.damage.note(0, "ogg: continued flag without an open packet")`) but is invisible through the demuxer's only damage accessor `damage_total()` (mod.rs:528), which sums bytes — a 0-byte note moves it not at all.
Why it matters: the "noted" half of the name cannot be falsified by any test through the public API; deleting the `note(0, …)` call would pass this suite unchanged. (Deeper issue — write-only damage channel — belongs to mod.rs, A05/A03.)
What correct looks like: assert an exposed observable (e.g. expose `DamageLog::events` or a note count), or drop "damage_noted" from the name.
Confidence: HIGH — Verification: read (mod.rs:828-847 orphan path; DamageLog at avi/header.rs:385-400: `total` sums bytes only; accessor list of OggDemuxer grepped).

### F-003 [S4]
Where: crates/demux/src/ogg/tests.rs:553-574
Code:
```rust
fn pool_stays_bounded_and_packets_are_zero_copy_views() {
```
…
```rust
    assert_eq!(packets(&events).len(), 32);
    // 2 arenas × 256 KiB ceiling, demand-grown: far below in practice.
    assert!(ogg.pool_bytes() <= 2 * super::MAX_PACKET_BYTES as usize);
    assert!(ogg.pool_bytes() > 0);
```
What is wrong: the name's second claim — "packets are zero-copy views" — is never asserted. No pointer identity, no allocation counter, nothing that would fail if `complete_packet` cloned every packet out of the pool. Only boundedness (`pool_bytes`) is proven.
Why it matters: the zero-copy property (mod.rs:55 "zero copy, zero alloc") is a documented performance invariant with zero test coverage under a name that claims the opposite.
What correct looks like: assert the packet's `SharedBytes` pointer lies inside a pool arena (or count allocations via a counting source/instrumented allocator).
Confidence: HIGH — Verification: read (whole test body 553-575; no such assertion exists).

### F-004 [S4]
Where: crates/demux/src/ogg/tests.rs:93-98
Code:
```rust
    let id_a = vorbis_id_packet(8_000);
    let id_b = {
        let mut v = b"OpusHead".to_vec();
        v.extend_from_slice(&[1, 2, 0x38, 0x01, 0x80, 0x3e, 0, 0, 0, 0, 0]);
        v
    };
```
What is wrong: the mux test hand-rolls a 19-byte OpusHead inline while `test_support::opus_head_packet()` (test_support.rs:98-108) exists for exactly this — and repo-wide grep shows that helper has zero callers: this inline duplicate is the reason the helper is dead code.
Why it matters: two sources of truth for the OpusHead shape; the dead helper will rot (it already disagrees: 48 kHz/1ch vs the inline 16 kHz/2ch).
What correct looks like: parameterize the helper (rate/channels) and use it here, or delete the dead helper.
Confidence: HIGH — Verification: grep (`opus_head_packet` → definition only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `drain` | 12-21 | REAL | — | stops at first error, returns code — adequate harness |
| `packets` | 23-33 | REAL | — | (handle, bytes, keyframe) projection |
| `single_stream_start_packets_and_duration` | 35-66 | REAL | — | packets+keyframe flags, 128 ms duration arithmetic (1024@8kHz ✓), header stash, links, counters — mutation-proof |
| `only_the_last_packet_on_a_page_owns_the_granule` | 68-89 | REAL | — | stamps `[None, Some(512), Some(1024)]` pinned |
| `mux_two_serials_two_tracks_interleaved` | 91-123 | REAL | S4 | interleave pinned; F-004 inline OpusHead |
| `chained_links_boundary_discontinuity_and_fresh_tracks` | 125-176 | REAL | — | boundary Discontinuity + fresh track ids + links()==2 pinned |
| `continued_packet_assembles_across_pages` | 178-208 | REAL | — | 600-byte reassembly byte-exact |
| `bad_crc_page_is_dropped_with_sync_recovery` | 210-238 | REAL | — | body-byte flip at 29 verified in-body; 1 Recovered, survivors pinned |
| `mid_stream_garbage_recovers_at_the_next_capture` | 240-260 | REAL | — | 2 KiB+ noise, damage ≥ 2058 (loose lower bound, acceptable) |
| `truncated_page_is_a_typed_sticky_error` | 262-285 | REAL | — | code + flow + replay pinned; sticky mechanism verified present in mod.rs:1310 |
| `sequence_discontinuity_is_damage_noted_and_continues` | 287-309 | REAL | — | seq gap 1→7 → Discontinuity + counter |
| `granule_regression_is_an_honest_discontinuity` | 311-331 | REAL | — | backwards clock → discontinuity |
| `minus_one_data_page_punches_a_visible_hole` | 333-365 | REAL | — | Missing stamp in the hole; re-anchor loud |
| `continued_flag_lies_are_damage_noted` | 367-398 | REAL | S4 | F-002: "noted" unasserted and unobservable via public API |
| `unterminated_lacing_run_is_a_typed_overrun` | 400-425 | REAL | — | overrun robust to start-vs-walk firing point |
| `no_bos_page_is_a_typed_no_streams` | 427-438 | REAL | — | invalid version byte (0x11) — realistic no-BOS |
| `unknown_codec_demuxes_opaque_with_missing_stamps` | 440-457 | REAL | — | "ogg-opaque" + duration None — honest unknown path |
| `skeleton_stream_mints_no_track_and_no_packets` | 459-477 | REAL | — | skeleton filtered, serials retained |
| `not_started_and_not_ogg_are_typed` | 479-489 | REAL | S3 | F-001: half the name untested; the error it names is minted nowhere |
| `seek_and_reanchor_land_on_the_last_page_before_the_target` | 491-550 | REAL | — | strongest test: landing report 768, 5 packets, one-mark discipline |
| `pool_stays_bounded_and_packets_are_zero_copy_views` | 552-575 | REAL | S4 | F-003: zero-copy claim unasserted |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| single_stream… | YES | exact packet bytes + keyframe flags + duration ns |
| only_the_last… | YES | stamp vector exact |
| mux_two… | YES | track count/codecs/interleave order |
| chained_links… | YES | boundary position, before/after handles, links count |
| continued_packet… | YES | 600-byte byte-exact reassembly |
| bad_crc… | YES | drop count, survivors, damage>0 |
| mid_stream_garbage… | YES | 3 survivors + damage bound |
| truncated_page… | YES (code/flow); WEAK (sticky mechanism) | replay also passes by re-derivation — but ogg genuinely implements sticky (mod.rs:1310), verified by read |
| sequence/granule/minus_one… | YES | counters + stamp vectors exact |
| continued_flag_lies… | YES (drop behavior); NO (damage noting) | F-002 |
| unterminated_lacing… | YES | typed code pinned |
| no_bos / unknown_codec / skeleton / not_started | YES | codes and counts pinned (except the not-ogg half, F-001) |
| seek_and_reanchor… | YES | landing ticks, packet count, mark vector |
| pool_stays_bounded… | YES (bound); NO (zero-copy) | F-003 |
| Baseline execution | — | 19/19 ok (nofailfast:1808-1826 region) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "this one pins the demuxer's … every typed error path" | tests.rs:2-4 (module header) | FALSE for one path | the `demux.ogg.not-ogg` path is neither exercised nor exercisable — error minted nowhere (F-001) |
| "repairs are counted and surfaced, never silent" | avi/header.rs:383-384 (DamageLog doc, shared type) | SHALLOW for ogg | the continued-lie note (mod.rs:838) is surfaced through no ogg accessor (F-002) |
| "body — zero copy, zero alloc" | ogg/mod.rs:55 | REAL-UNPROVEN | code reads as pooled views (mod.rs:889-893), but no test observes it (F-003) |

## 7. Dead / duplicated / unreachable code
- `test_support::opus_head_packet` — dead repo-wide (F-001's sibling evidence, F-004).
- `error::not_ogg` — dead in production paths (only its own unit test calls it) — cross-ref A04 (ogg/error.rs).

## 8. Dependencies & flags
- cfg(test) module; no feature gates; no unsafe; no `#[ignore]`.
- Uses only public seams: `links/serial_numbers/header_packets/granule_discontinuities/damage_total/pool_bytes/track_duration/tracks` (mod.rs:490-559) — the test surface equals the observability surface, which is exactly why F-002's note is invisible.

## 9. Open questions for the next agent
- A05/A03 (ogg/mod.rs parts 2-3): confirm `DamageLog::events` is exposed by NO demuxer (avi exposes only total too?) — if so, every `note(0, …)` decision-record is write-only repo-wide, which upgrades F-002's class.
- A04 (ogg/error.rs): the dead `not_ogg` constructor — same pattern as adts `bad_descriptor` (documented-unreachable residue), but here no production site ever existed.
- Whoever owns the probe: verify the "ogg" signature requires a capture pattern, so the untested not-ogg path cannot be reached by mis-routing from the shipped CLI (believed true; `signature/` module unverified by me).
