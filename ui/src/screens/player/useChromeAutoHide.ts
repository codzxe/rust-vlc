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
// w26.gfix-F (g06 P1-2): the lock is now fed from TWO sources. The call
// site's `locked` prop keeps carrying the host's open-panels truth, and the
// hook itself observes the document for OPEN OVERLAYS — any open menu
// (`[role="menu"]`: the canvas context menu, the ControlBar app menu, the
// playlist context menus) or modal dialog (`[role="dialog"]`: the player
// dialogs + the shell's open-media dialog) locks the chrome visible. Menus
// and dialogs are conditionally rendered by their owners, so "present in
// the DOM" === "open" — no ownership coupling, no API change (the writers
// live in other lanes: ControlBar.tsx, PlayerScreen.tsx, components/menu).
// A MutationObserver re-checks on every DOM change; the check is one
// querySelector. While locked: no idle timer, no fade — the earlier state
// (idle fade firing under an open menu, stranding focus inside a faded
// panel) is gone.
//
// LEGACY STANDALONE SEMANTICS (no PlayerControlsProvider above — isolated
// hosts/tests): `enabled` gates arming exactly as before (doc 08 §4.3's
// fullscreen scoping), `locked` locks as passed. The player composition is
// the only in-repo caller (PlayerScreen mounts the provider around itself).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { usePlayerControlsOrNull } from "./context";

/** [PROPOSAL] idle delay before chrome hides (doc 08 §4.3 leaves timing open). */
export const CHROME_AUTO_HIDE_DELAY_MS = 3000;

/**
 * w26.gfix-F (g06 P1-2): an open overlay anywhere in the document — an open
 * menu (`[role="menu"]`, conditionally rendered by its owner: present in
 * the DOM === open) or a modal dialog (`[role="dialog"]`, `Dialog` renders
 * null while closed). While one is present the chrome locks visible: an
 * idle fade firing under an open overlay strands keyboard focus inside a
 * faded panel (doc 08 §4.5).
 */
const OPEN_OVERLAY_SELECTOR = '[role="menu"], [role="dialog"]';

export type ChromeVisibility = "visible" | "hidden" | "locked";

export interface ChromeAutoHideOptions {
  /**
   * Legacy arming switch (doc 08 §4.3 scoped auto-hide to fullscreen).
   * Inside the player composition this is superseded — the fade is armed in
   * windowed AND fullscreen (w26.f10 F4); standalone callers keep the gate.
   */
  readonly enabled: boolean;
  /**
   * A menu/panel is open — chrome locks visible. The call site's open-panels
   * truth (doc 08 §4.5), honored directly. Orthogonally, the hook itself
   * locks while any open overlay is present in the document (open menu /
   * modal dialog — w26.gfix-F, g06 P1-2).
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

  // w26.gfix-F (g06 P1-2): overlay lock — the DOM-truth half of the lock
  // seam (the call site's `locked` prop is the other half). One query per
  // DOM mutation; the same value short-circuits setState.
  const [overlayOpen, setOverlayOpen] = useState(false);
  useEffect(() => {
    if (typeof document === "undefined" || typeof MutationObserver === "undefined") {
      return;
    }
    const check = (): void => {
      setOverlayOpen(document.querySelector(OPEN_OVERLAY_SELECTOR) !== null);
    };
    check();
    const observer = new MutationObserver(check);
    observer.observe(document.body, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, []);

  // w26.f10 cleanup: the call site passes the panels lock directly (doc 08
  // §4.5); the retired empty-state compensation is gone.
  const lockedEffective = locked || overlayOpen;

  const [hidden, setHidden] = useState(false);

  // w26.k06 (J06-1, P1): the idle countdown is a REF-HELD timer restarted
  // imperatively — activity never touches state. The wave-I nonce fix
  // (w26.i02) restarted the countdown by bumping a state nonce, which
  // re-rendered the host (PlayerScreen — the app's biggest tree) on EVERY
  // pointermove/pointerdown/keydown. The scheduler below keeps the i02
  // semantics exactly — activity while visible clears + re-arms the
  // countdown — at the cost of one clearTimeout/setTimeout pair per event,
  // zero renders (the reveal is a same-value functional update; React
  // bails out entirely while chrome is already visible).
  const idleTimerRef = useRef<number | null>(null);
  const clearIdleTimer = useCallback((): void => {
    if (idleTimerRef.current !== null) {
      window.clearTimeout(idleTimerRef.current);
      idleTimerRef.current = null;
    }
  }, []);
  const armIdleTimer = useCallback((): void => {
    clearIdleTimer();
    idleTimerRef.current = window.setTimeout(() => {
      idleTimerRef.current = null;
      setHidden(true);
    }, delayMs);
  }, [clearIdleTimer, delayMs]);

  const show = useCallback(() => {
    setHidden(false);
  }, []);

  // Idle-timer lifecycle: armed + unlocked + visible → the countdown runs;
  // anything else disarms it. Leaving auto-hide (standalone `enabled` off)
  // or entering a lock must also reveal chrome — a stale `hidden` must
  // never survive those transitions. A reveal (hidden → false) re-arms;
  // while hidden the timer stays off until activity/show reveals.
  useEffect(() => {
    if (!armed || lockedEffective) {
      clearIdleTimer();
      setHidden(false);
      return;
    }
    if (hidden) return;
    armIdleTimer();
    return clearIdleTimer;
  }, [armed, lockedEffective, hidden, armIdleTimer, clearIdleTimer]);

  // Activity reveals chrome: any pointer move/down or key press.
  useEffect(() => {
    if (!armed || lockedEffective) return;
    // w26.i02 (Lead-landed): activity while VISIBLE must also restart the
    // countdown — setHidden(false) is a no-op then, so the deadline must be
    // re-armed directly. w26.k06 (J06-1): that restart is the imperative
    // scheduler (armIdleTimer), NOT a state update — a same-value
    // functional reveal (React bails: no render, no effect churn) plus a
    // ref-held timer restart. From HIDDEN the reveal re-renders once and
    // the lifecycle effect above re-arms (same tick — same semantics).
    const onActivity = (): void => {
      setHidden((currentlyHidden) => (currentlyHidden ? false : currentlyHidden));
      armIdleTimer();
    };
    window.addEventListener("pointermove", onActivity);
    window.addEventListener("pointerdown", onActivity);
    window.addEventListener("keydown", onActivity);
    return () => {
      window.removeEventListener("pointermove", onActivity);
      window.removeEventListener("pointerdown", onActivity);
      window.removeEventListener("keydown", onActivity);
    };
  }, [armed, lockedEffective, armIdleTimer]);

  const visibility: ChromeVisibility = lockedEffective ? "locked" : hidden ? "hidden" : "visible";
  // w26.k06 (J06-3, P2): stable identity — the hook used to return a fresh
  // object literal per render, so consumers keying effect deps on it
  // (PlayerScreen's keymap keydown listener) removed/re-added the listener
  // on every render. `show` is already referentially stable; the memoized
  // object now only changes identity when visibility actually flips.
  return useMemo(() => ({ visibility, show }), [visibility, show]);
}
