# AUDIT: crates/playback-engine/tests/track_switch_fixture.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 895 / 3 tests + fixture (SwitchPlan parser, RealProvider, builders, poll drivers) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — composition acceptance test: the real byte-source → AVI demuxer → MJPEG/PCM decoders stack (dev-dependencies, disclosed at 11-19) through the engine's track-switch flow. The stack it composes IS the shipped `lumen` decode path (AVI/MJPEG/PCM — the codecs W00 proved decode). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `head -60`, inventory `rg`, `sed -n '558,687p'` (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25 — this suite needs the committed fixture file, present under the repo's fixture scheme). |

## 1. One-paragraph verdict

The strongest kind of acceptance test in the tree: the real multi-track fixture (`avi_mjpeg_2pcm.avi`) through the real demux/decode stack, with **every switched-on number read from the fixture's expectation TOML, never transcribed** (switch point, from/to track ids, the post-switch audio shape — 44.1 kHz mono u8 vs the default 48 kHz stereo s16, deliberately distinct so a wrong-track decode is observable at the sink as a format change). The main test asserts the full ordered event sequence (receipt at post-bump generation → C8 playing→playing → exactly one `tracks.changed` under the switch's own generation with the causing request as origin → no end-of-track for the retired limb), the sink truth (new track's first frame stamped with the switch generation, forward fill, zero old-track frames past the boundary), video decoding through the audio switch, the clock re-anchored on the new track's own pts, the decoder-cache round-trip (build counts stay 1 per track, `decoders_reused` counts the re-activation), and an honest `Ended`. Two hygiene notes: the expectation TOML is parsed with hand-rolled helpers rather than a TOML crate (fine for a flat table, but silent on malformed values), and the fixture-file dependency means the test's evidence value is tied to the fixture's provenance (W2-FIX-MT — another agent's row).

## 2. What actually works (verified)

- `parse_switch_plan` (123-191): reads the recorded plan from the fixture's expectation TOML — the "recorded truth" doctrine the header claims (24-28). Verified the plan fields flow into every assert (switch point via `poll_until_audio_at`, ids via selectors, shapes via format asserts at 599-608).
- The ordered-sequence asserts (615-681): receipt index < confirmation index (652-655), the selection-terms vector equality (658-667) demanding exactly one terminal with (generation, kind, track, origin) — a duplicate or mis-stamped terminal fails.
- The wrong-track-decode oracle (590-612): shape-per-track-id at the sink — a decode routed to the wrong limb fails as a format mismatch, not just a count.
- Decoder-cache accounting (566-572 and the switch-back section): exact build/reuse counters through provider `build_count` + engine stats.
- `the_decoder_cache_dies_with_the_session` (865-896): the cache's lifetime pinned separately.

## 3. Findings

### F-001 [S4] Hand-rolled TOML parsing of the expectation file
Where: track_switch_fixture.rs:105-121 (`toml_int`/`toml_quoted`)
What is wrong: a flat-table parser with no error taxonomy — a malformed expectation file fails as a parse panic or a wrong default rather than a clear fixture error. The `the_recorded_switch_plan_parses` test (851-864) guards presence, not malformedness. Hygiene.
Confidence: HIGH — Verification: read.

### F-002 [S4] Evidence tied to fixture provenance
The test proves the switch flow on ONE committed container (W2-FIX-MT). The header is explicit about this ("the committed multi-track parent"); recorded so matrix rows citing this file as evidence of "multi-track switching" carry the single-fixture qualifier.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `SwitchPlan` + `toml_int`/`toml_quoted`/`parse_switch_plan` | 90-191 | REAL | S4 (F-001) |
| `SharedNow`/`Collector` | 192-221 | REAL | frozen-ish virtual time, recording sink |
| `RealProvider` | 223-287 | REAL | real codecs behind the provider seam; per-track build counters |
| `fixture_queues`/`fixture_budget`/`Fixture`/`open_fixture` | 289-391 | REAL | real FileAccess + AviDemuxer wiring |
| snapshot/step/poll helpers | 393-556 | REAL | bounded with context strings; `poll_until_audio_at` plays to the recorded point |
| `real_fixture_audio_switch_follows_the_recorded_plan` | 558-850 | REAL | the eight-step proof of the header, asserted in order |
| `the_recorded_switch_plan_parses` | 851-864 | REAL | presence guard |
| `the_decoder_cache_dies_with_the_session` | 865-896 | REAL | cache lifetime |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| real_fixture_audio_switch_follows_the_recorded_plan | YES (reasoned) | ordered-index asserts, vector-equality terminal, shape-per-track, generation stamps, cache counters — wrong routing, stale frames, lost terminals, or cache misses each fail |
| the_recorded_switch_plan_parses | WEAK by design | presence guard only (disclosed) |
| the_decoder_cache_dies_with_the_session | YES (reasoned) | counter-based lifetime pin |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every number this test switches on is read from the fixture's expectation TOML, never transcribed" | header 24-28 | REAL | verified plan fields reach every assert |
| "switch tests on multi-track fixtures with event-sequence assertions" (card 3.A3 AC) | header 2-4 | REAL-UNPROVEN | asserted in full; run UNVERIFIED |
| video keeps decoding through the audio switch (S4.2) | header 43-45 | REAL-UNPROVEN | asserted via video-frame counting past the switch |

## 7-9. Dead code / Dependencies / Open questions

Dev-dependency composition disclosed (11-19, boundaries-manifest Lead flag). Open: fixture provenance (W2-FIX-MT) and the fake-harness sibling (`tests/track_switch.rs`, another agent) — the header claims this file mirrors that file's case-1 assertions; a drift between them would be worth a cross-check.
