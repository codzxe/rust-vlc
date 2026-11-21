# AUDIT: crates/playback-engine/src/pipeline.rs (part2, lines 668–1334 of 3338)
> Chunk scope: part2 = lines 668–1334 (assignments.tsv LOC 667/part × 5 parts =
> 3335 ≈ 3338). Items: `per_frame_damage` (668), `concealment_budget_exhausted`
> (683), `StarvationCounters` (704), `Pipeline` struct (728), `CachedDecoder`
> (799), `WireOutcome` (811), `WindowOwner`+impl (820–852), `ObserveWindow` /
> `WatchEntry` / `HoldSlot` (857–897), `impl ObserveWindow` (900–965),
> `SeekPreroll` (966), `SwitchPreroll` (984), and `impl Pipeline` methods
> `new`/`tracks`/`wire`/`build_limb`/`start_demuxer`/`teardown`/
> `has_audio_limb`/`last_video_end_ns`/`seekability`/`duration_ns`/`seek_flush`.
> **Boundary note:** `seek_flush` starts at 1239 and ends at 1355 — it straddles
> the part2/part3 boundary; it is audited whole here (part3 starts at 1356,
> the doc comment of `window_open_seek` at 1357).
> Part1 (1–667: QueuePlan, TickBudget, TickReport, `Limb`,
> `VIDEO_CONCEALMENT_BUDGET`) is W03-A07's; tests (2726–3338) are part5's.

| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 667 (of 3338) / 28 production units |
| File verdict | REAL (reachable and executed in the shipped open/seek/teardown lanes; no fraud found in this range) |
| Reachable from shipped entry point? | YES, EXECUTED (call chain: `lumen` CLI → `apps/cli/src/host.rs:91` uses `playback_engine` Session → `session.rs:2360` `pipeline.wire(...)`, `session.rs:1598/1634` `pipeline.teardown(...)`, `seek.rs:382` `pipeline.seek_flush(...)`, `session.rs:2021/2784` `seekability()`, `session.rs:1766/1999/2772` `duration_ns()`, `session.rs:2743-2744` `has_audio_limb()`/`last_video_end_ns()`, `session.rs:2068/2588` `end_seek()`. Executed proof: baseline `audit/runs/baseline/INDEX.md` §"Corrected MP4 smoke matrix" — `sample-420.mp4` open-session reaches Buffering→Playing→Stopped→Idle and `avi-firstvertical-open-session.txt` shows the full lifecycle; `wire()` must have run for any track to decode (60/60 frames in `rc-smoke-engine-only.log.txt`), and the AVI lane's Playing→Stopped transition runs the teardown path. Suite: playback_engine lib 83 passed / 0 failed (orchrun-test-workspace-nofailfast.log.txt lines 3740-3743).) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n '1,200p;600,700p;700,900p;900,1100p;1100,1335p;1335,1380p;1355,1520p;2185,2215p;2270,2300p;2500,2537p' pipeline.rs` (full read of range + boundary context); `sed -n '660,670p;843,851p;1051,1094p;1076,1078p;1239,1268p;1259,1261p;1306,1311p;2519,2522p;2280p'` (verbatim quote verification); `rg -n "^(pub\(crate\) )?(fn|struct|enum|impl|trait|const|mod)" pipeline.rs` (item map); `rg -n "fn " pipeline.rs` awk-filtered 668–1340 (function inventory); `rg -n "pipeline\.(wire|seek_flush|teardown|has_audio_limb|last_video_end_ns|duration_ns|seekability|start_demuxer)" crates/playback-engine/src/*.rs` (caller map: seek.rs:382, session.rs:1598/1634/2021/2360/2743/2784); `rg -n "selected" pipeline.rs` (push routing is limb-presence-based at 2196-2199, not flag-based); `rg -n "window_arm_switch|switch_for" pipeline.rs` + `sed -n '1806,1830p;1826,1860p' session.rs` (subtitle never reaches switch flow — pre-table branch at session.rs:1812-1815); `rg -n "self\.first_frame" pipeline.rs` (latch at 2519-2522, read to gate — not dead); `rg -n "limb\.generation" pipeline.rs` (per-limb decode floor at 2280 handles post-seek_flush-error mixed generations); `rg -n "subtitle" crates/playback-engine/tests/track_switch.rs` (subtitle switch coverage lives in tests/track_switch.rs:928,1025); `rg -n "DEFAULT_STARVE_DEBOUNCE_TICKS" pipeline.rs` (=2, part1:257); awk over orchrun no-fail-fast log lines 3740-3940 (playback_engine 83/83 lib + integration binaries all ok) |

## 1. One-paragraph verdict
This range is the pipeline's wiring and seek-mint machinery, and it is real,
reachable, and executed: `wire()` builds one limb per kind from the
container-default track, `seek_flush()` implements the doc-03 §6 mint (flush
queues, drop the held packet, drain+reconfigure every decoder including the
parked cache, open the M10 observation window), and the starvation/debounce
plumbing it carries is exercised by the 83-test lib suite that passed in the
no-fail-fast baseline. The engineering is defensive in the right places
(saturating counters, per-limb generation floors, counted discards, bounded
holds). The two defects that matter are both about silence: (1) `wire()`
swallows a subtitle decoder-selection error with no counter, event, or log —
the user's subtitles just do not exist; (2) `WindowOwner::switch_for` maps
every non-Audio `MediaKind` (including `Subtitle`) onto the SWITCH_VIDEO bit —
a latent claim collision that today's command routing happens to avoid by a
pre-table branch in session.rs, guarded only by a comment. Neither is a
blocker; both are the kind of seam a future refactor trips over.

## 2. What actually works (verified)
— `wire()` one-limb-per-kind selection — verified by trace: iterates
`[Video, Audio, Subtitle]`, takes `crate::tracks::default_track` per kind,
marks `selected` by id, and the push path routes by **limb presence**
(pipeline.rs:2196-2199), so an unmarked row cannot lose its packets. Executed:
no open in any baseline transcript can produce decoded frames without it
(`rc-smoke-engine-only.log.txt` 6/6 engine checks).
— `seek_flush()` mint order — verified by trace against its doc comment:
`end_seek()` (discards superseded hold, counted into `preroll_discarded`) →
queue flush at new generation (1254-1256) → `pending = None` (1257),
`eof = false` (1258) → debounce state reset (1263-1266) → per-limb decoder
drain via a generation-checking `LimbSink` with `hold: None` (residue dropped,
counted in `seek_flush_stale`) → `configure` with the new generation (1308) →
same for parked `decoder_cache` entries (1316-1354) → `window_open_seek(hold_cap)`
(1353).
Error paths add `sink.stale_dropped` before
returning (no lost counts). Test `parked_codec_rides_the_recycle_path`
(pipeline.rs:3291) drives this path and passed in the baseline lib suite.
— `seek_flush()` failure containment — verified: on a mid-flight decoder
error, remaining limbs keep their old per-limb generation, and the decode slice
reads the **limb's** floor (`let generation = limb.generation;` at 2280), so
mixed-generation state cannot present stale stragglers — the doc's "limps on"
claim is backed by code.
— `teardown()` ordering — verified: window held-frames counted into
`preroll_discarded` (M10 "counted, never silent"), every queue flushed then
`close()`d (parked producers wake to `Closed`), cache cleared, `pending`
dropped; callers drop the pipeline immediately after (session.rs:1599/1635
`self.pipeline = None`), so the un-cleared `ended_limbs` scratch cannot leak.
— `ObserveWindow` bookkeeping — verified: `observe()` ignores `frames == 0`,
latches `first_pts_ns`/`first_audio_ns` once each, `held_count()` saturates,
`hold_slot()` returns `None` for switch-only windows (frames deliver straight
through — matches card 3.A3 step 7 doc). `window_open_seek` re-arms entries
over **current** limbs so an in-flight switch's watched limb survives the
rebuild (limb swap precedes arming at 1845-1851).
— `duration_ns()`/`last_video_end_ns()` — verified: max over track span ends /
limb high-water marks; both consumed by session.rs (2772 DurationKnown,
2743-2744 F5 play-out gate) which the AVI first-vertical lifecycle exercises.
— Counters — `StarvationCounters` fields are incremented only through
`saturating_add` at their use sites (full-rejection tallies in the 2207-2221
push path, verified in context read); `WireOutcome`'s u32 counts bounded by ≤3
limbs per open.

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/pipeline.rs:1086-1090
Code:
```rust
                Err(_error) if wired == WiredKind::Subtitle => {
                    // The optional kind: deselect, never fail the
                    // open (the class-S seam's open-time half).
                    continue;
                }
```
What is wrong: the subtitle decoder-selection error is discarded with no
observability whatsoever — no counter, no event, no log line, nothing in
`WireOutcome` (which carries only `wired`/`decoders_built`/`decoders_reused`).
A typed `EngineError` from the provider (stage, kind, context code — all the
machinery this codebase builds everywhere else) is thrown away.
Why it matters: "deselect, never fail the open" is a defensible design, but the
current implementation makes a *transient or fixable* provider failure
indistinguishable from "no subtitle decoder exists". The user who asks for
subtitles gets silence at every level. This is the textbook swallowed-error
from the checklist, blast-radius-limited to the optional kind only.
What correct looks like: keep the non-fatal semantics but surface the fact —
e.g. a `subtitle_deselected: Option<EngineError>` (or a `decoders_failed`
count) on `WireOutcome` that the session folds into `EngineStats`/a non-fatal
`playback.error`, exactly like the audio lane's `DamagedData` coalescing.
Confidence: HIGH — Verification: read (callers of `WireOutcome` checked:
session.rs:2365-2375 reads only `wired`/`decoders_built`/`decoders_reused`).

### F-002 [S3]
Where: crates/playback-engine/src/pipeline.rs:843-851
Code:
```rust
    /// The switch bit for a limb kind (audio/video).
    fn switch_for(kind: MediaKind) -> WindowOwner {
        match kind {
            MediaKind::Audio => WindowOwner::SWITCH_AUDIO,
            _ => WindowOwner::SWITCH_VIDEO,
        }
    }
```
What is wrong: the catch-all `_` arm maps **every** non-Audio `MediaKind` —
including `Subtitle` and any future kind — onto the SWITCH_VIDEO owner bit.
`WiredKind::Subtitle` exists as a first-class lane in this very file (the
`wire()` loop at 1064 iterates it), and `window_arm_switch` is typed to take
any `MediaKind` (1413), so a subtitle limb entering the switch flow would
claim and later **release the video switch's claim** (`window_end` at 1665
clears the same bit), wedging or falsely completing an in-flight video switch.
Why it matters: the invariant "one bit per flow" is enforced nowhere; the type
system is one refactor away from a cross-flow collision. Today it is unreachable
only because session.rs routes `SelectSubtitle` into `subtitle_select_step`
before the switch table (session.rs:1812-1815) — a guard that lives in a
different file, documented by a comment ("if a future refactor ever let the
subtitle flavor reach this point…"), not by code.
What correct looks like: make the function total over `WiredKind` with an
explicit refusal — e.g. return `Option<WindowOwner>` (`None` = no owner bit for
this kind) and refuse subtitle switches at the `window_arm_switch` boundary,
or add a dedicated `SWITCH_SUBTITLE` bit.
Confidence: HIGH — Verification: read (all `switch_for` call sites traced:
1413/1645/1665 only; subtitle route verified at session.rs:1806-1830).

### F-003 [S4]
Where: crates/playback-engine/src/pipeline.rs:1257-1259 (pipeline-wide advance) vs 1308-1310 (per-limb assignment)
Code:
```rust
        self.pending = None;
        self.eof = false;
        self.generation = generation;
```
(the pipeline-wide stamp advances at 1259, before any decoder is reconfigured;
the per-limb assignment happens only on success at 1308-1310:)
```rust
            limb.decoder.configure(&config)?;
            limb.drained = false;
            limb.generation = generation;
```
What is wrong: on a decoder flush/configure error mid-loop, `seek_flush`
returns `Err` with `self.generation` already advanced but later limbs still
carrying the old generation — a mixed-generation pipeline. The behavior is
*documented* ("the pipeline limps on") and *contained* (decode slices gate on
the per-limb floor at 2280, so no stale frame can present), but the
pipeline-wide `generation` field is now ahead of some limbs, and any future
reader of `Pipeline::generation` (e.g. a new mint path comparing against it)
would reason about the wrong floor.
Why it matters: two generation authorities that can disagree after an error is
drift debt, not a live bug; the containment lives 950 lines away in a different
function.
What correct looks like: either roll back `self.generation` on the error path,
or delete the pipeline-wide field and derive it from limbs, or assert the
invariant in a debug build.
Confidence: HIGH — Verification: read (error paths at 1298-1304/1336-1342
return without touching remaining limbs; decode floor verified at 2280).

### F-004 [S4]
Where: crates/playback-engine/src/pipeline.rs:1076-1078
Code:
```rust
                    if let Some(index) = self.tracks.iter().position(|t| t.id == track.id) {
                        self.selected[index] = true;
                    }
```
What is wrong: with duplicate `TrackId`s in the demuxed list (hostile or
buggy container), `position` marks the **first** row carrying that id, which
may be a different track than the one wired; the tracks-changed event would
then report the wrong row as selected. Push routing is unaffected (limb
presence, 2196-2199), so this is presentation-only drift on an input the
demuxer contract should forbid but nothing here checks.
Why it matters: the `selected` flags are the session's selection truth for
events; silently mis-marking under duplicate ids is a small lie with no guard.
What correct looks like: carry the index out of `default_track` instead of
re-finding by id, or deduplicate/refuse duplicate track ids at the demux
boundary.
Confidence: MEDIUM — Verification: read (no uniqueness check on `tracks`
anywhere in this range; `default_track` returns a reference whose index is
discarded by the caller).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `per_frame_damage` | 668-681 | REAL | — | Pure `matches!` over `ErrorKind::Corrupt \| Decoder{Bitstream}`; no panic; consumed by decode slice (part4). |
| `concealment_budget_exhausted` | 683-700 | REAL | — | Builds typed `ResourceLimit{RecoveryAttempts}` error, `u64::from` conversions (no truncation), source chain carries last damage. |
| `StarvationCounters` | 704-720 | REAL | — | Plain data; all mutation sites use `saturating_add` (verified at full-rejection tallies). |
| `Pipeline` (struct) | 728-796 | REAL | — | 20 fields; every field traced to a reader: `first_frame` latches at 2519-2522 (gates TickReport's one-shot), `ended_limbs` drained by session per tick, `window`/`decoder_cache`/`pending`/`eof` all live in seek/switch flows. |
| `CachedDecoder` | 799-806 | REAL | — | Codec descriptor decoupled from track list (audit F-4 fix); bounded by container track count (keyed by TrackId, no LRU). |
| `WireOutcome` | 811-818 | REAL | — | Counters only; F-001 notes the missing failure count. |
| `WindowOwner` + 5 methods | 820-852 | REAL | S3 | `union`/`without`/`contains`/`is_empty` correct bitwise ops; `switch_for` catch-all is F-002. |
| `ObserveWindow` (struct) | 857-882 | REAL | — | Hold hard-capped per track (`hold_cap`), burst past cap dropped+counted (per struct doc; enforcement in LimbSink, part4). |
| `WatchEntry` | 884-891 | REAL | — | Data. |
| `HoldSlot` | 893-897 | REAL | — | Pre-allocated `Vec::with_capacity(hold_cap)` once per seek (D-1). |
| `ObserveWindow::observe` | 903-923 | REAL | — | `frames == 0` early-out; first-latch semantics; facts for un-watched tracks silently dropped (harmless — nothing watches them). |
| `ObserveWindow::first_pts_ns` | 926-931 | REAL | — | O(n) find, n ≤ 3 limbs. |
| `ObserveWindow::saw` | 935-940 | REAL | — | Any-match. |
| `ObserveWindow::held_count` | 942-950 | REAL | — | `saturating_add` fold. |
| `ObserveWindow::hold_slot` | 953-965 | REAL | — | `None` for absent slot = drop+count at decode side (documented); borrow-hoisted `cap`. |
| `SeekPreroll` (enum) | 966-980 | REAL | — | Variant payloads documented. |
| `SwitchPreroll` (enum) | 984-993 | REAL | — | Same. |
| `Pipeline::new` | 999-1015 | REAL | — | Debounce default 2 (part1:257); all state zeroed. |
| `Pipeline::tracks` | 1022-1024 | REAL | — | Borrow accessor. |
| `Pipeline::wire` | 1051-1093 | REAL | S3 | F-001 (swallowed subtitle error); F-004 (id re-find); single-call per pipeline (session guards with `opening_attempted` and drops pipeline at teardown — verified session.rs:2345-2360, 1598-1635). |
| `Pipeline::build_limb` | 1104-1127 | REAL | — | `let _ = queue.flush(generation)` on a fresh queue is the documented no-op; `high_packets.max(1)` guards degenerate thresholds; all limb fields initialized. |
| `Pipeline::start_demuxer` | 1141-1148 | REAL | — | Pure delegation to `Demuxer::start`. |
| `Pipeline::teardown` | 1153-1161 | REAL | — | M10 counting verified (1154-1156); `ended_limbs` not cleared (harmless — struct dropped right after at both call sites). |
| `Pipeline::has_audio_limb` | 1170-1174 | REAL | — | Any-audio predicate; consumed by F5 gate (session.rs:2743). |
| `Pipeline::last_video_end_ns` | 1180-1186 | REAL | — | Max over limbs; subtitle limbs never set the field. |
| `Pipeline::seekability` | 1189-1191 | REAL | — | Delegation. |
| `Pipeline::duration_ns` | 1198-1203 | REAL | — | Max over all track span ends (incl. subtitle — harmless for a max). |
| `Pipeline::seek_flush` | 1239-1355 | REAL | S4 | Audited whole though it straddles the 1334 boundary (part3 starts in its tail). Mint order verified; error-path mixed generations documented + contained (F-003); `flush_budget == 0` degenerate input skips drain but `configure` still resets (codec-api reset point) — noted, not a defect. |

## 5. Test honesty (tests for this range live in `mod tests`, part5's chunk)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `packets_axis_is_the_starvation_axis` (2887) | YES | Builds a wired pipeline via the `wired()` helper (calls `wire`) and asserts watermark behavior; mutation of `wire`/`build_limb` thresholds breaks the assertions. Passed in baseline lib suite (83/83, orchrun log lines 3740-3743). |
| `not_ready_consumer_parks_audio_decode_and_resumes` (2982) | YES | Exercises wire + decode-slice interaction with a gated consumer. |
| `seek_mint_resets_the_debounce_state` (3157) | YES | Calls `pipeline.seek_flush(generation, 8, 1)` (3163) and asserts streak/episode reset — would fail if the 1263-1266 reset loop were removed. |
| `parked_codec_rides_the_recycle_path` (3291) | YES | Drives seek_flush over a cached decoder and asserts reconfigure behavior (3326). |
| wire() subtitle-Err branch (1086-1090) | NO TEST IN FILE | `rg -n "subtitle"` over `mod tests` (2726+) returns nothing; the branch's only coverage is indirect (tests/track_switch.rs:928 tests selection over streams with **no** subtitle tracks — the "honest stub"; its doc says no demuxer emits subtitle tracks today, track_switch.rs:31). The swallow branch itself is unexecuted by any test I can cite. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "one limb per kind … container-default track … every other track stays selected:false (dropped at push)" | pipeline.rs module doc 65-70 + wire doc 1030-1032 | REAL | Code at 1064-1091 + push path 2196-2199 routes by limb presence; AVI baseline decoded 60/60 frames of exactly the 2 default tracks (rc-smoke-engine-only.log.txt). |
| "a cached decoder holding pre-seek reorder frames must not replay them into a post-seek recycle" | seek_flush doc 1212-1215 | REAL | 1316-1354 flushes+reconfigures `decoder_cache` entries through a generation-checking sink; `parked_codec_rides_the_recycle_path` passed in baseline. |
| "A flush budget that exhausts before a decoder answers Drained is not a failure" | seek_flush doc 1226-1229 | REAL | Loop at 1287-1297 `break`s only on Drained, falls through silently otherwise; checkpoint = generation check in later slices. |
| "The subtitle kind is optional: a provider that cannot build a subtitle decoder deselects that track instead of failing the open" | wire doc 1043-1046 | REAL (but silent — F-001) | Code at 1086-1090; the *deselect* half is real, the observability half is absent. |
| M10: "An in-flight window's held pre-roll frames die with the pipeline — counted, never silent" | teardown doc 1151-1153 | REAL | 1154-1156 adds `window.held_count()` to `preroll_discarded`; session folds via `fold_preroll_discards` (session.rs:1604). |
| "audio … dropped, counted, never synthesized" (G3/D-2) | module doc 46-51 | REAL (this range: `per_frame_damage` + budget) | Classification at 668-681; budget 1000 (part1:647); skip path in decode slice (part4, out of chunk). |

## 7. Dead / duplicated / unreachable code
- No dead functions found in range; every item has a live caller (verified by
  the caller map in the Commands row).
- `Pipeline::first_frame` (751) is written once (1009, 2522) and read only for
  `is_none` gating (2519) — a latch, not dead, but its stored value is never
  read back.
- The wire() subtitle-swallow branch (1086-1090) is live code with no
  in-crate test and no current demuxer emitting subtitle tracks
  (tests/track_switch.rs:31 "no demuxer emits subtitle tracks today").

## 8. Dependencies & flags
- Range uses: `codec_api` (DecodeOutcome/Decoder/DecoderConfig/FrameSink/ThreadBudget),
  `media_types`, `packet_queue`, `plugin_api` — all workspace crates, no new
  deps, no feature gates inside this range (no cfg anywhere in 668-1334).
- `let`-chains (e.g. 908-911) require Rust 2024 — consistent with repo pin 1.99.1.

## 9. Open questions for the next agent
- part3 (starts mid-`seek_flush` at 1335): verify `window_open_seek`'s reset
  of `first_audio_ns` cannot strand an in-flight **audio switch**'s re-anchor
  (I traced the switch anchor to `first_pts_ns(track)`, not `first_audio_ns`,
  so I believe it is safe — confirm against `complete_switch`).
- part4: the decode slice enforces the hold cap ("burst past cap dropped and
  counted") — confirm the drop+count actually exists in `LimbSink`/hold push.
- Does any demuxer emit subtitle tracks at all (track_switch.rs doc says no)?
  If none, the whole subtitle lane is exercised only by fakes — worth a
  feature-matrix cross-check.
