import { Navigate, Route, Routes } from "react-router-dom";
import { PlayerScreen } from "./screens/player";
import { PlaylistScreen } from "./screens/playlist";
import { ScreenPlaceholder } from "./screens/ScreenPlaceholder";
import { SettingsScreen } from "./screens/settings/SettingsScreen";
// Card 4.F1 first-run gate: /onboarding is the landing until the wizard is
// finished or skipped; after that, normal routing.
import { OnboardingEntry, RequireOnboarding } from "./screens/onboarding/OnboardingScreen";
import { TokenSandbox } from "./tokens/TokenSandbox";

// Routing skeleton for the doc 08 §1.6 screen inventory. Route names are the
// doc's [PROPOSAL] names. The player route is the real 4.C1 screen shell;
// the settings routes render the real schema-generated framework (card 4.E1);
// every other screen renders an honest placeholder until its card lands.

export function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/player" replace />} />
      <Route
        path="/player"
        element={
          <RequireOnboarding>
            <PlayerScreen />
          </RequireOnboarding>
        }
      />
      <Route
        path="/player/music"
        element={<ScreenPlaceholder routeKey="screen.player.music" />}
      />
      <Route path="/miniplayer" element={<ScreenPlaceholder routeKey="screen.miniplayer" />} />
      {/* Card 4.D1: real playlist panel + queue semantics. */}
      <Route path="/playlist" element={<PlaylistScreen />} />
      <Route path="/settings" element={<Navigate to="/settings/global" replace />} />
      <Route path="/settings/:area" element={<SettingsScreen />} />
      <Route path="/onboarding" element={<OnboardingEntry />} />
      <Route
        path="/keyboard-shortcuts"
        element={<ScreenPlaceholder routeKey="screen.keyboard-shortcuts" />}
      />
      <Route
        path="/open-network-stream"
        element={<ScreenPlaceholder routeKey="screen.open-network-stream" />}
      />
      {/* Card 4.A2 dev fixture: token-pipeline proof (NOT a product screen;
          real screens land with wave 4.B+ per doc 08 §1.6). */}
      <Route path="/dev/tokens" element={<TokenSandbox />} />
      <Route path="*" element={<ScreenPlaceholder routeKey="screen.not-found" />} />
    </Routes>
  );
}
