<!--FIXV1
agent: FX5-R1
title: remedial — cross-sibling shape reconcile + 65→69 pins
verdict: DONE
changed:
- apps/cli/src/dispatch.rs
- apps/cli/src/host.rs
- apps/cli/src/run.rs
- apps/cli/tests/command_surface.rs
- apps/cli/tests/fx5_a03_host_audio_verbs.rs
- apps/control-server/src/dispatch.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX5/FX5-A02.md
- audit/fix/waves/FX5/FX5-A03.md
- audit/fix/FIX-PLAN.md
- crates/commands/src/catalog/command.rs
- crates/commands/src/catalog/types.rs
- crates/commands/tests/catalog_shape.rs
- crates/audio-engine/src/controls.rs
- apps/cli/src/dispatch.rs
- apps/cli/src/host.rs
- apps/cli/src/run.rs
- apps/cli/tests/command_surface.rs
- apps/cli/tests/fx5_a03_host_audio_verbs.rs
- apps/cli/tests/fx4_a02_host_audio_verbs.rs
- apps/control-server/src/dispatch.rs
- schemas/commands.schema.json
- ui/src/generated/commands.ts
-->

# FX5-R1 — remedial: cross-sibling shape reconcile + the 65→69 count pins

## Approach

FX5-A03 coded the frozen audio-verb contract with guessed field names
(`AudioSetBalance { balance }`, flat `AudioBalance { balance }`, flat
`AudioEq { low_db, mid_db, high_db, preamp_db }`); FX5-A02 landed the
authoritative catalog shapes (`Command::AudioSetBalance { position }`,
`CommandResult::AudioBalance { position: Option<f32> }`,
`CommandResult::AudioEq { eq: Option<AudioEqBands> }`) and A02's report
pre-authorized exactly these renames ("a rename, no architecture" —
A03's own reconciliation list). My job: apply the renames at the seven
compile-error sites, sweep the same files for every other stale-shape
construction/match (semantics preserved — the host still answers the
chain's own read-back, never the request echoed), and move the four
out-of-domain count pins 65→69 with their narration extended.

Verified against the landed catalog before each edit (command.rs /
types.rs): the request shape `AudioSetEq { low_db, mid_db, high_db,
preamp_db }` was ALREADY the agreed shape (no rename needed there —
only the two result constructions and the one balance pattern); the
wire keys (schema `commands.schema.json:913` `"position"`; TS union
`{ kind: "audio-balance"; position?: (number | null) }` and
`{ eq?: (AudioEqBands | null); kind: "audio-eq" }`) confirm the
machine-mode wire edits. The engine seam (controls.rs:160/:198)
confirms `balance() -> Option<f32>` and
`eq_bands() -> Option<(f32, f32, f32, Volume)>` — so the new
constructions are the plain read-back with the fallback removed (A02's
pre-authorization: "`position: controls.balance()` — the Option
read-back, no fallback needed"; after an accepted set the read-back is
`Some` by construction, and `None` would honestly name an absent
stage — the same Option discipline the engine getters carry).

## Changes per file (before → after)

- `apps/cli/src/dispatch.rs:157` — route-arm pattern
  `Command::AudioSetBalance { balance }` → `{ position }`, the call
  `host.audio_set_balance(*balance)` → `host.audio_set_balance(*position)`
  (the host method's own parameter name is untouched — parameter
  names are free, per A02's note). Fixes E0026/E0027.
- `apps/cli/src/host.rs:82` — `AudioEqBands` added to the
  `commands::catalog::types` import (the one new name the nested
  result needs).
- `apps/cli/src/host.rs:1486-1493` (was :1486-1492) — the
  `CommandResult::AudioBalance` construction
  `balance: controls.balance().unwrap_or(balance)` →
  `position: controls.balance()` (the `Option<f32>` read-back; comment
  updated: the plain Option carries the stage truth, `null` only if
  the stage were absent, never a fabricated default). Fixes E0559
  (`AudioBalance` has no field `balance`).
- `apps/cli/src/host.rs:1526-1541` (was :1525-1537) — the flat
  `CommandResult::AudioEq { low_db: low, mid_db: mid, high_db: high,
  preamp_db: preamp.db() }` (fed by the
  `controls.eq_bands().unwrap_or((low_db, …))` request-echo fallback)
  → the nested `CommandResult::AudioEq { eq: controls.eq_bands()
  .map(|(low, mid, high, preamp)| AudioEqBands { low_db: low,
  mid_db: mid, high_db: high, preamp_db: preamp.db() }) }` — the
  tuple's `preamp` is still rendered back to dB from the stored
  linear `Volume`, exactly A03's read-back semantics, now wrapped as
  `Some(applied)` after the accepted set / `None` for an absent
  stage. The fallback destructure line is gone (the Option is the
  honest answer). Fixes E0559 ×4.
- `apps/cli/src/run.rs:962-966` — the pin
  `assert!(out.contains("ok: 65 command(s)"))` → `ok: 69 command(s)`;
  narration appended (same style, count history kept):
  `// 69 since FX5-A02 minted the audio quartet (set/get-balance
  // and set/get-eq, the sound-experience knobs).`
- `apps/cli/tests/command_surface.rs:89-102` — same 65→69 pin with
  the same appended quartet sentence; the doc line "all 64 ids" →
  "all 69 ids" (that line was PRE-EXISTING stale — it still said 64
  while the pin said 65, w31.1's recorded miss; fixed in passing,
  noted here per the honesty contract).
- `apps/control-server/src/dispatch.rs` — :130
  `assert_eq!(listings.len(), 65)` → 69 with the quartet sentence
  appended to the narration comment; :165
  `assert_eq!(commands.len(), 65)` → 69 (the second pin A02's report
  cites); :123 the test fn renamed
  `the_catalog_lists_all_65_commands_from_the_live_spec` →
  `the_catalog_lists_all_69_commands_from_the_live_spec` (the name
  carries the same count the pin asserts — see Risks).
- `apps/cli/tests/fx5_a03_host_audio_verbs.rs` — the sweep, all
  semantics preserved: :105 request `AudioSetBalance { balance:
  position }` → `{ position }` (shorthand); :106 result match
  `{ balance }` → `{ position: applied }` with the assertion
  `assert_eq!(balance, position)` → `assert_eq!(applied,
  Some(position))` (the Option read-back — `Some` by construction);
  :146 and :165 the two flat `AudioEq { low_db, mid_db, high_db,
  preamp_db }` matches → `AudioEq { eq: Some(bands) }` with the
  assertions reading `bands.low_db/mid_db/high_db/preamp_db` verbatim
  (a `None` answer would fall to the existing `unreachable!` arm —
  the regression stays caught); :191/:223 `AudioSetBalance {
  balance: 0.0 }` / `{ balance: -0.5 }` → `{ position: … }`; :255-264
  the bad-value loop `for balance in [1.5, -1.5, NaN]` + `{ balance }`
  → `for position in …` + `{ position }` (assert messages updated);
  :304 the machine-mode script's wire arg `{"balance": -0.5}` →
  `{"position": -0.5}` (the schema's own arg key, schema:913); :341
  the wire assert `"balance":-0.5` → `"position":-0.5`; :345-350 the
  eq wire asserts strengthened from the flat `"lowDb":-12.0`
  substring to `"eq":{"lowDb":-12.0` + an added `"midDb":0.0` —
  pinning the NESTED AudioEqBands byte-shape FX5-A02's schema and
  FX5-A05's landed TS test both already pin (a flat result would now
  fail this test loudly).

rustfmt applied to all six files: edition 2021 for dispatch.rs,
command_surface.rs, fx5_a03_host_audio_verbs.rs and control-server's
dispatch.rs; edition 2024 for host.rs and run.rs (both reject 2021 on
PRE-EXISTING let-chains — host.rs:686, run.rs:874 — the same
exception FX5-A03 recorded). All six verified `--check`-clean at their
formatting edition. No `.unwrap()`/`.expect()`/`unsafe`/TODO markers
added anywhere (one `unwrap_or` request-echo fallback REMOVED); no
`.git`/`Cargo.lock`/dependency touched; no scratch files left behind.

## Test design

No new test fns (`tests: []` — this is a compile/pin fix of the
siblings' files; the assignment's title says so). The regression
coverage is FX5-A03's six existing tests plus FX5-A05's landed
cross-checks, now compiled against and pinning the authoritative
shapes: the in-process read-back tests assert `Some(position)` and
`eq: Some(bands)`; the machine-mode test pins the byte-shape of the
wire (`"position":-0.5`, `"eq":{"lowDb":-12.0,…}`); the four count
pins pin 69. What WOULD have caught the drift earlier — a compile —
is what the orchestrator's central run now provides; FX5-A05's
`fx5_a05_eq_completed_answer_smears_no_player_state` and FX5-A02's
`fx5_a02_{balance,eq}_verbs_round_trip_the_wire` were landed pinning
the authoritative shapes already and will now compile-and-pass
together with the reconciled host.

## Acceptance vs FIX-PLAN

No FIX-PLAN row exists for FX5-R1 (remedial batch; verified: no
FX5-R1 entry in FIX-PLAN.md or wave-FX5.tsv) — the assignment brief
is the spec, and its two deliverables are complete: (1) all seven
orchestrator-collected compile errors fixed at the exact sites, plus
the full sweep of the same three files for stale shapes (a repo-wide
rg sweep confirms no other site constructs `AudioSetBalance`/
`AudioBalance`/`AudioEq` with the old field names); (2) all four
cited count pins moved 65→69 with narration extended in each file's
existing comment style. Semantics preserved throughout: the host
methods still answer the chain's own read-back (never the request
echoed), the preamp still renders back to dB from the stored linear
`Volume`, and the refusals/typed codes are untouched. F1.1c's
volume/EQ closure stays intact at every layer.

## Risks & followups

1. **The control-server test fn rename** (`…all_65_commands…` →
   `…all_69_commands…`): the fn NAME carried the count the pin
   asserts, and leaving "65" in the name over a 69 pin is the same
   lie the pins exist to prevent. Frozen audit-evidence files
   (audit/files/*.audit.md, the baseline cargo-test log) reference the
   old name — they describe audit-time state and are not broken by
   the rename, but any wave gate keyed on that literal would need the
   one-line revert.
2. **Not compile-verified by me** (cargo is banned for agents — the
   box constraint). The fix is mechanically shape-complete: every
   cited site + every swept site now uses the exact landed shapes,
   cross-checked against the schema/TS wire keys and the engine
   getters' signatures. The orchestrator's central compile is the
   oracle (same posture FX5-A03/A02 recorded).
3. The eq/balance set-verb answers are now `Some(applied)` by
   construction with NO request-echo fallback — if the engine seam
   ever answered `None` right after an accepted set, the CLI host
   would answer `eq: null` / `position: null` (honest absence, V3's
   never-a-sentinel rule) where A03's old code echoed the request;
   that is the Option shape A02 landed and FX5-A05 pinned — a
   deliberate sharpening, not a regression, but it changes the
   failure MODE on a (currently impossible) engine inconsistency.
4. `audio.get-balance`/`get-eq` remain unrouted (A02's risk #4
   records them as landed-with-no-consumer) — untouched here, out of
   scope for a shape reconcile.
5. command_surface.rs's doc line "all 64 ids" was already stale
   before this wave (w31.1's miss); now says 69. If the orchestrator
   prefers history-purist comments, it is one line.
