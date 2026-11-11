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

// Product flow (doc 08 §1.6 screen inventory — every route is a real screen;
// the placeholder routes are gone since the 2026-11-08 app-shell workstream).
//
//   first run (onboarding store absent / complete ≠ true — see
//   screens/onboarding/onboardingState.ts, persisted as lumen.onboarding.v1)
//     /            →  /player  →  first-run gate  →  /onboarding (wizard)
//     /player, /player/music, /miniplayer  →  gate  →  /onboarding
//     every other route renders directly (card 4.F1 gates ONLY the player
//     home, so settings/playlist/… stay reachable from the shell)
//
//   after finish or skip (complete = true, persisted)
//     /            →  /player — the home; the player empty state is the
//                    onboarding-style hero, never a blank void (W25 spec §2)
//     /player/music + /miniplayer — the same player screen, route-selected
//                    variant (data-variant); every open-media CTA dispatches
//                    the shared CustomEvent exactly ONCE (the player screens'
//                    own affordances fire it; the AppShell dialog listens)
//     /playlist, /media-info, /settings/:area (13 areas; /settings itself
//                    redirects to /settings/global), /discovery, /convert,
//     /keyboard-shortcuts, /open-network-stream, /dev/tokens (dev fixture),
//     *            →  404
//
//   ?onboarding=1 — test/screenshot bypass of the first-run gate: player
//     routes render as if the wizard were finished (dev-shots always starts
//     a fresh browser profile, so player shots need a way past the gate).
//     Per-navigation only — nothing is written to the onboarding store, so
//     the very next param-less navigation is gated again. Enter on the
//     player routes themselves (/player?onboarding=1, /player/music?…,
//     /miniplayer?…); the / → /player redirect does not carry the query.

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
 * the override never touches the persisted onboarding store.
 */
function useOnboardingBypass(): boolean {
  const [searchParams] = useSearchParams();
  return searchParams.get("onboarding") === "1";
}

/**
 * Player routes. The variant is CONTROLLED by the route (persisted-variant
 * state stays for host-owned free-floating windows); keymap toggles and the
 * miniplayer expand/close affordances call onVariantChange, which routes.
 * The first-run gate wraps the player home unless ?onboarding=1 bypasses it.
 */
function PlayerRoute({ variant }: { variant: PlayerVariant }) {
  const navigate = useNavigate();
  const bypassGate = useOnboardingBypass();
  // No onOpenMedia here: the player's own affordances (StageOverlays hero,
  // canvas context menu, ControlBar app menu) already dispatch the shell-wide
  // `lumen:open-media` CustomEvent — passing a re-dispatching callback made it
  // fire TWICE per CTA (a13 open issue 4, fixed w26.d1). The prop stays on
  // PlayerScreen's public API for hosts that need the composition seam.
  const player = (
    <PlayerScreen
      variant={variant}
      onVariantChange={(next) => navigate(playerVariantPath(next))}
    />
  );
  return bypassGate ? player : <RequireOnboarding>{player}</RequireOnboarding>;
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
          the shell provides the single main landmark. */}
      <Route
        element={
          <AppShell demo={DEMO_SHELL}>
            <Outlet />
          </AppShell>
        }
      >
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
          landmark per page; a nested <main> would be invalid HTML. */}
      <Route
        element={
          <AppShell demo={DEMO_SHELL} provideMain={false}>
            <Outlet />
          </AppShell>
        }
      >
        <Route path="/playlist" element={<PlaylistScreen />} />
        <Route path="/media-info" element={<MediaInfoScreen />} />
        <Route path="/settings" element={<Navigate to="/settings/global" replace />} />
        <Route path="/settings/:area" element={<SettingsScreen />} />
      </Route>
    </Routes>
  );
}
