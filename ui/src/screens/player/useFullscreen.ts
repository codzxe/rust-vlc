// Card 4.C1 — fullscreen on the player container (doc 08 §4.3, doc 05 §2.7).
//
// `video.set-fullscreen` is a SHELL-target command (doc 05 §2.7): desktop
// Tauri window fullscreen; web Fullscreen API on the player container. The
// UI state is synced ONLY from `fullscreenchange` events — a request is never
// assumed granted (honest state; the browser can refuse, e.g. no user
// gesture). ESC exits natively (browser) in addition to the keymap-driven
// handling in PlayerScreen (doc 05 §8.1 binding `exit-fullscreen`).

import { useCallback, useEffect, useRef, useState } from "react";
import type { RefObject } from "react";

/** Raised when the host exposes no Fullscreen API (request/exit refuse). */
export class FullscreenUnsupportedError extends Error {
  constructor() {
    super("fullscreen: the Fullscreen API is unavailable on this host");
    this.name = "FullscreenUnsupportedError";
  }
}

export interface FullscreenController {
  /** True only while document.fullscreenElement is set (event-synced). */
  readonly isFullscreen: boolean;
  /** False when the host exposes no Fullscreen API (honest capability). */
  readonly supported: boolean;
  /** Request fullscreen on the target container. Rejects if unsupported. */
  readonly request: () => Promise<void>;
  /** Exit fullscreen. Rejects if unsupported. */
  readonly exit: () => Promise<void>;
  /** Toggle based on the LIVE document state (not cached React state). */
  readonly toggle: () => Promise<void>;
}

export function useFullscreen(target: RefObject<HTMLElement | null>): FullscreenController {
  const [, setRevision] = useState(0);
  const revisionRef = useRef(0);

  // State is re-derived from document.fullscreenElement on every
  // fullscreenchange — never assumed from request()/exit() return values.
  useEffect(() => {
    const onChange = () => {
      revisionRef.current += 1;
      setRevision(revisionRef.current);
    };
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  const supported =
    typeof document !== "undefined" &&
    typeof document.documentElement.requestFullscreen === "function" &&
    typeof document.exitFullscreen === "function";

  const request = useCallback((): Promise<void> => {
    const element = target.current;
    if (!supported || element === null) return Promise.reject(new FullscreenUnsupportedError());
    return element.requestFullscreen().then(() => undefined);
  }, [supported, target]);

  const exit = useCallback((): Promise<void> => {
    if (!supported) return Promise.reject(new FullscreenUnsupportedError());
    return document.exitFullscreen().then(() => undefined);
  }, [supported]);

  const toggle = useCallback((): Promise<void> => {
    // Live document truth, not React cache. (?? — jsdom leaves the property
    // undefined rather than null; treat both as "not fullscreen".)
    if ((document.fullscreenElement ?? null) !== null) return exit();
    return request();
  }, [exit, request]);

  return {
    isFullscreen: (document.fullscreenElement ?? null) !== null,
    supported,
    request,
    exit,
    toggle,
  };
}
