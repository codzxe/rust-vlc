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

import { useId, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { Dialog } from "../../components/surface/Surface";
import { useClient } from "../../client/ClientProvider";
import { DEMO_CLIPS, isDemoTransport } from "../../client/demo-transport";
import type { SourceSpec } from "../../client/transport";
import { useMsg } from "../../i18n/messages";
import "./open-media.css";

/** The cross-agent CustomEvent name (see module header — never rename). */
export const OPEN_MEDIA_EVENT = "lumen:open-media";

/** Human location string for a source spec (data, not UI copy). */
const sourceLocation = (source: SourceSpec): string =>
  source.kind === "file" ? source.path : source.url;

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
  const nextRequestId = useRef(0);
  const titleId = useId();
  const clipsHeadingId = useId();
  const recentHeadingId = useId();

  // The "Recent" section exists only when the transport remembers openings
  // (the demo transport does; real hosts will surface their own history).
  const recent = isDemoTransport(client) ? client.recentOpenings : [];

  // v2 §4.1 — single click OPENS the clip. The selection state only exists
  // so keyboard users can focus an item and confirm with the Open button;
  // pointer users never need to know it exists. Enter/Space on a focused
  // list item button opens it (native button semantics).
  const openSource = (source: SourceSpec): void => {
    setOpening(true);
    nextRequestId.current += 1;
    void client
      .send({ cmd: "session.open", requestId: nextRequestId.current, args: { source } })
      .catch(() => {
        // Transport unreachable — the player screens surface their own
        // honest typed errors; the dialog still closes and routes.
      })
      .finally(() => {
        setOpening(false);
        setSelected(null);
        onClose();
        navigate("/player");
      });
  };

  return (
    <Dialog open={open} onClose={onClose} labelledBy={titleId} className="open-media__surface">
      <div className="open-media">
        <h2 id={titleId} className="open-media__title">
          {m("open.title")}
        </h2>
        <p className="open-media__subtitle">{m("open.subtitle")}</p>

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
                  <span className="open-media__item-title">{m(clip.titleKey)}</span>
                  <span className="open-media__item-source">{sourceLocation(clip.source)}</span>
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
                    <span className="open-media__item-title">
                      {entry.title ?? sourceLocation(entry.source)}
                    </span>
                    <span className="open-media__item-source">{sourceLocation(entry.source)}</span>
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
