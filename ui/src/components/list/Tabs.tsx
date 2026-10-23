import { createContext, useContext, useId, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import "./list.css";

export type TabsOrientation = "horizontal" | "vertical";

interface TabsContextValue {
  value: string;
  setValue: (value: string) => void;
  orientation: TabsOrientation;
  baseId: string;
}

const TabsContext = createContext<TabsContextValue | null>(null);

/**
 * Tabs — doc 08 §4.5 anatomy: `role=tablist`/`tab`/`tabpanel`, roving
 * tabindex (only the selected tab is tabbable), arrow-key navigation per
 * orientation, `aria-selected` + `aria-controls`/`aria-labelledby` wiring.
 * Selection follows focus (WAI-ARIA Tabs Pattern); Home/End jump to the ends.
 */
export function Tabs({
  value: controlledValue,
  defaultValue,
  orientation = "horizontal",
  children,
  className = "",
}: {
  value?: string;
  defaultValue?: string;
  orientation?: TabsOrientation;
  children: ReactNode;
  className?: string;
}) {
  const [internalValue, setInternalValue] = useState(defaultValue ?? "");
  const baseId = useId();
  const ctx: TabsContextValue = {
    value: controlledValue ?? internalValue,
    setValue: setInternalValue,
    orientation,
    baseId,
  };
  return (
    <TabsContext.Provider value={ctx}>
      <div className={`lumen-tabs ${className}`.trim()}>{children}</div>
    </TabsContext.Provider>
  );
}

export function TabsList({ children, className = "" }: { children: ReactNode; className?: string }) {
  const ctx = useTabsContext("TabsList");

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // Roving focus over the DOM-order tab list (doc 08 §4.5: DOM order =
    // visual order). Wrap at the ends per the ARIA Tabs Pattern.
    const tabs = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]:not([disabled])'));
    if (tabs.length === 0) return;
    const values = tabs.map((tab) => tab.dataset.value ?? "");
    const currentIndex = Math.max(0, values.indexOf(ctx.value));
    const forwardKey = ctx.orientation === "horizontal" ? "ArrowRight" : "ArrowDown";
    const backwardKey = ctx.orientation === "horizontal" ? "ArrowLeft" : "ArrowUp";
    let nextIndex: number | null = null;
    if (event.key === forwardKey) nextIndex = (currentIndex + 1) % values.length;
    else if (event.key === backwardKey) nextIndex = (currentIndex - 1 + values.length) % values.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = values.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    const nextValue = values[nextIndex]!;
    ctx.setValue(nextValue);
    tabs[nextIndex]?.focus();
  };

  return (
    <div role="tablist" aria-orientation={ctx.orientation} className={`lumen-tabs__list ${className}`.trim()} onKeyDown={onKeyDown}>
      {children}
    </div>
  );
}

export function Tab({ value, disabled = false, children, className = "" }: { value: string; disabled?: boolean; children: ReactNode; className?: string }) {
  const ctx = useTabsContext("Tab");
  const selected = ctx.value === value;
  return (
    <button
      type="button"
      role="tab"
      data-value={value}
      id={`${ctx.baseId}-tab-${value}`}
      aria-selected={selected}
      aria-controls={`${ctx.baseId}-panel-${value}`}
      tabIndex={selected ? 0 : -1}
      disabled={disabled}
      className={`lumen-tabs__tab${selected ? " lumen-tabs__tab--selected" : ""} ${className}`.trim()}
      onClick={() => ctx.setValue(value)}
    >
      {children}
    </button>
  );
}

export function TabsPanel({ value, children, className = "" }: { value: string; children: ReactNode; className?: string }) {
  const ctx = useTabsContext("TabsPanel");
  const selected = ctx.value === value;
  return (
    <div
      role="tabpanel"
      id={`${ctx.baseId}-panel-${value}`}
      aria-labelledby={`${ctx.baseId}-tab-${value}`}
      hidden={!selected}
      tabIndex={0}
      className={`lumen-tabs__panel ${className}`.trim()}
    >
      {children}
    </div>
  );
}

function useTabsContext(caller: string): TabsContextValue {
  const ctx = useContext(TabsContext);
  if (!ctx) throw new Error(`${caller} must be used inside <Tabs>`);
  return ctx;
}
