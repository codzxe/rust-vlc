// Settings area: Library — "Playlists, Folders & Files"
// (Settings - library.svg, analysis tools-settings.md §C9).
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen. Library
// is the merge home of the retired playlist-services area — the fold content
// mounts here UNCHANGED via import from ./playlist-services (that file is
// owned by another lane; this lane never edits it). No [[setting]] rows are
// seeded for library: schemas/settings.toml carries no library.* ui=true
// entry, so there is NO GENERATED_SCREENS.library record to embed (the
// generated area ids are global, interface, privacy, controls, playback,
// performance, recording, extensions — pinned in library.test.tsx).
//
// HONEST SURFACE RULES (config-never-invents, the sibling-area discipline):
// the §C9 frame rows — Opening Files ("Only allow one VLC window", "Apply
// to files opened from the file manager", "Add to the playlist instead of
// replacing it", "When opening a folder", "Show hidden files") and Metadata
// & Artwork ("Fetch titles and artwork", "Use the media library") — have NO
// schemas/settings.toml entries, and no media-library data source exists
// (no path store, no folder-listing command — and the frame carries no
// input + Browse row either) → they render as NOTHING, never dead controls
// that look alive. When the schema entries land, E1's generated
// SettingsSections renders them (feature gates included). Handoff note for
// the schema owner: docs/waves/w26/logs/w26.b08.md.

import type { ReactElement } from "react";
import { PlaylistServicesArea } from "./playlist-services";

/**
 * Library settings area (route /settings/library; Media group) — hosts the
 * playlist-services fold. Today that fold is the honest empty state; the
 * frame's Opening Files / Metadata & Artwork rows render nothing until the
 * schema carries them (see the module header).
 */
export function LibraryArea(): ReactElement {
  return (
    <div
      className="settings-sections"
      data-settings-area="library"
      data-testid="library-area"
    >
      {/* playlist-services fold (w26 merge: playlist-services → library).
          The sibling component owns its content — this lane does not edit
          that file; it renders the honest empty state today. */}
      <PlaylistServicesArea />
      {/* §C9 Opening Files / Metadata & Artwork rows: no schema keys and no
          data source → deliberately nothing (see the module header). */}
    </div>
  );
}
