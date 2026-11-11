// Open media flow (app-shell workstream, 2026-11-08; W26 redesign w26.b13).
//
// THE DECOUPLED TRIGGER CONTRACT (never rename): any surface (the player
// empty-state CTA, the File menu, future shortcuts) asks for the open-media
// flow by dispatching
// `window.dispatchEvent(new CustomEvent("lumen:open-media"))`.
// The AppShell owns the single listener and mounts this dialog.
//
// W26 Figma grammar (no direct frame — CONTRACT §3; composed from the
// dialog family, Open network stream.svg / Keyboard Shortcuts.svg, the same
// grammar the player-dialogs lane w26.a18 extracted): the surface primitive
// owns the chrome (scrim + focus trap + r14 #191919 panel + hairline +
// overlay shadow + the modal-layer z-index — the host :has() scrim patches
// are retired, w26.a08); this sheet owns the inner anatomy:
//   * header — 50px #090909 strip, title 15px semibold #8F8F8F, ✕ close;
//   * body — the paste-a-path form (label + well input + Open), a
//     full-bleed hairline, then the pickable sources with FILE / STREAM
//     affordance glyphs (the honest subset of the file/folder/stream
//     trio — no folder source exists in any transport, so none is shown);
//   * footer — Cancel left, Open right (100×32 r6 button API).
//
// The dialog offers the DemoTransport clip catalog plus the transport's
// remembered openings ("Recent"), and opens the picked SourceSpec through
// the ClientProvider transport (`session.open` / `playlist.load` — the ONE
// funnel, client/open-path.ts), then routes to the player/playlist.
//
// Honesty rules that survived the redesign:
//   * single click OPENS (the selection state exists only so keyboard
//     users can focus an item and confirm with the footer Open button);
//   * every source location renders IN FULL (two-line rows, wrap-anywhere,
//     title attribute) instead of truncating behind ellipses;
//   * a blank path input is honestly invalid — no fabricated source.

import { useId, useRef, useState } from "react";
import type { FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { IconButton } from "../../components/button/IconButton";
import { Icon } from "../../components/icon/Icon";
import type { IconName } from "../../components/icon/icons";
import { TextField } from "../../components/form/TextField";
import { Dialog, Separator } from "../../components/surface/Surface";
import { useClient } from "../../client/ClientProvider";
import { isDesktopShell } from "../../client/app-transport";
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
 * The source-kind affordance glyph (pinned Icon.svg vocabulary): file
 * sources wear the open-file glyph, stream sources the network-stream
 * glyph. Folders have no source kind in any transport — none is invented.
 * Exported for the affordance contract test.
 */
export function sourceIcon(source: SourceSpec): IconName {
  return source.kind === "file" ? "open-file" : "network-stream";
}

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

  // Single click OPENS the clip. The selection state only exists so
  // keyboard users can focus an item and confirm with the Open button;
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

  // The paste-a-path affordance, made real: a path or URL opens through
  // the same ONE funnel as the list rows.
  const onPathSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const source = parseSourceInput(pathValue);
    if (source === null) {
      setPathInvalid(true);
      return;
    }
    openSource(source);
  };

  const clearAndClose = (): void => {
    setSelected(null);
    setPathValue("");
    setPathInvalid(false);
    onClose();
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
        {/* Header — the dialog strip: quiet title + the visible close
         * affordance (Escape and scrim-click also close — the Dialog owns
         * both). */}
        <header className="open-media__header">
          <h2 id={titleId} className="open-media__title">
            {m("open.title")}
          </h2>
          <IconButton
            className="open-media__close"
            label={m("open.close")}
            title={m("open.close")}
            size="sm"
            onClick={clearAndClose}
          >
            <Icon name="close" size={16} />
          </IconButton>
        </header>

        <div className="open-media__body">
          {/* The paste-a-path form — label + well input + Open (the
           * Onboarding.svg "input + Browse" row grammar, submit-flavored). */}
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
          <p className="open-media__hint">{m("open.subtitle")}</p>

          <Separator className="open-media__rule" />

          {/* Issue #3: the demo clips are simulation-only fixtures (fake
           * /media/demo paths) — they render in the browser dev shell,
           * where the DemoTransport answers them, and stay hidden in the
           * desktop shell, where the real engine would only answer typed
           * errors. */}
          {!isDesktopShell() && (
            <section className="open-media__section" aria-labelledby={clipsHeadingId}>
              <h3 id={clipsHeadingId} className="open-media__section-title">
                {m("open.demo-clips")}
              </h3>
              <ul className="open-media__list">
                {DEMO_CLIPS.map((clip) => (
                  <li key={clip.id}>
                    <SourceRow
                      title={m(clip.titleKey)}
                      source={clip.source}
                      selected={selected}
                      onSelect={setSelected}
                      onOpen={openSource}
                    />
                  </li>
                ))}
              </ul>
            </section>
          )}

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
                    <SourceRow
                      title={entry.title ?? sourceLocation(entry.source)}
                      source={entry.source}
                      selected={selected}
                      onSelect={setSelected}
                      onOpen={openSource}
                    />
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>

        {/* Footer — Open network stream.svg grammar: Cancel left, the
         * confirm action right (the frame's tinted 100×32 r6 treatment). */}
        <div className="open-media__actions">
          <Button onClick={clearAndClose}>{m("open.cancel")}</Button>
          <Button
            variant="tinted"
            disabled={selected === null}
            loading={opening}
            onClick={() => {
              if (selected !== null) openSource(selected);
            }}
          >
            {m("open.action")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

/** One pickable source row: kind glyph + two lines (title + FULL location). */
function SourceRow({
  title,
  source,
  selected,
  onSelect,
  onOpen,
}: {
  title: string;
  source: SourceSpec;
  selected: SourceSpec | null;
  onSelect: (source: SourceSpec) => void;
  onOpen: (source: SourceSpec) => void;
}) {
  const isSelected =
    selected !== null && sourceLocation(selected) === sourceLocation(source);
  return (
    <button
      type="button"
      className="open-media__item"
      aria-pressed={isSelected}
      onFocus={() => onSelect(source)}
      onClick={() => onOpen(source)}
    >
      <span className="open-media__item-icon" aria-hidden="true">
        <Icon name={sourceIcon(source)} size={16} />
      </span>
      <span className="open-media__item-text">
        <span className="open-media__item-title">{title}</span>
        <span className="open-media__item-source" title={sourceLocation(source)}>
          {sourceLocation(source)}
        </span>
      </span>
    </button>
  );
}
