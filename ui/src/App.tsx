import { Navigate, Outlet, Route, Routes, useNavigate, useSearchParams } from "react-router-dom";
import { AppShell } from "./components/appshell/AppShell";
import { PlayerScreen } from "./screens/player";
import type { PlayerVariant } from "./screens/player";
import { PlaylistScreen } from "./screens/playlist";
// Card 4.F1 first-run gate: /onboarding is the landing until the wizard is
// finished or skipped; after that, normal routing.
import { OnboardingEntry, RequireOnboarding } from "./screens/onboarding/OnboardingScreen";
import { MediaInfoScreen } from "./screens/media-info";
import { SettingsScreen } from "./screens/settings/SettingsScreen";
// App-shell workstream (2026-11-08): real screens for the former placeholder
// routes + persistent navigation (AppShell).
import { NetworkStreamScreen } from "./screens/network-stream/NetworkStreamScreen";
// Discovery panel (card 7.D1, issue #42): the device-discovery surface —
// live devices.volume-* events + the honest network-lane verdict.
import { DiscoveryScreen } from "./screens/discovery/DiscoveryScreen";
// Card 7.E3 (issue #44): the conversion screen — target picker limited to
// the engine's real capability (WAV record / MP4-remux), job queue wired
// to the command-catalog capability probe.
import { ConvertScreen } from "./screens/convert/ConvertScreen";
import { ShortcutsScreen } from "./screens/shortcuts/ShortcutsScreen";
import { NotFoundScreen } from "./screens/not-found/NotFoundScreen";
import { TokenSandbox } from "./tokens/TokenSandbox";
import { isDesktopShell } from "./client/app-transport";

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
//     that used it keep working.
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
 * just the player family.
 */
function useOnboardingBypass(): boolean {
  const [searchParams] = useSearchParams();
  return searchParams.get("onboarding") === "1";
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
  const shell = (
    <AppShell demo={DEMO_SHELL} provideMain={provideMain}>
      <Outlet />
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

export function App() {
  return (
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
  );
}
