import { Navigate, Outlet, Route, Routes, useNavigate } from "react-router-dom";
import { AppShell } from "./components/appshell/AppShell";
import { PlayerScreen } from "./screens/player";
import type { PlayerVariant } from "./screens/player";
import { PlaylistScreen } from "./screens/playlist";
// Card 4.F1 first-run gate: /onboarding is the landing until the wizard is
// finished or skipped; after that, normal routing.
import { OnboardingEntry, RequireOnboarding } from "./screens/onboarding/OnboardingScreen";
import { MediaInfoScreen } from "./screens/media-info";
import { SettingsScreen } from "./screens/settings/SettingsScreen";
// Solstice app-shell workstream (2026-11-08): real screens for the former
// placeholder routes + persistent navigation (AppShell).
import { OPEN_MEDIA_EVENT } from "./screens/open-media";
import { NetworkStreamScreen } from "./screens/network-stream/NetworkStreamScreen";
import { ShortcutsScreen } from "./screens/shortcuts/ShortcutsScreen";
import { NotFoundScreen } from "./screens/not-found/NotFoundScreen";
import { TokenSandbox } from "./tokens/TokenSandbox";

// Doc 08 §1.6 screen inventory, now fully real:
//   * /player + /player/music + /miniplayer — the real player screen shell,
//     with the route preselected variant (data-variant) and the open-media
//     CTA wired to the AppShell dialog via the shared CustomEvent contract.
//   * /playlist, /media-info, /settings/:area — unchanged real screens.
//   * /keyboard-shortcuts, /open-network-stream, * — real screens (this
//     workstream); ScreenPlaceholder is gone.

/**
 * True for the browser dev shell: main.tsx injects DemoTransport there (a
 * simulated engine, badged "Demo" by the AppShell). Hardcoded here because
 * the transport choice is a main.tsx-level decision — the desktop host
 * renders the same tree with its real transport and no badge.
 */
const DEMO_SHELL = true;

/** Route path for a player variant (the URL is the variant's source of truth). */
function playerVariantPath(variant: PlayerVariant): string {
  if (variant === "miniplayer") return "/miniplayer";
  if (variant === "music") return "/player/music";
  return "/player";
}

/** The open-media trigger contract: the AppShell listens and opens the dialog. */
function dispatchOpenMedia(): void {
  window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT));
}

/**
 * Player routes. The variant is CONTROLLED by the route (persisted-variant
 * state stays for host-owned free-floating windows); keymap toggles and the
 * miniplayer expand/close affordances call onVariantChange, which routes.
 */
function PlayerRoute({ variant }: { variant: PlayerVariant }) {
  const navigate = useNavigate();
  return (
    <RequireOnboarding>
      <PlayerScreen
        variant={variant}
        onOpenMedia={dispatchOpenMedia}
        onVariantChange={(next) => navigate(playerVariantPath(next))}
      />
    </RequireOnboarding>
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
