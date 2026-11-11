import { useEffect, useId, useRef } from "react";
import type { HTMLAttributes, ReactNode, RefObject } from "react";
import "./surface.css";

/** Where a Sheet anchors to the viewport edge (doc 08 §4: Sheet = edge-anchored dialog). */
export type SheetSide = "top" | "right" | "bottom" | "left";

/** Reasons a surface can request close (doc 08 §4.5: `Esc` closes overlays). */
export type SurfaceCloseReason = "escape" | "outside";

/**
 * Surface tone (W26 Figma grammar). `panel` is the default #191919 card
 * surface; `deep` is the recessed #0e0e0e well surface — the Settings and
 * Onboarding windows render on it (`Settings - global.svg`, `Onboarding.svg`
 * both measure rx14 #0E0E0E window fills).
 */
export type SurfaceTone = "panel" | "deep";

/** Hairline edges a {@link Panel} draws its 1px separator on. */
export type PanelEdges = "start" | "end" | "both" | "none";

/** Card treatments (W26 Figma grammar, see `surface.css`). */
export type CardVariant = "flat" | "raised" | "interactive" | "floating";

/**
 * Elements considered focusable for the trap. Disabled controls are excluded
 * at query time; hidden ones at cycle time (offsetParent check).
 */
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

export interface SurfaceOverlayProps {
  /** Whether the dialog/sheet is mounted and trapped. */
  open: boolean;
  /**
   * Called when the user requests dismissal: Escape always; outside click only
   * when `closeOnOutsideClick` (destructive forms opt out — doc 08 §4.5).
   */
  onClose: (reason: SurfaceCloseReason) => void;
  /** Element id labelling the surface (preferred accessible name). */
  labelledBy?: string;
  /** Accessible name when no title element id is supplied. */
  label?: string;
  /** Allow scrim clicks to close. @default false */
  closeOnOutsideClick?: boolean;
  children: ReactNode;
}

/**
 * Focus trap shared by Dialog and Sheet (doc 08 §4.5: `role="dialog"` + focus
 * trap; `Esc` closes). On open: the opener is remembered, focus moves to the
 * first focusable descendant (else the surface itself, which has tabIndex=-1).
 * While open: Tab/Shift+Tab cycle inside the surface. On close: focus is
 * restored to the opener.
 */
function useSurfaceFocusTrap(
  open: boolean,
  onClose: (reason: SurfaceCloseReason) => void,
  ref: RefObject<HTMLElement | null>,
) {
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) return;
    const node = ref.current;
    if (!node) return;

    // Remember the opener so focus can be restored when the surface closes.
    const opener = document.activeElement instanceof Element ? document.activeElement : null;
    // Move focus into the surface (first focusable, else the surface itself).
    const focusables = node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    (focusables[0] ?? node).focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onCloseRef.current("escape");
        return;
      }
      if (event.key !== "Tab") return;
      // The trap takes over Tab entirely (preventDefault + explicit move) so
      // cycling is deterministic in browsers *and* jsdom (which applies no
      // default Tab movement). Visibility check is structural because jsdom
      // has no layout (offsetParent is always null): skip `hidden` subtrees.
      event.preventDefault();
      const items = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.closest("[hidden]") === null,
      );
      if (items.length === 0) {
        node.focus(); // Surface itself keeps focus (tabIndex=-1).
        return;
      }
      const active = document.activeElement;
      const currentIndex = active instanceof HTMLElement ? items.indexOf(active) : -1;
      const nextIndex = event.shiftKey
        ? currentIndex <= 0
          ? items.length - 1
          : currentIndex - 1
        : currentIndex === -1 || currentIndex === items.length - 1
          ? 0
          : currentIndex + 1;
      items[nextIndex]!.focus();
    };

    node.addEventListener("keydown", onKeyDown);
    return () => {
      node.removeEventListener("keydown", onKeyDown);
      if (opener instanceof HTMLElement) opener.focus();
    };
  }, [open, ref]);
}

/** Shared scrim + dialog semantics markup for Dialog and Sheet. */
function useSurfaceA11y(labelledBy?: string, label?: string) {
  const generated = useId();
  // Prefer an explicit title element; else synthesize one from `label` so the
  // surface always exposes aria-labelledby (doc 08 §4.5).
  const id = labelledBy ?? generated;
  return { titleId: id, ariaLabel: labelledBy ? undefined : (label ?? "") };
}

/**
 * Dialog — modal surface (doc 08 §4.5: `role="dialog"` + focus trap; §1.4:
 * settings window rx14 `#0E0E0E` → surface-panel + radius-window; scrim at
 * --opacity-scrim). Escape closes; focus restores to the opener.
 *
 * W26 Figma grammar (`Open network stream.svg` / `Keyboard Shortcuts.svg`):
 * rx14–19 window fill `#191919` floating on a soft black shadow — the chrome
 * lives in `surface.css` (.lumen-dialog: 1px stroke-border + radius-window +
 * shadow-overlay); `tone="deep"` swaps the fill to the recessed well surface.
 */
export function Dialog({
  open,
  onClose,
  labelledBy,
  label,
  closeOnOutsideClick = false,
  tone = "panel",
  children,
  className = "",
}: SurfaceOverlayProps & { tone?: SurfaceTone; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useSurfaceFocusTrap(open, onClose, ref);
  const { titleId, ariaLabel } = useSurfaceA11y(labelledBy, label);

  if (!open) return null;
  return (
    <div className="lumen-scrim" onClick={closeOnOutsideClick ? () => onClose("outside") : undefined}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={ariaLabel || undefined}
        tabIndex={-1}
        data-tone={tone !== "panel" ? tone : undefined}
        className={`lumen-dialog ${className}`.trim()}
        onClick={(e) => e.stopPropagation()}
      >
        {labelledBy ? null : (
          <span id={titleId} className="lumen-surface__sr-title">
            {label ?? ""}
          </span>
        )}
        {children}
      </div>
    </div>
  );
}

/**
 * Sheet — Dialog variant anchored to a viewport edge (doc 08 §4). Identical
 * trap + semantics; `side` picks the edge; the slide-in transition honors
 * `prefers-reduced-motion` (doc 08 §4.5). Same W26 chrome as Dialog, with
 * the rounding kept to the inner corners only (edge-anchored panes are
 * square where they meet the viewport).
 */
export function Sheet({
  open,
  onClose,
  labelledBy,
  label,
  closeOnOutsideClick = false,
  side = "right",
  tone = "panel",
  children,
  className = "",
}: SurfaceOverlayProps & { side?: SheetSide; tone?: SurfaceTone; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useSurfaceFocusTrap(open, onClose, ref);
  const { titleId, ariaLabel } = useSurfaceA11y(labelledBy, label);

  if (!open) return null;
  return (
    <div className="lumen-scrim" onClick={closeOnOutsideClick ? () => onClose("outside") : undefined}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={ariaLabel || undefined}
        tabIndex={-1}
        data-tone={tone !== "panel" ? tone : undefined}
        className={`lumen-sheet lumen-sheet--${side} ${className}`.trim()}
        onClick={(e) => e.stopPropagation()}
      >
        {labelledBy ? null : (
          <span id={titleId} className="lumen-surface__sr-title">
            {label ?? ""}
          </span>
        )}
        {children}
      </div>
    </div>
  );
}

/**
 * Panel — square-edged, full-height app panel (W26 Figma grammar:
 * `Player.svg` title bar / icon rail / sidebar / control bar all render as
 * flat fills with rx 0 — rounding lives on CONTROLS and CARDS, never on
 * window-level panels). Surface fill defaults to the panel tone; `tone="deep"`
 * renders the recessed rail surface (Player sidebar measures `#0E0E0E`).
 * Hairline separators between panels are opt-in via `edges` (1px
 * var(--color-stroke-border), the workhorse border of the frame set).
 *
 * Emits `.lumen-pane` (not `.lumen-panel` — that class name is the theme
 * base sheet's r12–14 CARD grammar; see surface.css).
 */
export function Panel({
  as = "section",
  edges = "none",
  tone = "panel",
  className = "",
  ...rest
}: HTMLAttributes<HTMLElement> & {
  /** Semantic element to render. @default "section" */
  as?: "section" | "aside" | "nav" | "header" | "footer" | "div";
  /** Which inline edges carry the 1px hairline separator. @default "none" */
  edges?: PanelEdges;
  /** Surface tone. @default "panel" */
  tone?: SurfaceTone;
}) {
  const Tag = as;
  return (
    <Tag
      data-edges={edges !== "none" ? edges : undefined}
      data-tone={tone !== "panel" ? tone : undefined}
      className={`lumen-pane ${className}`.trim()}
      {...rest}
    />
  );
}

/**
 * Card — the r12–14 bordered card of the W26 Figma grammar (`playlist.svg`
 * r14 panel, `Miniplayer.svg` r14 cards, Settings r14 windows): panel fill +
 * 1px stroke-border, NO drop shadow (frames show none on resting cards).
 * Variants:
 * - `raised` — permanently elevated fill (surface-raised), for hover-elevated
 *   cards and active list surfaces.
 * - `interactive` — resting card that elevates to surface-raised on hover
 *   (playlist cards, `new_playlist.svg`).
 * - `floating` — the Miniplayer floating card: adds the one soft shadow the
 *   frames show on floating surfaces (token: --shadow-card).
 */
export function Card({
  as = "div",
  variant = "flat",
  className = "",
  ...rest
}: HTMLAttributes<HTMLElement> & {
  /** Semantic element to render. @default "div" */
  as?: "div" | "section" | "article" | "li";
  /** Card treatment. @default "flat" */
  variant?: CardVariant;
}) {
  const Tag = as;
  return (
    <Tag
      className={["lumen-card", variant !== "flat" && `lumen-card--${variant}`, className]
        .filter(Boolean)
        .join(" ")}
      {...rest}
    />
  );
}

/**
 * Well — recessed surface for tracks, inputs and inset fields (W26 Figma
 * grammar: seekbar track / input fills measure `#0e0e0e` =
 * --color-surface-deep; inset rounding). Purely a surface: sizing, borders
 * and inner layout belong to the host control.
 */
export function Well({ className = "", ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`lumen-well ${className}`.trim()} {...rest} />;
}

/**
 * Separator — 1px hairline divider (W26 Figma grammar: `#404040`
 * stroke-border is the workhorse separator of the frame set; menus, settings
 * rows and section rules all use the same 1px line). Decorative by default
 * (aria-hidden); semantic dividers stay host-owned `<hr>`s.
 */
export function Separator({
  orientation = "horizontal",
  className = "",
  ...rest
}: HTMLAttributes<HTMLDivElement> & {
  /** Line direction. @default "horizontal" */
  orientation?: "horizontal" | "vertical";
}) {
  return (
    <div
      aria-hidden="true"
      data-orientation={orientation}
      className={`lumen-separator ${className}`.trim()}
      {...rest}
    />
  );
}

/**
 * SectionHeader — the Settings-area section treatment (W26 Figma grammar:
 * 15px semibold sentence-case label, optionally closed by a thin 1px rule —
 * "Appearance" / "Startup" / "Diagnostics" headers across
 * `Settings - *.svg`). The heading level is host-chosen (`as`), so the
 * document outline stays correct wherever the header lands.
 */
export function SectionHeader({
  as = "h2",
  rule = false,
  trailing,
  className = "",
  children,
  ...rest
}: HTMLAttributes<HTMLHeadingElement> & {
  /** Heading element/level to render. @default "h2" */
  as?: "h2" | "h3" | "h4" | "h5" | "div";
  /** Thin rule under the header (1px var(--color-stroke-border)). */
  rule?: boolean;
  /** Optional content at the inline end (count, action link). */
  trailing?: ReactNode;
}) {
  const Tag = as;
  return (
    <div className={`lumen-section-header ${className}`.trim()} data-rule={rule || undefined}>
      <Tag className="lumen-section-header__title" {...rest}>
        {children}
      </Tag>
      {trailing === undefined ? null : (
        <div className="lumen-section-header__trailing">{trailing}</div>
      )}
    </div>
  );
}

/**
 * HeroPill — the r30 hero surface (W26 radii table: "30 hero pills"). In the
 * Onboarding frames the 60×60 rx30 shape is the hero disc / selected-accent
 * ring (`Onboarding-3..10.svg`: 4px ring at 30% accent ink around the swatch).
 * Surface only — pill BUTTONS belong to the button lane.
 */
export function HeroPill({
  as = "div",
  ring = false,
  className = "",
  ...rest
}: HTMLAttributes<HTMLDivElement> & {
  /** Semantic element to render. @default "div" */
  as?: "div" | "span";
  /** Accent selection ring (4px var(--stroke-ring) at 30% accent ink). */
  ring?: boolean;
}) {
  const Tag = as;
  return (
    <Tag
      className={["lumen-hero-pill", ring && "lumen-hero-pill--ring", className]
        .filter(Boolean)
        .join(" ")}
      {...rest}
    />
  );
}
