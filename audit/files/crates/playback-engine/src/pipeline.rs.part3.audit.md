# AUDIT: crates/playback-engine/src/pipeline.rs (part3, lines 1335–2001 of 3339)
> Chunk scope: part3 = lines 1335–2001 (assignments.tsv LOC: part1 667 /
> part2 667 / **part3 667** / part4 667 / part5 670; 667·2+1=1335 …
> 1335+667−1=2001). Covers: the tail of `seek_flush` (its body starts at
> 1239 in part2 — audited whole here because a half-function audit is
> worthless; the head's findings belong to A08's part2 file), the whole
> observation-window family (`window_open_seek`, `window_arm_switch`,
> `window_end`, `window_live`), the seek-completion/terminal family
> (`take_seek_flush_stale`, `take_preroll_discarded`, `demuxer_reanchor`,
> `seek_preroll`, `release_preroll_hold`, `end_seek`), the switch family
> (`switch_preroll`, `end_switch`, `obtain_decoder`, `switch_limbs`,
> `take_ended_limbs`), `can_never_deliver`, `paces`, and `tick`'s head
> (1904–2001; the tick body continues into A10's part4 — boundary noted
> per-function). Tests live in `mod tests` (2727–3338, A06's part5) and
> `tests/track_switch.rs` (separate file); their honesty for THIS range is
> assessed in §5.

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 667 (of 3339) / 19 production units |
| File verdict | REAL (seek lane executed in shipped open lane; switch family UNREACHABLE from every shipped host — see F-003) |
| Reachable from shipped entry point? | SPLIT. Seek family YES, EXECUTED-ADJACENT: `lumen` CLI `seek` verb (apps/cli/src/parse.rs:203 → run.rs:815 `CliCommand::Seek` → dispatch.rs:131 `Command::PlaybackSeek` → host.rs:1061 `dispatch_seek` → session.rs seek step → seek.rs:382 `pipeline_seek` → pipeline.rs:1239 `seek_flush`; terminal via session.rs:2560 `release_preroll_hold`). The open/play/pump lane that runs `tick` is proven executed by baseline `mp4-smoke-open-session.txt` / `avi-firstvertical-open-session.txt` (Buffering→Playing→Stopped lifecycle impossible without `tick`). Switch family NO: no shipped host issues `tracks.set-*` — CLI verb list (parse.rs:197-214) has no track select; web host REJECTS it (apps/web/src/host.rs:1850-1851 "track switching is card 3.A3"); control-server embeds no engine (apps/control-server/src/dispatch.rs:32-35 NOT_READY for everything non-introspection); desktop-tauri carries no playback_engine usage; apps/desktop is a webview skeleton (exit 86). |
| Findings | S0: 0 S1: 0 S2: 3 S3: 1 S4: 2 |
| Commands run | `sed -n '1225,1334p;1330,1469p;1470,1689p;1690,2009p;2100,2184p;2269,2533p' pipeline.rs` (full read of range + context); `sed -n '1574,1604p;1413,1436p;1512,1533p;1844,1858p'` (quote verification); `rg -n "seek_flush|switch_limbs|release_preroll_hold|switch_preroll|end_seek|end_switch" crates apps` (caller map → session.rs:2068/2102/2278/2305/2559-2588/2676-2723, seek.rs:382-390); `rg -n "SessionCommandKind::" apps/cli/src` (shipped command surface: Play/Pause/Stop/Close only); `sed -n '1840,1935p' apps/web/src/host.rs` + `sed -n '30,60p' apps/control-server/src/dispatch.rs` (host rejection proof); `rg -n '"[a-z-]+" =>' apps/cli/src/parse.rs` (CLI verb list, no track select); `sed -n '1546,1556p' docs/feature-matrix.toml` (playback-track-switch row); `grep -n "track_switch" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (12/12 track_switch tests pass; 83/83 lib tests pass, incl. 8 pipeline::tests); `sed -n '1550,1700p' crates/playback-engine/tests/track_switch.rs` (interleaving tests 8a/8b read); `rg -n "self\.generation = " pipeline.rs` (mint sites: wire 1058, seek_flush 1259, switch_limbs 1849/1853); `rg -n "fn from_payload" crates/packet-queue/src/stamped.rs` + `sed -n '2140,2155p' pipeline.rs` (packet stamping: demux stamps `self.generation` at push, 2147); `sed -n '336,380p' crates/packet-queue/src/queue.rs` (flush returns Discontinuity, stats.flushed counts drops); `rg -n "generation" crates/codecs/src/video/mjpeg/mod.rs` (decoder stamps frames with its CONFIG generation — the release-vs-floor divergence proof) |

## 1. One-paragraph verdict
The seek/switch machinery in this range is the most heavily documented code audited so far, and its happy paths are genuinely implemented and unit-tested (12/12 `track_switch.rs` + 8 `pipeline::tests` pass in the new no-fail-fast baseline, `orchrun-test-workspace-nofailfast.log.txt`), but two generation-accounting seams are wrong, and both hide behind the same root cause: the observation window and the pre-roll hold track facts by **track id across generation mints**, while presentation validity is decided by **generation**. First, `release_preroll_hold` checks held frames against the *pipeline-wide* `self.generation`, but the decode path that captured them checks against the *limb's* floor (`limb.generation`, 2280) — so the mint of any mid-seek track switch retroactively condemns the **unchanged kind's** entire pre-roll hold (video held frames die at an audio switch's terminal and vice versa), directly contradicting the function's own contract that held frames "leave through the same FrameConsumer seam they would have used without the hold". Second, `window_arm_switch` re-uses a stale `WatchEntry` (saw_frame/first_pts recorded under an older generation) when a track is re-selected mid-seek, so `switch_preroll` answers `Ready` "under the switch's generation" without any frame of that generation existing — violating its own doc contract — and `seek_preroll` completes on the same stale fact. Both are currently latent at the shipped level only because **no shipped host can issue a track switch at all** (CLI lacks the verb, the web host rejects it, control-server embeds no engine) — which is itself the third finding: the feature-matrix row `playback-track-switch` claims `implemented` on windows/linux/macos with an evidence string that cites model code and keymap recipes but no run, for a lane no shipped binary can reach. The seek-only paths traced clean apart from a vacuous-set hole (`seek_preroll` returns `NotReady` forever on an empty limb set, contradicting its own never-wedge principle).

## 2. What actually works (verified)
— `tick`'s decode gate and priming block — verified by trace: decode runs in Buffering/Playing, or while any window owner lives (the Paused seek/switch carve-out, 1951); drained computed in Playing/Paused; priming in Buffering/Paused with the `paces()` exemption inlined (1996) matching `paces()` (1895-1900) exactly; episode close on high-watermark-or-parked-producer. Executed by baseline `avi-firstvertical-open-session.txt` (Buffering→Playing→Stopped→Idle lifecycle requires tick's primed/starved/drained signals).
— `seek_flush`'s prepare order — verified by trace: `end_seek()` first (superseded seek's hold discarded+counted, 1253→1615-1624), queues flushed, `pending`/`eof` cleared, generation published once (1259), debounce state reset per limb (1263-1266), decoders flushed-to-Drained bounded by `flush_budget` with stale drops counted before every exit (1294-1300), reconfigured at the new generation (1308). Unit-proven by `seek_mint_resets_the_debounce_state` (pipeline.rs:3157, passing in no-fail-fast baseline line 3795).
— `switch_limbs`' prepare-before-commit — verified by trace: decoder obtained before any retire (1761-1775), a provider failure leaves the old limb untouched (`?` at 1765 precedes all mutation), retire drains the retired decoder through a no-hold/no-latch generation-checking sink bounded by `flush_budget` (1787-1812), the retired decoder is parked by track id with the codec captured at park time (1814-1822, the F-4 recycle fix), pending packet of the retired track dropped (1827-1833). Unit-proven by `decoder_cache_recycles_instances_across_switches` + `rapid_double_switch_emits_one_terminal_for_the_newest_selection` (track_switch.rs, passing).
— `window_open_seek` pre-allocation claim — verified by trace: entries and `Vec::with_capacity(hold_cap)` hold built once per seek (1377-1384), pushes within capacity never allocate (matches `PrerollHoldSlot::push`, 2665-2672).
— `obtain_decoder` cache protocol — verified by trace: `swap_remove` on hit + re-`configure` as reset (1690-1699), provider on miss (1708), `(decoder, reused)` pair feeds `SwitchOutcome`/`WireOutcome` counts; cache is bounded by track count (one parked entry per track; retrieval removes it).
— The seek terminal order in the session — verified by trace (session.rs:2533-2561): clock re-anchored and generation published BEFORE `release_preroll_hold`, so released frames present against the new position — matching the M10 doc on 2553-2558.
— Shipped seek lane exists end-to-end — verified by call chain (§Reachability) + baseline: `lumen open` interactive accepts `seek` (parse.rs:203), and `first-vertical` journey tests exercise seeks through the real binary (journey_seek, no-fail-fast baseline footer item 4 — failing on decode accounting, i.e. the lane runs).

## 3. Findings

### F-001 [S2]
Where: crates/playback-engine/src/pipeline.rs:1574-1584 (release check); mint site 1849; contradicted limb-floor authority at 2274-2280 (part4, cross-ref)
Code:
```rust
        held.sort_by_key(|(_, frame)| stamp_ns(frame.pts()).unwrap_or(i64::MAX));
        let generation = self.generation;
        for (track, frame) in held {
            // Defense in depth: a frame that went stale between its
            // capture and the terminal (a threaded future's race)
            // never presents — dropped and counted, not guessed
            // around (doc 03 §5 checking point 5).
            if frame.is_stale(generation) {
                self.preroll_discarded = self.preroll_discarded.saturating_add(1);
                continue;
            }
```
and the mint that makes it bite (switch_limbs, 1848-1849):
```rust
            // 5. The demux stamp advances with the mint.
            self.generation = generation;
```
What is wrong: The release path validates held frames against the **pipeline-wide** generation, but the decode path that captured them validates against the **limb's** floor — decode_step builds its sink with `generation = limb.generation` (2280) precisely because "a switch mint advances the demux stamp without invalidating the unchanged kind's consumer path — its G-stamped residents stay presentable" (2274-2279). A mid-seek track switch (legal: `SwitchTrack` is table-legal from Buffering/Playing/Paused, state.rs:564-583, the same states a seek is in flight from) mints G2 over the seek's G1. The unchanged kind's limb keeps floor G1 and its decoder keeps the G1 config (switch_limbs touches only the switched kind), so its pre-roll frames are G1-stamped, were captured into the hold **legitimately** (they passed the limb's own checkpoint), and are exactly the content M10 exists to preserve. At the seek terminal, `self.generation` is G2, `G1 < G2` → every held frame of the unchanged kind is discarded as "stale". The comment claims this check exists for "a threaded future's race" — but in the synchronous engine the only way `self.generation` advances past a held frame's stamp mid-seek is the switch mint, i.e. the check fires precisely in the case where the limb floor says the frames are good. The doc contract of this very function (1535-1541: held frames "leave through the same [`FrameConsumer`] seam they would have used without the hold") is broken: without the hold they would have presented; held, they die.
Why it matters: User-visible loss bounded by `hold_cap` per limb (the pre-roll budget, `packets_per_limb`, session.rs:2030-2032): a seek landing right after a track switch skips the unchanged track's first pre-roll frames (an audible audio gap / visible video hitch at the landing), and `complete_seek` re-anchors the clock on `first_audio_ns` — the pts of a frame that was just discarded — so the clock position no longer corresponds to any presented frame. The loss is silently folded into `preroll_discarded` (a counter documented for *superseded or failed* seeks, 1482-1484 — this seek neither failed nor was superseded). The passing test `switch_during_seek_completes_both_flows_under_their_own_generations` (track_switch.rs:1557) cannot see it: it asserts terminal counts, generations and the *last* audio frame's generation, never the presented-frame count of the unchanged kind — the discard happens inside its exact scenario, unasserted.
What correct looks like: The release must validate each held frame against the floor of the limb that captured it — carry the capture-time limb generation (or the frame's own generation vs. the *capturing* floor) in the hold slot and check that; or switch_limbs must, when a seek window is live, re-stamp/re-floor the unchanged limbs at the mint (reconfigure + flush their holds into the new generation) so the two authorities never disagree. Either way, a test must assert the unchanged kind's held pre-roll frames are presented at the terminal after a mid-seek switch.
Confidence: HIGH — Verification: read (mint sites enumerated by `rg "self\.generation = "` — wire/seek_flush/switch_limbs only; decoder frame-stamping verified in crates/codecs/src/video/mjpeg/mod.rs:322/342 — frames carry the *config* generation; packet stamping verified at pipeline.rs:2147; test 8a read line-by-line).

### F-002 [S2]
Where: crates/playback-engine/src/pipeline.rs:1418-1425 (the stale-entry reuse), 1648-1651 (Ready from the stale fact); contract text at 1627-1633
Code:
```rust
                window.owners = window.owners.union(owner);
                if let Some(track) = track {
                    if !window.saw.iter().any(|entry| entry.track == track) {
                        window.saw.push(WatchEntry {
                            track,
                            saw_frame: false,
                            first_pts_ns: None,
                        });
                    }
```
and the consumer of the stale fact (switch_preroll, 1648-1651):
```rust
        if window.saw(selected) {
            return SwitchPreroll::Ready {
                anchor_ns: window.first_pts_ns(selected),
            };
        }
```
and the contract it breaks (1627-1633): "`Ready` once the newly-selected track has delivered ≥ 1 frame **under the switch's generation**".
What is wrong: Watch entries are created by `window_open_seek` (fresh, over current limbs) and by `window_arm_switch` (only if absent) — they are never invalidated when a limb is retired by a switch. Sequence: seek mints G1 and opens a window over limbs {V1, A1} → V1 delivers pre-roll under G1 (entry V1: saw_frame=true, first_pts_ns=X, frames held) → switch V1→V2 mints G2 (V2's entry added fresh) → switch V2→V1 mints G3 (V1's limb re-installed fresh: new queue, reconfigured decoder, `first_frame_pending=true`) — but V1's *window entry* still says saw_frame=true / first_pts=X from G1. `switch_preroll(Video, Some(V1))` hits `window.saw(selected)` → `Ready{anchor_ns: X}` with **zero** frames delivered under G3; `seek_preroll` (1522-1528) iterates current limbs and `window.saw(V1)` is the same stale true, so the seek also completes on a G1 fact. The anchor X references a frame that F-001 guarantees will be discarded at the release (G1 < G3). For an audio switch-back, `complete_switch` re-anchors the *clock* on X (audio switches re-anchor, session.rs:2144-2148) — a clock anchor onto content that never presents.
Why it matters: Both terminals fire on fabricated evidence; the switch "completes" before the new limb has produced anything, and the pre-roll the seek was holding is dropped wholesale (F-001), leaving a skip at the landing whose size is the entire held pre-roll of the re-selected track. The trigger is a user rapidly flipping between two tracks during one seek — exactly the interaction the in-flight-slot supersession rules (session.rs:2189-2205) were written to support, so the flow is supposed to work. No test covers switch-back-during-seek (track_switch.rs cases are single-switch, 8a/8b included), so nothing fails today.
What correct looks like: `window_arm_switch` must reset (not skip) an existing entry for the track it arms — saw_frame=false, first_pts_ns=None — because a switch mint invalidates prior delivery facts for that track; or entries must carry the generation they were recorded under and `saw()`/`first_pts_ns()` must compare it. Add the switch-back-during-seek case to track_switch.rs asserting the terminal fires only after a G3 frame delivers.
Confidence: HIGH — Verification: read (all writes to `window.saw` enumerated: 1366-1374 fresh-build, 1420-1424 push-if-absent, 907-915 observe — no invalidation on retire; switch_preroll/seek_preroll traced; the retire path 1777-1834 confirmed to leave the window untouched).

### F-003 [S2]
Where: docs/feature-matrix.toml:1546-1555 (row `playback-track-switch`); unreachable implementation at crates/playback-engine/src/pipeline.rs:1750-1858 (`switch_limbs`) and its callers
Code (matrix row):
```toml
id = "playback-track-switch"
category = "playback-op"
vlc_name = "Track switching"
description = "Audio/video/subtitle track select, cycle, reverse-cycle, disable"
priority = "P0"
phase_target = 3
inventory_ref = "inventory/options-linux.json#audio-track"
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
evidence = "crates/playback-engine/src/tracks.rs (track model + switch semantics); cycles seeded in keymaps/default.toml (b/v/Alt+V recipes over tracks.list → tracks.set-*)"
```
and the shipped web host's answer to the same command (apps/web/src/host.rs:1850-1851):
```rust
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
```
What is wrong: The row claims `implemented` on windows/linux/macos for a P0 playback op, but the implementation this row leans on (`switch_limbs` and its window/preroll family, this chunk) is reachable only through `SessionCommandKind::SelectVideo/SelectAudio/SelectSubtitle` — and **no shipped entry point issues those commands**: the `lumen` CLI's verb table (apps/cli/src/parse.rs:197-214) has no track-select verb (its `tracks` verb is list-only, help.rs:66); the web host answers every `tracks.set-*` with a not-in-served-set rejection (host.rs:1850-1851); control-server embeds no engine at all (dispatch.rs:32-35: "this control-server build embeds no engine"); desktop-tauri contains no playback_engine usage and its build is UNVERIFIED (baseline INDEX #2); apps/desktop is a webview skeleton that exits 86. The evidence string proves only that a *model* file exists and that UI keymap *recipes* reference the command id — no run, no transcript, nothing executed — so per the audit's own evidence rules the row cannot be REAL, and `implemented` on three platforms is a shipped-product claim the binaries contradict. The engine half itself is real and tested (12/12 `track_switch.rs` passing in `orchrun-test-workspace-nofailfast.log.txt` lines 4225-4239), which makes this a wiring/claim defect, not a stub — but a P0 row whose lane no user can reach is exactly the inflation the audit exists to catch.
Why it matters: The matrix is the project's truth table; `implemented` here fed wave-0's parity accounting (xtask parity-report already reports 0/252, baseline INDEX #9). A user who reads the row and buys the "b/v/Alt+V hotkeys" story cannot switch tracks in any shipped build. It also launders F-001/F-002 into "already shipped" severity: the moment any host wires the command, those defects become live.
What correct looks like: Either the row's desktop statuses become `partial`/`not_started` with an honest evidence string (engine seam implemented + engine-suite executed; no host wiring), or a shipped host actually wires `tracks.set-*` and a baseline run proves it. The web status is already honest (`not_started`).
Confidence: HIGH — Verification: read (CLI verb table, web host rejection, control-server dispatch, desktop manifests all traced; track_switch suite result verified in the no-fail-fast baseline).

### F-004 [S3]
Where: crates/playback-engine/src/pipeline.rs:1519-1521 (empty-limb early return); principle text at 1506-1509
Code:
```rust
        if self.limbs.is_empty() {
            return SeekPreroll::NotReady;
        }
```
and the principle two comments up (1506-1509):
```rust
    /// provably never delivers still completes — the alternative is
    /// a seek that never reports; the drained signal then ends the
    /// media honestly at the landing.
```
What is wrong: The stated principle is that a seek must never wedge — "a landing where every limb provably never delivers still completes". The empty limb set is the vacuous case of "every limb delivered" (and also of "every limb provably never delivers"), yet the explicit early return makes it `NotReady` forever: no limb can ever deliver, so the seek never reports. Reachable only by deselecting every A/V limb (both kinds) and then seeking — engine-API-legal (switch_limbs handles `new_track == None` as retire-only, 1745) but impossible from shipped hosts today (F-003). The wedge is bounded (stop/close still work; a newer seek supersedes), but `inflight_seek` stays set and the Paused decode carve-out stays armed via `window_live()` for the rest of the session.
Why it matters: A state-machine hole that contradicts the function's own anti-wedge rationale, one deselect away from any future host that wires track selection — the same hosts F-003 says the matrix already counts as shipped.
What correct looks like: `if self.limbs.is_empty() { return SeekPreroll::Ready { audio_anchor_ns: None }; }` — the vacuous completion — matching `switch_preroll`'s deselected handling (1639-1641: `Ready { anchor_ns: None }` for "nothing to wait for").
Confidence: HIGH — Verification: read (trace over all `seek_preroll` callers — session.rs:2526-2528 gates the terminal on `Ready` only).

### F-005 [S4]
Where: crates/playback-engine/src/pipeline.rs:1287-1311 (error path inside seek_flush's limb loop; the tail audited here starts at 1332 — the symmetric cached-decoder loop at 1332-1352 returns Err the same way)
Code:
```rust
                    Err(error) => {
                        // The frames this limb already purged are
                        // still gone — count them before refusing.
                        self.seek_flush_stale =
                            self.seek_flush_stale.saturating_add(sink.stale_dropped);
                        return Err(error);
                    }
```
What is wrong: A decoder flush/configure refusal mid-loop returns without reconfiguring the failing limb (1308 unreached for it) and without touching the limbs after it: those limbs keep their old generation floor and old decoder config while **every** queue was already flushed at the new generation (1254-1256) and `self.generation` already advanced (1259). The pipeline is left with mixed per-limb floors — coherent for the synchronous engine (the demuxer never moved, so content position is unchanged and the doc's "limps on" holds), but the mixed-floor state is nowhere called out, and a future multi-threaded producer/consumer split would turn it into a generation-check inconsistency factory (queue publishes G_new; sink checks limb floor G_old; parked-decoder recycle assumes post-flush uniformity).
Why it matters: Latent hazard documented as simple degradation; the invariant "after seek_flush, every limb carries `generation`" is silently false on the error path.
What correct looks like: Continue the loop on a per-limb refusal (collect the first error, reconfigure the remaining limbs, return the error at the end), or at minimum assert/comment the mixed-floor state and make the next mint's flush (which re-floors everything) the documented repair.
Confidence: MEDIUM — Verification: read (loop structure traced; no test drives a failing flush through seek_flush — the fakes' `flush` always returns `Drained`).

### F-006 [S4]
Where: crates/playback-engine/src/pipeline.rs:1784 (retire path; the same pattern at 1255 belongs to part2)
Code:
```rust
            let retired = limb.track.id;
            let _ = limb.queue.flush(generation);
            limb.queue.close();
```
What is wrong: `BoundedQueue::flush` returns a `Discontinuity` report (dropped items, bytes, duration — queue.rs:336-377) that is discarded with `let _ =`. The resident packets destroyed by the retire are not folded into any pipeline/session counter (`outcome.stale_frames_flushed` counts only decoder-flush drops); they survive only inside the queue's own `stats.flushed`, which nothing in the session reads back per-switch.
Why it matters: Observability asymmetry: decoder residue drops are counted and reported (`SwitchOutcome.stale_frames_flushed`), queue-resident drops are not — a switch away from a backed-up track silently destroys up to a full queue of packets with no engine-level count. Not a correctness defect (the queue's stats and the pop-side discontinuity marker keep it non-silent at the queue layer).
What correct looks like: `outcome.stale_packets_flushed = gap.dropped` (fold the `Discontinuity` into `SwitchOutcome`), or a comment stating the queue-stats accounting is the intended observability.
Confidence: HIGH — Verification: read (`Discontinuity` fields and `stats.flushed` verified at queue.rs:134-142/336-377; `SwitchOutcome` consumers at session.rs:2289-2300 read only `stale_frames_flushed`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `Pipeline::seek_flush` (tail 1332-1355; whole fn 1239-1355, straddles part2) | 1332-1355 | REAL | S4 (F-005) | Cached-decoder flush loop bounded by `flush_budget`, `Err` returns honestly after counting; reconfigure per parked decoder (1351). No panics; `saturating_add` everywhere; `let _ = queue.flush` see F-006. Reachable: YES via CLI seek (chain in header). Unit-proven (baseline line 3795). |
| `Pipeline::window_open_seek` | 1365-1403 | REAL | — | Fresh entries+hold over current limbs, `with_capacity(hold_cap)` (D-1 claim verified), owners unioned (in-flight switches keep bits). No panics. Reachable: YES (seek). |
| `Pipeline::window_arm_switch` | 1413-1455 | REAL (defective) | S2 (F-002) | Push-if-absent never resets a stale entry; hold slot added for new track when SEEK owner present; `hold_cap: 0` for a lone switch window is correct (no slot ever exists → no hold). Reachable: NO from shipped hosts (F-003). |
| `Pipeline::window_end` | 1459-1466 | REAL | — | Bit clear + window death at last owner. Trivial, correct. |
| `Pipeline::window_live` | 1471-1473 | REAL | — | `self.window.is_some()`. Trivial. |
| `Pipeline::take_seek_flush_stale` | 1478-1480 | REAL | — | `std::mem::take`. Called seek.rs:383/387, session via SwitchOutcome (1856). |
| `Pipeline::take_preroll_discarded` | 1486-1488 | REAL | — | `std::mem::take`. Called session.rs:2102/2590. |
| `Pipeline::demuxer_reanchor` | 1498-1500 | REAL | — | Pure delegation to `Demuxer::reanchor` (D4 compute-then-commit lives downstream). |
| `Pipeline::seek_preroll` | 1512-1533 | REAL (defective) | S3 (F-004), S2 exposure via F-002 | Empty-limb `NotReady` forever (F-004); per-limb saw/can_never_deliver loop correct for current limbs; stale entries leak in via F-002's switch-back. Note: `SeekPreroll::Ready` enum doc (971-973, part2) says "at least one did [deliver]" — this function deliberately completes when zero delivered (1506-1509): the enum doc contradicts the code comment (part2's territory, cross-noted). |
| `Pipeline::release_preroll_hold` | 1552-1604 | REAL (defective) | S2 (F-001) | Merge+stable sort by pts (`unwrap_or(i64::MAX)` last — verified reasoning); per-frame stale check uses the wrong authority (F-001); video latch cloned before accept and applied only on success — correct; consumer refusal counted, never swallowed into an Err — matches the degrade-not-unwind contract. |
| `Pipeline::end_seek` | 1615-1625 | REAL | — | Hold cleared, held counted into `preroll_discarded`, SEEK bit released, window death at last owner. Correct. |
| `Pipeline::switch_preroll` | 1634-1660 | REAL (defective) | S2 (F-002) | Deselected→Ready{None}; no-limb→Ready{None}; can_never_deliver→Ready{None} (honest non-hang); saw→Ready with anchor (stale-fact hazard F-002). Reachable: NO from shipped hosts. |
| `Pipeline::end_switch` | 1664-1666 | REAL | — | Delegates to `window_end`. Reachable: NO (F-003). |
| `Pipeline::obtain_decoder` | 1679-1710 | REAL | — | Cache hit: swap_remove + configure-as-reset; miss: provider. Both paths build identical `DecoderConfig` (duplicated literal block 1691-1697 vs 1701-1707 — cosmetic dup). Reachable: YES — also called from `wire` (1069, open path, executed in baseline open lane). |
| `Pipeline::switch_limbs` | 1750-1858 | REAL (defective) | S2 (F-001 mint, F-002 arming) | Prepare-before-commit verified; retire flushes queue/drains decoder/parks by track id; install arms `first_frame_pending`; mint publishes `self.generation` (the F-001 trigger); `outcome.stale_frames_flushed = self.take_seek_flush_stale()` (1856) is exact for the switch (the seek already drained the counter at accept — seek.rs:383/387). `None`-track-unreachable comment (1770-1772) is honest. Reachable: NO from shipped hosts (F-003). |
| `Pipeline::take_ended_limbs` | 1865-1867 | REAL | — | `drain(..)` keeps capacity (D-1). Called session.rs:2472. |
| `Pipeline::can_never_deliver` | 1874-1876 | REAL | — | `eof && pending none && drained && queue empty` — conjunctive, no panic. Seek-flush resets both flags (1258/1309) so "never" is post-seek truth. |
| `Pipeline::paces` | 1895-1900 | REAL | — | Subtitle never paces; `first_frame_pending` exemption until queue recovery (cleared at decode 2516-2518, NOT at first frame — comment match verified). Matches the inlined copy at 1996 exactly (duplication is deliberate, commented). |
| `Pipeline::tick` (head 1904-2001; continues in part4) | 1904-2001 | REAL | — | Report init; demux in every owning state; decode gate `Buffering|Playing||window_live()` (Paused carve-out); drained in Playing|Paused; priming in Buffering|Paused with `producer_parked` escape (`self.pending.is_some()`, 1992). The escape is a documented livelock tradeoff (comment 1972-1991) — it declares primed while a pacing limb is still low, accepting a possible immediate stall episode over a permanent Buffering wedge; episode bookkeeping consistent (fresh-stall semantics). Boundary: starvation/starved computation continues past 2001 in A10's part4. |

## 5. Test honesty (tests covering this range)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `seek_mint_resets_the_debounce_state` (pipeline.rs:3157; baseline ok) | YES | Drives `seek_flush` for real and asserts per-limb streak/episode reset — a regression in 1263-1266 fails it. |
| `parked_codec_rides_the_recycle_path` (pipeline.rs; baseline ok) | YES | The F-4 recycle observable: parked codec descriptor recorded at configure — mutation of 1814-1822 fails it. |
| `switch_during_seek_completes_both_flows_under_their_own_generations` (track_switch.rs:1557; baseline ok) | PARTIALLY | Asserts terminal counts, distinct generations, switch-terminal generation identity, and the last track-3 frame's generation — real assertions on 8a's flow. But it does NOT count the unchanged (video) kind's presented frames, so F-001's discard inside this exact scenario is invisible; and it never switches BACK, so F-002 is untested. Mutation test (reasoned): delete the `frame.is_stale` check in release_preroll_hold → test still passes (frames present that shouldn't); delete the `window.saw` reuse guard → still passes. |
| `seek_during_switch_completes_both_flows` (track_switch.rs:1656; baseline ok) | PARTIALLY | Same shape: terminals asserted, pre-roll content not. |
| `rapid_double_switch_emits_one_terminal_for_the_newest_selection` (track_switch.rs; baseline ok) | YES | Supersession terminal-count assertion is real. |
| Remaining 8 track_switch tests + 7 pipeline starvation tests | YES (spot-checked) | Ordered event-sequence and counter assertions against fakes that really decode/emit (fixtures read at track_switch.rs:84-330); all passing in `orchrun-test-workspace-nofailfast.log.txt:4225-4239` and `:3789-3798`. |
| Coverage hole | — | No test anywhere drives (a) a switch-back during a seek, (b) an assertion on pre-roll presented-frame counts after any interleaving, (c) an empty-limb seek (F-004), (d) a failing decoder flush through seek_flush (F-005). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Audio/video/subtitle track select … status implemented (windows/linux/macos)" | docs/feature-matrix.toml row `playback-track-switch` (1546-1555) | FALSE at shipped level | F-003: no shipped host issues the command (CLI verb table, web host rejection, control-server no-engine); evidence string cites model + keymaps, no run |
| "the held frames may finally present: they leave through the same FrameConsumer seam they would have used without the hold" | pipeline.rs:1535-1541 (release_preroll_hold doc) | SHALLOW | F-001: after a mid-seek switch mint the unchanged kind's held frames are discarded by the release's own generation check — they do NOT leave through the seam |
| "`Ready` once the newly-selected track has delivered ≥ 1 frame under the switch's generation" | pipeline.rs:1627-1633 (switch_preroll doc) | FALSE in the switch-back case | F-002: stale WatchEntry answers Ready with zero frames of the switch's generation |
| "A landing where every limb provably never delivers still completes — the alternative is a seek that never reports" | pipeline.rs:1506-1509 | SHALLOW | F-004: the empty-limb landing is exactly that alternative and returns NotReady forever |
| "a flush budget that exhausts before a decoder answers Drained is not a failure" | pipeline.rs:1228-1232 | REAL | Loops at 1287-1299/1332-1342 break on budget without erroring; sink generation check drops stragglers later (2701-2704 sink accept) |
| "Pre-allocated once per seek (not per tick — D-1): pushes within the capacity never allocate" | pipeline.rs:1375-1376, 1393-1396 | REAL | `Vec::with_capacity(hold_cap)` at 1382; `PrerollHoldSlot::push` only pushes within cap (2665-2672); zero-alloc claim separately proven by tests/track_switch_alloc.rs (passing in baseline at 4243) |
| "A parked demuxer means … waiting longer changes nothing. Primed." (the producer-parked escape) | pipeline.rs:1972-1991 | REAL (design-accepted risk) | Escape is `self.pending.is_some()` (1992) — the 6.FIX-pcm livelock it prevents is unit-proven by the family at 2887-3198; the cost (early primed with a low limb) is the documented tradeoff, noted in §4 |

## 7. Dead / duplicated / unreachable code
- UNREACHABLE from shipped entry points: `switch_limbs`, `window_arm_switch`, `switch_preroll`, `end_switch`, and the switch half of `obtain_decoder`/`release_preroll_hold` interplay — engine-API-only (F-003). Not dead: exercised by 12 passing integration tests.
- Duplication: `DecoderConfig::new(...)` literal block appears twice in `obtain_decoder` (1691-1697, 1701-1707) and twice in `seek_flush` (1301-1307 head, 1344-1350 tail) — four copies of the same five-line construction; cosmetic. `paces()` is deliberately inlined at 1996 (commented).
- The `LimbSink` literal appears 4× (seek_flush ×2, switch_limbs, decode_step) — struct-literal duplication across part boundaries; part4/part5 own the others.

## 8. Dependencies & flags
No feature flags gate anything in this range (no `cfg` between 1335-2001 — verified by read). No `unsafe`, no panicking calls (`unwrap`/`expect` absent; all arithmetic `saturating_add` or on ids). Crate-internal types only (`SeekPreroll`/`SwitchPreroll` are `pub(crate)`). The shipped-build facts (engine-info decoders mjpeg/pcm_s16le; features first-vertical) mean the *executed* shipped lanes touch seek_flush/tick with MJPEG/PCM limbs only.

## 9. Open questions for the next agent
1. A10 (pipeline part4): the release path presents video frames **directly** to `consumer.accept` (1596), bypassing any scheduler generation gate — after a mid-seek switch, the clock publishes the SEEK's generation (session.rs:2547 via complete_seek→set_generation) while released frames carry the SWITCH's generation. Trace whether the video-engine scheduler's "hold frames NEWER than the published token" gate (session.rs:2575-2576 wording) drops or delays those released newer-generation frames — if yes, that is a third interleaving defect (released pre-roll withheld by the gate).
2. Session auditor: `subtitle_select_step` (session.rs:2273-2306) calls `switch_limbs` at the CURRENT generation and then immediately `end_switch` — confirm the subtitle window arm/claim between those two calls can never interact with a concurrent seek's hold (I traced it as safe: no mint → no floor divergence, but the arm-then-end window is one receipt wide).
3. Matrix-rows agent: F-003's row correction (`playback-track-switch`) should be cross-checked against `menu-audio`/`menu-subtitle` rows the notes admit are absent.
4. Whoever owns tests: the four coverage holes in §5 row 6 are the cheap regression net for F-001/F-002/F-004/F-005.
