// The desktop transport target (card 4.H1; doc 05 §5.1): the REAL
// CommandTransport that routes every UI-originated command through the
// shell's single Tauri command `lumen_dispatch`, and forwards engine
// events from the shell's `lumen://event` channel to subscribers.
//
// No `@tauri-apps/api` import: the npm package is not in the dependency
// allowlist yet (docs/dependencies/records/tauri-wry-2.x.md —
// "exception-ADR-004, NOT imported"). Tauri 2.x always injects
// `window.__TAURI_INTERNALS__` — the exact surface that
// `@tauri-apps/api/core` and `/event` wrap — so this module binds to the
// internals directly: same wire, no new dependency. The backend seam
// ([TauriBackend]) is injectable so unit tests never need a webview
// (this sandbox cannot run one; see apps/desktop/tauri/INFRA-BLOCKED.md).
//
// Wire contract: the generated src/generated/commands.ts types ARE the
// contract (1.D4 codegen of the Rust serde types) — requests and
// responses cross the IPC as those exact JSON shapes.

import type {
  CommandError,
  CommandRequest,
  CommandResponse,
  Event,
} from "../generated/commands";
import type { CommandTransport } from "./transport";

/** The one and only registered shell command (doc 05 §5.1; dispatch.rs). */
export const LUMEN_DISPATCH_COMMAND = "lumen_dispatch";

/** The shell's engine-event channel (doc 05 §5.1 "events"). */
export const LUMEN_EVENT_CHANNEL = "lumen://event";

/** What `plugin:event|listen` hands the handler (Tauri 2.x event frame). */
export interface TauriEventFrame {
  event: string;
  id: number;
  payload: unknown;
}

/**
 * The Tauri 2.x backend seam — mockable. Mirrors @tauri-apps/api
 * `core.invoke` / `event.listen` so the ADR-004 admission build can swap
 * the internals binding for the package without touching TauriTransport.
 */
export interface TauriBackend {
  invoke(command: string, payload?: Record<string, unknown>): Promise<unknown>;
  listen(
    event: string,
    handler: (frame: TauriEventFrame) => void,
  ): Promise<() => void>;
}

/** The always-injected internals surface (Tauri 2.x). */
interface TauriInternals {
  invoke(command: string, payload?: unknown, options?: unknown): Promise<unknown>;
  transformCallback(
    callback: (response: unknown) => void,
    once?: boolean,
  ): number;
}

/** Thrown when constructed outside the desktop shell (no webview host). */
export class TauriTransportUnavailableError extends Error {
  constructor() {
    super(
      "tauri-transport: window.__TAURI_INTERNALS__ is absent — not running " +
        "inside the Lumen desktop shell; use another transport outside it",
    );
    this.name = "TauriTransportUnavailableError";
  }
}

function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as { code?: unknown; message?: unknown; stage?: unknown };
  return (
    typeof candidate.code === "string" &&
    typeof candidate.message === "string" &&
    typeof candidate.stage === "string"
  );
}

/** Bind a backend to the injected internals (what @tauri-apps/api wraps). */
function internalsBackend(internals: TauriInternals): TauriBackend {
  return {
    invoke: (command, payload) => internals.invoke(command, payload),
    listen: (event, handler) =>
      internals
        .invoke("plugin:event|listen", {
          event,
          // [VERIFY at the ADR-004 admission build] the frame shape and the
          // "Any" default target are per Tauri 2.x api/event; unverifiable
          // without a webview here (INFRA-BLOCKED.md §unblock step 3).
          // (CI half: tauri-launch.yml — clears when its no-emit-failure +
          // clean-exit proof lands on a real run; the GUI half stays human.)
          target: { kind: "Any" },
          handler: internals.transformCallback((frame) =>
            handler(frame as TauriEventFrame),
          ),
        })
        .then((eventId) => {
          const id = typeof eventId === "number" ? eventId : Number(eventId);
          return () => {
            void internals.invoke("plugin:event|unlisten", { event, eventId: id });
          };
        }),
  };
}

/** The webview-host probe: null outside the Lumen desktop shell. */
export function detectTauriBackend(): TauriBackend | null {
  const internals = (
    globalThis as { __TAURI_INTERNALS__?: TauriInternals }
  ).__TAURI_INTERNALS__;
  return internals ? internalsBackend(internals) : null;
}

/**
 * The desktop CommandTransport. Every `send` goes through the single
 * `lumen_dispatch` command; every engine event on `lumen://event` reaches
 * `subscribe` listeners in channel order (the shell's EventSink preserves
 * engine order — §4.3 guarantee 2).
 */
export class TauriTransport implements CommandTransport {
  private readonly backend: TauriBackend;
  private readonly onListenFailure: ((error: unknown) => void) | undefined;

  constructor(
    backend: TauriBackend,
    onListenFailure?: (error: unknown) => void,
  ) {
    this.backend = backend;
    this.onListenFailure = onListenFailure;
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    let response: unknown;
    try {
      // The Rust side is `async fn lumen_dispatch(state, request: CommandRequest)`
      // (dispatch.rs module docs): the payload key MUST be the parameter name.
      response = await this.backend.invoke(LUMEN_DISPATCH_COMMAND, { request });
    } catch (rejection) {
      // The shell answers `Err(CommandError)` as a rejected promise whose
      // payload is the typed error wire object — normalize it into the same
      // error envelope the engine produces so UI result handling stays
      // uniform. `gen` is unknown on this path (only the shell knows it);
      // 0 = "unset", never presented as engine truth.
      if (isCommandError(rejection)) {
        return { gen: 0, requestId: request.requestId, result: { error: rejection } };
      }
      throw new Error(`lumen_dispatch IPC failed: ${String(rejection)}`, {
        cause: rejection,
      });
    }
    return response as CommandResponse;
  }

  subscribe(listener: (event: Event) => void): () => void {
    let unlisten: (() => void) | null = null;
    let unsubscribed = false;
    void this.backend
      .listen(LUMEN_EVENT_CHANNEL, (frame) => listener(frame.payload as Event))
      .then((stop) => {
        if (unsubscribed) {
          // Unsubscribed before the channel came up — release immediately.
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((error: unknown) => {
        // The channel is the shell's lifeline; a failure here is surfaced to
        // the app layer (which owns diagnostics), never silently swallowed.
        this.onListenFailure?.(error);
      });
    return () => {
      unsubscribed = true;
      unlisten?.();
    };
  }
}

/** Build the transport from the ambient webview host (the app entry uses this). */
export function createTauriTransport(
  onListenFailure?: (error: unknown) => void,
): TauriTransport {
  const backend = detectTauriBackend();
  if (!backend) {
    throw new TauriTransportUnavailableError();
  }
  return new TauriTransport(backend, onListenFailure);
}
