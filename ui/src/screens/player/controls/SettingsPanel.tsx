// w26.i03 — the frame's settings-popover panels (Controls.svg variant sheets),
// replacing the w26.gfix-C MenuPopup mount the h04 rating graded "menus
// cosplaying as the frame's settings panels" (§2, 5.9 NO).
//
// Frame anatomy (Controls.svg, measured — every value cited in controls.css):
//   · panel 400 r10 #0E0E0E@95%, right-anchored, floating ABOVE the strip
//     (never over its own trigger — h04 defect #1);
//   · header 400×50 #090909 with the tab row: icon (1.25 stroke) + ≈13px
//     label, active = #8F8F8F label + 70×2 #D37918 underline on the header's
//     bottom edge, inactive #6B6B6B; 1px #222222 divider under the header;
//   · content inset 20 (360 wide): section headers (≈17px quiet), radio rows
//     (44px pitch, orange check column), label/value rows, toggles;
//   · footer = Button.png's quiet button (360×32 r4 #090909, centered
//     icon + "Show Advanced Settings") — B13's two rows map rest/hover.
//
// Semantics: role="dialog" (non-modal popover) + a real tablist (arrow keys)
// + radio rows/groups — the same engine-truth rows the menu carried, in the
// frame's panel grammar. Escape closes and restores trigger focus (e12 F8);
// Tab wraps inside the panel (the settings-window trap pattern, a11y.spec).
// w26.k07 (j07 P2-2): the rows ride a real radiogroup — roving tabindex +
// arrow-key selection, the RadioGroup primitive's pattern (doc 08 §4.5).
import { useEffect, useRef } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement, ReactNode } from "react";
import { Icon, Toggle } from "../../../components";
import type { IconName } from "../../../components";

/** One header tab (Controls.svg Frame 241: icon + label). */
export interface SettingsPanelTab {
  readonly id: string;
  /** Pre-resolved label (useMsg/lane seam at the call site). */
  readonly label: string;
  readonly icon: IconName;
}

/** Everything focusable inside the panel (the Tab-trap query). */
const FOCUSABLE_SELECTOR =
  'button:not([disabled]), [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

export interface SettingsPanelProps {
  /** Accessible name (the trigger's label). */
  readonly ariaLabel: string;
  /** Stable prefix for DOM ids (tab ↔ tabpanel wiring). */
  readonly idPrefix: string;
  readonly tabs: readonly SettingsPanelTab[];
  readonly activeTab: string;
  readonly onTabSelect: (id: string) => void;
  /** Escape / programmatic close — the host restores trigger focus. */
  readonly onRequestClose: () => void;
  readonly children: ReactNode;
  /** The quiet-button footer (Button.png) — omitted by the queue panel. */
  readonly footer?: ReactNode;
  readonly className?: string;
}

/**
 * The panel: dialog + tab header + scrollable body + footer. Focus moves to
 * the panel on open (Escape works immediately, focus never stays on the
 * trigger under the panel); Tab wraps within the panel.
 */
export function SettingsPanel({
  ariaLabel,
  idPrefix,
  tabs,
  activeTab,
  onTabSelect,
  onRequestClose,
  children,
  footer = null,
  className = "",
}: SettingsPanelProps): ReactElement {
  const panelRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef<Partial<Record<string, HTMLButtonElement | null>>>({});

  // Focus the panel on open (mount-time only — tab content is data-driven
  // and remounts per open).
  useEffect(() => {
    panelRef.current?.focus();
  }, []);

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onRequestClose();
      return;
    }
    if (event.key === "Tab") {
      // Focus trap (the settings-window pattern): Tab wraps to the panel's
      // own focusables — it never leaks to the bar under the panel.
      const focusables = panelRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
      if (focusables === undefined || focusables.length === 0) {
        event.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (first === undefined || last === undefined) {
        event.preventDefault();
        return;
      }
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  };

  // Arrow-key tab switching (WAI-ARIA tabs): focus follows selection.
  const onTablistKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    event.preventDefault();
    const index = tabs.findIndex((tab) => tab.id === activeTab);
    if (index < 0) return;
    const delta = event.key === "ArrowRight" ? 1 : -1;
    const next = tabs[(index + delta + tabs.length) % tabs.length];
    if (next === undefined) return;
    onTabSelect(next.id);
    tabRefs.current[next.id]?.focus();
  };

  return (
    <div
      ref={panelRef}
      role="dialog"
      aria-label={ariaLabel}
      tabIndex={-1}
      className={["lumen-controls__panel", className].filter(Boolean).join(" ")}
      onKeyDown={onKeyDown}
    >
      <div
        role="tablist"
        aria-label={ariaLabel}
        className="lumen-controls__panel-header"
        onKeyDown={onTablistKeyDown}
      >
        {tabs.map((tab) => {
          const active = tab.id === activeTab;
          return (
            <button
              key={tab.id}
              ref={(element) => {
                tabRefs.current[tab.id] = element;
              }}
              type="button"
              role="tab"
              id={`${idPrefix}-tab-${tab.id}`}
              aria-selected={active}
              aria-controls={`${idPrefix}-tabpanel`}
              tabIndex={active ? 0 : -1}
              className="lumen-controls__panel-tab"
              data-active={active || undefined}
              onClick={() => onTabSelect(tab.id)}
            >
              <Icon name={tab.icon} size={14} />
              <span className="lumen-controls__panel-tab-label">{tab.label}</span>
            </button>
          );
        })}
      </div>
      <div
        role="tabpanel"
        id={`${idPrefix}-tabpanel`}
        aria-labelledby={`${idPrefix}-tab-${activeTab}`}
        className="lumen-controls__panel-body"
      >
        {children}
      </div>
      {footer !== null ? <div className="lumen-controls__panel-footer">{footer}</div> : null}
    </div>
  );
}

/** Section header (Controls.svg "Embedded Tracks" / "Defaults" rows). The
 * optional trailing icon is the frame's right-aligned header affordance —
 * e.g. the "Output Device" rescan glyph (audio variant, x1870–1890): it
 * renders INERT (aria-hidden, no handler) because no engine seam exists
 * for it — the same honesty rule as the app menu's inert rows. */
export function PanelSection({
  title,
  trailingIcon,
  children,
}: {
  readonly title: string;
  readonly trailingIcon?: IconName;
  readonly children: ReactNode;
}): ReactElement {
  return (
    <section className="lumen-controls__panel-section">
      <div className="lumen-controls__panel-section-title">
        <span>{title}</span>
        {trailingIcon !== undefined ? (
          <Icon
            name={trailingIcon}
            size={16}
            className="lumen-controls__panel-section-icon"
            aria-hidden="true"
          />
        ) : null}
      </div>
      {children}
    </section>
  );
}

export interface PanelRadioRowProps {
  readonly label: string;
  readonly checked: boolean;
  /** Right-aligned quiet value (the queue rows' durations — Controls.svg
   * "23:06" at x1858–1895, right-aligned to the content edge). */
  readonly value?: string;
  readonly disabled?: boolean;
  readonly onSelect: () => void;
  /** w26.k07 (j07 P2-2): roving tabindex — owned by PanelRadioGroup
   * (the engine-reported selection is the group's Tab stop). */
  readonly tabIndex?: number;
  /** Ref collector for the group's arrow-key focus moves (PanelRadioGroup). */
  readonly focusRef?: (element: HTMLButtonElement | null) => void;
}

/** A radio row: orange check column + label (Controls.svg track rows). */
export function PanelRadioRow({
  label,
  checked,
  value,
  disabled = false,
  onSelect,
  tabIndex,
  focusRef,
}: PanelRadioRowProps): ReactElement {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      disabled={disabled}
      tabIndex={tabIndex}
      ref={focusRef}
      className="lumen-controls__panel-row"
      data-checked={checked || undefined}
      onClick={onSelect}
    >
      <span className="lumen-controls__panel-check" aria-hidden="true" />
      <span className="lumen-controls__panel-row-label">{label}</span>
      {value !== undefined ? (
        <span className="lumen-controls__panel-row-value">{value}</span>
      ) : null}
    </button>
  );
}

export interface PanelRadioOption {
  /** Stable option id (React key + focus-ref slot). */
  readonly key: string;
  readonly label: string;
  readonly checked: boolean;
  /** Right-aligned quiet value (the queue rows' durations). */
  readonly value?: string;
  readonly disabled?: boolean;
  readonly onSelect: () => void;
}

/**
 * The frame's radio rows as ONE ARIA radiogroup (w26.k07, j07 P2-2):
 * `role="radiogroup"` named by its section title, roving tabindex — the
 * engine-reported selection is the group's single Tab stop (first row while
 * nothing is checked) — and ArrowUp/Down/Left/Right that move focus and
 * selection together, the same pattern as the shared RadioGroup primitive
 * (doc 08 §4.5 focus rules). Checked stays engine-truth: an arrow select
 * dispatches onSelect and moves FOCUS, the check itself lands only when the
 * engine reports it (never optimistic).
 */
export function PanelRadioGroup({
  label,
  options,
}: {
  /** The group's accessible name — the section title it sits under. */
  readonly label: string;
  readonly options: readonly PanelRadioOption[];
}): ReactElement {
  const radios = useRef<Partial<Record<string, HTMLButtonElement | null>>>({});
  const checkedIndex = options.findIndex((option) => option.checked);
  const stopIndex =
    checkedIndex >= 0
      ? checkedIndex
      : Math.max(
          0,
          options.findIndex((option) => !option.disabled),
        );

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const forward = event.key === "ArrowDown" || event.key === "ArrowRight";
    const backward = event.key === "ArrowUp" || event.key === "ArrowLeft";
    if (!forward && !backward) return;
    event.preventDefault();
    const count = options.length;
    if (count === 0) return;
    const focusedIndex = options.findIndex(
      (option) => radios.current[option.key] === document.activeElement,
    );
    let index = focusedIndex >= 0 ? focusedIndex : stopIndex;
    // Wrap the full ring, skipping disabled rows (the RadioGroup pattern).
    for (let step = 0; step < count; step += 1) {
      index = (index + (forward ? 1 : -1) + count) % count;
      const option = options[index];
      if (option !== undefined && !option.disabled) {
        option.onSelect();
        radios.current[option.key]?.focus();
        return;
      }
    }
  };

  return (
    <div role="radiogroup" aria-label={label} onKeyDown={onKeyDown}>
      {options.map((option, index) => (
        <PanelRadioRow
          key={option.key}
          label={option.label}
          checked={option.checked}
          value={option.value}
          disabled={option.disabled}
          onSelect={option.onSelect}
          tabIndex={index === stopIndex ? 0 : -1}
          focusRef={(element) => {
            radios.current[option.key] = element;
          }}
        />
      ))}
    </div>
  );
}

/** Honest inert reason row (no engine seam / no media) — quiet, never a fake
 * control. The frame has no such face; doc 08 §4.4 honesty over theater. */
export function PanelReasonRow({ reason }: { readonly reason: string }): ReactElement {
  return <div className="lumen-controls__panel-reason">{reason}</div>;
}

/** Label left, value right (Controls.svg "Defaults": Audio Track → "1"). */
export function PanelValueRow({ label, value }: { label: string; value: string }): ReactElement {
  return (
    <div className="lumen-controls__panel-field">
      <span className="lumen-controls__panel-field-label">{label}</span>
      <span className="lumen-controls__panel-field-value">{value}</span>
    </div>
  );
}

/** Label left, Toggle right (Controls.svg "2 Pass" row — Toggle.png grammar). */
export function PanelToggleRow({
  label,
  checked,
  disabled = false,
  onChange,
}: {
  readonly label: string;
  readonly checked: boolean;
  readonly disabled?: boolean;
  readonly onChange: (checked: boolean) => void;
}): ReactElement {
  return (
    <div className="lumen-controls__panel-field">
      <span className="lumen-controls__panel-field-label">{label}</span>
      <Toggle checked={checked} disabled={disabled} onChange={onChange} aria-label={label} />
    </div>
  );
}

/**
 * Button.png's quiet button — the 3-tab track panels' footer (B13):
 * Default row 360×32 r4 #090909, icon 1.25 stroke #8F8F8F, label #6B6B6B,
 * content centered; Variant2's row (#191919 + #8F8F8F label) is the hover
 * face. The 4-tab settings variants carry the SAME footer content WITHOUT
 * the button chrome (audio-settings variant: bare ⓘ + label at y5982, no
 * 360×32 rect exists in that zone) — the `bare` prop renders that face.
 */
export function QuietButton({
  label,
  onClick,
  bare = false,
}: {
  readonly label: string;
  readonly onClick: () => void;
  readonly bare?: boolean;
}): ReactElement {
  return (
    <button
      type="button"
      className={bare ? "lumen-controls__quiet-label" : "lumen-controls__quiet-button"}
      onClick={onClick}
    >
      {/* The icon stroke stays on the icon token (#8F8F8F, Button.png row 1);
       * only the label carries the button's quiet ink. */}
      <Icon name="info" size={14} />
      <span>{label}</span>
    </button>
  );
}
