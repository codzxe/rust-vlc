// w26.b10 — DEV-ONLY vision harness (never imported by app routes): composes
// the four reference states side by side on the app surface so the
// comparison loop (contract §5) can screenshot them next to
// design/figma-renders/{Sorting_Options,View_Options,thumbnail,media}.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
//
//   W26B10_HARNESS_OUT=/tmp/w26b10-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandTransport } from "../../../client/transport";
import { PlaylistProvider, type PlaylistEntry } from "../PlaylistProvider";
import { PlaylistOptionsProvider } from "./PlaylistOptionsProvider";
import { PlaylistOptionsToolbar, PlaylistOptionsViews } from "./PlaylistOptionsToolbar";
import { SortingOptionsMenu } from "./SortingOptionsMenu";
import { ViewOptionsMenu } from "./ViewOptionsMenu";
import { MediaList } from "./MediaList";
import { ThumbnailGrid } from "./ThumbnailGrid";
import type { PlaylistEntryMeta } from "./sorting";
import "./harness.css";

/** Fixture mirroring the frames: "The Wand_S01E01.mp4" ×4 (media.svg) plus
 * episodes with durations (thumbnail.svg "00:02:10"). */
export const HARNESS_ENTRIES: readonly PlaylistEntry[] = [
  { id: 1, title: "The Wand_S01E01.mp4", source: { kind: "file", path: "/tv/The Wand_S01E01.mp4" }, durationSeconds: 130 },
  { id: 2, title: "The Wand_S01E02.mp4", source: { kind: "file", path: "/tv/The Wand_S01E02.mp4" }, durationSeconds: 2579 },
  { id: 3, title: "The Wand_S01E10.mp4", source: { kind: "file", path: "/tv/The Wand_S01E10.mp4" }, durationSeconds: null },
  { id: 4, title: "The Wand_S02E02.mp4", source: { kind: "file", path: "/tv/The Wand_S02E02.mp4" }, durationSeconds: 42 },
];

/** media.svg's four row states: watched / unwatched / hover-watched /
 * hover-unwatched (hover is CSS; the seam pins watched + progress). */
export const HARNESS_META: Record<number, PlaylistEntryMeta> = {
  1: { watched: true }, // full bar (Rectangle 12)
  2: { watched: true, watchedProgress: 0.45 }, // partial seam demo
  3: {},
  4: {},
};

/** Transport stub — the harness never talks to an engine. */
export const harnessTransport = (): CommandTransport => ({
  subscribe: () => () => {},
  send: async () => ({
    gen: 1,
    requestId: 1,
    result: { completed: { kind: "state-snapshot", snapshot: { playlist: {} } as never } },
  }),
});

/** State 1+2 — both option menus open, anchored under the toolbar. */
export function OptionsMenusHarness(): ReactElement {
  return (
    <ClientProvider transport={harnessTransport()}>
      <PlaylistProvider initialEntries={HARNESS_ENTRIES}>
        <PlaylistOptionsProvider resolveMeta={(e) => HARNESS_META[e.id]}>
          <div className="w26b10-stage">
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">Sorting Options</p>
              <div className="w26b10-frame w26b10-frame--menu">
                <div className="w26b10-anchor">
                  <PlaylistOptionsToolbar />
                </div>
              </div>
            </div>
          </div>
        </PlaylistOptionsProvider>
      </PlaylistProvider>
    </ClientProvider>
  );
}

/** State 3+4 — thumbnail card grid + media list. */
export function OptionsViewsHarness(): ReactElement {
  return (
    <ClientProvider transport={harnessTransport()}>
      <PlaylistProvider initialEntries={HARNESS_ENTRIES}>
        <PlaylistOptionsProvider resolveMeta={(e) => HARNESS_META[e.id]}>
          <div className="w26b10-stage">
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">thumbnail</p>
              <div className="w26b10-frame">
                <ThumbnailGrid />
              </div>
            </div>
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">media</p>
              <div className="w26b10-frame w26b10-frame--media">
                <MediaList />
              </div>
            </div>
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">views (toolbar)</p>
              <div className="w26b10-frame w26b10-frame--media">
                <PlaylistOptionsViews />
                <PlaylistOptionsToolbar />
              </div>
            </div>
          </div>
        </PlaylistOptionsProvider>
      </PlaylistProvider>
    </ClientProvider>
  );
}

/** Menu-open closeups: sorting (16 rows) and view (2 rows) popups rendered
 * permanently open for pixel comparison (the triggers are click-gated, so
 * the harness drives them via the public popup API in the test). */
export function OptionsPopupCloseups(): ReactElement {
  return (
    <ClientProvider transport={harnessTransport()}>
      <PlaylistProvider initialEntries={HARNESS_ENTRIES}>
        <PlaylistOptionsProvider resolveMeta={(e) => HARNESS_META[e.id]}>
          <div className="w26b10-stage">
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">Sorting Options.svg</p>
              <SortingOptionsMenu />
            </div>
            <div className="w26b10-cell">
              {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
              <p className="w26b10-caption">View Options.svg</p>
              <ViewOptionsMenu />
            </div>
          </div>
        </PlaylistOptionsProvider>
      </PlaylistProvider>
    </ClientProvider>
  );
}
