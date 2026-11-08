// Solstice restyle — the keycap hint strip (design spec §5 "Keyboard hints").
//
// A subtle bottom-centered row of `.solstice-key` chips with tiny action
// labels, shown while the chrome is visible AND nothing is playing (no media
// open, or the engine reported `paused`). While playing it renders nothing —
// the chrome auto-hide (useChromeAutoHide) already handles fullscreen idle,
// and `data-chrome-part` makes the strip fade with the rest of the chrome.
//
// Honesty rules kept: key spellings + action labels come from REAL keymap
// data (keymaps/default.toml via the generated hotkeys bindings — the same
// source the settings hotkeys editor renders), never hardcoded shortcuts.
// Arrow keys render as glyphs (←/→) the way keycaps do; that is a graphical
// rendering of the keymap spelling, not invented copy.

import { HOTKEY_BINDINGS } from "../settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../settings/areas/hotkeys/generated/bindings";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { usePlayerControls } from "./context";

/** Keymaps/default.toml binding ids surfaced in the strip (Space ← → F M). */
const STRIP_BINDING_IDS: readonly string[] = [
  "navigation.toggle-play-pause",
  "navigation.seek-backward",
  "navigation.seek-forward",
  "video.fullscreen",
  "audio.mute",
];

/** Keycap glyph for a keymap spelling — arrows are glyphs, not words. */
const KEY_GLYPHS: Readonly<Record<string, string>> = {
  Left: "←",
  Right: "→",
};

const BINDINGS_BY_ID: ReadonlyMap<string, HotkeyBinding> = new Map(
  HOTKEY_BINDINGS.map((binding) => [binding.id, binding]),
);

/**
 * Renders the keycap hint strip. `hasMedia` comes from the shell's media
 * context; the paused check reads the shared player state seam (engine-truth
 * playback state, never a UI prediction).
 */
export function PlayerKeyHintStrip({ hasMedia }: { hasMedia: boolean }) {
  const m = useMsg();
  const { state } = usePlayerControls();

  // Visible only when there is nothing playing: first-run empty state or a
  // real engine-reported pause. Everything else (playing/buffering/…) hides
  // the strip — hints would only fight the content.
  if (hasMedia && state.playbackState !== "paused") return null;

  const hints = STRIP_BINDING_IDS.flatMap((id) => {
    const binding = BINDINGS_BY_ID.get(id);
    // Bindings that ship disabled in the keymap data are not hinted.
    return binding === undefined || binding.disabled ? [] : [binding];
  });
  if (hints.length === 0) return null;

  return (
    <div
      className="lumen-player__keyhints"
      role="group"
      aria-label={m("player.keys.strip.label")}
      data-chrome-part=""
    >
      {hints.map((binding) => (
        <span className="lumen-player__keyhint" key={binding.id}>
          <span className="solstice-key solstice-key--sm" aria-hidden="true">
            {KEY_GLYPHS[binding.key] ?? binding.key}
          </span>
          <span className="lumen-player__keyhint-label">
            {m(binding.labelMsgid as MessageKey)}
          </span>
        </span>
      ))}
    </div>
  );
}
