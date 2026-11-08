// /open-network-stream (Solstice app-shell workstream, 2026-11-08) — the
// real screen behind the doc 08 §1.6 route: a validated URL field (http/
// https only), a localStorage-persisted recent-streams list, and an Open
// action that drives `session.open` through the ClientProvider transport
// and routes to /player.
//
// Solstice v2 craft pass (2026-11-09, brief §1/§2.6): an APP-tier screen —
// the backdrop veils to "calm", the form anchors in a centered column
// (~560px — a form, not a poster), the Open action is a STANDARD primary
// button (no full-width slab), and a secondary "Back to player" affordance
// always exits honestly (v2 audit: no close affordance). The recent list
// rides a quiet glass panel with AA-contrast labels (secondary ink, never
// tertiary on the dark gradient).
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { TextField } from "../../components/form/TextField";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import "./network-stream.css";

/** localStorage key for the persisted recent-streams list (URL strings). */
const RECENT_STREAMS_KEY = "lumen.streams.recent.v1";
const RECENT_LIMIT = 6;

// Secondary exit affordance (v2 audit: the screen had no close/cancel).

/**
 * Is this a wire-valid network URL? http/https only (the demo/real engines
 * speak those; rtsp/ftp etc. would be an honest catalog question, not a
 * guess). Pure + total so tests pin it directly.
 */
export function isValidNetworkUrl(candidate: string): boolean {
  try {
    const parsed = new URL(candidate);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

function loadRecentStreams(): readonly string[] {
  try {
    const raw = window.localStorage.getItem(RECENT_STREAMS_KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((entry): entry is string => typeof entry === "string").slice(0, RECENT_LIMIT);
  } catch {
    return []; // unreadable/corrupt store — honest empty, never a crash
  }
}

function saveRecentStreams(urls: readonly string[]): void {
  try {
    window.localStorage.setItem(RECENT_STREAMS_KEY, JSON.stringify(urls.slice(0, RECENT_LIMIT)));
  } catch {
    // Storage unavailable (private mode etc.) — the list just won't persist.
  }
}

export function NetworkStreamScreen() {
  const m = useMsg();
  const client = useClient();
  const navigate = useNavigate();
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [recents, setRecents] = useState<readonly string[]>(() => loadRecentStreams());
  const [opening, setOpening] = useState(false);
  const nextRequestId = useRef(0);

  // Solstice v2 (brief §1): APP-tier screen — calm backdrop while mounted,
  // cleaned up on unmount.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  const openStream = (target: string): void => {
    if (!isValidNetworkUrl(target)) {
      setError(m("stream.invalid"));
      return;
    }
    setError(null);
    const next = [target, ...recents.filter((entry) => entry !== target)].slice(0, RECENT_LIMIT);
    setRecents(next);
    saveRecentStreams(next);
    setOpening(true);
    nextRequestId.current += 1;
    void client
      .send({
        cmd: "session.open",
        requestId: nextRequestId.current,
        args: { source: { kind: "url", url: target } },
      })
      .catch(() => {
        // Transport unreachable — the player surfaces its own honest error.
      })
      .finally(() => {
        setOpening(false);
        navigate("/player");
      });
  };

  return (
    <div className="network-stream">
      {/* v2 §2.5: the form + recents ride ONE group with auto block margins —
         the composition centers vertically on tall viewports (symmetric
         breathing room around the centered card) and stays fully reachable
         when it outgrows the viewport (auto margins, unlike
         justify-content: center, never clip the scrollable top). */}
      <div className="network-stream__group">
        <section
          className="network-stream__card solstice-rise"
          aria-labelledby="network-stream-title"
          data-testid="network-stream-card"
        >
          <h1 id="network-stream-title" className="network-stream__title">
            {m("stream.title")}
          </h1>
          <p className="network-stream__subtitle">{m("stream.subtitle")}</p>

          <form
            className="network-stream__form"
            onSubmit={(event) => {
              event.preventDefault();
              openStream(url.trim());
            }}
          >
            <TextField
              label={m("stream.url.label")}
              placeholder={m("stream.url.placeholder")}
              value={url}
              error={error ?? undefined}
              type="url"
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => {
                setUrl(event.target.value);
                // Clear the error as soon as the user edits the rejected value.
                if (error !== null) setError(null);
              }}
            />
            <div className="network-stream__actions">
              <Button type="submit" variant="primary" loading={opening} data-testid="network-stream-open">
                {m("stream.open")}
              </Button>
              <Button
                variant="default"
                onClick={() => navigate("/player")}
                data-testid="network-stream-back"
              >
                {m("stream.back")}
              </Button>
            </div>
          </form>
        </section>

        <section
          className="network-stream__recents"
          aria-labelledby="network-stream-recents-title"
          data-testid="network-stream-recents"
        >
          <h2 id="network-stream-recents-title" className="network-stream__recents-title">
            {m("stream.recent")}
          </h2>
          {recents.length === 0 ? (
            <p className="network-stream__recents-empty">{m("stream.recent.empty")}</p>
          ) : (
            <ul className="network-stream__recents-list">
              {recents.map((entry) => (
                <li key={entry}>
                  <button
                    type="button"
                    className="network-stream__recent"
                    onClick={() => {
                      setUrl(entry);
                      setError(null);
                    }}
                  >
                    {entry}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
