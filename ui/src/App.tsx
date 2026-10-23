import { Navigate, Route, Routes, useParams } from "react-router-dom";
import { ScreenPlaceholder } from "./screens/ScreenPlaceholder";

// Routing skeleton for the doc 08 §1.6 screen inventory. Route names are the
// doc's [PROPOSAL] names; every screen renders an honest placeholder until
// its Figma-exact composition card lands.

function SettingsScreen() {
  const { area } = useParams();
  return <ScreenPlaceholder routeKey="screen.settings" detail={area} />;
}

export function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/player" replace />} />
      <Route path="/player" element={<ScreenPlaceholder routeKey="screen.player" />} />
      <Route
        path="/player/music"
        element={<ScreenPlaceholder routeKey="screen.player.music" />}
      />
      <Route path="/miniplayer" element={<ScreenPlaceholder routeKey="screen.miniplayer" />} />
      <Route path="/playlist" element={<ScreenPlaceholder routeKey="screen.playlist" />} />
      <Route path="/settings/:area" element={<SettingsScreen />} />
      <Route path="/onboarding" element={<ScreenPlaceholder routeKey="screen.onboarding" />} />
      <Route
        path="/keyboard-shortcuts"
        element={<ScreenPlaceholder routeKey="screen.keyboard-shortcuts" />}
      />
      <Route
        path="/open-network-stream"
        element={<ScreenPlaceholder routeKey="screen.open-network-stream" />}
      />
      <Route path="*" element={<ScreenPlaceholder routeKey="screen.not-found" />} />
    </Routes>
  );
}
