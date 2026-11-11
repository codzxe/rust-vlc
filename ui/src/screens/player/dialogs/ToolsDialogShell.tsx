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
//     chevron) above the settings card (r8, #1B1B1B-band fill) when
//     expanded;
//   * card rows — label left, control right (value chips r4 dark #090909,
//     tabular).
//
// The shell stays GENERIC: it owns chrome + the card skeleton and exposes
// slots (body → cards/rows, footer → dialog actions); the tool value of a
// row, its slider and its presets stay the mounting lane's to compose.
//
// COPY NOTE: local module copy (the established dialogs pattern —
// BOOKMARKS_COPY / GOTO_COPY); central keys are pre-registered for the
// Lead in ui/src/i18n/keys/w26.a18.ts.
import { useId, useState } from "react";
import type { ReactNode } from "react";
import { IconButton } from "../../../components/button/IconButton";
import { Dialog } from "../../../components/surface/Surface";
import type { SurfaceCloseReason } from "../../../components/surface/Surface";
import { ChevronGlyph, CloseGlyph } from "./dialogIcons";
import "./dialogs.css";

/** The shell's English copy (see COPY NOTE — central i18n is follow-up). */
export const TOOLS_DIALOG_COPY = {
  /** Collapsed card toggle label (frame: "Expand Settings"). */
  expand: "Expand Settings",
  /** Expanded card toggle label (frame: "Collapse Settings"). */
  collapse: "Collapse Settings",
  /** Accessible name of the header close ✕. */
  close: "Close",
} as const;

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
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {title}
          </h2>
          <IconButton
            className="player-dialog__close"
            label={TOOLS_DIALOG_COPY.close}
            title={TOOLS_DIALOG_COPY.close}
            size="sm"
            onClick={() => onClose("escape")}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>
        <div className="player-dialog__body">
          {subtitle ? <p className="player-dialog__subtitle">{subtitle}</p> : null}
          <div className="tools-dialog__cards">{children}</div>
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
          {open ? TOOLS_DIALOG_COPY.collapse : TOOLS_DIALOG_COPY.expand}
        </span>
        <ChevronGlyph size={16} className="tools-card__chevron" />
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
