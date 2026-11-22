# Wave 05 SUMMARY

Agent logs: 6 — W05-A01.log.md, W05-A02.log.md, W05-A03.log.md, W05-A04.log.md, W05-A05.log.md, W05-A06.log.md

## Severity counts (from audit files)
S0: 0 | S1: 3 | S2: 23 | S3: 44 | S4: 61

## Worst 10 findings (by severity, then file)
- [S1] audit/files/crates/demux/src/mkv/mod.rs.audit.md: F-001 [S1] Block header straddling the 4 MiB window boundary is misclassified as stream truncation → valid files abort playback
- [S1] audit/files/crates/demux/src/mp4/mod.rs.audit.md: F-001 [S1] Track-ID aliasing: `start()` list and minted packets use different ID spaces when a chapter trak precedes an exposed trak
- [S1] audit/files/crates/demux/src/mp4/mod.rs.audit.md: F-002 [S1] Chunk expansion is O(chunks × stsc-entries): one 16 MiB `moov` ⇒ up to ~10¹² inner iterations
- [S2] audit/files/crates/demux/src/mkv/attachments.rs.audit.md: F-001 [S2] A corrupt attachment record aborts the entire file open — non-essential metadata has whole-file blast radius
- [S2] audit/files/crates/demux/src/mkv/cues.rs.audit.md: F-001 [S2] — Promised degradation to SlowLinear does not exist: a cue-bomb refuses the whole file
- [S2] audit/files/crates/demux/src/mkv/cues.rs.audit.md: F-002 [S2] — Cue index present but no landing point ⇒ hard seek error; empty `Cues` advertises Fast while every seek fails
- [S2] audit/files/crates/demux/src/mkv/cues.rs.audit.md: F-003 [S2] — `CueTime` u64→i64 wrap: hostile values silently become negative ticks
- [S2] audit/files/crates/demux/src/mkv/editions.rs.audit.md: F-001 [S2] Module is dead code in the shipped product while its docs claim it is the tags/UI join seam
- [S2] audit/files/crates/demux/src/mkv/header.rs.audit.md: F-001 [S2] One bad `CodecID` fails the entire file with a false "no tracks" error — containment doctrine broken
- [S2] audit/files/crates/demux/src/mkv/mod.rs.audit.md: F-002 [S2] `chapter_targets` divides by `timestamp_scale` with no zero guard — division-by-zero panic on a malformed file

Gate: PASS (0 fabrications after manual adjudication). MP4 demuxer REAL with S1 track-ID aliasing; 'voe' language bug root-caused at mp4/mod.rs:536.
