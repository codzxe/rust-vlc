import { lazy, Suspense } from "react";
import { Navigate, Outlet, Route, Routes, useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { useEffect } from "react";
import { AppShell } from "./components/appshell/AppShell";
import { PlayerScreen } from "./screens/player";
import type { PlayerVariant } from "./screens/player";
// Card 4.F1 first-run gate: /onboarding is the landing until the wizard is
// finished or skipped; after that, normal routing. The onboarding MODULE
// stays eager (not lazy): RequireOnboarding is the app-wide first-run gate —
// it must render without any chunk load, and it lives in the same module as
// OnboardingEntry (w26.k20 code-splitting decision).
import { OnboardingEntry, RequireOnboarding } from "./screens/onboarding/OnboardingScreen";
import { isDesktopShell } from "./client/app-transport";
import { useMsg, type MessageKey } from "./i18n/messages";
// w26.k19 (CONTRACT §4.5): the document-title template — resolved
// seam-first (registered catalog value once the Lead merges the key, lane
// copy until then; same pattern as k17's unreachable note).
import { keys as k19Keys } from "./i18n/keys/w26.k19";

// w26.k20 (j20 P2-3) — route-level code splitting. The whole app previously
// shipped as ONE 853 kB chunk (Vite's >500 kB warning fired on every build).
// Split: every non-player route becomes a lazy chunk via React.lazy; the
// player stays EAGER by design — it is the home screen (contract §1), the
// first thing a completed profile sees, and by a wide margin the largest
// screen, so making it lazy would just move the wait to app start. The
// onboarding module stays eager for the gate (above). Shared modules
// (components, i18n catalog, tokens) stay in the entry chunk — only screen
// code moves. The lazy() indirection maps each module's NAMED export onto
// the default export React.lazy expects (no change to the screens' public
// APIs — contract §4.3). Suspense boundaries: one inside GatedShell (the
// app chrome persists while a screen chunk loads) and one around <Routes>
// (covers the ungated /dev/tokens). fallback is null — no i18n copy needed
// for a sub-100 ms local chunk fetch, and the desktop shell loads from disk.
const PlaylistScreen = lazy(() =>
  import("./screens/playlist").then((m) => ({ default: m.PlaylistScreen })),
);
const MediaInfoScreen = lazy(() =>
  import("./screens/media-info").then((m) => ({ default: m.MediaInfoScreen })),
);
const SettingsScreen = lazy(() =>
  import("./screens/settings/SettingsScreen").then((m) => ({ default: m.SettingsScreen })),
);
// App-shell workstream (2026-11-08): real screens for the former placeholder
// routes + persistent navigation (AppShell).
const NetworkStreamScreen = lazy(() =>
  import("./screens/network-stream/NetworkStreamScreen").then((m) => ({
    default: m.NetworkStreamScreen,
  })),
);
// Discovery panel (card 7.D1, issue #42): the device-discovery surface —
// live devices.volume-* events + the honest network-lane verdict.
const DiscoveryScreen = lazy(() =>
  import("./screens/discovery/DiscoveryScreen").then((m) => ({
    default: m.DiscoveryScreen,
  })),
);
// Card 7.E3 (issue #44): the conversion screen — target picker limited to
// the engine's real capability (WAV record / MP4-remux), job queue wired
// to the command-catalog capability probe.
const ConvertScreen = lazy(() =>
  import("./screens/convert/ConvertScreen").then((m) => ({ default: m.ConvertScreen })),
);
const ShortcutsScreen = lazy(() =>
  import("./screens/shortcuts/ShortcutsScreen").then((m) => ({ default: m.ShortcutsScreen })),
);
const NotFoundScreen = lazy(() =>
  import("./screens/not-found/NotFoundScreen").then((m) => ({ default: m.NotFoundScreen })),
);
const TokenSandbox = lazy(() =>
  import("./tokens/TokenSandbox").then((m) => ({ default: m.TokenSandbox })),
);

// Product flow (contract §1 — ONE flow; doc 08 §1.6 screen inventory — every
// route is a real screen; the placeholder routes are gone since the
// 2026-11-08 app-shell workstream).
//
//   first run (onboarding store absent / complete ≠ true — see
//   screens/onboarding/onboardingState.ts, persisted as lumen.onboarding.v1)
//     /            →  /player  →  APP GATE  →  /onboarding (wizard)
//     w26.i20 (h20 top-5 #3): the first-run gate wraps the WHOLE shell, not
//     just the player family — pre-onboarding, EVERY AppShell route
//     (/player, /player/music, /miniplayer, /playlist, /media-info,
//     /settings/:area — all 13, /discovery, /convert, /keyboard-shortcuts,
//     /open-network-stream, the 404) redirects into the wizard; only
//     /onboarding itself and /dev/tokens (dev fixture) render ungated.
//     Onboarding IS the first-run gate for the product (contract §1); a
//     hostile first-run user can no longer walk straight into settings or
//     the dialogs. The e2e suites + dev-shots keep working unchanged: they
//     seed lumen.onboarding.v1 with complete:true (the same shape since
//     card 4.F1 — never touched by this change).
//
//   after finish or skip (complete = true, persisted)
//     /            →  /player — the home; the player empty state is the
//                    onboarding-style hero, never a blank void (W25 spec §2)
//     /player/music + /miniplayer — the same player screen, route-selected
//                    variant (data-variant); every open-media CTA dispatches
//                    the shared CustomEvent exactly ONCE (the player screens'
//                    own affordances fire it; the AppShell dialog listens).
//                    The shell keeps its ONE chrome (banner + rail +
//                    playlists sidebar) on music/mini even though
//                    Player - Music.png and Miniplayer.png depict rail-less
//                    floating cards — h20 hops 8–9 called the tradeoff
//                    "defensible but undocumented"; this is the
//                    documentation (w26.i20): per-route shell surgery would
//                    strand navigation (no rail = no way back from
//                    /miniplayer) and break the music-mode centering math
//                    that keys on (rail 52 + dock) — the product stays ONE
//                    shell (h20: "one shell, one rail … genuinely coherent").
//     /playlist, /media-info, /settings/:area (13 areas; /settings itself
//                    redirects to /settings/global), /discovery, /convert,
//     /keyboard-shortcuts, /open-network-stream, /dev/tokens (dev fixture),
//     *            →  404
//
//   ?onboarding=1 — test/screenshot bypass of the first-run gate, honored on
//     ANY shell route (w26.i20 extended it from the player family to the
//     whole shell). Per-navigation only — nothing is written to the
//     onboarding store, so the very next param-less navigation is gated
//     again. dev-shots does NOT need it (it seeds the store); player shots
//     that used it keep working. w26.k19 (j19 J19-8): DEV BUILDS ONLY —
//     the bypass is a dev/test affordance and no longer ships in
//     production bundles (see useOnboardingBypass).
//
//   provideMain (single main landmark per route, kept aligned with
//     e2e/a11y.spec.ts): the shell renders the <main> landmark ONLY for
//     screens that do not render their own — playlist, settings and
//     media-info ship their own <main>, so their layout passes
//     provideMain={false} and the page keeps exactly ONE main landmark
//     (a nested <main> would be invalid HTML and a duplicate landmark).

/**
 * True for the browser dev shell: the app-layer transport probe
 * (client/app-transport.ts — the same one main.tsx uses) selected the
 * DemoTransport simulation there, badged "Demo" by the AppShell. The
 * desktop shell runs the real TauriTransport and renders no badge.
 */
const DEMO_SHELL = !isDesktopShell();

/** Route path for a player variant (the URL is the variant's source of truth). */
function playerVariantPath(variant: PlayerVariant): string {
  if (variant === "miniplayer") return "/miniplayer";
  if (variant === "music") return "/player/music";
  return "/player";
}

/**
 * `?onboarding=1` on the current location: the first-run gate is treated as
 * satisfied for this navigation (see the flow map above). Read per render —
 * the override never touches the persisted onboarding store. w26.i20:
 * honored by the shell layout gate, so it now bypasses on every route, not
 * just the player family. w26.k19 (j19 J19-8): the bypass is DEV-ONLY — a
 * test/screenshot affordance must not ship as product surface. The dev
 * server, vitest and dev-shots all run DEV builds (import.meta.env.DEV is
 * true there — Surface.tsx gates its dev-only warning the same way), so no
 * committed test or harness changes behavior; production builds ignore the
 * param.
 */
function useOnboardingBypass(): boolean {
  const [searchParams] = useSearchParams();
  return import.meta.env.DEV && searchParams.get("onboarding") === "1";
}

/**
 * The first-run gate around the WHOLE shell (w26.i20, h20 top-5 #3 + the
 * contract §1 story: onboarding is the gate for the product, not five
 * routes). Until the wizard is finished or skipped, every AppShell route
 * redirects to /onboarding (RequireOnboarding — the onboarding lane's gate
 * component, reactive to the same lumen.onboarding.v1 store the wizard
 * writes); `?onboarding=1` keeps its per-navigation bypass for tests and
 * screenshots. Used for both shell layout groups below (provideMain true
 * and false) — the gate is route-tree-level, so screens added to either
 * group are gated automatically; nobody can opt a route out by forgetting
 * a wrapper.
 */
function GatedShell({ provideMain = true }: { provideMain?: boolean }) {
  const bypassGate = useOnboardingBypass();
  // The Suspense boundary sits INSIDE the shell so lazy route chunks load
  // under the persistent app chrome (rail/topbar stay painted; only the
  // screen region is briefly empty) — w26.k20 code splitting.
  const shell = (
    <AppShell demo={DEMO_SHELL} provideMain={provideMain}>
      <Suspense fallback={null}>
        <Outlet />
      </Suspense>
    </AppShell>
  );
  return bypassGate ? shell : <RequireOnboarding>{shell}</RequireOnboarding>;
}

/**
 * Player routes. The variant is CONTROLLED by the route (persisted-variant
 * state stays for host-owned free-floating windows); keymap toggles and the
 * miniplayer expand/close affordances call onVariantChange, which routes.
 * w26.i20: the first-run gate moved UP to the shell layout (GatedShell
 * wraps every AppShell route) — PlayerRoute owns variant→URL routing only.
 */
function PlayerRoute({ variant }: { variant: PlayerVariant }) {
  const navigate = useNavigate();
  // No onOpenMedia here: the player's own affordances (StageOverlays hero,
  // canvas context menu, ControlBar app menu) already dispatch the shell-wide
  // `lumen:open-media` CustomEvent — passing a re-dispatching callback made it
  // fire TWICE per CTA (a13 open issue 4, fixed w26.d1). The prop stays on
  // PlayerScreen's public API for hosts that need the composition seam.
  return (
    <PlayerScreen
      variant={variant}
      onVariantChange={(next) => navigate(playerVariantPath(next))}
    />
  );
}

/** Route → document.title (w26.k19, j19 J19-7 — WCAG 2.4.2 Page Titled in
 * spirit for an SPA): before this, NOTHING in ui/src wrote document.title,
 * so every route's tab and history entry read "Lumen" (index.html) — a
 * screen-reader user with two windows could not tell /player from
 * /settings/privacy. Titles compose EXISTING screen msgids (nav labels,
 * screen headings, the b01 settings window-title family) through the
 * document-title template — no per-screen copy was invented. The player
 * variants (music/miniplayer) share "Player": one product screen, three
 * routes. Unknown paths render the 404 screen, so they get the 404 title. */
const ROUTE_TITLES: Readonly<Record<string, MessageKey>> = {
  "/player": "nav.player",
  "/player/music": "nav.player",
  "/miniplayer": "nav.player",
  "/playlist": "nav.playlist",
  "/media-info": "nav.media-info",
  "/keyboard-shortcuts": "shortcuts.title",
  "/open-network-stream": "stream.title",
  "/discovery": "discovery.title",
  "/convert": "convert.title",
  "/onboarding": "onboarding.welcome.title",
};

/** /settings/:area → the b01 window-title family ("Global Settings",
 * "Audio Defaults", …). Unknown areas fall back to plain "Settings" — the
 * same grammar as SettingsScreen, which never 404s an area. */
const SETTINGS_AREA_TITLES: Readonly<Record<string, MessageKey>> = {
  global: "settings.window-title.global",
  interface: "settings.window-title.interface",
  privacy: "settings.window-title.privacy",
  playback: "settings.window-title.playback",
  video: "settings.window-title.video",
  audio: "settings.window-title.audio",
  subtitles: "settings.window-title.subtitles",
  controls: "settings.window-title.controls",
  library: "settings.window-title.library",
  network: "settings.window-title.network",
  performance: "settings.window-title.performance",
  recording: "settings.window-title.recording",
  extensions: "settings.window-title.extensions",
};

/** The title key for a pathname: the exact map, the settings-area family,
 * the dev fixture (bare app name), or the 404 heading for anything unknown.
 * Redirect-only paths (/, /settings) never render — their destination
 * re-resolves on the navigation they perform. */
function routeTitleKey(pathname: string): MessageKey {
  const exact = ROUTE_TITLES[pathname];
  if (exact !== undefined) return exact;
  if (pathname.startsWith("/settings/")) {
    return SETTINGS_AREA_TITLES[pathname.slice("/settings/".length)] ?? "nav.settings";
  }
  if (pathname === "/dev/tokens") return "app.name"; // dev fixture — bare name
  return "not-found.heading"; // the * route: unknown paths 404
}

/** Keeps document.title in sync with the route (see ROUTE_TITLES). Runs on
 * every location change (and re-render — msg identity changes with the
 * active catalog, so a locale switch re-titles too). Returns null; it is
 * render-order-independent chrome, not layout. */
function DocumentTitle(): null {
  const msg = useMsg();
  const { pathname } = useLocation();
  useEffect(() => {
    const screen = msg(routeTitleKey(pathname));
    // The wizard's title already names the product ("Welcome to Lumen") —
    // suffixing the app name would read "Welcome to Lumen — Lumen".
    const bare = pathname === "/onboarding";
    const template =
      msg("app.document-title.template" as MessageKey) ?? k19Keys["app.document-title.template"];
    document.title = bare ? screen : template.replace("{screen}", screen);
  }, [pathname, msg]);
  return null;
}

export function App() {
  return (
    <>
      <DocumentTitle />
      {/* Routes-level boundary: catches the ungated /dev/tokens lazy fixture
          (and any future ungated lazy route) that renders outside GatedShell. */}
      <Suspense fallback={null}>
      <Routes>
        <Route path="/" element={<Navigate to="/player" replace />} />

        {/* Full-bleed routes (no app chrome): the onboarding hero and the
            token dev fixture. */}
        <Route path="/onboarding" element={<OnboardingEntry />} />
        <Route path="/dev/tokens" element={<TokenSandbox />} />

        {/* AppShell routes whose screens do NOT render their own <main> —
            the shell provides the single main landmark. w26.i20: the whole
            group sits inside GatedShell — first-run gate for the product. */}
        <Route element={<GatedShell />}>
          <Route path="/player" element={<PlayerRoute variant="full" />} />
          <Route path="/player/music" element={<PlayerRoute variant="music" />} />
          <Route path="/miniplayer" element={<PlayerRoute variant="miniplayer" />} />
          <Route path="/keyboard-shortcuts" element={<ShortcutsScreen />} />
          <Route path="/open-network-stream" element={<NetworkStreamScreen />} />
          <Route path="/discovery" element={<DiscoveryScreen />} />
          <Route path="/convert" element={<ConvertScreen />} />
          <Route path="*" element={<NotFoundScreen />} />
        </Route>

        {/* AppShell routes whose screens render their own <main> (playlist,
            settings, media-info) — provideMain=false keeps exactly one main
            landmark per page; a nested <main> would be invalid HTML. w26.i20:
            the same first-run gate wraps this group — settings, playlists and
            media-info are no longer reachable pre-onboarding (h20 #3). */}
        <Route element={<GatedShell provideMain={false} />}>
          <Route path="/playlist" element={<PlaylistScreen />} />
          <Route path="/media-info" element={<MediaInfoScreen />} />
          <Route path="/settings" element={<Navigate to="/settings/global" replace />} />
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Route>
      </Routes>
      </Suspense>
    </>
  );
}
