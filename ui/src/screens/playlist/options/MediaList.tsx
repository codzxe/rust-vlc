// w26.b10 — media (list) view (`media.svg`): 51px rows — 32×32 r2 media
// tile, title beside it, orange watched-progress bar under the tile, and a
// "•••" actions button that appears on hover/focus. The frame pins FOUR row
// states: watched (dim title + full bar), unwatched, hover-watched and
// hover-unwatched (bright title + •••).
//
// Watched state arrives through the metadata seam (PlaylistOptionsProvider:
// the engine resolveMeta seam merged with the client-local Mark-as-Watched
// override); with neither wired every row renders honestly unwatched (no
// bar, normal title). One click on a row plays the item (v2 flow brief §1
// single-click intent); the engine queue itself is never reordered by views.
//
// w26.f06 — ENGINE vs LOCAL scope: with an engine PlaylistProvider ancestor
// rows stay the frame's click-to-play rows. In the /playlist saved-playlist
// browse (local data, names only) there is NO play seam — the row renders
// as honest presentation, never a dead click target (the "no dead
// affordance" rule).
//
// w26.gfix-E (g18 P2-1 + g20 P2-4) — the hover "•••" now has a real seam:
// pass `entryMenu` (the file context menu.svg builder) and each row wraps in
// the shared ContextMenu — right-click on the row AND the ••• open it (the
// ••• synthesizes a contextmenu at its own position, the LibraryRow kebab
// precedent). The raw `onEntryMenu` callback seam stays for hosts that own
// their own menu surface.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { ContextMenu } from "../../../components/menu";
import type { MenuEntry } from "../../../components/menu";
import { VirtualList } from "../../../components/list";
import { usePlaylistOptional, type PlaylistEntry } from "../PlaylistProvider";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { entryDisplayTitle, type PlaylistEntryMeta } from "./sorting";
import { useOptionsMsg } from "./useOptionsMsg";

/** media.svg row pitch: tiles at y 20 / 71 / 122 / 173 — 51px. Structural. */
export const MEDIA_ROW_HEIGHT = 51;

export interface MediaListProps {
  /** "•••" hover affordance callback (the raw host seam). */
  onEntryMenu?: (entry: PlaylistEntry) => void;
  /**
   * w26.gfix-E — per-entry context-menu entries (file context menu.svg):
   * when set, each row wraps in the shared ContextMenu; right-click and the
   * ••• both open it. Supplied by the host (e.g. the /playlist browse view's
   * fileRowMenu builder).
   */
  entryMenu?: (entry: PlaylistEntry) => MenuEntry<string>[];
  /** Label resolver for the entryMenu rows (i18n seam). */
  menuResolveLabel?: (key: string) => string;
  /** Accessible name for the entry-menu popup (defaults to the ••• label). */
  menuAriaLabel?: string;
}

export function MediaList({ onEntryMenu, entryMenu, menuResolveLabel, menuAriaLabel }: MediaListProps): ReactElement {
  const m = useMsg();
  const t = useOptionsMsg();
  const playlist = usePlaylistOptional();
  const { sortedEntries, metaOf } = usePlaylistOptions();

  if (sortedEntries.length === 0) return <></>;
  const currentId = playlist?.currentId ?? null;

  // The ••• renders when EITHER seam is wired (a raw callback or the
  // context-menu entries) — never as a dead button. The ContextMenu wrapper
  // (and with it the •••'s synthesized contextmenu) needs BOTH the entries
  // and a resolver — entries without a resolver would render raw keys.
  const wrapMenu = entryMenu !== undefined && menuResolveLabel !== undefined;

  return (
    <VirtualList
      data={sortedEntries}
      itemHeight={MEDIA_ROW_HEIGHT}
      label={t("playlist.options.list-label")}
      className="lumen-plo-media"
      scrollToIndex={
        currentId === null
          ? null
          : (() => {
              const index = sortedEntries.findIndex((entry) => entry.id === currentId);
              return index >= 0 ? { index, align: "start" } : null;
            })()
      }
      semantics="grid"
      renderItem={(entry) => {
        const row = (
          <MediaRow
            key={entry.id}
            isCurrent={entry.id === currentId}
            title={entryDisplayTitle(entry, m("playlist.entry.unknown-title"))}
            meta={metaOf(entry)}
            onPlay={playlist !== null ? () => playlist.playItem(entry.id) : undefined}
            onMenu={onEntryMenu !== undefined ? () => onEntryMenu(entry) : undefined}
            onOpenMenuAt={wrapMenu ? openMenuAtKebab : undefined}
            menuLabel={t("playlist.options.entry-menu")}
          />
        );
        if (!wrapMenu) return row;
        return (
          <ContextMenu
            key={entry.id}
            entries={entryMenu(entry)}
            resolveLabel={menuResolveLabel}
            ariaLabel={menuAriaLabel ?? t("playlist.options.entry-menu")}
            className="lumen-plo-mrow-wrap"
          >
            {row}
          </ContextMenu>
        );
      }}
    />
  );
}

/**
 * The ••• opens the SAME context menu as right-click (LibraryRow kebab
 * precedent): the button carries no pointer event to the ContextMenu's
 * contextmenu handler, so it synthesizes one at its own screen position —
 * the shared MenuPopup clamps the fixed popup inside the viewport (w26.f07).
 */
function openMenuAtKebab(button: HTMLElement): void {
  const rect = button.getBoundingClientRect();
  button.dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: rect.right - 24,
      clientY: rect.bottom + 4,
    }),
  );
}

function MediaRow({
  isCurrent,
  title,
  meta,
  onPlay,
  onMenu,
  onOpenMenuAt,
  menuLabel,
}: {
  isCurrent: boolean;
  title: string;
  meta: PlaylistEntryMeta | undefined;
  /** Play seam — undefined in local scope (no dead click targets). */
  onPlay: (() => void) | undefined;
  /** Raw ••• callback (undefined when no seam). */
  onMenu: (() => void) | undefined;
  /** Open the row's ContextMenu at this ••• element (entryMenu mode). */
  onOpenMenuAt: ((button: HTMLButtonElement) => void) | undefined;
  menuLabel: string;
}): ReactElement {
  const watched = meta?.watched === true;
  // Watched progress 0..1; a fully watched item reports watched=true with
  // no partial progress — the frame's completed bar (media.svg Rectangle 12).
  const progress = meta?.watchedProgress ?? (watched ? 1 : 0);
  return (
    <div
      className={[
        "lumen-plo-mrow",
        watched ? "lumen-plo-mrow--watched" : "",
        isCurrent ? "lumen-plo-mrow--current" : "",
        onPlay === undefined ? "lumen-plo-mrow--inert" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      data-watched={watched || undefined}
      aria-current={isCurrent || undefined}
      onClick={onPlay}
    >
      <span className="lumen-plo-mrow__tile" aria-hidden="true">
        {progress > 0 && (
          <span
            className="lumen-plo-mrow__progress"
            style={{ width: `${Math.round(Math.max(0, Math.min(1, progress)) * 100)}%` }}
          />
        )}
      </span>
      <span className="lumen-plo-mrow__title">{title}</span>
      {(onMenu !== undefined || onOpenMenuAt !== undefined) && (
        <button
          type="button"
          className="lumen-plo-mrow__menu"
          aria-label={menuLabel}
          onClick={(event) => {
            event.stopPropagation();
            onMenu?.();
            if (onOpenMenuAt !== undefined) onOpenMenuAt(event.currentTarget);
          }}
        >
          <span className="lumen-plo-mrow__dots" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
        </button>
      )}
    </div>
  );
}
