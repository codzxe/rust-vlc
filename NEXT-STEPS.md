# NEXT-STEPS — Task 7-g: 4 h soak baseline

**STATUS: BLOCKED in this sandbox — the run is NOT in flight. Do NOT trust any
process from the 7-g session; there is none alive.**

Branch `wip/w7-g` (from main @ ce9cc4a). Gate refs: `docs/gates/phase3-exit.md`
§1 soak row, §4 gap 3, §9 caveat 2.

## The verified invocation (from the harness source)

`crates/playback-engine/tests/soak/main.rs:17-20` documents the nightly tier:

```
SOAK_MINUTES=240 cargo test -p playback-engine --test soak --release
```

Knobs (main.rs:12-16): `SOAK_MINUTES` (default 2 = CI-short), `SOAK_SAMPLE_MS`
(default 1000), `SOAK_REPORT` (unset = report only goes to stdout/captured log;
**set it** so the report lands as a file — main.rs:316-320).

## What this session did (2026-10-23 ~20:55–21:00 UTC)

1. Disk check: 3.8 GB free — fine, no incremental-cache cleanup needed.
2. Launched the run detached (PID 8541, start 2026-10-23T20:56:14Z):
   `setsid nohup env SOAK_MINUTES=240 SOAK_REPORT=<worktree>/soak-4h-report.md
   cargo test -p playback-engine --test soak --release > <worktree>/soak-4h.log 2>&1 &`
   → it compiled 8 crates, then was **silently reaped** (log stops mid-build,
   no error, no OOM — 3.4 GB was available).
3. Diagnosed why (see blocker). Removed the heartbeat probe; kept the dead
   `soak-4h.log` as evidence (untracked).

## THE BLOCKER (measured, not guessed)

This sandbox **reaps every background process when the tool call that spawned
it ends**:

- Heartbeat probe (`setsid nohup bash -c 'while true; do date >>hb.txt; sleep 5; done' &`)
  died the instant its call ended: hb.txt stops 5 s after launch, `/proc/<pid>` gone.
- `nohup` + `setsid` + orphan-reparenting did **not** save the soak process.
- The shell PID changes between tool calls (10374 → 10440) — no persistent
  process context to host a 4 h child.
- Bash tool max timeout = 10 min < 240 min, and consecutive calls cannot chain
  a live process across the inter-call gap.
- No escape hatch: no `at`/`batch`, no `crontab`, no systemd (`systemd-run`
  fails: PID 1 is not systemd).

=> A 240-min continuous run cannot be hosted from this agent sandbox.
   (Shared-target cargo lock is NOT the issue — cargo waits, never fails.)
   Chunked restarts are INVALID for this metric: the harness fits one
   least-squares slope over a single measured window after in-run warmup;
   a restart resets both.

## Successor instructions

1. **First test your own runner**: launch the heartbeat one-liner above, make
   two separate tool calls, confirm hb.txt keeps growing ≥60 s. If it dies
   too, do not bother launching the soak — go to step 4.
2. If (and only if) your runner survives: launch exactly —
   ```
   cd /home/z/my-project/worktrees/w7-g
   export PATH="$HOME/.cargo/bin:$PATH"
   nohup env SOAK_MINUTES=240 SOAK_REPORT=/home/z/my-project/worktrees/w7-g/soak-4h-report.md \
     cargo test -p playback-engine --test soak --release \
     > /home/z/my-project/worktrees/w7-g/soak-4h.log 2>&1 &
   ```
   Release build from this worktree takes minutes (shared target reuses
   registry deps only). Record PID + start time here when you do.
3. At completion (+4 h): `tail soak-4h.log` → expect `test result: ok`
   (asserts live-bytes slope < 50 MB/h ceiling + D-6 liveness; the 5 MB/h
   figure is `[PROPOSAL]` — reported, never asserted). Verify
   `soak-4h-report.md` exists and its measured-window slope is stated.
4. If blocked (expected): **escalate to Lead** — the 4 h baseline needs a real
   scheduled host (CI nightly job), which `soak/main.rs:19-20` and
   `phase3-exit.md` already flag as a Lead/handoff item.
5. Either way, at completion: copy report → `docs/perf/soak-4h-20261024.md`,
   raw log → `docs/perf/raw/soak-4h-20261024.log`, commit, append the worklog
   entry (`---`, `Task ID: 7-g`, Work Log, Stage Summary incl. the D-10
   block: MEASURE? / NOT tested? / What breaks? / Who hits it first?).

## Artifacts in this worktree

- `NEXT-STEPS.md` (this file) — committed.
- `soak-4h.log` — the reaped attempt's log (8 `Compiling` lines, untracked
  evidence; overwrite it on relaunch).

<<<<<<< HEAD
## 4.B3 — menu primitives (2026-10-24, session 1)

Card 4.B3 DONE for this session: `ui/src/components/menu/**` — MenuBar,
Dropdown/Select, ContextMenu, Tooltip + shared MenuPopup/MenuItem core, all
rendering from doc 05 §8.3 data (`{id, labelKey, accelerator?, submenu?,
disabled?, checked?}` via the `resolveLabel`/useMsg seam; accelerators render
verbatim from keymap data). WAI-ARIA model: roving tabindex (doc 08 §4.5),
arrow/Home/End nav, submenu ArrowRight/hover-intent (150/100 ms), Escape
close + focus restore, Tab focus trap, typeahead in Select, right-click
coordinates in ContextMenu, tooltip delays 500/150 ms. Portal-free absolute
positioning (doc 08 specifies no portals). 38 new tests, ui suite 120/120,
tsc strict clean, eslint 0 errors.

Remaining for a successor:
1. Token gap: popup surfaces need a translucent rgba token for doc 08 §115
   ("#191919 @ 0.95") — currently solid `--color-surface` (states.md notes).
2. ContextMenu coordinates unclamped against viewport (no layout in jsdom) —
   clamp when composed into real screens.
3. Menubar typeahead omitted (APG-optional; Select has it).
4. Visual snapshots (card `+SB` AC) await 4.G1's screenshot harness — states.md
   carries the state tables + design provenance (context menu rx10, items 48px,
   hover `#1F1F1F` rx6, dropdown trigger rx6/inner rx4).
5. Compose app MenuBar from `layouts/menus.toml` + keymap binding-id
   accelerators when those config files land (doc 05 §8.1/§8.3).
=======
---

## Task 4.B4 — surface/list/icon primitives (agent P4-B4)

**Status: COMPLETE (core scope).** Worktree `wip/p4-b4`, based on main @ 91e1f15.
Gates: `tsc --noEmit` ✓ · `eslint` clean ✓ · vitest **117/117** (35 new:
Surface 11, List 16, Icon 6).

### Shipped (`ui/src/components/{surface,list,icon}/**`)

- **Dialog/Sheet** (`surface/`): focus trap (Tab/Shift+Tab cycle taken over
  explicitly — deterministic in jsdom *and* browsers), Escape closes, focus
  restores to opener, `aria-modal` + `role=dialog` + `aria-labelledby`
  (synthesized sr-only title when no `labelledBy`). Sheet = edge-anchored
  variant (`top/right/bottom/left`), slide-in honoring reduced-motion.
- **Tabs**: `tablist/tab/tabpanel`, roving tabindex, arrows per orientation +
  Home/End + wrap, selection-follows-focus, full aria wiring.
- **ScrollArea**: labelled native scroll region; thin token-styled scrollbars
  (`scrollbar-width: thin` + webkit pseudos) — no fake physics.
- **VirtualList/VirtualTable**: hand-rolled windowing in `list/range.ts`
  (`computeRange`, pure + unit-tested), fixed `itemHeight` + overscan +
  absolute rows in a `count × itemHeight` spacer. **AC proven: 10k rows →
  30 DOM row nodes (< 100 asserted), spacer 280000px, contiguous window with
  no gaps after scroll-jump, `scrollToIndex` (start/center/end, clamped).**
  Table adds `table/row/columnheader/cell` roles + sticky head + aria-rowcount.
- **Icon**: 32-name registry — ONLY names doc 08 §1.5/§1.6 actually names
  (transport, volume mute/low/med/high, customize-panel catalog, menu
  accelerators). 1.5 stroke, `currentColor`, size prop, decorative-by-default,
  state variants via `.lumen-icon-host*` CSS classes (§4.4).

### Known gaps / next actions

1. **Icon glyphs are provisional geometry** (TODO [VERIFY-DESIGN] in
   `icon/icons.tsx`): `design/figma/Icon.svg` layer ids are anonymous
   ("Frame 1_17"), so names→paths can't be attributed mechanically yet. Next
   agent: mine real paths from Icon.svg/Controls.svg (318 stroke-width-1.5
   elements available) or get named exports from design; also emphasis
   variants (stroke 2/3) are unimplemented.
2. Dialog has **no portal** (renders in place, fixed-position scrim). Fine for
   current hosts; revisit if stacking contexts clip it (DialogHost overlay is
   a doc 08 §4.1 `[PROPOSAL]`).
3. Virtualization is fixed-height only (per doc). Variable heights / grid
   virtualization not needed by any current screen — do not add speculatively.
4. `scrollToIndex` is prop-driven (target object), not ref-imperative; if
   4.C*/4.D* screens need programmatic control mid-gesture, wrap in
   `useImperativeHandle`.
>>>>>>> wip/p4-b4
