# Format record — MPEG-DASH (ISO/IEC 23009-1 MPD + DASH-IF behavior, self-build)

Status: **partial (library stack; evidence Linux-only)** — the MPD
model and parse core (6.C1a), the SegmentTemplate/SegmentList/
SegmentBase resolution with typed plan budgets (6.C1b), the MPD fuzz
target (6.C1c), and the buffer-based ABR + segment scheduling with
live-edge join (6.C2) are landed and tested in `crates/playlists` +
`crates/streaming`. The matrix row `network-dash` stays **partial** on
every platform per the 6.C2 flip: the reload loop, time-shift
trimming, multi-Period stitching, CENC decrypt, the mockhttp socket
e2e, and the DASH-IF vectors are pending (matrix `notes`, quoted in
§ Honest gaps below). Evidence is Linux-only like every streaming lane
(R4: no win/macos runs yet). This record was authored by the 8.G1 docs
sweep (2026-11-10) to close the record gap: the DASH lane had landed
with its evidence living only in the matrix row and the wave-6 worklog
(docs/audit/docs-sweep-20261110.md finding F-5). Front-matter keys per
doc 04 §6:

- id = `network-dash` (`docs/feature-matrix.toml`)
- status = partial — matrix and this record agree; nothing here upgrades it
- phase = 6 (waves 6.C1–6.C2)
- fuzz_target = `fuzz/fuzz_targets/fuzz_demux_mpd.rs` (6.C1c) + the
  in-crate deterministic mutation sweep `crates/streaming/src/dash/fuzz.rs`
- crates = [] — zero external deps admitted for the DASH lane
  (`dash-mpd-core` 0.1.0 was used as a **parse candidate-oracle only**,
  never linked; matrix note)
- self_built = [`crates/playlists/src/mpd.rs` (MPD model),
  `crates/streaming/src/dash/{resolve,sched,abr,fuzz}.rs`]
- last_audit = 2026-11-10 (8.G1 docs sweep — record authored from the
  matrix row's recorded evidence; no code was re-run or re-claimed)

## What landed when (the stack this record covers)

| Card | Landed | Where / evidence |
|---|---|---|
| 6.C1a | MPD model: parse core, transport-free by policy | `crates/playlists/src/mpd.rs`; the transport half of the decision (http(s)-only, URLs verbatim, `ContentProtection` recorded for the session's typed encrypted refusal) is recorded in `docs/formats/network-http.md` § "DASH transport" |
| 6.C1b | SegmentTemplate `$Identifier$` substitution incl. printf format tags; SegmentTimeline `t+n×d` expansion incl. negative `@r`; SegmentList/SegmentBase; BaseURL folding; typed plan budgets | `crates/streaming/src/dash/resolve.rs` (12 `#[test]` fns on disk today) |
| 6.C1c | `fuzz_demux_mpd` cargo-fuzz target + in-crate deterministic mutation sweep | `fuzz/fuzz_targets/fuzz_demux_mpd.rs` + `crates/streaming/src/dash/fuzz.rs` (4 `#[test]` fns) |
| 6.C2 | Buffer-based ABR: EWMA throughput, up/down margin bands, dwell, stall-forced down-steps over representation ladders; segment scheduling; ISO 8601 `availabilityStartTime` live-edge join (static + dynamic MPDs) | `crates/streaming/src/dash/{abr,sched}.rs` (4 + 7 `#[test]` fns) |

Test counts: **27 dash `#[test]` fns**, re-countable with
`rg -c '#\[test\]' crates/streaming/src/dash/*.rs` (12+4+4+7 —
re-counted 2026-11-10, unchanged since authoring). The crate-wide
figure the matrix row records — 191 lib tests (2 ignored) — is the
state at the 6.C2 flip commit `93b7614`, where
`crates/streaming/src` held 193 `#[test]` fns; the crate has since
grown with later waves: 206 `#[test]` fns as of 2026-11-10
(post-review correction, W24-R2 P3-6 — the record originally
prefixed 191 with "on disk at authoring", which was already stale by
13 tests).

## Working subset (what a user can actually play)

A static or dynamic MPD whose representations are fMP4 segments behind
plain http(s): the session resolves the manifest (template, timeline,
list, or base URLs), schedules segments against the buffer, and steps
the representation ladder up/down on the EWMA throughput estimate with
hysteresis (margin bands + dwell) and stall-forced down-steps. Live
MPDs join at the availability-time-derived live edge.

## Honest gaps (quoted from the matrix row `notes` — the recorded 6.C2 state)

> Partial per 6.C2 flip: MPD parse, template/list resolution, MPD
> fuzz, ABR selection, static+dynamic scheduling with live
> availability join landed; MPD reload loop (minimumUpdatePeriod),
> timeShiftBufferDepth trimming, multi-Period stitching, CENC decrypt,
> mockhttp socket e2e + DASH-IF vectors pending.

Additionally recorded there, unchanged by this record:

- fMP4 corpus classes 8–10 are a prerequisite for end-to-end media
  evidence (feasibility §1.2 — the DASH lane exercises the manifest/
  scheduling layer, not yet full media decode through fragmented MP4).
- DASH-IF conformance vectors are the reference suite owed (risk R-37).
- The ABR algorithm **class** (the HLS-proven watermark/hysteresis
  approach now in `abr.rs` vs a BBA/BOLA-class rate model) is a
  recorded `[PROPOSAL]` Lead decision — this record does not settle it.

## Why the record lived only in the matrix until now

Waves 6.C1/6.C2 recorded their evidence in the matrix row and the
wave-6 worklog; the sibling lanes (HLS → `streaming-hls.md`, HTTP →
`network-http.md`) got records because their cards carried them
explicitly. The 8.G1 completeness sweep flagged the asymmetry
(F-5) and closed it by authoring this record from the recorded
evidence — no new claims, no re-verification of code, statuses
untouched. Every sentence above traces to the matrix row, the source
files named, or both.
