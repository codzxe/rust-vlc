// Card 4.F2 — MusicMode: layout re-weighting for `player.music`, composed
// to the W26 frames `design/figma/Player - Music.svg` + `Player - Music-1.svg`
// — the ONE theatrical screen of the redesign:
//   * large centered artwork treatment (frame: 738×737 rx20 art on the
//     1920×1080 stage) — an honest placeholder face until the engine
//     reports artwork (no fabricated art, no Solstice sun),
//   * ambient tint — a quiet accent-tinted radial glow behind the art (the
//     frame's full-bleed art backdrop), staying inside the near-black +
//     orange system (NO sunset gradients),
//   * quiet meta — the engine-reported title under the art (bright), or the
//     honest placeholder caption when nothing is playing,
//   * compact controls — the real C2 ControlBar drives transport in the
//     controls-bar region (PlayerScreen); this component owns the stage.
//
// w26.f11 (e03 F7): the frame's waveform seek treatment lives INSIDE the
// controls row (Player - Music.svg Controls group: the 1246×28 r6 #090909
// well is a dense 2px-tick field — unplayed #404040, played #AB6314
// accent-strong, a 4×24 rx2 #D37918 playhead at the progress edge) — NOT in
// a separate stage strip. The old decorative stage waveform (a doubled seek
// surface above the bar) is GONE; variants.css restyles the REAL C2 seek
// cluster in place under `.lumen-player[data-variant="music"]` (the same
// hook the timecodes already use), so there is exactly ONE seek surface and
// it is the interactive one.
//
// w26.f11 (e03 F6): the music playlist column is the player's LEFT dock
// (Player - Music-1.svg) rendered by PlayerScreen's docked-sidebar seam —
// open by default, closable through the shell's `lumen:toggle-sidebar`
// event (collapsing to the Player - Music.png no-column state). The stage
// carries a quiet playlist toggle so the column is reachable in BOTH
// directions from inside music mode (the shell's banner toggle is close-
// only for this dock while the column is open — AppShell's one-sidebar
// gate, f08 seam).

import { useLayoutEffect, useState } from "react";
import { useMsg } from "../../../i18n/messages";
import { usePlayerControls } from "../context";
import { usePlaylist } from "../../playlist";
import { Icon, IconButton } from "../../../components";
import { PLAYER_TOGGLE_SIDEBAR_EVENT } from "../PlayerScreen";
import type { LayoutRegion } from "../layout";

export interface MusicModeProps {
  /** The album-art region (center anchor, from the music layout). */
  readonly artRegion: LayoutRegion;
}

/** The player root selector — the stage lives inside the player screen and
 * the dock's open/closed truth lives on its `data-sidebar` attribute
 * (PlayerScreen owns the state; this component only mirrors it). */
const PLAYER_ROOT_SELECTOR = ".lumen-player";

/** Read the dock's open truth from the player root (post-commit DOM). */
function isPlayerDockOpen(): boolean {
  return (
    document.querySelector(PLAYER_ROOT_SELECTOR)?.getAttribute("data-sidebar") === "open"
  );
}

export function MusicMode({ artRegion }: MusicModeProps) {
  const m = useMsg();
  const { state } = usePlayerControls();
  const playlist = usePlaylist();

  // The playlist dock's open truth, mirrored from the player root's
  // data-sidebar attribute (the only writer is the toggle event this
  // component and the shell banner both dispatch). useLayoutEffect + a
  // MutationObserver keep the toggle's aria-pressed honest across every
  // flip — including ones this stage did not initiate (the shell toggle).
  const [dockOpen, setDockOpen] = useState(false);
  useLayoutEffect(() => {
    setDockOpen(isPlayerDockOpen());
    const root = document.querySelector(PLAYER_ROOT_SELECTOR);
    if (root === null || typeof MutationObserver === "undefined") return;
    const observer = new MutationObserver(() => setDockOpen(isPlayerDockOpen()));
    observer.observe(root, { attributes: true, attributeFilter: ["data-sidebar"] });
    return () => observer.disconnect();
  }, []);

  // Real data only (never a fabricated title): engine truth first, the
  // current playlist entry's title second.
  const entryTitle =
    playlist.entries.find((entry) => entry.id === playlist.currentId)?.title ?? null;
  const title = state.mediaTitle ?? entryTitle;

  const toggleLabel = m(dockOpen ? "player.canvas.menu.hide-queue" : "player.canvas.menu.show-queue");

  return (
    <div
      data-region-id={artRegion.id}
      role="region"
      aria-label={m(artRegion.labelKey)}
      className="lumen-music__art"
      data-placeholder="artwork-engine-data-unwired"
    >
      {/* Playlist column toggle (e03 F6): the dock is the shell-toggled
          left column; this quiet stage affordance keeps it reachable in
          both directions from inside music mode. Same seam + copy as the
          full player's canvas-menu dock toggle (w26.c6 keys). */}
      <IconButton
        size="sm"
        data-control="playlist-toggle"
        data-chrome-part=""
        className="lumen-music__playlist-toggle"
        label={toggleLabel}
        title={toggleLabel}
        aria-pressed={dockOpen}
        onClick={() => window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT))}
      >
        <Icon name="list" size={16} />
      </IconButton>

      <div className="lumen-music__stage">
        {/* Honest placeholder face: no fabricated album art (real artwork
            renders here once the engine reports it). The ambient tint is
            the frame's quiet accent glow — purely decorative. */}
        <figure className="lumen-music__artwork" aria-hidden="true">
          <span className="lumen-music__artwork-glyph">
            <Icon name="music" size={64} variant="strong" />
          </span>
        </figure>

        {/* Quiet meta — the title/artist layout slot. Engine truth while a
            session is up; the honest placeholder caption otherwise (the
            same slot renders real metadata when the engine reports it). */}
        {title === null ? (
          <p className="lumen-music__caption">{m("player.art.placeholder")}</p>
        ) : (
          <h2 className="lumen-music__title">{title}</h2>
        )}
      </div>
    </div>
  );
}
