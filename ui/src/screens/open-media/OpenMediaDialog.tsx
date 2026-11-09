// Open media flow (Solstice app-shell workstream, 2026-11-08).
//
// The DECOUPLED TRIGGER CONTRACT: any surface (the player empty-state CTA,
// the File menu, future shortcuts) asks for the open-media flow by
// dispatching `window.dispatchEvent(new CustomEvent("lumen:open-media"))`.
// The AppShell owns the single listener and mounts this dialog; the event
// name below is the cross-agent contract — do not rename it.
//
// The dialog offers the DemoTransport clip catalog plus the transport's
// remembered openings ("Recent"), and opens the picked SourceSpec through
// the ClientProvider transport (`session.open`), then routes to /player.
//
// SOLSTICE v2 (2026-11-09, audit pass): the subtitle's "or paste a path" is
// now REAL — a path/URL input opens anything the demo transport accepts
// (a file path or an http(s) URL, parsed into the same SourceSpec shape the
// clips use). The dialog centers at the viewport's OPTICAL center (45%) on
// the shared .solstice-scrim veil, carries a visible quiet Close affordance
// (plus Escape and click-outside), and shows every source location IN FULL
// (two-line rows, wrap-anywhere mono, title attribute) instead of
// truncating the paths behind ellipses.

import { useId, useRef, useState } from "react";
import type { FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { IconButton } from "../../components/button/IconButton";
import { Icon } from "../../components/icon/Icon";
import { TextField } from "../../components/form/TextField";
import { Dialog } from "../../components/surface/Surface";
import { useClient } from "../../client/ClientProvider";
import { openPathRequest, openPathRoute } from "../../client/open-path";
import { DEMO_CLIPS, isDemoTransport } from "../../client/demo-transport";
import type { SourceSpec } from "../../client/transport";
import { useMsg } from "../../i18n/messages";
import "./open-media.css";

/** The cross-agent CustomEvent name (see module header — never rename). */
export const OPEN_MEDIA_EVENT = "lumen:open-media";

// i18n keys wired centrally (messages.ts): open.close, open.path.label,
// open.path.placeholder, open.path.invalid.

/** Human location string for a source spec (data, not UI copy). */
const sourceLocation = (source: SourceSpec): string =>
  source.kind === "file" ? source.path : source.url;

/**
 * Parse a raw path/URL line into the SourceSpec the transport accepts.
 * Anything with a URL scheme (http://, https://, …) opens as a url source;
 * everything else is treated as a file path — the demo transport synthesizes
 * honest info for both (see client/demo-transport.ts infoForSource).
 */
export function parseSourceInput(raw: string): SourceSpec | null {
  const value = raw.trim();
  if (value.length === 0) return null;
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(value)) return { kind: "url", url: value };
  return { kind: "file", path: value };
}

export function OpenMediaDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const m = useMsg();
  const client = useClient();
  const navigate = useNavigate();
  const [selected, setSelected] = useState<SourceSpec | null>(null);
  const [opening, setOpening] = useState(false);
  const [pathValue, setPathValue] = useState("");
  const [pathInvalid, setPathInvalid] = useState(false);
  const nextRequestId = useRef(0);
  const titleId = useId();
  const clipsHeadingId = useId();
  const recentHeadingId = useId();
  const pathFieldId = useId();

  // The "Recent" section exists only when the transport remembers openings
  // (the demo transport does; real hosts will surface their own history).
  const recent = isDemoTransport(client) ? client.recentOpenings : [];

  // v2 §4.1 — single click OPENS the clip. The selection state only exists
  // so keyboard users can focus an item and confirm with the Open button;
  // pointer users never need to know it exists. Enter/Space on a focused
  // list item button opens it (native button semantics).
  //
  // Issue #3 (4.H1 handoff b/d): every open goes through the ONE funnel
  // (client/open-path.ts) — media files `session.open`, playlist files
  // `playlist.load` (parse is engine-side); routing follows the kind.
  const openSource = (source: SourceSpec): void => {
    setOpening(true);
    nextRequestId.current += 1;
    void client
      .send(openPathRequest(source, nextRequestId.current))
      .catch(() => {
        // Transport unreachable — the player screens surface their own
        // honest typed errors; the dialog still closes and routes.
      })
      .finally(() => {
        setOpening(false);
        setSelected(null);
        setPathValue("");
        setPathInvalid(false);
        onClose();
        navigate(openPathRoute(source));
      });
  };

  // The subtitle's promise, made real: paste a path or URL → open it.
  const onPathSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const source = parseSourceInput(pathValue);
    if (source === null) {
      setPathInvalid(true);
      return;
    }
    openSource(source);
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy={titleId}
      className="open-media__surface"
      closeOnOutsideClick
    >
      <div className="open-media">
        <div className="open-media__header">
          <h2 id={titleId} className="open-media__title">
            {m("open.title")}
          </h2>
          {/* v2 §4.4: every dialog carries a visible quiet close affordance
              (Escape and click-outside also close — the Dialog owns both). */}
          <IconButton
            className="open-media__close"
            label={m("open.close")}
            title={m("open.close")}
            size="sm"
            onClick={() => {
              setSelected(null);
              setPathValue("");
              setPathInvalid(false);
              onClose();
            }}
          >
            <Icon name="quit" size={16} />
          </IconButton>
        </div>
        <p className="open-media__subtitle">{m("open.subtitle")}</p>

        <form className="open-media__path" onSubmit={onPathSubmit}>
          <TextField
            id={pathFieldId}
            label={m("open.path.label")}
            placeholder={m("open.path.placeholder")}
            value={pathValue}
            error={pathInvalid ? m("open.path.invalid") : undefined}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => {
              setPathValue(event.target.value);
              if (pathInvalid) setPathInvalid(false);
            }}
          />
          <Button type="submit" disabled={opening} className="open-media__path-open">
            {m("open.action")}
          </Button>
        </form>

        <section className="open-media__section" aria-labelledby={clipsHeadingId}>
          <h3 id={clipsHeadingId} className="open-media__section-title">
            {m("open.demo-clips")}
          </h3>
          <ul className="open-media__list">
            {DEMO_CLIPS.map((clip) => (
              <li key={clip.id}>
                <button
                  type="button"
                  className="open-media__item"
                  aria-pressed={
                    selected !== null &&
                    sourceLocation(selected) === sourceLocation(clip.source)
                  }
                  onClick={() => openSource(clip.source)}
                >
                  <span className="open-media__thumb solstice-sun" aria-hidden="true" />
                  <span className="open-media__item-text">
                    <span className="open-media__item-title">{m(clip.titleKey)}</span>
                    <span
                      className="open-media__item-source"
                      title={sourceLocation(clip.source)}
                    >
                      {sourceLocation(clip.source)}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section className="open-media__section" aria-labelledby={recentHeadingId}>
          <h3 id={recentHeadingId} className="open-media__section-title">
            {m("open.recent")}
          </h3>
          {recent.length === 0 ? (
            <p className="open-media__empty">{m("open.recent.empty")}</p>
          ) : (
            <ul className="open-media__list">
              {recent.map((entry) => (
                <li key={sourceLocation(entry.source)}>
                  <button
                    type="button"
                    className="open-media__item"
                    aria-pressed={
                      selected !== null &&
                      sourceLocation(selected) === sourceLocation(entry.source)
                    }
                    onClick={() => openSource(entry.source)}
                  >
                    <span className="open-media__thumb solstice-sun" aria-hidden="true" />
                    <span className="open-media__item-text">
                      <span className="open-media__item-title">
                        {entry.title ?? sourceLocation(entry.source)}
                      </span>
                      <span
                        className="open-media__item-source"
                        title={sourceLocation(entry.source)}
                      >
                        {sourceLocation(entry.source)}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <div className="open-media__actions">
          <Button
            variant="primary"
            disabled={selected === null}
            loading={opening}
            onClick={() => {
              if (selected !== null) openSource(selected);
            }}
          >
            {m("open.action")}
          </Button>
          <Button
            onClick={() => {
              setSelected(null);
              setPathValue("");
              setPathInvalid(false);
              onClose();
            }}
          >
            {m("open.cancel")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
