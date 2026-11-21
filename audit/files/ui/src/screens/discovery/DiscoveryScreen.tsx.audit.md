# AUDIT: ui/src/screens/discovery/DiscoveryScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 333 / 8 (DiscoveryScreen, applyVolumeEvent, volumeRowMenu, kindKey, kindIcon, useDiscoveryMsg, openRowMenuOnKey, VolumeRow) |
| File verdict | SHALLOW (volume lane runs ONLY on DemoTransport's fabricated fixtures; against the real engine the screen is permanently the empty state — the file's own header admits "a museum piece") |
| Reachable from shipped entry point? | YES as web-UI route: App.tsx:49-52 lazy-loads `/discovery` → DiscoveryScreen (baseline #22 build). Real shell: no GUI binary (W14); and even built, the volume lane receives nothing — no host emits `devices.volume-*` (crates/playback-engine/src/events.rs:106-107 "wired-by-later 7.D1"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "volume-added\|VolumeMonitor" crates apps --type rust` (NO engine/host emitter — events.rs marks both devices events wired-by-later); `sed -n '330,380p' ui/src/client/demo-transport.ts` (the ONLY emitter: demo choreography seeding "Lumen demo media"/"Field Kit" + subscribe-time replay); `ls crates/device-discovery/src` (ssdp.rs exists — the network-lane verdict is truthful); `grep -n "it(" DiscoveryScreen.test.tsx` (15 tests incl. XSS-safety + event folding); `grep -n discovery ui/src/App.tsx` (route wired); baseline #22/#23 |

## 1. One-paragraph verdict
The most honest screen in the wave: every capability claim in its header is checkable and true (SSDP searcher exists at crates/device-discovery/src/ssdp.rs but is unexposed; no browse/rescan/info command exists so five of six context-menu rows render disabled; the one real action — "Add file…" — fires the shell-wide open-media event from both the empty state and the menu; untrusted OS volume labels render as text nodes with a test proving it). The cost of that honesty: in any real shell the volume lane can NEVER show a row, because `devices.volume-added/removed` is marked "wired-by-later 7.D1" in the engine's own event table and nothing emits it — the only emitter in the repository is DemoTransport's startup choreography, which fabricates two volumes so the dev shell has something to show. The entire row machinery (~150 lines: applyVolumeEvent, kindIcon/kindKey, VOLUME_ROW_MENU, keyboard-synthesized context menu, the row JSX) is exercised exclusively by tests and fake demo data.

## 2. What actually works (verified)
— verified by `rg` over crates/apps → zero production emitters for `devices.volume-*`; `crates/playback-engine/src/events.rs:106-107` documents both as "wired-by-later 7.D1" — the header's "Until a host wires the engine's VolumeMonitor…" is a true statement about HEAD, not an excuse.
— verified by `ls crates/device-discovery/src` → `ssdp.rs`, `monitor.rs`, `volume.rs` exist — the network lane's "searcher exists … but no command exposes it" verdict matches the tree.
— verified by DiscoveryScreen.test.tsx (baseline 1705-pass): `applyVolumeEvent` fold semantics (re-add replaces, unknown-id removal no-op, insertion order), V7 untrusted-label-as-text, 6-row pinned menu order, empty-state CTA fires OPEN_MEDIA_EVENT, non-devices events ignored.
— verified by App.tsx:49-52 → the route is lazy-loaded; e2e visual baselines include `discovery-*.png` (route renders in committed, never-diffed baselines).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/discovery/DiscoveryScreen.tsx:204-217, 240-315 (the volume lane) vs crates/playback-engine/src/events.rs:106-107 and ui/src/client/demo-transport.ts:337-345
Code:
```
//! | `devices.volume-added` | wired-by-later 7.D1 (device-discovery emits from real mount-table diffs; the engine-side relay joins with host wiring) | n/a |
```
What is wrong: the volume lane's entire interactive surface (rows, per-kind glyphs, the 6-row context menu, the Shift+F10/ContextMenu keyboard path) is unreachable against the real engine — no production code emits `devices.volume-added/removed` (events.rs:106-107 marks both wired-by-later; rg over crates/apps finds emitters only in playback-engine's own event-table plumbing and tests). The single live emitter is DemoTransport's choreography (demo-transport.ts:337-345), which fabricates `demo-media`/`usb-fieldkit` volumes at 600ms/1200ms and replays them to new subscribers. So: real shell → permanent empty state; browser dev shell → two fake rows.
Why it matters: this is the wave's question "is any UI element wired to a real backend?" answered for /discovery: NO, by design, with the gap honestly labeled — but ~150 lines of tested row machinery have zero production-reachable inputs, and the demo lane trains users/developers on data no host can produce.
What correct looks like: wire device-discovery's VolumeMonitor → host events (the 7.D1 card), or until then mark the row machinery `@deprecated`-in-comment the way the header already marks the capability, and keep the demo seeding explicitly badged (the AppShell demo badge partially covers this).
Confidence: HIGH — Verification: read + rg emitter census + demo-transport read.

### F-002 [S4]
Where: ui/src/screens/discovery/DiscoveryScreen.tsx:261-262
Code:
```
              <span className="discovery__count-num">{String(rows.length)}</span>{" "}
              {t("discovery.volumes.count-suffix")}
```
What is wrong: number + count-suffix pluralization debt — identical pattern to PlaylistPanel.tsx:176-178 (cross-ref my W15-A06 PlaylistPanel audit F-004); three such sites exist in i18n/messages.ts. Untranslatable beyond English-class plurals.
Why it matters: same as PlaylistPanel F-004 — cementing a broken i18n seam into new screens while W11 records the localization crate as orphaned.
What correct looks like: ICU plural at the messages layer.
Confidence: HIGH — Verification: read + messages.ts grep.

### F-003 [S4]
Where: ui/src/screens/discovery/DiscoveryScreen.tsx:292-296
Code:
```
            <ul
              className="discovery__volume-list"
              aria-label={t("discovery.volumes.live")}
              aria-live="polite"
```
What is wrong: `aria-live="polite"` on the whole `<ul>` announces every mutation of every child — a volume re-add (replace) or removal announces the full row text, and on the demo choreography boot the screen talks twice at 600ms/1200ms. A finer live region (announcing just "added X"/"removed X" text) is the standard pattern; region-level live on a list also re-announces on any re-render that replaces nodes (React keys are stable, so in practice only real adds/removes — mitigated).
Why it matters: chatty live regions are the classic screen-reader annoyance; blast radius limited to AT users of a lane that never populates in real shells (F-001) — hence S4, not S3.
What correct looks like: a visually-hidden status line with the change message, list not live.
Confidence: MEDIUM — Verification: read; announcement verbosity UNVERIFIED (no AT run in evidence).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useDiscoveryMsg` | 87-93 | REAL | — | lane-catalog fallback pattern; no literals (doc 07 §6.5 honored — spot-checked all t() keys exist in laneCatalog/messages) |
| `VolumeRow` (interface) | 96-100 | REAL | — | exact wire payload shape |
| `kindKey` | 103-114 | REAL | — | exhaustive switch, no default — TS ensures all VolumeKind cases |
| `volumeRowMenu` | 66-148 | REAL | S3 | 6 rows, 5 honestly disabled; frozen module-scope instance (VOLUME_ROW_MENU :151) — no per-render rebuild |
| `openRowMenuOnKey` | 164-179 | REAL | S3 | ContextMenu/Shift+F10 → synthesized contextmenu at row's bottom-left through the REAL open path; plain F10 honest no-op; unreachable in real shells (F-001) |
| `kindIcon` | 186-197 | REAL | — | pinned Icon.svg vocabulary, no invented geometry (icons verified present in icons.tsx registry: folder/open-folder/open-disc/network-stream) |
| `applyVolumeEvent` | 204-217 | REAL | — | pure fold; re-add replaces; exported for tests; correct Map semantics |
| `DiscoveryScreen` | 219-333 | SHALLOW | S3 | subscribe-only seam (no dispatch — nothing to fail); empty state CTA = the one real action; network lane = static truthful verdict; count-suffix F-002; aria-live F-003 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| applyVolumeEvent fold tests (77-109) | YES | pure-function exact assertions |
| "renders the honest initial state … sends nothing" (131) | YES | asserts no dispatch + empty state — catches fabricated snapshots |
| V7 untrusted label test (223) | YES | injects markup-shaped label, asserts text rendering |
| 6-row pinned order (261) + Add file… seam (269) | YES | order + event assertions |
| all tests | NO for real-engine reachability | MockTransport emits devices events no host ever will (F-001) — tests prove the folding logic, not the lane's real-world liveness |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the engine's SSDP/UPnP searcher exists (crates/device-discovery, src/ssdp.rs) but no command exposes it to the UI yet" | DiscoveryScreen.tsx:27-30 | REAL | ssdp.rs present; catalog (generated/commands.ts) has no discovery command — verified |
| "there is no devices.list command in the catalog, so no fabricated initial snapshot is possible" | DiscoveryScreen.tsx:21-24 | REAL | commands.ts grep: no devices.list |
| "volume labels are untrusted OS text (V7): … never interpolated into markup" | DiscoveryScreen.tsx:31-34 | REAL | rendered as {row.label} text node; test :223 |
| "a working screen whose working parts cannot work is a museum piece" (the screen's own framing) | DiscoveryScreen.tsx:45-46 | REAL (self-aware) | F-001: exactly the current state — empty-state CTA is the only working part in a real shell |

## 7. Dead / duplicated / unreachable code
Unreachable-as-fed: the whole volume-row surface (F-001) — live code, real tests, zero production inputs. `applyVolumeEvent` exported but consumed only by tests + screen.

## 8. Dependencies & flags
Imports 12 local modules (all present); no feature flags; depends on Card/SectionHeader/ListRow/ListEmpty/ContextMenu primitives (audited elsewhere in this wave: list.css, menu.css mine).

## 9. Open questions for the next agent
- Who owns the 7.D1 "wired-by-later" card — is there an ETA recorded anywhere in docs, or is device-discovery (crate exists, monitor.rs/volume.rs implemented?) permanently unwired? If monitor.rs is REAL but unconnected, that's an S2-grade integration gap for the engine wave (implemented-but-unreachable engine code).
- DemoTransport's subscribe-time replay (demo-transport.ts:361-371) mutates `this.seq` — does the seq ever collide with the clock's emissions across dispose/re-subscribe? (demo-transport is not my file; the choreography is my lane's only feeder.)
