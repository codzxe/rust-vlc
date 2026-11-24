# AUDIT: apps/cli/src/containers/avi.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 142 / 2 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: main.rs -> host.rs:904 open_source -> demux_host.rs:458 containers::open -> "avi" arm (mod.rs:54) -> avi::open (avi.rs:35); end-to-end proven by my session_script run (21/21 through the real binary on the AVI parent fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `lumen_cli-5f13e10be716a8b0 demux_host --test-threads=1` -> 10 passed (incl. parent_fixture_opens_with_expected_facts, start_time_clamps_to_duration); session_script 21/21 (AVI end-to-end); sed quote checks; read demux-side StreamFormat/codec_descriptor |

## 1. One-paragraph verdict
The first vertical's container host, moved verbatim from the D-018 gate — and
it shows: 142 lines, two functions, closed vocabulary, no invention. The
demuxer owns all AVI validation (header parse, stream filtering — unsupported
handlers/streams are filtered with damage notes per demux tests
:453/:487/:1065); this host owns the strf facts harvest (MJPEG dimensions +
frame cadence, PCM channels/rate through the demuxer's own descriptor
mapping — verified no local tag table copy), the start-time clamp, and the
shared wrapper. Everything it does is executed end-to-end by the session
suite. The only debt: it is the one container host with ZERO in-file tests,
plus two micro-inconsistencies (non-saturating division; the shared clamp
seam).

## 2. What actually works (verified)
— verified by `./target/debug/deps/session_script-871be5754d608fa7 --test-threads=1`
-> 21 passed (open/play/pause/seek/stop/teardown on the AVI MJPEG+PCM
fixture through the real binary: facts surface as "container: avi",
"#0 video mjpeg 320x240", "#1 audio pcm_s16le 2ch 44100Hz"); and
`lumen_cli-5f13e10be716a8b0 demux_host` -> 10 passed
(parent_fixture_opens_with_expected_facts + start_time_clamps_to_duration
drive THIS file's open()).

## 3. Findings
F-001 [S4]
Where: apps/cli/src/containers/avi.rs:87-97
Code:
```
                // One tick of the video base is one frame (AVI scale=1).
                let frame_ns = (u64::from(track.timing.base.num)).saturating_mul(1_000_000_000)
                    / u64::from(track.timing.base.den);
```
What is wrong: plain `/` on den while the ogg sibling
(ogg.rs:212-216) uses `saturating_div` for the identical computation —
inconsistent defensiveness. den==0 is unreachable (TimeBase construction
validates), so this is style, not a live panic. Confidence: HIGH —
Verification: read.
F-002 [S4]
Where: apps/cli/src/containers/avi.rs:43-45
Code:
```
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth).
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
```
What is wrong: the same latent clamp-panic seam as ps.rs/ogg.rs (a negative
duration_us would panic "min > max"); AVI span ends are non-negative so
unreachable today. Copy-paste carrier of the pattern across hosts.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: whole file (no #[cfg(test)] mod)
Code:
```
            None => ("unknown".to_string(), None, None, None),
```
What is wrong: avi.rs is the only wired container host with no in-file test
module (ps/ogg/mp4/ts/adts/wav all carry lanes). Coverage is real but
external (demux_host parent-fixture tests, session_script, conformance
suites — I ran those). The `None` arm at :117 is defensive dead weight:
StreamDesc.format is non-optional (header.rs:251), so stream_format() answers
Some for every minted track. Confidence: HIGH — Verification: read + ran
external suites.

## 4. Per-function table (file < 100 LOC of logic; table for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 35-78 | REAL | — | clamp + pure seek query + shared wrapper; identical shape to ps/ogg hosts |
| harvest_facts | 84-142 | REAL | S4 | MJPEG: dimensions + frame_ns from base (correct for any scale: frame = scale/rate s); PCM: demuxer's codec_descriptor, no local tag table (verified header.rs:217-227); extra_data honestly None (AVI carries no out-of-band blob) |

## 5. Test honesty
No in-file tests. External: demux_host::tests::parent_fixture_opens_with_expected_facts
(asserts this harvest's facts for the parent fixture — would fail on a
harvest regression), start_time_clamps_to_duration, session_script 21
(ordered trails + exact positions). The demux crate's AVI suite (33 tests,
:341-1065) pins the validation split this host relies on (unsupported
streams filtered with damage, degenerate streams typed, PCM tag matrix,
EXTENSIBLE GUIDs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The AVI-specific remainder is exactly this file's two functions:" + "else (skip wrapper, engine wiring, codec dispatch) is shared code." | :8, :10 | REAL | SelectDemuxer/demux_host/provider are shared (read); only open+harvest here |
| "channels/rate through the demuxer's own descriptor mapping — not a" + "local copy of the tag/bits table)" | :82-83 | REAL | codec_descriptor() is the demuxer's (header.rs:217); no tag table in this file |
| "AVI carries no out-of-band codec blob" | :127-132 | REAL | extra_data: None for both arms |

## 7. Dead / duplicated / unreachable code
The `None => ("unknown", ...)` harvest arm (:117) is unreachable for minted
tracks (StreamDesc.format non-Optional; ordinal keyed by the same streams
that minted the tracks). Defensive residue, not a defect.

## 8. Dependencies & flags
No feature gates; demux::avi + shared host types only. No unsafe. Release
lane shipped (baseline build exit 0); the first vertical's rc-smoke
headless-decode (AVI/MJPEG/PCM) passes per baseline.

## 9. Open questions for the next agent
- Non-MJPEG/PCM AVI streams are demuxer-filtered before this host sees them
  (demux tests :453/:487) — an XVID AVI therefore opens as "no supported
  streams" typed error rather than a track list; if product wants
  demux-but-refuse-decode parity with the VOB lanes, that is a demux-side
  scope decision.
