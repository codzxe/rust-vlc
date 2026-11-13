// ToolsDialogShell (w26.a18) — the generic dialog shell the wave-c audio /
// video tool lanes (EQ, compressor, spatializer, widener, visualizer,
// colors, crop, essential, HDR) mount their tool cards into.
//
// Shared anatomy, measured across the tool frames (design/figma/
// "Audio - Equalizer.svg", "EQ.svg", "Audio - Compressor Settings.svg",
// "Video - Colors.svg", "Video - Crop.svg", "HDR Options.svg", …):
//   * dialog chrome — the same player-dialog grammar as Go-to-Time and
//     Bookmarks (scrim, rx14 panel + hairline border, 50px #090909 title
//     strip with title + close ✕, Escape / scrim-click close via the
//     shared Dialog);
//   * ONE collapsible card per tool: a full-width toggle bar (rx6, 48px,
//     #090909 surface fill, "Expand Settings"/"Collapse Settings" +
//     chevron) above the settings card (r8, raised fill) when expanded;
//   * card rows — label left, control right (value chips r4 dark #090909,
//     tabular).
//
// FOCUS ORDER (w26.i09, the h19 P4 dialog-semantics pattern): the body
// renders BEFORE the header in the DOM and the header paints first via
// `order: -1` (dialogs.css) — so the surface trap's first-focusable
// policy lands on the first BODY control (the tab pair / the form
// input), never on the ✕, and Enter on a fresh dialog acts on the
// content instead of dismissing the window. The ✕ stays keyboard
// reachable at the END of the tab cycle; the accessible name still comes
// from aria-labelledby, which is position-independent.
//
// The shell stays GENERIC: it owns chrome + the card skeleton and exposes
// slots (body → cards/rows, footer → dialog actions); the tool value of a
// row, its slider and its presets stay the mounting lane's to compose.
//
// COPY (w26.k09, j09 P2): the shell's three strings render through
// useMsg directly — the player.tools.* keys are already registered central
// (messages.ts, recorded in ui/src/i18n/keys/w26.a18.ts), so pseudo-locale
// covers the shell today. The derived TOOLS_DIALOG_COPY constant lives in
// useDialogsMsg.ts (frozen index.ts surface, test reference).
import { useId, useState } from "react";
import type { ReactNode } from "react";
import { IconButton } from "../../../components/button/IconButton";
import { Dialog } from "../../../components/surface/Surface";
import type { SurfaceCloseReason } from "../../../components/surface/Surface";
import { useMsg } from "../../../i18n/messages";
import { ChevronGlyph, CloseGlyph } from "./dialogIcons";
import "./dialogs.css";

export interface ToolsDialogShellProps {
  /** Whether the dialog is mounted and trapped. */
  open: boolean;
  /** Dismissal callback (Escape always; scrim click too — both Figma grammar). */
  onClose: (reason: SurfaceCloseReason) => void;
  /** Header title (15px semibold, title bar strip). */
  title: string;
  /** Optional quiet subtitle under the title. */
  subtitle?: string;
  /**
   * Body slot — tool cards mount here. Compose with `ToolsCard` /
   * `ToolsRow` / `ToolsValueChip` (this module) or freeform nodes.
   */
  children: ReactNode;
  /**
   * Footer slot — the Figma dialog footer grammar (quiet action left,
   * primary action right). Rendered only when provided.
   */
  footer?: ReactNode;
  /** Extra classes for the inner dialog root (width variants etc.). */
  className?: string;
}

/**
 * The generic tools dialog shell. Mount per-open like the other player
 * dialogs: `{open ? <ToolsDialogShell open onClose={…} …/> : null}`.
 */
export function ToolsDialogShell({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  className = "",
}: ToolsDialogShellProps) {
  const m = useMsg();
  const titleId = useId();
  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy={titleId}
      className="player-dialog__surface"
      closeOnOutsideClick
    >
      <div className={["player-dialog", "player-dialog--tools", className]
        .filter(Boolean)
        .join(" ")}>
        {/* Body first in the DOM (focus order — see module header); the
         * header paints above it via `order: -1`. */}
        <div className="player-dialog__body">
          {subtitle ? <p className="player-dialog__subtitle">{subtitle}</p> : null}
          <div className="tools-dialog__cards">{children}</div>
        </div>
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {title}
          </h2>
          <IconButton
            className="player-dialog__close"
            label={m("player.tools.close")}
            title={m("player.tools.close")}
            size="sm"
            onClick={() => onClose("escape")}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>
        {footer ? (
          /* The Figma footer grammar — quiet action left, tinted confirm
           * right (compose with the Button primitive's variants). */
          <div className="player-dialog__footer">{footer}</div>
        ) : null}
      </div>
    </Dialog>
  );
}

export interface ToolsCardProps {
  /**
   * Initial expanded state (uncontrolled). Frames ship tools COLLAPSED —
   * `defaultOpen` exists for lanes that want their card open on mount.
   * @default false
   */
  defaultOpen?: boolean;
  /** Row content — compose `ToolsRow` (label left / control right). */
  children: ReactNode;
  /** Stable id for the toggle (aria wiring); generated when omitted. */
  id?: string;
  /** Extra classes on the card root. */
  className?: string;
}

/**
 * ToolsCard — the tool frames' shared collapsible card skeleton: the
 * full-width toggle bar (r6 quiet fill, "Expand/Collapse Settings" +
 * chevron, chevron rotates when expanded) and, when expanded, the settings
 * card (r8 raised fill) hosting the rows. The expanded body stays in the
 * DOM (`hidden`) so `aria-controls` always resolves and the Dialog's focus
 * trap skips collapsed cards coherently.
 */
export function ToolsCard({
  defaultOpen = false,
  children,
  id: idProp,
  className = "",
}: ToolsCardProps) {
  const m = useMsg();
  const [open, setOpen] = useState(defaultOpen);
  const generatedId = useId();
  const id = idProp ?? generatedId;
  const bodyId = `${id}-body`;
  return (
    <section
      className={["tools-card", open ? "tools-card--open" : "", className]
        .filter(Boolean)
        .join(" ")}
    >
      <button
        type="button"
        id={id}
        className="tools-card__toggle"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((prev) => !prev);
        }}
      >
        <span className="tools-card__toggle-label">
          {open ? m("player.tools.collapse-settings") : m("player.tools.expand-settings")}
        </span>
        {/* 24px glyph box — frame Vector 27 measures ≈11.5×6.5 paint-
         * inclusive at sw1.5, which a 16px box undersizes ~30% (h14 P3;
         * the visualizer lane ships the same 24 box three cards away). */}
        <ChevronGlyph size={24} className="tools-card__chevron" />
      </button>
      <div
        id={bodyId}
        className="tools-card__body"
        role="group"
        aria-labelledby={id}
        hidden={!open}
      >
        {children}
      </div>
    </section>
  );
}

export interface ToolsRowProps {
  /** Row label (left). */
  label: ReactNode;
  /** Optional helper line under the label (frame: field helper copy). */
  hint?: ReactNode;
  /** The control slot (right) — chip, toggle, dropdown, … */
  children: ReactNode;
  /** Associates the label with a control id when the control isn't nested. */
  htmlFor?: string;
  /** Extra classes on the row. */
  className?: string;
}

/**
 * ToolsRow — the card's label-left / control-right row (the grammar every
 * tool frame shares: Preamp, Saturation, Threshold, … all sit the same way).
 */
export function ToolsRow({
  label,
  hint,
  children,
  htmlFor,
  className = "",
}: ToolsRowProps) {
  return (
    <div className={["tools-row", className].filter(Boolean).join(" ")}>
      <div className="tools-row__label">
        <label className="tools-row__label-text" htmlFor={htmlFor}>
          {label}
        </label>
        {hint ? <span className="tools-row__hint">{hint}</span> : null}
      </div>
      <div className="tools-row__control">{children}</div>
    </div>
  );
}

export interface ToolsValueChipProps {
  /** The row's current value (tabular render, full value on hover). */
  children: ReactNode;
  /** Full/precise value when the chip clips (title tooltip). */
  title?: string;
  /** Extra classes on the chip. */
  className?: string;
}

/**
 * ToolsValueChip — the r6 dark value chip at the right end of a row
 * (frame: EQ/Compressor rows' `138×34 rx5 #090909` read-outs, mono ink).
 */
export function ToolsValueChip({ children, title, className = "" }: ToolsValueChipProps) {
  return (
    <span
      className={["tools-row__value", className].filter(Boolean).join(" ")}
      title={title}
    >
      {children}
    </span>
  );
}
