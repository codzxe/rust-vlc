// /open-network-stream (Solstice app-shell workstream, 2026-11-08) — the
// real screen behind the doc 08 §1.6 route: a sticky-note card on the
// sunset, a validated URL field (http/https only), a localStorage-persisted
// recent-streams list, and an Open action that drives `session.open`
// through the ClientProvider transport and routes to /player.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { TextField } from "../../components/form/TextField";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import "./network-stream.css";

/** localStorage key for the persisted recent-streams list (URL strings). */
const RECENT_STREAMS_KEY = "lumen.streams.recent.v1";
const RECENT_LIMIT = 6;

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
    <div className="network-stream solstice-rise">
      <section className="network-stream__card solstice-note" data-tape aria-labelledby="network-stream-title">
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
          <Button type="submit" variant="primary" loading={opening}>
            {m("stream.open")}
          </Button>
        </form>
      </section>

      <section className="network-stream__recents" aria-labelledby="network-stream-recents-title">
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
  );
}
