// Card 4.D2 — media-info state seam (doc 08 §4.2 "engine-owned, UI-rendered").
//
// OWNERSHIP: ui/src/screens/media-info/** (wave program row 4.D2). Shape
// rule (mirrors 4.C2/4.D1): only engine-reported state — the `media.get-info`
// command result and `media.info-ready` events (doc 05 catalog) — plus
// honestly-labelled unknowns. `info: null` means "no media info reported"
// (nothing open / command failed), never a fabricated MediaInfo.

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useClient } from "../../client/ClientProvider";
import type { Event, MediaInfo } from "../../client/transport";

export interface MediaInfoState {
  /**
   * Last engine-reported media info; `null` = nothing reported (no media
   * open, or the command errored) — the panel renders the honest empty
   * state, never placeholder values that look real.
   */
  readonly info: MediaInfo | null;
  /** True once `media.get-info` answered `feature-not-exposed` — panel then
   * shows the honest not-connected note instead of pretending. */
  readonly notExposed: boolean;
}

const MediaInfoContext = createContext<MediaInfoState | null>(null);

export function MediaInfoProvider({ children }: { children: ReactNode }) {
  const transport = useClient();
  const nextRequestId = useRef(0);
  const [info, setInfo] = useState<MediaInfo | null>(null);
  const [notExposed, setNotExposed] = useState(false);

  // Seed from the session's current media via the command seam
  // (`media.get-info` is session-scoped — no args, doc 05 §2.9).
  useEffect(() => {
    let cancelled = false;
    nextRequestId.current += 1;
    void (async () => {
      try {
        const response = await transport.send({
          requestId: nextRequestId.current,
          cmd: "media.get-info",
        });
        if (cancelled) return;
        const { result } = response;
        if ("completed" in result && result.completed.kind === "media-info") {
          setInfo(result.completed.info);
        } else if ("error" in result) {
          if (result.error.code === "feature-not-exposed") setNotExposed(true);
          // Other error codes (not-found / not-ready / …) = nothing open:
          // keep `info: null` — the honest empty state.
        }
      } catch {
        // Host unreachable — treat as not-exposed honest note.
        setNotExposed(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport]);

  // Stay authoritative from `media.info-ready` events (new media opened).
  useEffect(() => {
    return transport.subscribe((event: Event) => {
      if (event.kind === "media.info-ready") setInfo(event.info);
    });
  }, [transport]);

  return (
    <MediaInfoContext.Provider value={{ info, notExposed }}>
      {children}
    </MediaInfoContext.Provider>
  );
}

export function useMediaInfo(): MediaInfoState {
  const ctx = useContext(MediaInfoContext);
  if (ctx === null) {
    throw new Error("useMediaInfo requires a MediaInfoProvider ancestor");
  }
  return ctx;
}
