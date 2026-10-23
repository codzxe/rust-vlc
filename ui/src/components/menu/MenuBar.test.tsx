import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MenuBar } from "./MenuBar";
import { SUBMENU_OPEN_DELAY_MS } from "./MenuPopup";
import type { MenuBarMenu } from "./MenuBar";

/**
 * Card 4.B3 — MenuBar tests: WAI-ARIA menubar model, roving tabindex,
 * submenu keyboard + hover-intent behavior, focus trap, focus restoration.
 * Labels flow through the resolveLabel seam (useMsg() stand-in) — nothing is
 * hardcoded in the component.
 */

const dict: Record<string, string> = {
  "menu.file": "File",
  "menu.playback": "Playback",
  "menu.open": "Open File…",
  "menu.save": "Save Playlist…",
  "menu.quit": "Quit",
  "menu.disabled": "Convert",
  "menu.recent": "Open Recent",
  "menu.recent.last1": "movie.avi",
  "menu.recent.last2": "clip.mp4",
  "menu.check": "Shuffle",
};
const resolve = (key: string): string => dict[key] ?? key;

const onSelect = vi.fn();
const onQuit = vi.fn();

const menus: MenuBarMenu<string>[] = [
  {
    id: "file",
    labelKey: "menu.file",
    entries: [
      { id: "open", labelKey: "menu.open", accelerator: "Ctrl+O", onSelect },
      { id: "sep", kind: "separator" },
      {
        id: "recent",
        labelKey: "menu.recent",
        submenu: [
          { id: "last1", labelKey: "menu.recent.last1", onSelect },
          { id: "last2", labelKey: "menu.recent.last2", onSelect },
        ],
      },
      { id: "convert", labelKey: "menu.disabled", disabled: true, onSelect },
    ],
  },
  {
    id: "playback",
    labelKey: "menu.playback",
    entries: [
      { id: "shuffle", labelKey: "menu.check", checked: true, onSelect },
      { id: "quit", labelKey: "menu.quit", onSelect: onQuit },
    ],
  },
];

const openFileMenu = (): HTMLElement => {
  fireEvent.click(screen.getByRole("menuitem", { name: "File" }));
  return screen.getByRole("menu", { name: "File" });
};

// The accelerator is part of the item's accessible name (screen readers
// announce shortcuts), so exact-name queries use a prefix regex.
const openItem = (): HTMLElement => screen.getByRole("menuitem", { name: /Open File/ });

describe("MenuBar", () => {
  beforeEach(() => {
    onSelect.mockClear();
    onQuit.mockClear();
  });

  it("renders a menubar whose labels come from the resolveLabel seam (no hardcoded text)", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    expect(screen.getByRole("menubar")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "File" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Playback" })).toBeInTheDocument();
    // Menus are closed initially.
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("exposes haspopup + expanded ARIA state on triggers", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    const file = screen.getByRole("menuitem", { name: "File" });
    expect(file).toHaveAttribute("aria-haspopup", "true");
    expect(file).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(file);
    expect(file).toHaveAttribute("aria-expanded", "true");
  });

  it("ArrowDown opens the menu and focuses its first item (roving tabindex)", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    const file = screen.getByRole("menuitem", { name: "File" });
    fireEvent.keyDown(file, { key: "ArrowDown" });
    const menu = screen.getByRole("menu", { name: "File" });
    const open = openItem();
    expect(menu).toBeInTheDocument();
    expect(open).toHaveFocus();
    expect(open).toHaveAttribute("tabindex", "0");
  });

  it("navigates with ArrowDown/ArrowUp and skips separators", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    const open = openItem();
    fireEvent.keyDown(open, { key: "ArrowDown" });
    // Separator at index 1 is skipped → focus lands on the submenu parent.
    expect(screen.getByRole("menuitem", { name: "Open Recent" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Open Recent" }), { key: "ArrowUp" });
    expect(open).toHaveFocus();
  });

  it("Home/End jump to the first/last item", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    fireEvent.keyDown(openItem(), { key: "End" });
    expect(screen.getByRole("menuitem", { name: "Convert" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Convert" }), { key: "Home" });
    expect(openItem()).toHaveFocus();
  });

  it("Enter activates an item: onSelect fires, menu closes, focus returns to the trigger", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    fireEvent.keyDown(openItem(), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "File" })).toHaveFocus();
  });

  it("Escape closes the menu and restores focus to the trigger", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    expect(openItem()).toHaveFocus();
    fireEvent.keyDown(openItem(), { key: "Escape" });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "File" })).toHaveFocus();
  });

  it("ArrowRight opens a submenu and focuses its first item; ArrowLeft closes back to the parent", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    const recent = screen.getByRole("menuitem", { name: "Open Recent" });
    // Move the active item onto the submenu parent first (roving focus).
    fireEvent.keyDown(openItem(), { key: "ArrowDown" });
    expect(recent).toHaveFocus();
    fireEvent.keyDown(recent, { key: "ArrowRight" });
    const submenu = screen.getByRole("menu", { name: "Open Recent" });
    expect(submenu).toBeInTheDocument();
    expect(recent).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("menuitem", { name: "movie.avi" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "movie.avi" }), { key: "ArrowLeft" });
    expect(screen.queryByRole("menu", { name: "Open Recent" })).not.toBeInTheDocument();
    expect(recent).toHaveFocus();
  });

  it("Escape inside a submenu closes only the submenu and refocuses its parent item", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    const recent = screen.getByRole("menuitem", { name: "Open Recent" });
    fireEvent.keyDown(openItem(), { key: "ArrowDown" });
    fireEvent.keyDown(recent, { key: "ArrowRight" });
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "movie.avi" }), { key: "Escape" });
    expect(screen.queryByRole("menu", { name: "Open Recent" })).not.toBeInTheDocument();
    expect(screen.getByRole("menu", { name: "File" })).toBeInTheDocument();
    expect(recent).toHaveFocus();
  });

  it("opens a submenu on hover after the delay (hover intent, fake timers)", async () => {
    vi.useFakeTimers();
    try {
      render(<MenuBar menus={menus} resolveLabel={resolve} />);
      openFileMenu();
      const recent = screen.getByRole("menuitem", { name: "Open Recent" });
      const host = recent.parentElement as HTMLElement;
      // React derives mouseenter from mouseover (bubbling) in jsdom.
      fireEvent.mouseOver(host);
      expect(screen.queryByRole("menu", { name: "Open Recent" })).not.toBeInTheDocument();
      await act(async () => {
        vi.advanceTimersByTime(SUBMENU_OPEN_DELAY_MS - 1);
      });
      expect(screen.queryByRole("menu", { name: "Open Recent" })).not.toBeInTheDocument();
      await act(async () => {
        vi.advanceTimersByTime(1);
      });
      expect(screen.getByRole("menu", { name: "Open Recent" })).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it("ArrowRight/ArrowLeft move between menubar triggers and switch the open menu", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    const file = screen.getByRole("menuitem", { name: "File" });
    file.focus();
    fireEvent.keyDown(file, { key: "ArrowRight" });
    expect(screen.getByRole("menuitem", { name: "Playback" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Playback" }), { key: "ArrowDown" });
    expect(screen.getByRole("menu", { name: "Playback" })).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Playback" }), { key: "ArrowLeft" });
    // Open menu switched to File, focus follows into its first item.
    expect(screen.getByRole("menu", { name: "File" })).toBeInTheDocument();
    expect(screen.queryByRole("menu", { name: "Playback" })).not.toBeInTheDocument();
  });

  it("Tab is trapped inside the open menu (wraps at the edges)", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    fireEvent.keyDown(openItem(), { key: "End" });
    expect(screen.getByRole("menuitem", { name: "Convert" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Convert" }), { key: "Tab" });
    // Wraps back to the first item instead of leaving the menu.
    expect(openItem()).toHaveFocus();
  });

  it("disabled items carry aria-disabled and are not activatable", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    fireEvent.keyDown(openItem(), { key: "End" });
    const convert = screen.getByRole("menuitem", { name: "Convert" });
    expect(convert).toHaveFocus();
    expect(convert).toHaveAttribute("aria-disabled", "true");
    fireEvent.keyDown(convert, { key: "Enter" });
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("renders accelerator strings from keymap data verbatim", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    expect(screen.getByText("Ctrl+O")).toBeInTheDocument();
  });

  it("checked entries render menuitemcheckbox with aria-checked", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    fireEvent.click(screen.getByRole("menuitem", { name: "Playback" }));
    const shuffle = screen.getByRole("menuitemcheckbox", { name: "Shuffle" });
    expect(shuffle).toHaveAttribute("aria-checked", "true");
  });

  it("renders separators with role=separator", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    expect(screen.getByRole("separator")).toBeInTheDocument();
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});
