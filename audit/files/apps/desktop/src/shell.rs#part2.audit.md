# AUDIT: apps/desktop/src/shell.rs#part2 (lines 436–871)
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2026-11-21 |
| Lines / functions | 436 (of 872; boundary pinned from audit/waves/wave-00/assignments.tsv:765 — part1=1–435 is A01's; my part is the TEST MODULE: every production item lives in part1) / 14 test items |
| File verdict | REAL (test module; the two proof tests are the strongest in the file) |
| Reachable from shipped entry point? | The audited range is `#[cfg(test)]` — not shipped. What it PROVES is part1's production surface, which IS reachable: `ShellRuntime` is consumed by the tauri wrapper (apps/desktop-tauri/src/app.rs:93 `use desktop::shell::{DEFAULT_EVENT_CAPACITY, ShellRuntime}`, :113 `ShellState(Mutex<ShellRuntime>)`, pump_session at :204, take_frame_records at :211, on_files_dropped at :318) — the `lumen-tauri` entry point (build UNVERIFIED in sandbox, baseline #2). All 13 shell tests ran green in BOTH baseline runs (test-workspace.log.txt:2329-2341,2361; nofailfast:2288-2300). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read shell.rs 1–445 (part1 context) + 446–872 (my range); rg `ShellRuntime\|pump_session\|take_pending_events\|on_files_dropped\|pick_and_open\|take_frame_records` over apps (tauri app.rs consumption map; pick_and_open has NO production caller — app.rs:78 documents the intentional non-wiring); sed app.rs:40–95; rg `theme` in schemas/settings.toml (:74 `key = "interface.theme"`, enum ["dark","light","auto"], `default_ref = "defaults.theme"`) + product.toml:108 `theme = "dark"` (default IS "dark" — F-001); sed host.rs:1366–1375 (`setting_get` falls back to default when unset); rg shell test names in both baseline logs (all 13 green ×2); PACER_TICK arithmetic re-derived (1470×1e6/48000 = 30 625 µs exact). |

## 1. One-paragraph verdict
Part 2 is the shell's test module, and it is mostly the good kind: `pacer_proof_pump_once_drives_a_real_session_to_end` and `frame_channel_proof_pump_once_ships_luf1_records` drive a REAL AVI fixture through the GUI's own funnel (open_path → single dispatch → play → only pump ticks) to EOS with hang guards, event-order checks, and egress conservation — exactly the regression bars their names claim, both green in the two baseline workspace runs. The defects are test-honesty rot, not fakeness: the settings "round-trip" test writes the value that IS the schema default ("dark"), so the storage half of the round trip is unprovable (get falls back to the same default when the key is absent); the playlist test's name promises "loads the list" while the body pins the typed refusal; and the drop-funnel test's comment claims "the fixture is real" over a nonexistent `/tmp/movie.avi`. The dialog test thoroughly covers a function (`pick_and_open`) that the shipped wrapper deliberately does not wire.

## 2. What actually works (verified)
- **The card 3.B3 proof is real** (574-648): real fixture `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`, open accepted → play accepted → 1024-pump bound + 120 s hang guard → asserts Playing reached, time-ticks > 0, EndOfMedia, Ended, and `event_stats().conserves()`. If `pump_once` lost its time advance (the frozen-GUI regression it exists for), `ended`/`time_ticks` fail. Baseline: test-workspace.log.txt:2361 `… ok`.
- **The w31.1 LUF1 proof is real** (677-745): bind-first (the real UI's startup order), open, play, pump loop, `take_frame_records` > 0, every record starts `b"LUF1"`. Baseline: `frame_channel_proof_pump_once_ships_luf1_records … ok` in BOTH runs (:2339 / :2298).
- **Pacer cadence pin** (751-760): `PACER_TICK == 30_625 µs == RENDER_QUANTUM×1e6/48_000` — I re-derived the arithmetic (1470×1_000_000/48_000 = 30_625, exact integer division). A change to `RENDER_QUANTUM` breaks this test loudly.
- **Bounded egress pinned directly** (844-870): capacity-2 sink, 5 offers → survivors `[3,4]`, `dropped_overflow == 3`, conserves. Real bound, not a mock of it.
- **Event order end-to-end** (511-535): real open → settle → drain; seq vector asserted sorted AND the second take empty (monotonic drain).
- **Typed negatives**: unknown settings key → `InvalidArgument` (491-507); un-classifiable paths → `None` (764-777); canceled dialog → `Canceled`, webview-unlinked → `Pending(HostError::WebviewPending{adr:"ADR-004"})` verbatim (809-840).
- **Frame-channel interception** (655-668): bind answers `Unit` at the SHELL (the engine would refuse `UnsupportedForHost`), one-shot binding take asserted.

## 3. Findings

### F-001 [S3] — The settings "round-trip" test writes the schema DEFAULT, making the storage half of the round trip unprovable
Where: apps/desktop/src/shell.rs:448-487 (asserts at 478-486); root cause chain: host.rs:1372-1375, schemas/settings.toml:74-80, product.toml:108
Code:
```
    fn config_writes_round_trip_through_dispatch() {
        let mut runtime = ShellRuntime::default();
        let set_id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            set_id,
            Command::ConfigSetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
                value: commands::catalog::types::SettingValue::String("dark".to_string()),
```
What is wrong: the test sets `interface.theme = "dark"` and then asserts `config.get-setting` answers `"dark"`. But `"dark"` IS the default (`schemas/settings.toml:74` `key = "interface.theme"`, `default_ref = "defaults.theme"` → `product.toml:108` `theme = "dark"`), and the get path falls back to the default when nothing was stored — `apps/cli/src/host.rs:1372-1375`:
```
        let value = match self.settings.get(&full) {
            Some(stored) => stored.clone(),
            None => default_value(def),
        };
```
So the mutation "`setting_set` never stores" passes this test unchanged: the get answers the default "dark", and the set response's `applied` field echoes the requested value regardless of storage.
Why it matters: the test's name promises a round-trip proof of the settings seam; the one value it could have chosen to prove storage (`"light"` or `"auto"` — both in the enum) is the one value that cannot. A real regression in the store path would ship green.
What correct looks like: set `"light"` (or set "dark" from a preconditioned `"light"` state), then assert the get answers the non-default value.
Confidence: HIGH — Verification: read (all three files cited; default chain `default_ref = "defaults.theme"` → `[defaults] theme = "dark"` verified in the repo's own schema/product tables).

### F-002 [S4] — Test name says "loads the list"; the body proves the refusal
Where: apps/desktop/src/shell.rs:540-559
Code:
```
    fn playlist_files_load_the_list_through_the_command() {
        let mut runtime = ShellRuntime::default();
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        let response = match runtime.open_path(&PathBuf::from("/tmp/queue.m3u")) {
```
What is wrong: the body asserts `ErrorCode::UnsupportedForHost` with detail `playlist.load` and the comment admits "playlist.load is not served by this engine build" — no list is ever loaded. The name is the artifact a reader greps; it claims the opposite of what is proven.
Why it matters: name-vs-body drift; inflates apparent playlist coverage ("playlist files load…" reads as a working feature).
What correct looks like: `playlist_files_classify_and_refuse_typed_through_the_command`.
Confidence: HIGH — Verification: read (assertions at 548-557 are refusal asserts).

### F-003 [S4] — Comment claims "the fixture is real"; the dropped path is a nonexistent `/tmp/movie.avi`
Where: apps/desktop/src/shell.rs:787-797
Code:
```
                PathBuf::from("/tmp/notes.txt"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                PathBuf::from("/tmp/movie.avi"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ],
        ) {
            Some(response) => response,
            None => unreachable!("a media file was dropped"),
        };
        // The open path's own honest answer decides (the fixture is
        // real); what is proven here is the FUNNEL, not the decode.
```
What is wrong: `/tmp/movie.avi` is not a committed fixture (the real one lives at `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`); the comment's parenthetical "(the fixture is real)" is false, and the response — an engine file-not-found error — is discarded (`let _ = response;`). The test still proves what its second sentence says (the funnel), but the first sentence is fiction.
Why it matters: a maintainer reading "the fixture is real" will believe the drop path opened real media.
What correct looks like: drop the real fixture path and assert `Ok(Accepted)`, or delete the false parenthetical.
Confidence: HIGH — Verification: read + ls (no /tmp/movie.avi; the committed fixture path used by the two proofs is different).

### F-004 [S4] — The dialog test thoroughly covers a function the shipped wrapper deliberately does not wire
Where: apps/desktop/src/shell.rs:809-840 (subject: `pick_and_open`, part1:329-340)
Code:
```
    fn dialog_picks_dispatch_cancels_noop_and_pending() {
        let mut runtime = ShellRuntime::default();
        // Canceled: a no-op, never an error.
        let mut canceled = ScriptedDialog { answer: Ok(None) };
```
What is wrong: not the test — the seam it exercises. `pick_and_open` has zero production callers: apps/desktop-tauri/src/app.rs:78 documents "`pick_and_open` (native dialog) is NOT wired here: it needs a dialog plugin admission of its own … recorded as a follow-up row in the admission record". The test (correctly, honestly — its `Pending` arm exists for exactly this) proves wiring that no shipped lane reaches.
Why it matters: green coverage of an unwired surface reads as "dialog works in the GUI" in coverage reports; it does not ship.
What correct looks like: fine as a forward-honest seam test, but the admission-record row should be cited in the test doc so the unwired state is visible at the test site too.
Confidence: HIGH — Verification: read + rg (`pick_and_open` callers: shell.rs def+test, app.rs comment only).

## 4. Per-function table (test items; boundary note: `dispatch_routes_observes_and_emits` 415-443 STRADDLES 435 — its head is A01's, its tail asserts 436-443 are mine)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `dispatch_routes_observes_and_emits` (tail) | 436-443 | REAL | — | straddler; the tail is the egress-conserves assert; the head (same "dark" choice) has F-001's flaw — flagged to A01 |
| `config_writes_round_trip_through_dispatch` | 448-487 | SHALLOW | S3 | F-001: default-value round trip voids the storage proof |
| `invalid_config_writes_refuse_typed` | 491-507 | REAL | — | genuine typed negative through real schema validation |
| `engine_events_reach_the_bounded_queue_in_order` | 511-535 | REAL | — | real fixture, order + monotonic drain + conservation |
| `playlist_files_load_the_list_through_the_command` | 540-559 | REAL (name lies) | S4 | F-002 |
| `pacer_proof_pump_once_drives_a_real_session_to_end` | 574-648 | REAL | — | THE 3.B3 proof; baseline :2361 ok; hang-guarded, conserves |
| `frame_channel_bind_is_intercepted_by_the_shell` | 655-668 | REAL | — | interception + one-shot binding |
| `frame_channel_proof_pump_once_ships_luf1_records` | 677-745 | REAL | — | THE w31.1 proof; LUF1 magic asserted per record; baseline green ×2 |
| `pacer_tick_is_one_render_quantum` | 751-760 | REAL | — | 30 625 µs pin, arithmetic re-derived |
| `non_openable_paths_are_noops` | 764-777 | REAL | — | extension-based classification negatives |
| `drops_funnel_through_the_runtime` | 781-804 | REAL (comment lies) | S4 | F-003 |
| `dialog_picks_dispatch_cancels_noop_and_pending` | 809-840 | REAL (unwired subject) | S4 | F-004 |
| `egress_queue_is_bounded_drop_oldest_and_conserves` | 844-870 | REAL | — | direct sink drive, drop-oldest + counters pinned |
| (part1-owned infra used here: `PROOF_PUMP_BOUND`=1024 :392, `ScriptedDialog` :395, `request` :405) | — | REAL | — | A01's range; sane |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| config_writes_round_trip_through_dispatch | **NO for a broken store** (F-001) — yes for broken set/get response shapes | get falls back to the same "dark" default; only the wire shapes are pinned |
| invalid_config_writes_refuse_typed | YES | unknown area/key must refuse InvalidArgument |
| engine_events_reach_the_bounded_queue_in_order | YES | empty queue, unsorted seq, or re-delivery each fail an assert |
| playlist_files_load_the_list… | YES (for what it actually pins) | classification + typed refusal; NOT loading (F-002) |
| pacer_proof… | YES | frozen pacer → no time_ticks/ended; bound breach → explicit hang assert |
| frame_channel_bind… | YES | engine refusal would hit the `unreachable!`; one-shot take asserted |
| frame_channel_proof… | YES | no records / wrong magic / no EOS each fail; content beyond the magic is transport tests' job (desktop/tests/protocol.rs, baseline green) |
| pacer_tick_is_one_render_quantum | YES | two-sided constant pin |
| non_openable_paths_are_noops | YES | classification regression → Some(response) fails is_none |
| drops_funnel… | YES for the funnel (no openable → None; openable → Some) | F-003: response content unasserted |
| dialog_picks… | YES | cancel/pending/picked arms each asserted; Pending payload compared verbatim |
| egress_queue… | YES | exact survivors [3,4] + counters |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "THE card 3.B3 proof — the GUI plays" (test doc 561-573) | shell.rs:561-573 | REAL | baseline :2361 ok; assertions cover Playing/ticks/EOS/Ended |
| "THE w31.1 proof — … LUF1 frame records flow" (670-676) | shell.rs:670-676 | REAL | baseline :2339 + :2298 ok; records>0 + magic per record |
| "the bound is pure headroom" (PROOF_PUMP_BOUND doc, 387-391, part1) | shell.rs:387-391 | REAL-UNPROVEN | measured ~170-pump claim not re-measured here; both proofs finished far under bound in baseline logs (0.05s suite) |
| "the oldest events dropped, order preserved" | shell.rs:860-863 | REAL | [3,4] pin |
| part1 module doc: "every method here runs under `cargo test` today" | shell.rs:31-32 | REAL | all 13 tests green ×2 baseline runs |

## 7. Dead / duplicated / unreachable code
- Nothing dead in MY range (tests). The dead-ish PRODUCTION item is `pick_and_open` (part1, A01's) — tested here, wired nowhere (F-004 evidence).
- No duplication; the two proof tests share structure legitimately (different lanes: events vs frames).

## 8. Dependencies & flags
Test-only: `testkit::{Must, assert_contains}`; no feature gates; no unsafe; no sleeps (the two deadlines are hang guards, asserted only — never pacing inputs). `#[allow]` count in range: zero (the `clippy::panic` allow lives in diagnostics.rs, not here).

## 9. Open questions for the next agent
- A01 (shell.rs#part1): `pick_and_open`/`NativeFileDialog` unwired in the shipped wrapper (app.rs:78) — your dead-code call; and `dispatch_routes_observes_and_emits`'s head has the same "dark"-default weakness as F-001.
- Tauri wave: `frame_channel_proof` proves records flow headlessly; the last mile (records → `tauri::ipc::Channel` → webview paint) has no test outside the unbuildable tauri crate — if any wave gets a GUI build, that is the missing proof.
- settings wave: the same default-value round-trip pattern may exist in other hosts' settings tests (the CLI's `settings_seam_validates_through_the_schema` uses `engine.log-level = "debug"` — check that "debug" is not that key's default).
