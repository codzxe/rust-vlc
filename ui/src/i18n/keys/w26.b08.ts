// w26.b08 lane keys (settings areas: extensions) — the Extensions area's
// hand-authored copy (the add-on list empty state + toolbar; Settings -
// extensions.svg §C13). Registered into ui/src/i18n/messages.ts by the Lead
// after the wave merge (contract §4.5, wave-d lane b19); until then
// extensions.tsx resolves these via its lane-key seam (`useAreaMsg`, the
// w26.b07 sibling pattern) so the copy renders identically before and after
// registration.
//
// All other Extensions copy reuses existing keys — grepped first:
//   • the seeded section/row labels ("Add-ons", "Load add-ons on startup")
//     live in ui/src/i18n/keys/w26.b02.ts and render through the generated
//     screen's own seam;
//   • the toolbar's disabled-with-reason note reuses the registered
//     "settings.capability-unavailable" ("Not available in this build").
// The Library area renders NO hand copy of its own (the playlist-services
// fold carries the registered "settings.area.empty"), so this file carries
// Extensions keys only. Copy is sentence case per CONTRACT §2 (the frames'
// title-case export styling is normalized; frame provenance per key).
export const keys = {
  // Add-on list empty state (§C13: the frame's 782×192 list well exports no
  // rows — the empty state is composed from the design language's quiet
  // ListEmpty grammar; honest copy: the list data source does not exist yet,
  // wasm-extensions is Phase 7 / rule F2).
  "settings.extensions.empty-title": "No add-ons installed",
  "settings.extensions.empty-body":
    "Add-ons appear here once the extensions feature ships in this build.",
  // Toolbar (§C13 y311): the search input's placeholder + accessible label
  // (frame-verbatim "Search"), then the three frame actions.
  "settings.extensions.search": "Search",
  "settings.extensions.find-more": "Find more extensions",
  "settings.extensions.add": "Add",
  "settings.extensions.remove": "Remove",
} as const;
