// Default seekbar time formatter (card 4.C2). It lives in its own module
// (not next to the Seekbar component) so Seekbar.tsx only exports the
// component (react-refresh/only-export-components, issue #36) while the
// barrel (slider/index.ts) and the control bar keep importing the formatter
// from the same package surface.

/** Default time formatter: `M:SS` / `H:MM:SS`, negative-safe. */
export const formatSeekTime = (seconds: number): string => {
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return `${h > 0 ? `${h}:` : ""}${mm}:${String(s).padStart(2, "0")}`;
};
