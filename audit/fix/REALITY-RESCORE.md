# REALITY-RESCORE — the interim re-score at the Phase-1 code-side exit (Devil-Fix waves FX1–FX7)

| Field | Value |
|---|---|
| Auditor | FX7-A03 (REAL% interim re-score, FIX-PLAN §1.6 / §7) |
| Date | 2026-11-26 |
| Scope | ALL 252 rows of `docs/feature-matrix.toml`, re-scored with the SAME verdict buckets as `audit/final/REALITY-SCORE.md`; only the rows whose substance the Devil-Fix code waves (FX1–FX6 + FX7's W1-06 row) actually flipped change verdict — no inflation |
| Headline metric | **REAL% = 52 / 252 = 20.63%** (floor 20% — no rounding up) |
| Previous | 49 / 252 = 19.44% (floor 19%), audit of 2026-11-24 |
| Re-score position | end of the **code-side** of Phase 1 (F1.1 audio path, F1.2 frame lane, F1.3 seek landed across FX3–FX6) — **before** the G1 Windows hardware run |
| Commands run this session | `python3` tomllib parse of docs/feature-matrix.toml (252 rows re-verified); `rg` evidence censuses over the tree (reanchor, host verbs, catalog quartet, engine controls, resampler, frame lane, backend factory — every flip's code anchor re-verified in-tree); `python3` bucket arithmetic (§4); re-read of the FX1–FX6 wave reports incl. all remedial R-reports |

---

## 1. Method recap — same buckets, same denominator, same rules

The original methodology (`audit/final/REALITY-SCORE.md` §1/§2) is preserved
verbatim — the buckets, their canonical meanings, and the 252-row denominator:

| Verdict | Meaning (canonical, unchanged) |
|---|---|
| **REAL** | Row claims implemented/partial AND the claimed feature was executed working (linux lane; orchestrator baseline or ledger-run evidence) |
| ACCURATE-ABSENT | Row claims not_started/blocked/unsupported and the claim is TRUE — the matrix telling the truth about MISSING features. NOT REAL |
| SHALLOW | Claims a working subset; only a happy-path/data/editor shell works; core refused, unexecuted, or unreachable |
| UNDERSTATED | Row says not_started/"no work yet" while real code has landed (the anti-inflation lie direction) |
| UNREACHABLE | Real, tested library code that NO shipped entry point links/calls |
| FALSE | Status/claim contradicts the code (inflated or misclassified) |
| REAL-UNPROVEN | Genuine code, could not execute (TLS handshake, real-device audio, git-hash evidence, sandbox blocks) |

Rules that bind this re-score (from the original §2): **R2** truthful absence
is NOT REAL; **R3** real code with no shipped consumer is UNREACHABLE, never
REAL; **R6** an honest-*partial* row whose claimed subset is executed working
counts as REAL; **R7** one verdict per row, linux-executed basis — win/mac
unproven cells are notes, not extra rows. Honesty rule (FIX-PLAN §7): floor,
never round up, re-measured — not promised.

**The one evidence-standard delta, stated:** the original's executions were
the orchestrator's baseline runs and ledger walks; this interim's executions
are the Devil-Fix waves' own acceptance oracles — each is a committed,
regenerable command (`cargo test -p lumen-cli --test fx5_seek_mp4`, …) that
the orchestrator's central wave build ran between waves (the remedial
R-reports FX3-R*/FX4-R*/FX5-R*/FX6-R* are the visible residue of those central
runs — compile/pin fixes, no remedial names any of the flipped rows' oracles
red). That these oracles can be trusted at all is itself Phase 0's product:
FX2-A01/A02/A03 single-sourced the four lying registries (engine-info,
decode-bench, landed-string, the dispatch table) — the instruments this
re-score's evidence rides on were repaired before the flips were measured.

The matrix was re-parsed this session (tomllib, 252 rows) and the flipped
rows' *code* anchors re-verified in the working tree by rg (§2's table) —
the re-score is verdict-level over the matrix; prose drift inside un-updated
rows' notes/evidence fields is recorded in §8, not silently absorbed.

---

## 2. The flip table — rows whose substance the FX3–FX7 code changed

Four rows change verdict. Three move **to** REAL; one moves in the
conservative direction (ACCURATE-ABSENT → UNDERSTATED — the honest result of
code landing while the row still says not_started).

| Row | id | Old verdict (REALITY-SCORE) | New verdict | The substance that flipped | Evidence chain (wave ids + regenerating command) |
|---|---|---|---|---|---|
| 103 | playback-seek | **SHALLOW** (L8: "engine real; MP4/TS in-session seek executed-FAIL (S0), no caveat in row") | **REAL** | The S0 (W05-A01 / W20-A05 F-001): `Mp4Demuxer` implemented `seek()` but not `reanchor` — the only entry the engine's seek flow calls — so every in-session MP4 seek returned `plugin.demux.reanchor.unsupported`. FX5-A01 implemented `reanchor` on the existing stts/stsc/stco/stss + moof/trun/tfdt index machinery (progressive: `order_cursor` rewound + pending flushed + EOS cleared; fragmented: scan from file start); FX5-A07 verified every hop of the engine lane forwards it (`SelectDemuxer` → `StartedMp4` → `Mp4Demuxer`) and pinned the previously-untested wrapper seam (the demux_host audit's "test-pinned reanchor" claim was FALSE — an audit drift worth noting); FX5-A04 pinned the CLI-level acceptance: `seek 1s` → `TimingSeekCompleted`, position ~1.0 s, playback continues to the exact 2.000 s end, over the committed class-A MP4 fixture; FX5-A06 flipped the matrix row to implemented x3 + closed C-003 from the code side. TS/es/fragmented-sidx truths unchanged (typed refusals). | FX5-A01 (`crates/demux/src/mp4/mod.rs:2086` `fn reanchor` — rg-verified this session), FX5-A04, FX5-A06, FX5-A07. Regenerating: `cargo test -p demux --test fx5_a01_mp4_reanchor` (12 tests), `cargo test -p lumen-cli --test fx5_seek_mp4` (4 acceptance tests), `cargo test -p lumen-cli --test fx5_a07_seek_lane` (7 seam tests). Central-run status: FX5's central run left exactly one failing assertion repo-wide (a 65→69 catalog pin, FX5-R3) — no remedial names these oracles red. |
| 115 | afeat-volume | **FALSE** (L9: "runtime volume uncontrollable in any shipped host; demo-shell only") | **REAL** (R6 — the row's honest-partial claim now executed working) | The FALSE was real: the volume "worked" only in the browser dev-shell (DemoTransport) while `audio.set-volume` was refused by every shipped host (wave-07 S2). FX3-A03 built the engine's runtime master chain (`AudioControls`: volume → mute → balance → EQ on the mixed quantum, identity until controlled); FX4-A01 placed it in the live session seam (`AudioLimb::audio_controls()`); FX4-A02 routed the verbs through the real command layer — `audio.set-volume`/`get-volume`/`set-mute` complete against the live chain and answer the applied truth (the `unsupported-for-host` refusal is gone from the whole NDJSON transcript); FX4-A05 proved the controls observable IN THE RENDERED SAMPLES (volume 0.5 halves every rendered sample exactly; mute zeroes with frame count conserved; the session still plays to its exact end). The desktop/tauri hosts inherit the verbs by construction (FX4-A03/A04, delegation pinned). Volume/mute/gain is this row's own claim surface (vlc_name "Volume/mute/gain"). | FX3-A03 (`crates/audio-engine/src/controls.rs:78`), FX4-A01 (`apps/cli/src/pipeline_host.rs:1938/2033`), FX4-A02 (`apps/cli/src/host.rs:1408/1448` — rg-verified), FX4-A03/A04, FX4-A05. Regenerating: `cargo test -p audio-engine` (fx3_a03_* master-chain suite), `cargo test -p lumen-cli --test fx4_audio_path` (6 tests, incl. `fx4_a05_volume_is_observable_in_rendered_samples`), `cargo test -p lumen-cli --test fx4_a02_host_audio_verbs` (6 tests, incl. the machine-mode run through the real binary). Central-run status: FX4's central compile errors were exactly the FX4-R1/R2 classes (imports/format strings); no remedial names these oracles red at runtime. |
| 116 | afeat-equalizer | **FALSE** (L9: "Eq3Band no caller; UI sliders inert-with-reason") | **REAL** (R6 — the row's 3-band claimed subset now executed working) | The FALSE was real: real RBJ biquads (Eq3Band) with zero production callers and inert UI sliders awaiting a command that did not exist. FX3-A03 put Eq3Band in the streaming path (fail-atomic runtime `set_bands`, ±40 dB bound typed); FX4-A01/FX4-A05 made the EQ observable in rendered samples (−12 dB mid cut settles at 10^(−12/20) of the uncontrolled output through the whole session path); FX5-A02 minted the missing catalog verbs (`audio.set-eq`/`get-eq`/`set-balance`/`get-balance`, 69-command catalog, artifacts regenerated); FX5-A03 routed `set-eq`/`set-balance` through the host to the live chain; FX4-A08 cut the UI to the engine's 3-band truth and wired every fader/preset commit to the real `audio.set-eq` dispatch; FX5-A05 reconciled the UI payload one-to-one to the landed catalog verb and repaired the tree's only tsc errors. The row stays honest-partial (3-band fixed corners, not VLC's 10-band; presets/2-pass pending) — the claimed subset works. | FX3-A03, FX4-A01/A05, FX5-A02 (`crates/commands/src/catalog/command.rs:219/240` — rg-verified), FX5-A03 (`apps/cli/src/host.rs:1476/1511`), FX4-A08, FX5-A05. Regenerating: `cargo test -p lumen-cli --test fx4_audio_path` (`fx4_a05_eq_band_cut_attenuates_a_band_centered_tone`), `cargo test -p lumen-cli --test fx5_a03_host_audio_verbs` (6 tests incl. machine-mode set-eq), `cargo test -p commands --test fx5_a02_catalog_audio_verbs` (5), UI lane: `npx vitest run` (fx4_a08_* 12 pins, fx5_a05_* 5 pins — FX5-A05 ran the suite green locally, 786 passed). |
| 120 | afeat-resampler | **ACCURATE-ABSENT** (L10: "honest; adjacent MicroResampler real+unreachable") | **UNDERSTATED** (the conservative direction — code landed, the row still says not_started) | The old verdict was true THEN: no SRC existed (convert.rs is sample-format only; the pipeline refused rate mismatch). FX3-A04 landed `RateResampler` — real track-rate SRC (linear kernel, exact i128 cursor, bit-exact passthrough at equal rates, 10 pinned tests); FX4-A01 wired it at the session seam (`open_audio_session_for` constructs it exactly when track rate ≠ sink rate and runs the clock over the sink's counter — pinned by an exact 1599-frame closed-form test). No shipped surface drives a rate mismatch yet (the host opens the sink at the track rate), and the matrix row was never updated by any wave's docs pass — so the honest verdict is UNDERSTATED (real, tested, seam-wired code behind a not_started claim), the same class as rows 084/095. Not REAL: no shipped surface exercises it; not UNREACHABLE: the lane is linked inside the shipped binary's session constructor and driven by the acceptance tests. | FX3-A04 (`crates/audio-engine/src/resample.rs:358` `pub struct RateResampler` — rg-verified), FX4-A01. Regenerating: `cargo test -p audio-engine --test fx3_a04_rate_resampler` (10 tests), `cargo test -p lumen-cli --test fx4_a01_backend_route` (`fx4_a01_rate_mismatch_resampler_wires_the_output_clock`). |

**Note on "balance":** balance has no matrix row of its own (VLC's balance is
not the matrix's stereo-mode). FX3-A03's `set_balance` + FX5-A02's
`audio.set-balance` verb + FX5-A03's routing are counted as supporting
evidence inside rows 115/116's audio-controls chain — no separate row was
invented for it. See §3 for the stereo-mode row deliberately NOT flipped.

---

## 3. Substance-improved or name-checked rows that do NOT flip (no inflation)

Every row the waves touched in substance but whose verdict bucket is
unchanged — listed so the re-score cannot be accused of either direction:

| Row(s) | Old verdict | Stays | Why it does not flip |
|---|---|---|---|
| 002 container-mp4-mov-3gp | REAL | REAL | The row was already REAL (open→decode→Ended executed); FX5's reanchor closes the row's own "in-session seek executed-FAIL (S0)" note — substance improved, bucket unchanged (the seek flip belongs to row 103). |
| 004 container-mpegts, 001 mkv, 003 ogg, 012 adts, 013 mp3 | REAL | REAL | FX1's panic guards (TS-188 probe, MKV lacing/livelock, adts zero-length, mp3 VBRI, ogg-flac header) harden already-executed REAL lanes; guards change no verdict. |
| 007 container-asf-wmv | FALSE | FALSE | FX1-A07 fixed the ASF header child-size panic; the demuxer still cannot open ASF (probe.no-demuxer, no dispatch arm) — the win/mac "implemented" fiction stands. |
| 097 network-hls, 098 network-dash | SHALLOW | SHALLOW | FX1-A06/FX2-A10's typed-refusal guards landed, but no shipped entry point opens m3u8/mpd — the lanes remain unreachable from shipped hosts. |
| 057–068 subtitle rows, 074 textrender | UNREACHABLE | UNREACHABLE | FX1-A09's PGS/DVB/CC guards harden the crate; they do not mint shipped consumers. |
| 169 platform-aout-cpal | REAL-UNPROVEN | REAL-UNPROVEN | The row's substance is heavily upgraded — FX3-A01's real cpal adapter admitted behind `backend-cpal`, FX4-A01's backend factory routing (null fallback the operative build), FX3-A07's windows-latest compile oracle — but no real-device execution exists; audibility stays [VERIFY] pending the `audio-cpal-check` Windows CI run + a Windows release smoke. The bucket definition ("genuine code, could not execute") still fits exactly; the matrix row stays partial, truthfully. |
| 170/171 platform-aout-wasapi/coreaudio | UNDERSTATED | UNDERSTATED | FX2-A05/A06/A07 fixed PropVariant/dxgi/ELEMENT_MAIN in the in-tree FFI — the rows still say "no work yet" for code that is still not wired into any shipped host. |
| 173 platform-vout-wgpu | UNDERSTATED | UNDERSTATED | FX6-A06 verified the frame lane never was a matrix row; renderer-wgpu stays the CLI's software compositor (matrix not_started; FX6-A06's tomllib guard). |
| 193 ui-screen-player (+ 194–202) | REAL | REAL | The canvas was already real (wave-15 finding 1); FX6-A05's webview frame-counter readout strengthens already-REAL screens. |
| 209 ui-dialog-goto-time | REAL | REAL | The Ctrl+T→`playback.seek` dialog's engine half now actually lands seeks — strengthens an already-REAL row. |
| 236 hotkey-seek, 238 hotkey-volume, 235–246 hotkey family | SHALLOW | SHALLOW | The seek and volume VERBS now work — but the hotkey engine half is still seed+editor-only (~8 of 141 keys live); no hotkey was bound by any wave. The verbs existing does not serve the keys. |
| 105/108/109/110 playback frame-step/chapters/bookmarks/snapshot | FALSE | FALSE | F3.4 scope, deliberately deferred by the G1 sequencing rule; nothing landed. |
| 121 afeat-stereo-mode | ACCURATE-ABSENT | ACCURATE-ABSENT | The honest boundary: balance (L/R pan) is NOT stereo-mode (Stereo/Left/Right/Reverse channel remap). The row's claimed substance — channel-mode remap — did not land; the balance control does not flip this row. |
| 125 afeat-audio-device-switch | ACCURATE-ABSENT | ACCURATE-ABSENT | `audio.list-devices`/`set-device`/`set-delay` remain refused by every host (FX4-A02's own owner note); cpal's enumeration is behind the feature, not the verb. |
| registry/instrument work (FX2-A01/A02/A03) | — | — | No matrix row covers engine-info/decode-bench/landed-string — the instruments are not the feature surface. Phase 0's own prediction held: "REAL% does not move much (the rows' substance did not change) — but every subsequent measurement becomes trustworthy." Zero verdicts moved; this re-score's evidence rides the repaired instruments. |

**The frame lane (F1.2), explicitly:** FX6-A01/A02/A03/A05 landed the
bounded mailbox/channel, drop attribution, timestamp passthrough, the
cadenced produced-vs-dropped receipt, and the webview counter readout — and
FX6-A06 verified **no matrix row covers the frame lane** (its tomllib guard:
zero FX6 ids in the matrix; `platform-vout-wgpu` truthfully stays
not_started; `ui-screen-player` already REAL). The lane's acceptance
remainder — the 60/60 produced-covers-decoded intent — is the W1-06
registered defect, re-scoped by FX6-R4 to the loss-ledger conservation with
the 60/60 gap measured and attributed (18 produced / 60 decoded; the W1-06
hold-retried I420 refusals). W1-06's closure is FX7-A01's row in THIS wave —
its diff and report are not in the tree at re-score time (the hold still
stashes the UNCONVERTED frame at `pipeline_host.rs:1712`, verified this
session), so nothing about the frame lane is counted as landed here. It
would not change a verdict even when it lands (§3's row coverage).

---

## 4. The arithmetic — buckets before/after (must total 252 both times)

| Verdict | Before (2026-11-24) | Flips | Interim (2026-11-26) |
|---|---|---|---|
| **REAL** | **49** | +3 (rows 103 SHALLOW→REAL, 115 FALSE→REAL, 116 FALSE→REAL) | **52** |
| ACCURATE-ABSENT | 110 | −1 (row 120 → UNDERSTATED) | 109 |
| SHALLOW | 29 | −1 (row 103) | 28 |
| UNDERSTATED | 24 | +1 (row 120) | 25 |
| UNREACHABLE | 18 | — | 18 |
| FALSE | 14 | −2 (rows 115, 116) | 12 |
| REAL-UNPROVEN | 8 | — | 8 |
| **TOTAL** | **252** | 0 net row movement | **252** |

Per-category deltas (the only two categories that moved):
- **playback-op (102–114):** REAL 3→4, SHALLOW 3→2 (row 103).
- **audio-feature (115–125):** FALSE 2→0, REAL 0→2 (rows 115, 116),
  ACCURATE-ABSENT 9→8, UNDERSTATED 0→1 (row 120).

Re-derivation (the command the machine gate can re-run verbatim):

```
python3 -c "before={'REAL':49,'ACCURATE-ABSENT':110,'SHALLOW':29,'UNDERSTATED':24,'UNREACHABLE':18,'FALSE':14,'REAL-UNPROVEN':8}; after={'REAL':52,'ACCURATE-ABSENT':109,'SHALLOW':28,'UNDERSTATED':25,'UNREACHABLE':18,'FALSE':12,'REAL-UNPROVEN':8}; assert sum(before.values())==sum(after.values())==252; assert after['REAL']==before['REAL']+3; print(after['REAL']/252, int(after['REAL']*100/252))"
```
→ `0.20634920634920634 20`

Old-verdict re-derivation (the flip rows' originals, from the audit file):

```
rg -n 'playback-seek|afeat-volume|afeat-equalizer|afeat-resampler' audit/final/REALITY-SCORE.md
```
→ rows 103 (SHALLOW), 115 (FALSE), 116 (FALSE), 120 (ACCURATE-ABSENT).

In-tree evidence re-derivation for the flipped substance (all rg-verified
this session): `rg -n 'fn reanchor' crates/demux/src/mp4/mod.rs` (:2086);
`rg -n 'fn audio_set_volume|fn audio_set_muted|fn audio_set_balance|fn audio_set_eq' apps/cli/src/host.rs` (:1408/:1448/:1476/:1511);
`rg -n 'AudioSetBalance|AudioSetEq' crates/commands/src/catalog/command.rs` (:219/:240);
`rg -n 'pub struct AudioControls' crates/audio-engine/src/controls.rs` (:78);
`rg -n 'pub struct RateResampler' crates/audio-engine/src/resample.rs` (:358);
`rg -n 'pub enum AudioBackendPosture|pub fn open_audio_session_for' apps/cli/src/pipeline_host.rs` (:1938/:2033);
`ls apps/cli/tests/fx5_seek_mp4.rs apps/cli/tests/fx4_audio_path.rs apps/cli/tests/fx4_a02_host_audio_verbs.rs apps/cli/tests/fx5_a03_host_audio_verbs.rs apps/cli/tests/fx4_a01_backend_route.rs` (all present).

---

## 5. The interim REAL%

**REAL% = 52 / 252 = 20.63% → floor 20% (no rounding up).**

The same contrast super-metrics as the original, updated:
REAL + REAL-UNPROVEN = 60/252 = 23.8% (was 22.6%); add UNREACHABLE =
78/252 = 31.0% (was 29.8%). None of these is the headline; the headline is
20.63%, floored to 20%.

---

## 6. The [VERIFY]-pending list (Windows hardware items)

Everything below stays honestly unproven at this interim and gates the G1
exit; each is a named owed run, not a promised flip:

1. **Audible AAC on Windows hardware** — `lumen open sample.mp4` produces
   audible audio; volume and mute work in-session on the real device
   (F1.1's acceptance bar; FX4-A07's guard). Until then row 169's audio path
   substance stays REAL-UNPROVEN at the hardware half.
2. **First green run of the `audio-cpal-check` windows-latest CI lane**
   (FX3-A07) — the compile oracle for the admitted cpal adapter; the lane is
   provisioned, its first run is the proof event (plus the lockfile refresh
   the `--locked` steps need).
3. **A Windows release smoke with an in-session seek** — FX5-A06's owed run;
   the desktop/tauri GUI seek lanes are "pending verification" (FX4-A03/A04
   posture) until a GUI-lane seek is observed on Windows.
4. **A user-watched end-to-end Windows GUI run** — frames actually seen,
   smoothness, 4K starvation at the 10 ms cadence (F1.2's hardware half;
   FX6-A06's [VERIFY]; `decode-bench` is the count oracle per F0.2).
5. **W1-06 closure (in-wave, not hardware):** FX7-A01's assignment (stash the
   CONVERTED frame in the present hold) is not in the tree at re-score time
   — the fx6_a04 60/60 intent and the F1.2 acceptance's "counter matches
   decode count" await it (FX6-R4's ledger carries the gap: 18/60 measured).
6. G1-closed projection, honestly: if the Windows run is green, row 169
   becomes a REAL candidate (+1 → 53/252 = 21.03%) and rows 115/116/103 gain
   hardware corroboration; playback-fullscreen (111, REAL-UNPROVEN) may
   resolve with the GUI run. Nothing else moves on hardware alone.

---

## 7. Trajectory position vs FIX-PLAN §7

| Milestone | Expected | Re-measured |
|---|---|---|
| Today (audit) | 19.44% | 19.44% (49/252) |
| Phase 0 exit | ~20–22% | — (Phase 0's verdict-level prediction held: zero rows moved) |
| **Phase 1 exit (G1)** | **~30–35%** | **20.63% at the code-side exit (52/252) — BELOW the band** |
| Phase 2 exit | ~40–45% | not started |
| Phase 3 exit | ~55–60% | not started |

**The interim sits inside the Phase-0-exit band's upper region and well
below the Phase-1-exit band.** The honest decomposition of the gap:

1. **The band's own third clause never scoped to this phase.** "Several
   SHALLOW playback rows become REAL" — frame-step (105, FALSE), track-switch
   (106, SHALLOW), av-sync (107, SHALLOW), bookmarks, snapshot — are F3.4
   (Phase 3) rows, deliberately deferred by the plan's own G1 rule ("open no
   new feature lane until G1 closes"). Phase 1's honest row-level scope was
   always just seek + volume/EQ (+ resampler landing as UNDERSTATED):
   49 + 3 = 52. The 30–35% band was not supported by its own listed clauses —
   it was an over-projection baked into §7's estimate column.
2. **Hardware-dependent rows are counted REAL-UNPROVEN, not REAL.** The
   band's arithmetic implicitly assumed G1's Windows run had already happened
   (the Phase-1 exit gate IS the hardware run). This interim is the
   *code-side* exit, pre-G1: audible audio (row 169's hardware half), GUI
   seek, and user-watched frames are all [VERIFY] (§6). Counting them as REAL
   now would be exactly the inflation the audit's methodology forbids.
3. **The big REAL% movers are Phase 2/3 scopes that have not started:** the
   24-row UNDERSTATED sweep (F2.1, +~24 potential — rows 084/095/099-101,
   the platform 170-175 cluster, ~10 UI rows) and the 18-row UNREACHABLE
   wiring (F3.1 subtitles + playlists). No Devil-Fix wave has swept either
   yet — every UNDERSTATED and UNREACHABLE row carries its original verdict.
4. One flip moved the *wrong* way by design: row 120 resampler
   ACCURATE-ABSENT → UNDERSTATED — code landing under a stale not_started
   row is a real (if conservative) degradation of the matrix's honesty score,
   cured only by F2.1's status generator.

**Consequence for the plan (carried to FX7-A04's trajectory refresh):** even
at G1-closed, the honest projection is ~21.0% (53/252). The Phase-1 exit band
should be re-projected to ~20–22% at the next trajectory refresh; the 30–35%
band's distance is only reachable through Phase 2's UNDERSTATED sweep, which
§7 itself schedules for the Phase-2 exit (~40–45%). The trajectory's honesty
rule — floor, never round up, re-measured not promised — is hereby applied to
the plan's own estimates.

What this interim DOES show, on the positive side: the three flips are the
exact rows §7's Phase-1 line predicted by name ("mp4-seek row flips
TRUE→REAL; volume/EQ rows become REAL"), each carrying a committed
regenerable command instead of a promise, and each verified against
single-sourced instruments — the first REAL% movement in the repo's history
that cannot be an artifact of a lying registry.

---

## 8. Method notes and known drift (recorded, not absorbed)

1. **Matrix prose drift on the flipped rows:** rows 115/116's evidence
   fields and notes still describe the pre-FX3 state ("UI sliders render
   inert-with-reason until that command lands" — stale since FX5-A02/A03;
   commits 8bd13ed/0a8e70d cited as the live surface), and row 120's notes
   still say "decisions pending Phase 3". Only rows 103 (FX5-A06) and 169
   (FX3-A08/FX4-A07's dated refreshes) received matrix updates. The re-score
   is verdict-level (substance vs code); row-motion is F2.1's status
   generator, which owns the prose truth sweep.
2. **Evidence provenance:** the flipped rows' execution evidence is the
   waves' acceptance oracles run by the orchestrator's central builds
   (between-wave compiles are directly evidenced by the remedial
   R-reports' compile-error lists; the absence of any runtime remedial
   against the cited test files is the negative evidence their runs were
   green). The one acceptance oracle that failed centrally — fx6_a04's
   60/60 — is exactly the frame-lane item this re-score does NOT count
   (§3/§6).
3. **W1-06/FX7-A01 timing:** this re-score ran in parallel with FX7-A01
   (same wave). Its diff/report were absent from the tree at measurement
   time (the UNCONVERTED-stash defect verified still present at
   `pipeline_host.rs:1712`); if it lands with this wave, nothing in §2–§5
   changes — the frame lane owns no matrix row (§3).
4. **Same-floor discipline:** 52/252 = 20.634920…% — the headline floors to
   20%, matching the original's "19.44% → floor 19%" convention. R7 held:
   win/mac unproven cells (seek on Windows GUI, audibility) are notes in §6,
   never extra rows and never REAL flips.
