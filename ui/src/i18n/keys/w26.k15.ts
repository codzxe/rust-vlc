// w26.k15 lane keys (playlist options + subtitle flows, fix wave K) — copy
// with no existing catalog entry. Registered into ui/src/i18n/messages.ts by
// the Lead after the wave merge (contract §4.5); until then the options layer
// resolves these via the seam-first resolver (useOptionsMsg merges this file),
// so the copy renders identically before and after registration.
//
// Copy provenance: honest-degradation hint for the sorting menu (j15 P3-11).
// `Sorting Options.svg` pins plain rows with no such suffix — the hint renders
// ONLY while the metadata seam is unwired (the frame-comparison harness mounts
// a resolver, so frame shots stay pure); sentence case per contract §2.
export const keys = {
  // Quiet suffix on metadata-dependent sort rows while no engine metadata
  // (and no local watched mark) can influence the order — the honest signal
  // that the row is a stable no-op in this build (j15 P3-11).
  "playlist.options.sort.needs-meta": "Needs media metadata",
} as const;
