// w26.b10 — PlaylistOptionsMenu tests: the anchored `role="menu"` popup in
// the option frames' grammar. Rendering, radio semantics + check, click
// selection with dismiss + focus restore, the full APG keyboard model
// (arrows cycle, Home/End, Enter/Space, Escape, Tab trap), outside-pointer
// dismiss, disabled inertness, align/placement classes.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

describe("PlaylistOptionsMenu — open-direction measurement (w26.k15, j15 P3-2/P3-8)", () => {
  /** Stubs the trigger geometry: a 40px-tall trigger at the given viewport
   * position (the shared measure() only reads the TRIGGER's rect). */
  const stubTriggerRect = (top: number): void => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      top,
      bottom: top + 40,
      height: 40,
      width: 40,
      left: 0,
      right: 40,
      x: 0,
      y: top,
      toJSON: () => ({}),
    } as DOMRect);
  };

  afterEach(() => {
    vi.restoreAllMocks();
    window.innerHeight = 768; // jsdom default
  });

  it("flips ABOVE when the preferred side cannot fit and the other can", () => {
    // 3 rows: natural 3×48+8 = 152. Viewport 600, trigger at y500: below =
    // 600−540−4−8 = 48 (< 152), above = 500−4−8 = 488 (≥ 152) → flip.
    stubTriggerRect(500);
    window.innerHeight = 600;
    setup();
    open();
    expect(screen.getByRole("menu").className).toContain("lumen-plo__menu--above");
    expect(screen.getByRole("menu").style.maxHeight).toBe(""); // no cap needed
  });

  it("caps the panel to a scrollport when NEITHER side fits (the 16-row shape)", () => {
    const SIXTEEN = Array.from({ length: 16 }, (_, index) => ({ id: `k${index}`, labelKey: `Row ${index}` }));
    // natural 16×48+8 = 776. Viewport 600, trigger at y300: below = 248,
    // above = 288 — neither fits → roomier side (above) + a max-height cap.
    stubTriggerRect(300);
    window.innerHeight = 600;
    render(
      <PlaylistOptionsMenu
        entries={SIXTEEN}
        value="k0"
        onSelect={() => {}}
        resolveLabel={(key) => key}
        menuLabel="m"
        triggerLabel="t"
        triggerIcon="list"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "t" }));
    const menu = screen.getByRole("menu");
    expect(menu.className).toContain("lumen-plo__menu--above");
    expect(menu.style.maxHeight).toBe("288px"); // above room, floored at 160
  });

  it("measures with the LIVE token values, not the TS fallbacks", () => {
    // jsdom's getComputedStyle returns "" for stylesheet custom properties —
    // the stub makes the trigger resolve --space-item-pitch as 10px, so the
    // 3-row panel is 38px tall and fits below with no flip/cap.
    vi.spyOn(window, "getComputedStyle").mockImplementation(
      () =>
        ({
          getPropertyValue: (token: string) =>
            token === "--space-item-pitch" ? "10px" : token === "--space-grid" ? "4px" : "",
        }) as unknown as CSSStyleDeclaration,
    );
    stubTriggerRect(500);
    window.innerHeight = 900; // below = 900−540−12 = 348 ≥ 38
    setup();
    open();
    const menu = screen.getByRole("menu");
    expect(menu.className).toContain("lumen-plo__menu--below");
    expect(menu.style.maxHeight).toBe("");
  });

  it("re-measures on window resize while open (was resize-blind)", () => {
    stubTriggerRect(500);
    window.innerHeight = 900; // below = 348 ≥ 152 → stays below
    setup();
    open();
    expect(screen.getByRole("menu").className).toContain("lumen-plo__menu--below");
    // Shrink the viewport under the open menu: below = 48 < 152, above = 488.
    window.innerHeight = 600;
    fireEvent(window, new Event("resize"));
    expect(screen.getByRole("menu").className).toContain("lumen-plo__menu--above");
  });

  it("zero-height triggers (jsdom/display:none hosts) keep the anchor side", () => {
    // height 0: the measure keeps the anchor placement unchanged (nothing to
    // measure — the jsdom path every OTHER test in this suite rides on).
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      top: 0,
      bottom: 0,
      height: 0,
      width: 0,
      left: 0,
      right: 0,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    } as DOMRect);
    setup();
    open();
    expect(screen.getByRole("menu").className).toContain("lumen-plo__menu--below");
    expect(screen.getByRole("menu").style.maxHeight).toBe("");
  });
});
