# Solstice v2 — Craft & Flow Redesign Brief

Date: 2026-11-09 · Status: ACTIVE (Lead-approved) · Supersedes the 2026-11-08
first pass wherever they conflict.

## 0. Why v2

The 2026-11-08 Solstice pass landed the right *ingredients* (sunset stage,
sticky notes, washi tape, keycaps, Nunito) but composed them badly. A full
visual audit (browser + vision model, 10 screens) found:

1. **One metaphor, one volume** — the vivid sunset bleeds behind EVERY screen,
   so working screens (settings, playlist, shortcuts) read as "boxes floating
   on a screensaver": unanchored, loud, low-legibility.
2. **Craft defects** — clipped text at viewport edges (keycap strips),
   sun-orb/tape overlapping card edges (z-fighting), 60–90% dead space on
   most screens, staggered panel bottoms, misaligned baselines.
3. **Dev jargon in user copy** — "wired by the app shell (card 4.H1)".
4. **Flow traps** — open-media requires a DOUBLE-CLICK on list items
   (undiscoverable); three track dropdowns whose trigger text reads
   "Disabled" when no media is open; dialogs without scrims or close
   affordances; lexical sort (Bookmark 1, 10, 2, 3).
5. **Illegible keycaps on dense screens** — 10px caps, dark-on-dark in
   places, monospace spill ("BrowserRefresh").

v2 keeps the token pipeline, the honesty rules and the ingredient set, and
re-composes them with a two-tier system.

## 1. The two-tier system (the core change)

**Tier STAGE — emotional moments.** Full-strength sunset
(`html[data-backdrop="vivid"]`), taped notes, big display type, sun art.
Used ONLY by: onboarding, the player first-run empty state, 404, and the
music-mode ambient stage. These screens may be centered, airy, theatrical.

**Tier APP — working moments.** Quiet, veiled backdrop
(`html[data-backdrop="calm"` — new in solstice.css v2): the sunset is a
warm memory at ~88% veil, dense screens sit on structured, opaque-ish
cards anchored to a consistent grid. Used by: settings, playlist,
shortcuts, media-info, network-stream. These screens use full width
responsively, real scrolling, tighter type scale.

**Default veil (no attribute)** stays for the player chrome (moderate
sunset glow through the transparent stage).

Rules:
- A screen sets its tier in a mount effect (`root.dataset.backdrop = …`,
  cleaned up on unmount) — see OnboardingScreen for the pattern.
- Never mix tiers inside one screen.
- Cards on APP tier must visually ANCHOR: aligned tops, a shared max-width
  column (min(1200px, 92vw)), consistent gutters. No floating islands at
  random offsets.

## 2. Craft rules (all screens, non-negotiable)

1. **Nothing clips at the viewport edge.** Keycap strips, labels, and
   footers need safe padding (≥ 24px) and must survive 1440×900 AND
   1024×768. Test both.
2. **Z-discipline for ornaments**: tape sits ON the card's top edge
   (straddling, above the card, below nothing); the sun NEVER overlaps a
   card — it sits fully behind (z-index below) or fully beside it.
3. **Contrast floor**: body text ≥ 4.5:1, large text/labels ≥ 3:1 (WCAG AA).
   Ember ink on ember surface passes; verify muted inks on dark panels and
   fix in screen CSS where the audit found failures (recent-streams label,
   keycaps on shortcuts, "RECENT STREAMS" heading).
4. **Alignment**: every screen has ONE vertical rhythm — headings, cards,
   and toolbars share the same left edge (or center axis). Baseline-align
   mixed controls in one row (text link + primary button).
5. **Dead space is a bug** on APP tier (fill with content, tighten the
   column, or let the page scroll naturally). On STAGE tier, dead space is
   allowed only as symmetric breathing room around a centered composition.
6. **One button language**: primary = accent pill (`.lumen-button--primary`),
   secondary = quiet outline/ghost, destructive = red. Same sizes across
   screens. Never a giant slab out of proportion to its dialog.
7. **Scrolling is honest**: if content exceeds the viewport, the container
   scrolls (visible warm scrollbar); never crop mid-row.
8. **Motion**: `solstice-rise` on card/step entrances only; nothing loops
   except the sun glow; all disabled under `prefers-reduced-motion`.
9. **Keycap legibility**: caps render ≥ 12px (≥ 13px on reference screens),
   `--font-size-micro` never below 11px, modifier text abbreviated
   sensibly (Ctrl/⌥/⇧… no raw "BrowserRefresh" — map media keys to
   friendly labels).
10. **Numbers sort numerically** (bookmarks: 1, 2, 3 … 10).

## 3. Honest-UI copy rules

Keep the project's truthfulness (nothing claimed unbuilt), but speak
user language — no card ids, no internal handoff notes:

- BAD: "Opening playlist files (.m3u) is wired by the app shell (card 4.H1)."
- GOOD: "Open a file or drop it anywhere — playlists arrive with the
  desktop build."
- The screen-reader note may carry one short clause of what's pending,
  phrased for users.

## 4. Flow contracts (must all work in the browser demo shell)

1. **Open media**: single click on a list item OPENS it (selection state
   only exists to enable the Open button for keyboard users; Enter/Space
   on a focused item opens it too). Dialog closes → routes to /player →
   demo transport starts the session (opening → playing, tracks, position).
2. **Empty state CTA** → opens the same dialog (already wired via
   `lumen:open-media`).
3. **Track dropdowns**: when no media, triggers read "—" (not "Disabled"),
   disabled styling, honest tooltip/aria description.
4. **Every dialog**: scrim + Escape + visible Close affordance + focus
   trap + initial focus. Centered in the viewport (optical center, 45%).
5. **Menus (menubar)**: open on click, navigate with arrows, close on
   Escape/outside; accelerators shown right-aligned.
6. **Shortcuts screen**: search filters across all groups; the list
   scrolls; groups collapse/expand.
7. **Settings**: nav and content panel share top + bottom edges; the
   content area scrolls internally when long.
8. **404**: stage tier, one clear CTA home.

## 5. Ownership map (parallel workstreams — hard boundaries)

| Workstream | Owns (exclusive) |
|---|---|
| A — Stage moments | `ui/src/screens/onboarding/**`, `ui/src/screens/not-found/**` |
| B — Content screens | `ui/src/screens/playlist/**`, `ui/src/screens/settings/**`, `ui/src/screens/shortcuts/**`, `ui/src/screens/network-stream/**`, `ui/src/screens/media-info/**` |
| C — Player & dialogs | `ui/src/screens/player/**`, `ui/src/components/appshell/**`, `ui/src/screens/open-media/**` |

Shared files (`ui/src/theme/solstice.css`, `ui/src/i18n/messages.ts`,
`ui/src/App.tsx`, `ui/src/App.test.tsx`, tokens) are Lead-only. Request
changes; do not edit. New i18n keys: add a `// NEWCOPY:` comment in your
own file using an inline fallback and report the strings in your final
message — the Lead wires them centrally.

## 6. Verification bar (per workstream)

- `npx tsc --noEmit` clean; `npx eslint <owned dirs>` no NEW errors;
  `npx vitest run` — own tests updated and green (never weaken asserts;
  update expectations to the new honest DOM).
- Manual browser pass at 1440×900 and 1024×768 (dev server
  `npx vite --port 519x --strictPort`, use your own port): click through
  the flow, screenshot, and self-review against §2 rules before reporting.
- Report format: files changed, tests run + results, screenshots taken,
  NEWCOPY list, limitations.
