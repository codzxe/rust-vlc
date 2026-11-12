// Card 4.C1 — player chrome show/auto-hide (doc 08 §4.3 + §4.5).
//
// Doc 08 §4.3: "Playback canvas, unobtrusive controls … auto-hide on idle in
// fullscreen [PROPOSAL] timing; fade per reduced-motion (§4.5)". The design
// exports pin no number (no frame encodes the delay — see "Player - hide
// UI.svg" target state only), so the delay is an exported constant marked
// [PROPOSAL]; VERIFY against the design package when a timing lands.
//
// w26.f10 (e03 F4, Lead decision 2026-11-12): the [PROPOSAL] fullscreen-only
// scoping is RETIRED. The reference frame "Player - hide UI.svg" is a
// full-bleed video behind an r19 WINDOW stroke (a windowed window border —
// fullscreen has none), i.e. the hidden-chrome target state is reachable
// WINDOWED; VLC parity hides controls on windowed idle too. Inside the
// player composition (a PlayerControlsProvider is mounted above — the hook
// reads its engine truth) the idle fade is therefore armed in BOTH windowed
// and fullscreen AND with the empty first-run hero: the chrome AROUND the
// canvas fades (every [data-chrome-part], player.css), the canvas content —
// video, the empty hero — never does (states.css pins the hero).
//
//   locked — a menu/panel is open: chrome must NOT hide (doc 08 §4.5 focus
//   semantics; an open overlay that vanished would strand keyboard focus).
//   Activity (pointer move/down, key press) reveals + restarts the idle timer.
// prefers-reduced-motion is honored in player.css (fade disabled there); the
// state machine is identical either way.
//
// LEGACY STANDALONE SEMANTICS (no PlayerControlsProvider above — isolated
// hosts/tests): `enabled` gates arming exactly as before (doc 08 §4.3's
// fullscreen scoping), `locked` locks as passed. The player composition is
// the only in-repo caller (PlayerScreen mounts the provider around itself).

import { useCallback, useEffect, useState } from "react";
import { usePlayerControlsOrNull } from "./context";

/** [PROPOSAL] idle delay before chrome hides (doc 08 §4.3 leaves timing open). */
export const CHROME_AUTO_HIDE_DELAY_MS = 3000;

export type ChromeVisibility = "visible" | "hidden" | "locked";

export interface ChromeAutoHideOptions {
  /**
   * Legacy arming switch (doc 08 §4.3 scoped auto-hide to fullscreen).
   * Inside the player composition this is superseded — the fade is armed in
   * windowed AND fullscreen (w26.f10 F4); standalone callers keep the gate.
   */
  readonly enabled: boolean;
  /**
   * A menu/panel is open — chrome locks visible. Inside the player
   * composition the flag is honored only while the engine reports media
   * open (the screen's legacy call also folded the retired no-media gate
   * into this flag — see the compensation note in the hook body).
   */
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
  // Player-composition truth (w26.f10 F4). The engine-reported playback
  // state is the same seam the overlays consume — and it now survives route
  // remounts (context-bridge snapshot seed, w26.f10 F3).
  const controls = usePlayerControlsOrNull();
  const inPlayerComposition = controls !== null;

  const armed = inPlayerComposition || enabled;
  // w26.f10 cleanup: the call site passes the panels/menus lock directly
  // (doc 08 §4.5); the retired empty-state compensation is gone.
  const lockedEffective = locked;

  const [hidden, setHidden] = useState(false);
  const show = useCallback(() => {
    setHidden(false);
  }, []);

  // Leaving auto-hide (standalone `enabled` off) or entering a lock must
  // reveal chrome — a stale `hidden` must never survive those transitions.
  useEffect(() => {
    if (!armed || lockedEffective) setHidden(false);
  }, [armed, lockedEffective]);

  // Idle timer: (re)starts whenever chrome is shown while auto-hide is
  // active and unlocked; cleared otherwise. `hidden` in the deps makes any
  // reveal restart the countdown.
  useEffect(() => {
    if (!armed || lockedEffective || hidden) return;
    const timer = setTimeout(() => setHidden(true), delayMs);
    return () => clearTimeout(timer);
  }, [armed, lockedEffective, hidden, delayMs]);

  // Activity reveals chrome: any pointer move/down or key press.
  useEffect(() => {
    if (!armed || lockedEffective) return;
    const onActivity = () => setHidden(false);
    window.addEventListener("pointermove", onActivity);
    window.addEventListener("pointerdown", onActivity);
    window.addEventListener("keydown", onActivity);
    return () => {
      window.removeEventListener("pointermove", onActivity);
      window.removeEventListener("pointerdown", onActivity);
      window.removeEventListener("keydown", onActivity);
    };
  }, [armed, lockedEffective]);

  const visibility: ChromeVisibility = lockedEffective ? "locked" : hidden ? "hidden" : "visible";
  return { visibility, show };
}
