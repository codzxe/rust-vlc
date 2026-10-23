// Command-layer client seam (doc 05 §5.1, doc 08 §8). The wire types are
// the 1.D4 codegen output (src/generated/commands.ts — never hand-edit);
// this file is the ONLY module that reaches into src/generated, so a later
// move to src/bindings/ touches one import site.

import type {
  CommandListing,
  CommandRequest,
  CommandResponse,
  Event,
} from "../generated/commands";

export type { CommandRequest, CommandResponse, Event };
// Wire types consumed by the player state seam (4.C2 context.tsx) — type-only
// re-exports keep src/generated imports in one place (single import-site rule).
export type {
  CommandError,
  CommandResult,
  Duration,
  PlaybackState,
  PlaylistChange,
  PlaylistItemId,
  RepeatMode,
  ResultWire,
  SourceSpec,
  StateSnapshot,
  TimeRange,
  TrackId,
  TrackInfo,
  TrackKind,
  TrackSelector,
  TrackSelections,
} from "../generated/commands";

/** Host-configurable transport: apps/desktop (Tauri) and apps/web (wasm). */
export interface CommandTransport {
  send(request: CommandRequest): Promise<CommandResponse>;
  subscribe(listener: (event: Event) => void): () => void;
}

const EMPTY_LISTINGS: readonly CommandListing[] = [];

// Scaffold/test transport. Honest about its limits: only
// `system.list-commands` is simulated (empty listing); every other command
// answers with the typed `feature-not-exposed` error envelope instead of
// inventing data.
export class MockTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  private generation = 0;
  private readonly sentRequests: CommandRequest[] = [];

  /** Requests seen so far — lets tests assert what the UI issued. */
  get sent(): readonly CommandRequest[] {
    return this.sentRequests;
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sentRequests.push(request);
    const requestId = request.requestId;
    const gen = ++this.generation;
    let result: CommandResponse["result"];
    switch (request.cmd) {
      case "system.list-commands":
        result = {
          completed: { kind: "command-listings", commands: [...EMPTY_LISTINGS] },
        };
        break;
      default:
        result = {
          error: {
            code: "feature-not-exposed",
            details: { cmd: request.cmd },
            message: "mock transport: command not simulated yet",
            stage: "control",
          },
        };
    }
    return { gen, requestId, result };
  }
}
