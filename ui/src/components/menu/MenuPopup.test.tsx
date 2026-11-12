import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ContextMenu } from "./ContextMenu";
import { MenuPopup } from "./MenuPopup";
import type { MenuEntry } from "./types";

/**
 * w26.f07 — MenuPopup viewport clamping (e02 F1 / e12 F1+F2): a popup opened
 * at the pointer flips up/left across the pointer when it would overflow the
 * bottom/right of the window (8px margin, shift as a last resort), and
 * submenus flip left of their parent row near the right edge / shift up near
 * the bottom. jsdom rects are all-zero, so geometry is pinned on the
 * prototype (the slider-suite pattern, hoisted so the mount-time layout
 * effect — which runs before the test can touch the element — sees it).
 */

const asRect = (x: number, y: number, width: number, height: number): DOMRect =>
  ({
    x,
    y,
    top: y,
    left: x,
    right: x + width,
    bottom: y + height,
    width,
    height,
    toJSON: () => ({}),
  }) as DOMRect;

/** Pin geometry for every element the clamp effect measures: the popup
 * (`.lumen-menu`) and, for submenus, the parent row host. */
const mockMenuGeometry = (popupRect: DOMRect, hostRect: DOMRect = asRect(0, 0, 0, 0)): void => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    if (this.classList.contains("lumen-menu__host")) return hostRect;
    if (this.classList.contains("lumen-menu")) return popupRect;
    return asRect(0, 0, 0, 0);
  });
};

const stubViewport = (width: number, height: number): void => {
  Object.defineProperty(window, "innerWidth", { value: width, configurable: true });
  Object.defineProperty(window, "innerHeight", { value: height, configurable: true });
};

const close = (): void => undefined;

const pointerEntries: MenuEntry<string>[] = [
  { id: "play", labelKey: "Play" },
  { id: "sep", kind: "separator" },
  { id: "quit", labelKey: "Quit VLC" },
];

const submenuEntries: MenuEntry<string>[] = [
  { id: "open", labelKey: "Open Media" },
  {
    id: "playback",
    labelKey: "Playback",
    submenu: [{ id: "faster", labelKey: "Faster" }],
  },
];

/** Render the shared popup exactly the way the player canvas menu mounts it
 * (PlayerScreen pattern: pointer px style + the fixed-position class). */
const renderPointerPopup = (x: number, y: number): HTMLElement => {
  render(
    <MenuPopup
      entries={pointerEntries}
      resolveLabel={(key) => key}
      idPrefix="test-pointer-menu"
      className="lumen-context-menu__popup"
      style={{ left: x, top: y }}
      ariaLabel="Canvas actions"
      onRequestClose={close}
      onDismissAll={close}
    />,
  );
  return screen.getByRole("menu", { name: "Canvas actions" });
};

describe("MenuPopup viewport clamping (pointer-anchored popups)", () => {
  it("flips the popup up-left across the pointer when it would overflow the bottom-right corner", () => {
    // w26.e12 F1 reproduction @1440×900: pointer (1300, 800), panel 280×683.
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(1300, 800, 280, 683));
    const menu = renderPointerPopup(1300, 800);
    expect(menu.style.left).toBe("1020px"); // 1300 − 280: right edge at the pointer
    expect(menu.style.top).toBe("117px"); // 800 − 683: bottom edge at the pointer
  });

  it("clamps to the viewport margin when even the flipped position cannot fit", () => {
    // Panel taller than a 768px window: flip up runs past the top → margin.
    stubViewport(1024, 768);
    mockMenuGeometry(asRect(900, 700, 280, 849));
    const menu = renderPointerPopup(900, 700);
    expect(menu.style.left).toBe("620px"); // flip left fits (≥ 8px)
    expect(menu.style.top).toBe("8px"); // flip up does not → shift to margin
  });

  it("keeps the pointer coordinates untouched when the popup fits", () => {
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(100, 100, 280, 683));
    const menu = renderPointerPopup(100, 100);
    expect(menu.style.left).toBe("100px");
    expect(menu.style.top).toBe("100px");
  });

  it("shifts popups opened at the window's origin to the margin", () => {
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(2, 3, 280, 683));
    const menu = renderPointerPopup(2, 3);
    expect(menu.style.left).toBe("8px");
    expect(menu.style.top).toBe("8px");
  });

  it("clamps through the ContextMenu wrapper (shared primitive, every consumer)", () => {
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(1300, 800, 280, 683));
    render(
      <ContextMenu entries={pointerEntries} resolveLabel={(key) => key} ariaLabel="Playlist item actions">
        <button type="button">Row</button>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 1300, clientY: 800 });
    const menu = screen.getByRole("menu", { name: "Playlist item actions" });
    expect(menu.style.left).toBe("1020px");
    expect(menu.style.top).toBe("117px");
  });
});

describe("MenuPopup viewport clamping (submenus)", () => {
  const openSubmenu = (): HTMLElement => {
    render(
      <MenuPopup
        entries={submenuEntries}
        resolveLabel={(key) => key}
        idPrefix="test-submenu-menu"
        ariaLabel="Canvas actions"
        onRequestClose={close}
        onDismissAll={close}
      />,
    );
    // Clicking the submenu parent opens it (activation path, no hover timers).
    fireEvent.click(screen.getByRole("menuitem", { name: "Playback" }));
    return screen.getByRole("menu", { name: "Playback" });
  };

  it("flips a submenu to the left of its parent row when it would overflow the right edge", () => {
    // w26.e12 F2 reproduction @1440×900: submenu right edge 1637 > 1432.
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(1237, 200, 400, 300), asRect(1000, 196, 280, 48));
    const submenu = openSubmenu();
    // Right edge lands 4px left of the host's left edge: 1000 − 4 − 400 = 596.
    expect(submenu.style.transform).toBe("translate(-641px, 0px)");
  });

  it("shifts a submenu up when it would overflow the bottom edge", () => {
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(500, 600, 300, 400), asRect(200, 596, 280, 48));
    const submenu = openSubmenu();
    // Bottom edge 1000 → 892 (viewport − margin): dy = −108.
    expect(submenu.style.transform).toBe("translate(0px, -108px)");
  });

  it("leaves a submenu that fits untouched", () => {
    stubViewport(1440, 900);
    mockMenuGeometry(asRect(500, 100, 300, 400), asRect(200, 96, 280, 48));
    const submenu = openSubmenu();
    expect(submenu.style.transform).toBe("");
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});
