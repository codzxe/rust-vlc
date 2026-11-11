import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MenuItem } from "./MenuItem";
import type { MenuEntryBase } from "./types";

/**
 * Card 4.B3 / W26 figma grammar — MenuItem presentational slots: optional
 * leading line icon (data-driven, additive), selective icon-accent flag,
 * check slot for checkable + icon-less rows, accelerator + submenu chevron.
 * Keyboard behavior is owned by MenuPopup (covered by MenuBar/ContextMenu
 * suites); these tests pin the new visual structure only.
 */

const dict: Record<string, string> = { "menu.open": "Open File…", "menu.profile": "Profile (Movies)" };
const resolve = (key: string): string => dict[key] ?? key;

describe("MenuItem", () => {
  it("renders an optional leading icon in a decorative slot", () => {
    const entry: MenuEntryBase<string> = { id: "open", labelKey: "menu.open", icon: "open-file" };
    render(<MenuItem entry={entry} resolveLabel={resolve} active />);
    const item = screen.getByRole("menuitem", { name: "Open File…" });
    const icon = item.querySelector(".lumen-menu__icon svg");
    expect(icon).not.toBeNull();
    // Decorative: the label stays the accessible name.
    expect(icon?.getAttribute("aria-hidden")).toBe("true");
  });

  it("flags selective icon accents (profile/wand rows in the frames)", () => {
    const entry: MenuEntryBase<string> = {
      id: "profile",
      labelKey: "menu.profile",
      icon: "customize-player",
      iconAccent: true,
    };
    render(<MenuItem entry={entry} resolveLabel={resolve} active />);
    expect(screen.getByRole("menuitem", { name: "Profile (Movies)" })).toHaveClass(
      "lumen-menu__item--icon-accent",
    );
  });

  it("keeps the check slot for checkable rows and for plain rows without icons", () => {
    const checked: MenuEntryBase<string> = { id: "shuffle", labelKey: "menu.open", checked: true };
    const { rerender } = render(<MenuItem entry={checked} resolveLabel={resolve} active />);
    expect(screen.getByRole("menuitemcheckbox").querySelector(".lumen-menu__check")).not.toBeNull();
    const plain: MenuEntryBase<string> = { id: "open", labelKey: "menu.open" };
    rerender(<MenuItem entry={plain} resolveLabel={resolve} active />);
    const item = screen.getByRole("menuitem");
    expect(item.querySelector(".lumen-menu__check")).not.toBeNull();
    expect(item.querySelector(".lumen-menu__icon")).toBeNull();
  });

  it("renders the accelerator and the submenu chevron affordances", () => {
    const entry: MenuEntryBase<string> = {
      id: "recent",
      labelKey: "menu.open",
      accelerator: "Ctrl+O",
      submenu: [{ id: "last", labelKey: "menu.open" }],
    };
    render(<MenuItem entry={entry} resolveLabel={resolve} active hasSubmenu />);
    const item = screen.getByRole("menuitem", { name: /^Open File/ });
    expect(item).toHaveTextContent("Ctrl+O");
    expect(item.querySelector(".lumen-menu__submenu-arrow")).not.toBeNull();
    expect(item).toHaveAttribute("aria-haspopup", "true");
    expect(item).toHaveAttribute("aria-expanded", "false");
  });
});
