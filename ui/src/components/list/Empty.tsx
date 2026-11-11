import type { HTMLAttributes, ReactNode } from "react";
import "./list.css";

/**
 * ListEmpty — the quiet empty-list state (W25 spec §2: "empty space is fine
 * — a quiet rail"): a secondary-tone title, an optional tertiary body line
 * and at most one action. No artwork, no illustration — the player canvas is
 * the app's one hero moment; lists stay calm.
 */
export interface ListEmptyProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  /** Quiet heading (secondary text). */
  title: ReactNode;
  /** Optional explanatory line (tertiary text). */
  body?: ReactNode;
  /** Optional single action (e.g. an "Open media" Button). */
  action?: ReactNode;
}

export function ListEmpty({ title, body, action, className = "", children, ...rest }: ListEmptyProps) {
  return (
    <div className={`lumen-empty ${className}`.trim()} {...rest}>
      <p className="lumen-empty__title">{title}</p>
      {body != null ? <p className="lumen-empty__body">{body}</p> : null}
      {action != null ? <div className="lumen-empty__action">{action}</div> : null}
      {children}
    </div>
  );
}
