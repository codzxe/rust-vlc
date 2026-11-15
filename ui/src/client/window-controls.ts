// OS window controls (w28 fix 2 — the frameless title bar half): the
// seam that drives the REAL window from the shell's custom banner.
//
// Why this exists: the window is now frameless (tauri.conf.json
// `decorations: false` — the banner IS the title bar), so the banner's
// right cluster carries real − / □ / × controls. Those are OS chrome
// commands, not engine commands — they ride `plugin:window|…` on the
// same `window.__TAURI_INTERNALS__` surface TauriTransport binds
// (no `@tauri-apps/api` import — the dependency posture of
// records/tauri-wry-2.x.md is unchanged), NEVER through
// `lumen_dispatch` (doc 05 §5.1: engine commands only; the
// single_command.rs audit stays untouched).
//
// Permissions (capabilities/default.json, w28 fix 3):
//   plugin:window|minimize        — core:window:allow-minimize
//   plugin:window|toggle_maximize — core:window:allow-toggle-maximize
//   plugin:window|close           — core:window:allow-close
//   plugin:window|start_dragging  — core:window:allow-start-dragging
//     (the drag region itself is `data-tauri-drag-region` on the banner
//     — drag.js, injected by the runtime, does the mousedown wiring)
// `is_maximized` and the `tauri://resize` listen are covered by
// `core:window:default` / `core:event:default` inside `core:default`.
//
// The backend is injectable (the TauriTransport pattern) so unit tests
// never need a webview.

/** The seam every control goes through — mockable in tests. */
export interface OsWindowBackend {
  /** The window label (`metadata.currentWindow.label`; "main"). */
  readonly label: string;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  /** True when the window is maximized (drives the □/restore glyph). */
  isMaximized(): Promise<boolean>;
  /**
   * Observe window resizing (maximize/restore/reflow) — the shell
   * refreshes its □/restore state from this. Returns the unlisten.
   */
  onResized(handler: () => void): Promise<() => void>;
}

/** The always-injected internals surface (mirrors tauri-transport.ts). */
interface TauriInternals {
  invoke(command: string, payload?: unknown, options?: unknown): Promise<unknown>;
  transformCallback(callback: (response: unknown) => void, once?: boolean): number;
  metadata?: { currentWindow?: { label?: string } };
}

/** Thrown when constructed outside the desktop shell. */
export class OsWindowControlsUnavailableError extends Error {
  constructor() {
    super(
      "window-controls: window.__TAURI_INTERNALS__ is absent — not running " +
        "inside the Lumen desktop shell; the banner renders browser mode",
    );
    this.name = "OsWindowControlsUnavailableError";
  }
}

function backendFromInternals(internals: TauriInternals): OsWindowBackend {
  // The label the runtime stamped into THIS webview — never a hardcoded
  // "main" (a second window would silently drive the wrong one).
  const label =
    internals.metadata?.currentWindow?.label ?? "main";
  const invoke = (command: string): Promise<void> =>
    internals.invoke(command, { label }).then(() => undefined);
  return {
    label,
    minimize: () => invoke("plugin:window|minimize"),
    toggleMaximize: () => invoke("plugin:window|toggle_maximize"),
    close: () => invoke("plugin:window|close"),
    isMaximized: () =>
      internals
        .invoke("plugin:window|is_maximized", { label })
        .then((value) => value === true),
    onResized: (handler) =>
      internals
        .invoke("plugin:event|listen", {
          event: "tauri://resize",
          // Same "Any" target shape TauriTransport.listen uses — the
          // event carries its window's label in the payload, and the
          // shell has exactly one window to care about.
          target: { kind: "Any" },
          handler: internals.transformCallback(() => handler()),
        })
        .then((eventId) => {
          const id = typeof eventId === "number" ? eventId : Number(eventId);
          return () => {
            void internals.invoke("plugin:event|unlisten", {
              event: "tauri://resize",
              eventId: id,
            });
          };
        }),
  };
}

/**
 * Probe the webview host for the OS window seam. Null outside the
 * desktop shell (plain browser: the banner renders browser mode, no
 * OS controls). This is the same presence probe `isDesktopShell()`
 * uses — chrome reads THIS for the controls so the two never drift.
 */
export function detectOsWindowBackend(): OsWindowBackend | null {
  const internals = (
    globalThis as { __TAURI_INTERNALS__?: TauriInternals }
  ).__TAURI_INTERNALS__;
  return internals ? backendFromInternals(internals) : null;
}
