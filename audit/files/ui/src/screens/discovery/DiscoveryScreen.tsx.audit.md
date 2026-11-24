# AUDIT: ui/src/screens/discovery/DiscoveryScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 333 / 7 (useDiscoveryMsg, kindKey, volumeRowMenu, openRowMenuOnKey, kindIcon, applyVolumeEvent, DiscoveryScreen) |
| File verdict | REAL (code is real and reachable; the volume DATA lane has no shipped emitter — dead surface) |
| Reachable from shipped entry point? | YES — ui/src/App.tsx:51-53 lazy route + App.tsx:308 `<Route path="/discovery" ...>` -> DiscoveryScreen (both web build and desktop shell serve the same router) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | `rg -n "volume-added" crates/ apps/` -> engine crate only, no app relay; `rg -n "devices" apps/desktop-tauri/src` -> 0 hits; `rg -n "lumen_dispatch" apps/desktop-tauri/src/app.rs` -> :145,:293 (exactly one handler); `sed -n` quote verifications |

## 1. One-paragraph verdict
The screen itself is honestly built: it renders an empty state, 5 of 6
context-menu rows are explicitly `disabled: true` with reasons, and the one
enabled row ("Add file…") dispatches the shell-wide open-media CustomEvent —
a real UI-internal seam, not a backend call. But the master-question answer
for this file is brutal: NOTHING here touches a backend. The volume lane
subscribes to `devices.volume-*` events that no shipped entry point ever
emits — apps/desktop-tauri registers exactly one IPC handler
(`lumen_dispatch`, app.rs:293) and contains zero references to devices
events, so in the shipped desktop build this screen is a permanently empty
list plus a static i18n status line. In the browser/dev shell the
demo-transport FABRICATES two volumes ("Lumen demo media", "Field Kit",
demo-transport.ts:341-343) that this screen then renders as live rows.

## 2. What actually works (verified)
— `rg` reachability: route exists (App.tsx:308). Test file (343 LOC) drives
the real fold through a fake transport seam and asserts DOM: adds/replaces/
removes rows, keyboard ContextMenu/Shift+F10 path, disabled rows — these
tests would fail if the fold or menu broke. Vitest baseline: 1705 passed
(baseline §13), includes this suite.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/discovery/DiscoveryScreen.tsx:229-238 (+ 20-26)
Code:
  useEffect(
    () =>
      client.subscribe((event) => {
        if (event.kind !== "devices.volume-added" && event.kind !== "devices.volume-removed") {
          return;
        }
        setVolumes((table) => applyVolumeEvent(table, event));
      }),
    [client],
  );
What is wrong: the ONLY data source is an event stream no shipped host emits.
apps/desktop-tauri has exactly one invoke handler (`lumen_dispatch`,
app.rs:293) and no devices-event relay (`rg devices apps/desktop-tauri/src`
-> 0 hits). The engine side has the events (crates/commands/src/catalog/
event.rs:294, crates/device-discovery) but nothing connects them to the UI.
Why it matters: the whole /discovery volume lane is dead surface in the
release build — permanently the empty state the file's own comment admits
("Until a host wires the engine's VolumeMonitor into those events, the
honest empty state is the truth this screen shows", :24-26). What correct
looks like: the tauri shell relaying engine devices events on lumen://event,
or the route hidden until a host exists. Confidence: HIGH — Verification:
read + rg over apps/ and crates/.
F-002 [S2 — cross-file, logged here for the screen's surface]
Where: ui/src/client/demo-transport.ts:341-343 (rendered by this screen)
Code:
    this.volumes.push({ id: "demo-media", label: "Lumen demo media", volumeKind: "fixed" });
    this.volumes.push({ id: "usb-fieldkit", label: "Field Kit", volumeKind: "removable" });
    this.emitIn(600, { kind: "devices.volume-added", ...this.volumes[0]! });
What is wrong: in the browser/dev shell (no `__TAURI_INTERNALS__`) this
screen displays two fabricated volumes with no "demo" marker on the rows —
mock data presented as real device mounts. Why it matters: screenshots of
the dev-shell /discovery look like a working device panel while the shipped
desktop build shows the same screen permanently empty. What correct looks
like: a visible demo badge or the demo rows suppressed in production builds.
Confidence: HIGH — Verification: read (demo-transport.ts:318-370).
F-003 [S3]
Where: ui/src/screens/discovery/DiscoveryScreen.tsx:130-146
Code:
    // 5 — Refresh (frame: two-tone resync arrows). The volume table is
    // `devices.volume-*` event-driven — the catalog has no rescan command
    // (a no-op "refresh" would be theater) — disabled.
    { id: "refresh", labelKey: "discovery.menu.refresh", icon: "refresh", iconAccent: true, disabled: true },
What is wrong: 5 of 6 context-menu rows are `disabled: true` — honest, but
it means the ONLY wired action on this screen is UI-internal (open a
dialog), not backend; the lane cannot even be re-polled. Severity S3
(documented, intentional dead controls, limited blast radius).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useDiscoveryMsg | 87-93 | REAL | — | i18n seam, falls back to lane catalog |
| kindKey | 103-114 | REAL | — | exhaustive switch, no default — safe |
| volumeRowMenu | 122-148 | REAL | — | 5/6 rows disabled (F-003) |
| openRowMenuOnKey | 164-179 | REAL | — | synthesizes contextmenu via getBoundingClientRect — real path |
| kindIcon | 186-197 | REAL | — | exhaustive switch |
| applyVolumeEvent | 204-217 | REAL | — | pure fold, exported, test-pinned |
| DiscoveryScreen | 219-333 | REAL | — | renders; data lane dead in shipped build (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| applyVolumeEvent fold tests (:74-99) | YES | pure-function asserts on real output maps |
| folds live devices.volume-added (:167) | YES | emits through fake transport, asserts rendered rows |
| no-seam rows render honestly disabled (:286) | YES | queries disabled state of the 5 rows |
| Add file… fires open-media event (:269) | YES | spies on window event |
Mock is the transport seam (legit); no test asserts a real backend event
arrives — none exists (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "card 7.D1, issue #42 — device-discovery panel" | :1 | SHALLOW | UI real, host wiring absent (F-001) |
| "never a dead search button that pretends" | :29-30 | REAL | network lane is a static <p>, no button |

## 7. Dead / duplicated / unreachable code
Data lane is runtime-dead in the release lane (F-001); no dead code in-file.
VOLUME_ROW_MENU built once at module scope (no per-render garbage) — fine.

## 8. Dependencies & flags
Imports ClientProvider/useClient (bridge seam), generated/commands
VolumeKind, i18n w26.b12 + w26.c6 keys, OPEN_MEDIA_EVENT from open-media
screen. No feature flags.

## 9. Open questions for the next agent
- Does the tauri app relay ANY engine event to lumen://event (ui/src/client/
  tauri-transport.ts subscribe path)? If not, every event-driven screen is
  dead surface in the desktop build, not just this one.
