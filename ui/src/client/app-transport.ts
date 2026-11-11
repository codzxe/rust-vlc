// App-layer transport selection (issue #3 — the 4.H1 handoff a; doc 05
// §5.1; phase4-exit §4.4 gap 4): the ONE place that decides which
// CommandTransport the app runs on.
//
// The probe rule: inside the Lumen desktop shell, Tauri 2.x always
// injects `window.__TAURI_INTERNALS__`. When that surface is present
// the app runs on the REAL transport — TauriTransport over the shell's
// single `lumen_dispatch` command and the `lumen://event` channel. When
// it is absent this is a plain browser context (dev shell, visual
// harness) and the friendly DemoTransport simulation drives the UI.
//
// Honesty rule: the choice is VISIBLE — exactly one debug line states
// which transport is live and what that means (real engine vs
// simulated engine, no decoded media). Nothing silently pretends.

import { DemoTransport } from "./demo-transport";
import type { CommandTransport } from "./transport";
import { TauriTransport, detectTauriBackend } from "./tauri-transport";

/** Which host the app layer decided it is running in. */
export type AppTransportOrigin =
  | "desktop-shell"
  | "browser-fallback";

/** The app layer's transport decision. */
export interface AppTransport {
  /** The transport the whole UI runs on. */
  readonly transport: CommandTransport;
  /** Where it came from (drives chrome like the demo badge). */
  readonly origin: AppTransportOrigin;
  /** The one-line honest description (also the debug log's message). */
  readonly label: string;
}

/**
 * True when running inside the Lumen desktop shell (the webview host
 * surface is present). This is the same probe `selectAppTransport`
 * uses — chrome that needs to know the host reads THIS, never a guess.
 */
export function isDesktopShell(): boolean {
  return detectTauriBackend() !== null;
}

export interface SelectAppTransportOptions {
  /** Browser-context transport factory (defaults to the demo simulation). */
  readonly browserFallback?: () => CommandTransport;
  /** Channel-listen failure surface (diagnostics); desktop path only. */
  readonly onListenFailure?: (error: unknown) => void;
  /** Log sink for the one honest choice line (defaults to console.debug). */
  readonly log?: (message: string) => void;
}

/**
 * Probe the host and build the transport the app runs on. Exactly one
 * debug line announces the choice — never silent, never spammy (one
 * call at app startup, one line).
 */
export function selectAppTransport(
  options: SelectAppTransportOptions = {},
): AppTransport {
  const log = options.log ?? ((message: string) => console.debug(message));
  const backend = detectTauriBackend();
  if (backend !== null) {
    // The desktop shell: the real transport. The backend is probed
    // once and handed straight to the transport (no second probe to
    // drift against the decision).
    const transport = new TauriTransport(backend, options.onListenFailure);
    const label =
      "lumen app transport: desktop shell — TauriTransport (real engine: every command via lumen_dispatch, events on lumen://event)";
    log(label);
    return { transport, origin: "desktop-shell", label };
  }
  // Browser context: the demo simulation (an honest simulated engine —
  // real wire protocol, no decoded media; badged in the app chrome).
  const transport = options.browserFallback?.() ?? new DemoTransport();
  const label =
    "lumen app transport: browser fallback — DemoTransport (simulated engine: real wire protocol, no decoded media)";
  log(label);
  return { transport, origin: "browser-fallback", label };
}
