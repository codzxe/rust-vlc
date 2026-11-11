// /open-network-stream — W26 Figma redesign (w26.b15). Frame:
// design/figma/Open network stream.svg (1048×631 rx14 #191919 window on the
// shared scrim), ported 1:1 at the frame's own scale onto the surface
// primitive's dialog chrome (.lumen-dialog: panel fill + hairline +
// --radius-window r14 + --shadow-overlay; scrim + focus trap + Escape/scrim
// close come from <Dialog>):
//   * header — 50px #090909 strip, title 15px semibold #8F8F8F, ✕ close
//     20px off the right edge (same grammar the player dialogs lane w26.a18
//     extracted from this frame family);
//   * "Network URL" section — label #C6C6C6 + helper #6B6B6B ("You can
//     enter multiple links to load.") over the 192px rx5 #090909 textarea
//     with the quiet #404040-class placeholder;
//   * full-bleed hairline divider;
//   * "Playlist Name (optional)" section — two-tone label + helper ("The
//     playlist will be saved and displayed with this title.") + 35px input;
//   * footer — Cancel (default button, frame #1B1B1B) left, Open (tinted
//     button, frame #371F06 + #D37918 ink) right, both 100×32 r6.
// The frame shows NO protocol dropdown and no play-locally toggle — none is
// invented. The persisted recent-streams list (a real behavior contract,
// localStorage `lumen.streams.recent.v1`) rides the URL section as a quiet
// live-history affordance the static frame does not depict.
//
// HONESTY: the open wiring is the untouched pre-W26 contract — `session.open`
// with the ONE url source (validated http/https by isValidNetworkUrl), then
// route to /player. The frame's "multiple links" promise still awaits an
// engine catalog command (doc 05 §2.9 has no multi-open) — the textarea
// validates as one URL and the error is honest. A non-empty Playlist Name
// makes the helper copy TRUE: the opened URL is recorded as a LOCAL saved
// playlist (playlist/playlistsStore, the b09 seam) — no engine command
// exists for it, same honesty rule as the create-playlist dialog.
//
// i18n: reuse first (stream.title / stream.url.label / stream.open /
// stream.invalid / stream.recent / stream.recent.empty + open.cancel /
// open.close); frame copy that has no catalog entry resolves through the
// lane seam (useNetworkStreamMsg, keys/w26.b15.ts).
//
// The screen renders NO <main> of its own — the AppShell provides it; the
// dialog's scrim veils the shell.

import { useId, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button, IconButton } from "../../components/button";
import { Icon } from "../../components/icon";
import { Dialog, Separator } from "../../components/surface";
import "../../components/form/form.css";
import { useClient } from "../../client/ClientProvider";
import { createPlaylist } from "../playlist/playlistsStore";
import { useNetworkStreamMsg } from "./useNetworkStreamMsg";
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
  const m = useNetworkStreamMsg();
  const client = useClient();
  const navigate = useNavigate();
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [recents, setRecents] = useState<readonly string[]>(() => loadRecentStreams());
  const [opening, setOpening] = useState(false);
  const nextRequestId = useRef(0);
  const titleId = useId();
  const urlId = useId();
  const urlHintId = useId();
  const urlErrorId = useId();
  const nameId = useId();
  const nameHintId = useId();
  const recentsTitleId = useId();

  // Every dismissal (Escape / scrim / ✕ / Cancel) is the honest exit the
  // pre-W26 "Back to player" affordance provided: back to the player home.
  const exit = (): void => {
    navigate("/player");
  };

  const openStream = (target: string): void => {
    if (!isValidNetworkUrl(target)) {
      setError(m("stream.invalid"));
      return;
    }
    setError(null);
    const next = [target, ...recents.filter((entry) => entry !== target)].slice(0, RECENT_LIMIT);
    setRecents(next);
    saveRecentStreams(next);
    // Frame section copy: a named stream also lands in the LOCAL playlist
    // library (b09 seam) — "saved and displayed with this title", honestly.
    const title = name.trim();
    if (title !== "") {
      createPlaylist({ title, itemNames: [target] });
    }
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
    <Dialog open onClose={exit} labelledBy={titleId} className="network-stream-dialog" closeOnOutsideClick>
      <div className="network-stream" data-testid="network-stream-card">
        {/* Header — frame strip: 50px #090909, title #8F8F8F, ✕ right. */}
        <header className="network-stream__head">
          <h1 id={titleId} className="network-stream__title">
            {m("stream.title")}
          </h1>
          <IconButton
            className="network-stream__close"
            label={m("open.close")}
            title={m("open.close")}
            size="sm"
            onClick={exit}
          >
            <Icon name="close" size={16} />
          </IconButton>
        </header>

        <form
          className="network-stream__form"
          onSubmit={(event) => {
            event.preventDefault();
            openStream(url.trim());
          }}
        >
          {/* Body — the scrollable middle (header + footer stay pinned). */}
          <div className="network-stream__body">
            <section className="network-stream__section">
              <label className="network-stream__label" htmlFor={urlId}>
                {m("stream.url.label")}
              </label>
              <p className="network-stream__hint" id={urlHintId}>
                {m("stream.url.hint")}
              </p>
              <textarea
                id={urlId}
                className="lumen-field__control network-stream__urls"
                placeholder={m("stream.url.example")}
                spellCheck={false}
                autoComplete="off"
                aria-describedby={error === null ? urlHintId : `${urlHintId} ${urlErrorId}`}
                aria-invalid={error === null ? undefined : true}
                value={url}
                onChange={(event) => {
                  setUrl(event.target.value);
                  // Clear the error as soon as the user edits the rejected value.
                  if (error !== null) setError(null);
                }}
              />
              {error === null ? null : (
                <p className="network-stream__error" id={urlErrorId}>
                  {error}
                </p>
              )}

              {/* Recent streams — the persisted behavior contract, riding the
                  URL section as quiet live history (absent from the static
                  frame). Clicking a row refills the field. */}
              <section
                className="network-stream__recents"
                aria-labelledby={recentsTitleId}
                data-testid="network-stream-recents"
              >
                <h2 id={recentsTitleId} className="network-stream__recents-title">
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
            </section>

            {/* Frame: the full-bleed hairline between the two sections. */}
            <Separator className="network-stream__rule" />

            <section className="network-stream__section">
              <label className="network-stream__label" htmlFor={nameId}>
                {m("stream.playlist.label")}{" "}
                <span className="network-stream__optional">{m("stream.playlist.optional")}</span>
              </label>
              <p className="network-stream__hint" id={nameHintId}>
                {m("stream.playlist.hint")}
              </p>
              <input
                id={nameId}
                className="lumen-field__control network-stream__name"
                type="text"
                placeholder={m("stream.playlist.placeholder")}
                autoComplete="off"
                aria-describedby={nameHintId}
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </section>
          </div>

          {/* Footer — frame Frame 225: Cancel left (default button), Open
              right (the tinted confirm), both 100×32 r6 on the panel fill. */}
          <footer className="network-stream__actions">
            <Button onClick={exit} data-testid="network-stream-back">
              {m("open.cancel")}
            </Button>
            <Button
              type="submit"
              variant="tinted"
              loading={opening}
              data-testid="network-stream-open"
            >
              {m("stream.open")}
            </Button>
          </footer>
        </form>
      </div>
    </Dialog>
  );
}
