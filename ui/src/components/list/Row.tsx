import type { HTMLAttributes, ReactNode } from "react";
import type { RowDensity } from "./density";
import "./list.css";

export interface ListRowProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  /** Row density. @default "comfortable" */
  density?: RowDensity;
  /** Leading icon / thumbnail slot (Icon glyphs tint via currentColor). */
  icon?: ReactNode;
  /** Primary label. */
  title: ReactNode;
  /** Secondary line under the title (item counts, source location). */
  meta?: ReactNode;
  /** Right-aligned duration — rendered with tabular numerals (W25 §1). */
  duration?: ReactNode;
  /** Trailing slot after the duration (shortcuts, chevrons, badges). */
  trailing?: ReactNode;
  /** Hover-revealed trailing actions — arbitrary markup replacing the kebab. */
  actions?: ReactNode;
  /**
   * The `•••` overflow affordance (playlist.svg): a kebab button revealed on
   * row hover/focus-within. `label` is the accessible name (i18n: useMsg).
   */
  kebab?: { label: string; onClick: () => void };
  /** Selected state: --color-surface-selected fill + accent icon. */
  selected?: boolean;
}

/**
 * ListRow — the Figma list-row grammar (W25 spec §2) as one primitive:
 * optional drag handle · icon · title/meta · right-aligned tabular duration ·
 * trailing · hover-revealed actions. States: hover fill + bright text,
 * selected surface + orange icon, focus-visible accent ring. Roles and aria
 * wiring belong to the consumer (role="option" / "menuitem" / … via rest
 * props) so the same row serves lists, menus and settings sidebars.
 */
export function ListRow({
  density = "comfortable",
  icon,
  title,
  meta,
  duration,
  trailing,
  actions,
  kebab,
  selected = false,
  className = "",
  children,
  ...rest
}: ListRowProps) {
  const showHandle = Boolean(rest.draggable);
  const overflow =
    actions ??
    (kebab ? (
      <button
        type="button"
        className="lumen-lrow__kebab"
        aria-label={kebab.label}
        onClick={(event) => {
          event.stopPropagation(); // the row's own onClick must not fire
          kebab.onClick();
        }}
      >
        <span className="lumen-lrow__kebab-dots" aria-hidden="true" />
      </button>
    ) : null);
  return (
    <div
      className={`lumen-lrow lumen-lrow--${density}${selected ? " lumen-lrow--selected" : ""} ${className}`.trim()}
      data-density={density}
      data-selected={selected || undefined}
      {...rest}
    >
      {showHandle ? <span className="lumen-lrow__handle" aria-hidden="true" /> : null}
      {icon != null ? <span className="lumen-lrow__icon">{icon}</span> : null}
      <span className="lumen-lrow__main">
        <span className="lumen-lrow__title">{title}</span>
        {meta != null ? <span className="lumen-lrow__meta">{meta}</span> : null}
      </span>
      {duration != null ? <span className="lumen-lrow__duration">{duration}</span> : null}
      {trailing != null ? <span className="lumen-lrow__trailing">{trailing}</span> : null}
      {overflow ? <span className="lumen-lrow__actions">{overflow}</span> : null}
      {children}
    </div>
  );
}
