# AUDIT: fuzz/corpus/fuzz_demux_mpd/hostile_timeline.mpd
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 419 / 7b0a3ec9b7155279 |
| File verdict | REAL (hand-authored hostile seed) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Hand-authored hostile DASH MPD seed (fuzz/README.md:106-107 class). Content
verified by read: one-line XML with three deliberate attacks —
`type="dynamic"` + `minimumUpdatePeriod="PT2S"` (live-profile arithmetic), a
`SegmentTimeline` whose second `<S>` carries `r="-1"` (the
repeat-to-end-of-period sentinel: unbounded segment multiplication if the
parser fails to treat r=-1 specially) on top of `d="450000"`, and a
`$Time000000000$` template with a 12-digit zero-padded macro. The seed is
exactly the kind of input that catches unbounded-allocation bugs in segment
enumeration.
## 2. Verified
— Read (1 line, 419 B); sha256 7b0a3ec9b7155279….
— Consumer: fuzz/Cargo.toml bin fuzz_demux_mpd; the target is
streaming::dash::fuzz::fuzz_demux_mpd (fuzz/README.md:88); corpus dir has 7
seeds (ffmpeg×2, hand×3, hostile×2 — ls).
## 3. Findings
None.
