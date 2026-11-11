import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ContextMenu } from "./ContextMenu";
import type { MenuEntry } from "./types";

/**
 * Card 4.B3 — ContextMenu tests: opens at right-click coordinates, full
 * menu keyboard model, Escape restores focus to the previously-focused
 * element, activation dismisses.
 */

const dict: Record<string, string> = {
  "ctx.play": "Play",
  "ctx.info": "Media Information",
  "ctx.remove": "Remove from Playlist",
};
const resolve = (key: string): string => dict[key] ?? key;

const onPlay = vi.fn();
const onRemove = vi.fn();

const entries: MenuEntry<string>[] = [
  { id: "play", labelKey: "ctx.play", onSelect: onPlay },
  { id: "info", labelKey: "ctx.info", disabled: true },
  { id: "remove", labelKey: "ctx.remove", onSelect: onRemove },
];

describe("ContextMenu", () => {
  it("opens a role=menu popup on right-click at the pointer coordinates", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve} ariaLabel="Playlist item actions">
        <button type="button">Row</button>
      </ContextMenu>,
    );
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 120, clientY: 80 });
    const menu = screen.getByRole("menu", { name: "Playlist item actions" });
    expect(menu).toBeInTheDocument();
    expect(menu.style.left).toBe("120px");
    expect(menu.style.top).toBe("80px");
    expect(screen.getByRole("menuitem", { name: "Play" })).toHaveFocus();
  });

  it("suppresses the native browser context menu", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    const button = screen.getByRole("button", { name: "Row" });
    const event = fireEvent.contextMenu(button, { clientX: 0, clientY: 0 });
    // jsdom reports the preventDefault flag through the dispatch result.
    expect(event).toBe(false);
  });

  it("Escape closes the menu and restores focus to the previously-focused element", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    const button = screen.getByRole("button", { name: "Row" });
    button.focus();
    fireEvent.contextMenu(button, { clientX: 10, clientY: 10 });
    expect(screen.getByRole("menu")).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Play" }), { key: "Escape" });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(button).toHaveFocus();
  });

  it("supports arrow-key navigation and skips nothing (roving tabindex)", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 10, clientY: 10 });
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Play" }), { key: "ArrowDown" });
    expect(screen.getByRole("menuitem", { name: "Media Information" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Media Information" }), { key: "ArrowDown" });
    expect(screen.getByRole("menuitem", { name: "Remove from Playlist" })).toHaveFocus();
  });

  it("activating an item fires onSelect and dismisses the menu", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 10, clientY: 10 });
    // Move focus down to the target item first (Enter activates the active item).
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Play" }), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Media Information" }), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Remove from Playlist" }), { key: "Enter" });
    expect(onRemove).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("outside pointer press dismisses the menu", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 10, clientY: 10 });
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("applies the W26 density class to the popup (default comfortable)", () => {
    render(
      <ContextMenu entries={entries} resolveLabel={resolve}>
        <button type="button">Row</button>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByRole("button", { name: "Row" }), { clientX: 10, clientY: 10 });
    expect(screen.getByRole("menu")).toHaveClass("lumen-menu--comfortable");
  });
});
