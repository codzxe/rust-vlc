// Open media flow (app-shell workstream, 2026-11-08; W26 redesign w26.b13;
// fixer pass w26.k16).
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
//   * body — the paste-a-path form (label + well input), a full-bleed
//     hairline, then the pickable sources with FILE / STREAM affordance
//     glyphs (the honest subset of the file/folder/stream trio — no folder
//     source exists in any transport, so none is shown);
//   * footer — Cancel left, Open right (100×32 r6 button API; the family's
//     confirm-in-footer grammar — w26.k16/j16 P3-17: the footer Open submits
//     the pasted path, so it is ALWAYS a live control).
//
// The dialog offers the DemoTransport clip catalog plus the transport's
// remembered openings ("Recent"), and opens the picked SourceSpec through
// the ClientProvider transport (`session.open` / `playlist.load` — the ONE
// funnel, client/open-path.ts), then routes to the player/playlist.
//
// Honesty rules that survived the redesign:
//   * single click OPENS a row (v2 §4.1), and Enter/Space on a focused row
//     opens it too (native button semantics) — rows are plain activation
//     buttons, no focus-as-selection conflation (w26.k16/j16 P3-17);
//   * every source location renders IN FULL (two-line rows, wrap-anywhere,
//     title attribute) instead of truncating behind ellipses;
//   * a blank path input is honestly invalid — no fabricated source.

import { useEffect, useId, useRef, useState } from "react";
import type { FormEvent, ReactElement } from "react";
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
  const [opening, setOpening] = useState(false);
  const [pathValue, setPathValue] = useState("");
  const [pathInvalid, setPathInvalid] = useState(false);
  const nextRequestId = useRef(0);
  const pathFieldRef = useRef<HTMLInputElement | null>(null);
  const titleId = useId();
  const pathFormId = useId();
  const clipsHeadingId = useId();
  const recentHeadingId = useId();
  const pathFieldId = useId();

  // w26.i17 family fix (j16 P2-3): the shared trap's first-focusable policy
  // lands initial focus on the ✕ glyph (first in DOM order), so at rest
  // Enter DISMISSED the dialog instead of submitting and :focus-visible
  // could ring the close glyph — the same defect the netstream
  // (NetworkStreamScreen.tsx) and shortcuts (ShortcutsScreen.tsx) dialogs
  // already fix with a first-field focus effect; this is the missing third
  // copy. Runs after the child Dialog's trap effect, so the field wins.
  // Deps [open]: the AppShell keeps this dialog mounted and toggles `open`,
  // so focus must move on every OPENING, not just mount.
  useEffect(() => {
    if (open) pathFieldRef.current?.focus();
  }, [open]);

  // The "Recent" section exists only when the transport remembers openings
  // (the demo transport does; real hosts will surface their own history).
  const recent = isDemoTransport(client) ? client.recentOpenings : [];
  // The demo clips are simulation-only fixtures (fake /media/demo paths):
  // they render only where the DEMO transport actually answers them — the
  // browser dev shell (never the desktop shell) AND only for the demo
  // transport itself (w26.k16/j16 P3-10: the old shell-only gate would have
  // shown fake clips to a browser host wired to a real engine transport).
  const showClips = !isDesktopShell() && isDemoTransport(client);

  // Single click OPENS the clip (v2 §4.1); Enter/Space on a focused row
  // opens it too (native button semantics).
  //
  // Issue #3 (4.H1 handoff b/d): every open goes through the ONE funnel
  // (client/open-path.ts) — media files `session.open`, playlist files
  // `playlist.load` (parse is engine-side); routing follows the kind.
  //
  // w26.k16 (j16 P3-9): one open in flight — a stray double-click or a
  // second submit must not double-send, and a superseded request's late
  // resolution must not clobber the newer one's close/route.
  const openSource = (source: SourceSpec): void => {
    if (opening) return;
    setOpening(true);
    nextRequestId.current += 1;
    const requestId = nextRequestId.current;
    void client
      .send(openPathRequest(source, requestId))
      .catch(() => {
        // Transport unreachable — the player screens surface their own
        // honest typed errors; the dialog still closes and routes.
      })
      .finally(() => {
        // A newer activation superseded this one — it owns the close/route.
        if (requestId !== nextRequestId.current) return;
        setOpening(false);
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
          {/* Body — the paste-a-path form (label + well input; Enter submits,
           and the footer Open is this form's submit button — the dialog
           family's confirm-in-footer grammar, w26.k16/j16 P3-17). */}
          <form id={pathFormId} className="open-media__path" onSubmit={onPathSubmit}>
            <TextField
              ref={pathFieldRef}
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
          </form>
          <p className="open-media__hint">{m("open.subtitle")}</p>

          <Separator className="open-media__rule" />

          {/* Issue #3 + w26.k16 (j16 P3-10): the demo clips are
           * simulation-only fixtures (fake /media/demo paths) — they render
           * only for the DEMO transport in the browser dev shell, where
           * DemoTransport answers them; hidden in the desktop shell AND for
           * any browser host wired to a real engine transport. */}
          {showClips && (
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
                      onOpen={openSource}
                    />
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>

        {/* Footer — the dialog family grammar (Open network stream.svg):
           * Cancel left; the confirm right. w26.k16 (j16 P3-17): the footer
           * Open is the paste-a-path form's submit button (type=submit +
           * form association), so it is ALWAYS live — a typed path opens
           * through the same funnel, a blank one surfaces the field's honest
           * invalid copy. (The old disabled-until-row-selected wiring could
           * never be enabled: rows open on click and the dialog closes.) */}
        <div className="open-media__actions">
          <Button onClick={clearAndClose}>{m("open.cancel")}</Button>
          <Button type="submit" form={pathFormId} variant="tinted" loading={opening}>
            {m("open.action")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

/** One pickable source row: kind glyph + two lines (title + FULL
 * location). A plain activation button (w26.k16/j16 P3-17): click or
 * Enter/Space opens — no aria-pressed, no focus-as-selection. */
function SourceRow({
  title,
  source,
  onOpen,
}: {
  title: string;
  source: SourceSpec;
  onOpen: (source: SourceSpec) => void;
}): ReactElement {
  return (
    <button
      type="button"
      className="open-media__item"
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
