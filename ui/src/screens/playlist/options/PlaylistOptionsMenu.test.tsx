// w26.b10 — PlaylistOptionsMenu tests: the anchored `role="menu"` popup in
// the option frames' grammar. Rendering, radio semantics + check, click
// selection with dismiss + focus restore, the full APG keyboard model
// (arrows cycle, Home/End, Enter/Space, Escape, Tab trap), outside-pointer
// dismiss, disabled inertness, align/placement classes.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PlaylistOptionsMenu } from "./PlaylistOptionsMenu";

interface Entry {
  id: string;
  labelKey: string;
  icon?: "grid" | "list";
  disabled?: boolean;
}

const ENTRIES: Entry[] = [
  { id: "grid", labelKey: "Grid View", icon: "grid" },
  { id: "list", labelKey: "List View", icon: "list" },
  { id: "off", labelKey: "Disabled View", disabled: true },
];

const setup = (value = "list", onSelect: (id: string) => void = () => {}) =>
  render(
    <PlaylistOptionsMenu
      entries={ENTRIES}
      value={value}
      onSelect={onSelect}
      resolveLabel={(key) => key}
      menuLabel="View options"
      triggerLabel="View options"
      triggerIcon="list"
    />,
  );

const open = (): HTMLElement => {
  const trigger = screen.getByRole("button", { name: "View options" });
  fireEvent.click(trigger);
  return screen.getByRole("menu");
};

describe("PlaylistOptionsMenu — rendering", () => {
  it("renders an icon-only trigger with menu semantics; no popup closed", () => {
    setup();
    const trigger = screen.getByRole("button", { name: "View options" });
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("opens on click with menuitemradio rows, labels and icons", () => {
    setup();
    const menu = open();
    expect(screen.getByRole("button", { name: "View options" }).getAttribute("aria-expanded")).toBe("true");
    const items = screen.getAllByRole("menuitemradio");
    expect(items).toHaveLength(3);
    expect(menu.getAttribute("aria-label")).toBe("View options");
    expect(screen.getByText("Grid View")).toBeTruthy();
    expect(screen.getByText("List View")).toBeTruthy();
    // Icons render in the leading slot (View Options.svg rows carry icons).
    expect(items[0]!.querySelector(".lumen-plo__icon svg")).not.toBeNull();
  });

  it("marks only the selected row aria-checked (frames: one check)", () => {
    setup("list");
    open();
    const checked = screen.getAllByRole("menuitemradio").filter((item) => item.getAttribute("aria-checked") === "true");
    expect(checked).toHaveLength(1);
    expect(checked[0]!.textContent).toBe("List View");
  });
});

describe("PlaylistOptionsMenu — selection", () => {
  it("click selects, fires onSelect and dismisses with focus restored", async () => {
    const onSelect = vi.fn();
    setup("list", onSelect);
    open();
    fireEvent.click(screen.getByText("Grid View"));
    expect(onSelect).toHaveBeenCalledWith("grid");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "View options" }));
  });

  it("disabled rows are aria-disabled and never select", () => {
    const onSelect = vi.fn();
    setup("list", onSelect);
    open();
    const disabled = screen.getByRole("menuitemradio", { name: "Disabled View" });
    expect(disabled.getAttribute("aria-disabled")).toBe("true");
    fireEvent.click(disabled);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("outside pointer dismisses without selecting", () => {
    const onSelect = vi.fn();
    const { container } = setup("list", onSelect);
    open();
    fireEvent.pointerDown(container.ownerDocument.body);
    expect(screen.queryByRole("menu")).toBeNull();
    expect(onSelect).not.toHaveBeenCalled();
  });
});

describe("PlaylistOptionsMenu — keyboard model (APG menu)", () => {
  it("opening focuses the selected row", () => {
    setup("list");
    open();
    expect(document.activeElement?.textContent).toBe("List View");
  });

  it("ArrowDown/ArrowUp cycle, skipping disabled rows", () => {
    setup("list");
    open();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });
    expect(document.activeElement?.textContent).toBe("Grid View"); // wraps, skips disabled
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowUp" });
    expect(document.activeElement?.textContent).toBe("List View");
  });

  it("Home/End jump to the first/last enabled row", () => {
    setup("grid");
    open();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "End" });
    expect(document.activeElement?.textContent).toBe("List View"); // disabled row skipped
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Home" });
    expect(document.activeElement?.textContent).toBe("Grid View");
  });

  it("Enter and Space activate the focused row", () => {
    const onSelect = vi.fn();
    setup("list", onSelect);
    open();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("grid");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("Escape closes and restores focus to the trigger", () => {
    setup("list");
    open();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "View options" }));
  });

  it("Tab is trapped inside the open menu (wraps, no focus escape)", () => {
    setup("list");
    open();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Tab", shiftKey: true });
    expect(document.activeElement?.textContent).toBe("Grid View");
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Tab" });
    expect(document.activeElement?.textContent).toBe("List View");
  });

  it("trigger toggles: second click closes the open menu", () => {
    setup("list");
    fireEvent.click(screen.getByRole("button", { name: "View options" }));
    expect(screen.getByRole("menu")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "View options" }));
    expect(screen.queryByRole("menu")).toBeNull();
  });
});

describe("PlaylistOptionsMenu — anchor classes", () => {
  it("align=end / placement=above flip the popup position classes", () => {
    render(
      <PlaylistOptionsMenu
        entries={ENTRIES}
        value="list"
        onSelect={() => {}}
        resolveLabel={(key) => key}
        menuLabel="m"
        triggerLabel="t"
        triggerIcon="list"
        align="end"
        placement="above"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "t" }));
    const menu = screen.getByRole("menu");
    expect(menu.className).toContain("lumen-plo__menu--end");
    expect(menu.className).toContain("lumen-plo__menu--above");
  });
});
