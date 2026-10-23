// Card 4.C1 — player chrome show/auto-hide (doc 08 §4.3 + §4.5).
//
// Doc 08 §4.3: "Playback canvas, unobtrusive controls … auto-hide on idle in
// fullscreen [PROPOSAL] timing; fade per reduced-motion (§4.5)". The design
// exports pin no number (no frame encodes the delay — see "Player - hide
// UI.svg" target state only), so the delay is an exported constant marked
// [PROPOSAL]; VERIFY against the design package when a timing lands.
//
// States: "visible" | "hidden" | "locked".
//   locked — a menu/panel is open: chrome must NOT hide (doc 08 §4.5 focus
//   semantics; an open overlay that vanished would strand keyboard focus).
//   Activity (pointer move/down, key press) reveals + restarts the idle timer.
// prefers-reduced-motion is honored in player.css (fade disabled there); the
// state machine is identical either way.

import { useCallback, useEffect, useState } from "react";

/** [PROPOSAL] idle delay before chrome hides (doc 08 §4.3 leaves timing open). */
export const CHROME_AUTO_HIDE_DELAY_MS = 3000;

export type ChromeVisibility = "visible" | "hidden" | "locked";

export interface ChromeAutoHideOptions {
  /** Auto-hide active (doc 08 §4.3 scopes it to fullscreen; windowed stays visible). */
  readonly enabled: boolean;
  /** A menu/panel is open — chrome locks visible. */
  readonly locked: boolean;
  /** Idle delay; defaults to CHROME_AUTO_HIDE_DELAY_MS. */
  readonly delayMs?: number;
}

export interface ChromeAutoHide {
  readonly visibility: ChromeVisibility;
  /** Reveal chrome and restart the idle timer (also any activity does this). */
  readonly show: () => void;
}

export function useChromeAutoHide({
  enabled,
  locked,
  delayMs = CHROME_AUTO_HIDE_DELAY_MS,
}: ChromeAutoHideOptions): ChromeAutoHide {
  const [hidden, setHidden] = useState(false);
  const show = useCallback(() => {
    setHidden(false);
  }, []);

  // Leaving auto-hide (fullscreen exit) or entering a lock must reveal
  // chrome — a stale `hidden` must never survive those transitions.
  useEffect(() => {
    if (!enabled || locked) setHidden(false);
  }, [enabled, locked]);

  // Idle timer: (re)starts whenever chrome is shown while auto-hide is
  // active and unlocked; cleared otherwise. `hidden` in the deps makes any
  // reveal restart the countdown.
  useEffect(() => {
    if (!enabled || locked || hidden) return;
    const timer = setTimeout(() => setHidden(true), delayMs);
    return () => clearTimeout(timer);
  }, [enabled, locked, hidden, delayMs]);

  // Activity reveals chrome: any pointer move/down or key press.
  useEffect(() => {
    if (!enabled || locked) return;
    const onActivity = () => setHidden(false);
    window.addEventListener("pointermove", onActivity);
    window.addEventListener("pointerdown", onActivity);
    window.addEventListener("keydown", onActivity);
    return () => {
      window.removeEventListener("pointermove", onActivity);
      window.removeEventListener("pointerdown", onActivity);
      window.removeEventListener("keydown", onActivity);
    };
  }, [enabled, locked]);

  const visibility: ChromeVisibility = locked ? "locked" : hidden ? "hidden" : "visible";
  return { visibility, show };
}
