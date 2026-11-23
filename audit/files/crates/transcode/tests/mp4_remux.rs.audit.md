# AUDIT: crates/transcode/tests/mp4_remux.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 744 (7 tests + 14 helpers/fixtures) |
| File verdict | REAL — the remux-without-reencode capability EXISTS and the suite executes it end-to-end (I ran all 7 tests, pass); verification is self-referential (own demuxer, no external oracle) and the capability is unreachable from shipped entry points |
| Reachable from shipped entry point? | The TEST is test-only by nature; the code under test (`transcode::record::Remuxer` + `transcode::mp4mux::Mp4Remuxer`) has ZERO production callers — rg `Remuxer|transcode` in apps/cli, apps/desktop, apps/web → 0 hits; no app Cargo.toml depends on transcode. Matrix rows `playback-record` (feature-matrix.toml:1630-1641) and `ui-dialog-convert` (2913-2924) honestly say not_started / empty evidence. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/mp4_remux-2cce5ceb8b8d5666 --test-threads=2` → 7 passed / 0 failed, exit 0; `file tests/fixtures/generated/valid/mp4_h264_aac.mp4` → ISO Media MP4 v1; fixture ls (4 committed MP4s, 13.7-424 KB); rg reachability; sed quote verification |

## 1. One-paragraph verdict
Answering the hostile-focus question directly: YES, remux-without-re-encode exists and works. `Remuxer::new(demuxer, sink).track(cfg…).remux(limits)` copies H.264/AAC sample payloads byte-exact into a fresh progressive MP4 — I executed the suite myself: all 7 tests pass in 0.01s, including the fragmented-input→progressive-output conversion, the edit-list re-anchoring, three typed refusals (`transcode.mp4.codec-unsupported` / `.config-track-missing` / `.tracks-empty`), and a mid-remux cancel that leaves a structurally valid prefix whose tables describe exactly the landed bytes (`tables = stats` assertion). The weaknesses: the copy is verified only by the repo's own `Mp4Demuxer` (no ffprobe/oracle differential, unlike the demux suites' own standard), absolute-timeline identity is waived by design (stamps compare relative to each track's first DTS), fixtures are tiny (≤14 KB), and nothing shipped can invoke any of it.

## 2. What actually works (verified)
— verified by `./target/debug/deps/mp4_remux-2cce5ceb8b8d5666` → `test result: ok. 7 passed; 0 failed` (exit 0). Payload bytes, sync flags, durations, per-track counts compared exact against the source demux; output re-demuxes through the real `Mp4Demuxer` over a real `input::FileAccess` open of the committed fixture (not a mock source); cancel test drives a scripted channel-backed demuxer and parks the pump mid-flight, then proves whole-packets-only + tables=stats on the prefix.

## 3. Findings
F-001 [S3]
Where: crates/transcode/tests/mp4_remux.rs:1-5 (and the whole AC)
Code:
```
//! The card 7.E2 MP4-remux acceptance suite (issue #44 AC): the
//! [`Remuxer`] one-call assembly copies a committed H.264/AAC fixture
//! into a fresh MP4, and the copy is verified by OUR own demuxer —
```
What is wrong: the acceptance oracle is the same code family that feeds the muxer. The demux suites in this repo hold themselves to live ffprobe/`-count_packets` differentials (e.g. container-asf evidence, feature-matrix.toml:216); the remux output gets no external check — a symmetric bug in muxer+demuxer (e.g. both mishandling the same timescale or ctts convention) passes. Fixtures are also minimal (mp4_h264_aac 13.7 KB, mp4_frag 14 KB, mp4_edts 14.4 KB — handfuls of samples). What correct looks like: one ffprobe/ffmpeg `-c copy` differential lane on the remuxed output, as the demux suites already do.
Confidence: HIGH — Verification: read + ran (tests pass; absence of oracle verified by rg)

F-002 [S3]
Where: crates/transcode/tests/mp4_remux.rs:260-263 (the Remuxer call) / repo-wide
Code:
```
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(configs[0].clone())
        .track(configs[1].clone())
        .remux(JobLimits::default())
```
What is wrong: the capability this suite proves is dead code in the shipped build — no app references the transcode crate (rg over apps/cli, apps/desktop, apps/web → nothing). The matrix is honest (playback-record / ui-dialog-convert = not_started, evidence ""), so this is unwired-real, not fraud — but a reader of the suite header ("the card 7.E2 MP4-remux acceptance suite") could believe record/remux ships. It does not.
Confidence: HIGH — Verification: read + rg

F-003 [S4]
Where: crates/transcode/tests/mp4_remux.rs:249, 287-291
Code:
```
    // The fresh movie re-anchors each track's timeline at its first
    // sample DTS (the muxer's documented honesty table — an edit-list
    // source can surface a negative first DTS), so stamps compare
    // relative to that origin; payload bytes, sync flags, durations
    // and counts are exact.
```
What is wrong: the test name promises `…_packet_identical`, but absolute DTS/PTS identity is waived (per-track origin subtraction); only payload/sync/duration/count are exact. The waiver is documented in-line and is a real muxer property (fresh timeline), so this is a naming overstatement, not a hidden hole.
Confidence: HIGH — Verification: read

F-004 [S4]
Where: crates/transcode/tests/mp4_remux.rs:220-227
Code:
```
fn config_for(demuxer: &Mp4Demuxer, track: &Track) -> Mp4TrackConfig {
    Mp4TrackConfig {
        track: track.id,
        sample_entry: SharedBytes::from_vec(
            demuxer.sample_entry_bytes(track.id).unwrap_or(&[]).to_vec(),
        ),
    }
}
```
What is wrong: a `None` sample entry is silently converted to an empty config (swallowed error in test plumbing) instead of failing the test loudly; if `sample_entry_bytes` ever regressed to None, the remux would run with a zero-byte sample entry and only downstream demuxer rejection would catch it.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedCursor (Write/Seek) | 40-85 | REAL | — | in-memory sink; resize-on-write; seek clamps via try_from |
| sink_pair | 45-54 | REAL | — | |
| MemStream (SourceStream) | 88-144 | REAL | — | size-known, seekable; seek clamps to len; no panic path |
| mem_seek_error | 146-154 | REAL | — | typed test error |
| open_fixture | 160-165 | REAL | — | REAL file access stack (`input::FileAccess::new().open`) — not a mock |
| ticks / duration_nanos | 177-189 | REAL | — | Stamp::Missing → i64::MIN sentinel |
| demux_all | 192-210 | REAL | — | drains to EndOfStream |
| of_track | 213-215 | REAL | — | |
| config_for | 220-227 | REAL | S4 | F-004 |
| h264_aac | 230-240 | REAL | — | |
| progressive_source_remuxes… | 249-346 | REAL | S4 | F-003 name overstates; body strong |
| fragmented_source_remuxes… | 352-415 | REAL | — | frag→progressive proven; cts offsets compared |
| edit_list_source_re_anchors… | 420-477 | REAL | — | origin=0 + cts preserved |
| remux_refuses_codecs_outside… | 485-507 | REAL | — | exact code + stage asserted |
| remux_refuses_a_config_naming… | 511-531 | REAL | — | exact code |
| submit_refuses_an_empty_remux | 535-546 | REAL | — | sync refusal at submit |
| ScriptedDemuxer | 561-594 | REAL | — | mock of the SOURCE demuxer (legitimate — muxer under test is real) |
| tick_of / scripted_tracks / sample entries | 598-643 | REAL | — | structurally real sample entries (width/height, channels/rate fields) |
| eventually | 646-654 | REAL | — | 10 s bounded poll |
| mid_remux_cancel_leaves… | 656-744 | REAL | — | strongest test: whole-packet prefix, tables=stats, real re-demux |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| progressive_source_remuxes_and_redemuxes_packet_identical | YES | byte-exact payload compare per sample per track; count/flag/duration exact |
| fragmented_source_remuxes_into_a_progressive_output | YES | `!round.is_fragmented()` + payload/sync/cts exact |
| edit_list_source_re_anchors_the_media_timeline | YES | origin==0 + payload + cts |
| remux_refuses_codecs_outside_the_stream_copy_set | YES | exact code + PipelineStage::Transcode |
| remux_refuses_a_config_naming_a_missing_track | YES | exact code |
| submit_refuses_an_empty_remux | YES | exact code, synchronous |
| mid_remux_cancel_leaves_an_honest_playable_prefix | YES | prefix bounds, whole-packet invariant, tables==stats, real Mp4Demuxer accept |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "copies a committed H.264/AAC fixture into a fresh MP4" | mp4_remux.rs:1-3 | REAL | ran: 7/7 pass, `&output[4..8] == b"ftyp"` + re-demux |
| "verified by OUR own demuxer" | mp4_remux.rs:3 | REAL (and honest about the limitation) | F-001: no external oracle exists |
| "a mid-remux cancel leaves a playable honest prefix" | mp4_remux.rs:8-10 | REAL | cancel test asserts structural validity + tables=stats (ran, ok) |
| Matrix `playback-record` "MP4 remux (H.264/AAC stream copy)" | feature-matrix.toml:1630-1641, status not_started | REAL-UNPROVEN (code+tests exist; not wired) | rg: no app caller |

## 7. Dead / duplicated / unreachable code
Test-local duplication of the record-suite plumbing (SharedCursor/MemStream "reproduced test-locally" — acknowledged at L34-37, integration suites can't share code). The `Remuxer`/`mp4mux` production code is unreachable from shipped entry points (F-002).

## 8. Dependencies & flags
Test uses demux (Mp4Demuxer), input (FileAccess), plugin_api, media_types, testkit, transcode — all workspace crates; no features. Runs in the default `cargo test -p transcode` lane (baseline cargo-test-workspace.txt:6405-6414, all ok).

## 9. Open questions for the next agent
- Would an ffprobe differential on the remuxed output pass (F-001)? The muxer's own unit tests (mp4mux.rs) may already cover box layout, but no external check exists anywhere for the composed Remuxer path.
- When ui-dialog-convert (7.E3) lands, re-check that the honest-prefix cancel contract survives a real file sink (SharedCursor tolerates seek-past-end via resize; a real file needs the same).
