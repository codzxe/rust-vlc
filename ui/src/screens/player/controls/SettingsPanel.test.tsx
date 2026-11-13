// w26.k07 — SettingsPanel direct keyboard tests (j07 P2-3): the panel's 45
// lines of keyboard interaction (focus-on-open, the Tab focus trap, the
// arrow-key tablist) previously had ZERO direct tests — the only keyboard
// coverage of a panel was the ControlBar suite's Escape-restore pin. These
// mount the panel bare (no providers needed — it is presentational) and
// drive the real key paths.
import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { SettingsPanel } from "./SettingsPanel";
import type { SettingsPanelTab } from "./SettingsPanel";

const TABS: readonly SettingsPanelTab[] = [
  { id: "queue", label: "Queue", icon: "queue" },
  { id: "bookmarks", label: "Bookmarks", icon: "bookmark" },
  { id: "chapters", label: "Chapters", icon: "chapters" },
];

/** The host's real wiring: controlled activeTab (the PanelPopover mount). */
function Harness(): ReactElement {
  const [activeTab, setActiveTab] = useState("queue");
  return (
    <SettingsPanel
      ariaLabel="Queue"
      idPrefix="test-panel"
      tabs={TABS}
      activeTab={activeTab}
      onTabSelect={setActiveTab}
      onRequestClose={() => {}}
      footer={<button type="button">Show Advanced Settings</button>}
    >
      <button type="button">Row one</button>
      <button type="button">Row two</button>
    </SettingsPanel>
  );
}

describe("SettingsPanel keyboard interaction (w26.k07, j07 P2-3)", () => {
  it("focus lands on the panel itself on open (Escape works immediately)", () => {
    render(<Harness />);
    const panel = screen.getByRole("dialog", { name: "Queue" });
    expect(panel).toHaveFocus();
    // Focus never stays on the trigger under the panel — the panel is a
    // focusable container (tabIndex -1) that takes focus at mount.
    expect(panel).toHaveAttribute("tabindex", "-1");
  });

  it("Tab wraps inside the panel — never leaks past its own focusables", () => {
    render(<Harness />);
    const panel = screen.getByRole("dialog", { name: "Queue" });
    const firstTab = screen.getByRole("tab", { name: "Queue" });
    const footer = screen.getByRole("button", { name: "Show Advanced Settings" });

    // Forward wrap: Tab on the LAST focusable (the footer button, after the
    // tab row + body rows) returns to the FIRST (the active tab).
    footer.focus();
    expect(document.activeElement).toBe(footer);
    fireEvent.keyDown(panel, { key: "Tab" });
    expect(document.activeElement).toBe(firstTab);

    // Backward wrap: Shift+Tab on the FIRST focusable lands on the LAST.
    fireEvent.keyDown(panel, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(footer);
  });

  it("ArrowRight/ArrowLeft switch tabs with focus following selection (ARIA tabs)", () => {
    render(<Harness />);
    const queue = screen.getByRole("tab", { name: "Queue" });
    queue.focus();

    // Right: Queue → Bookmarks (selected AND focused).
    fireEvent.keyDown(queue, { key: "ArrowRight" });
    const bookmarks = screen.getByRole("tab", { name: "Bookmarks" });
    expect(bookmarks).toHaveAttribute("aria-selected", "true");
    expect(bookmarks).toHaveFocus();
    // The tabpanel is labelled by the newly active tab (id wiring follows).
    expect(screen.getByRole("tabpanel")).toHaveAttribute(
      "aria-labelledby",
      "test-panel-tab-bookmarks",
    );

    // Left: Bookmarks → Queue (selected AND focused).
    fireEvent.keyDown(bookmarks, { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { name: "Queue" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "Queue" })).toHaveFocus();

    // Both directions wrap the ring: Queue (index 0) → Chapters (index 2)
    // backward, then Chapters (index 2) → Queue (index 0) forward. Focus
    // follows selection, so the selected tab is the key anchor.
    fireEvent.keyDown(screen.getByRole("tab", { name: "Queue" }), { key: "ArrowLeft" });
    const chapters = screen.getByRole("tab", { name: "Chapters" });
    expect(chapters).toHaveAttribute("aria-selected", "true");
    expect(chapters).toHaveFocus();
    fireEvent.keyDown(chapters, { key: "ArrowRight" });
    expect(queue).toHaveAttribute("aria-selected", "true");
    expect(queue).toHaveFocus();
  });

  it("roving tabindex: only the active tab is a Tab stop", () => {
    render(<Harness />);
    expect(screen.getByRole("tab", { name: "Queue" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("tab", { name: "Bookmarks" })).toHaveAttribute("tabindex", "-1");
    expect(screen.getByRole("tab", { name: "Chapters" })).toHaveAttribute("tabindex", "-1");
  });
});
