import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Dropdown } from "./Dropdown";
import type { DropdownOption } from "./Dropdown";

/**
 * Card 4.B3 — Dropdown/Select tests: combobox trigger ARIA, listbox roving
 * tabindex, keyboard selection, typeahead, disabled-option semantics, focus
 * restoration. Option labels flow through the resolveLabel seam.
 */

const dict: Record<string, string> = {
  "track.none": "None",
  "track.1": "English",
  "track.2": "Français",
  "track.3": "日本語",
};
const resolve = (key: string): string => dict[key] ?? key;

const onChange = vi.fn();
const options: DropdownOption<string>[] = [
  { value: "none", labelKey: "track.none" },
  { value: "en", labelKey: "track.1" },
  { value: "fr", labelKey: "track.2", disabled: true },
  { value: "ja", labelKey: "track.3" },
];

const openList = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "Subtitle track" }));
  return screen.getByRole("listbox", { name: "Subtitle track" });
};

describe("Dropdown", () => {
  it("renders a combobox trigger with the selected label resolved from data", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    const trigger = screen.getByRole("button", { name: "Subtitle track" });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveTextContent("English");
  });

  it("opens a listbox with options; the selected option carries aria-selected", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    const list = openList();
    expect(list).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "English", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Français" })).toHaveAttribute("aria-disabled", "true");
  });

  it("focuses the selected option on open (roving tabindex)", () => {
    render(
      <Dropdown options={options} value="ja" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    expect(screen.getByRole("option", { name: "日本語" })).toHaveFocus();
  });

  it("moves with ArrowDown/ArrowUp, skipping disabled options", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.keyDown(screen.getByRole("option", { name: "English" }), { key: "ArrowDown" });
    // Français is disabled → skipped.
    expect(screen.getByRole("option", { name: "日本語" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("option", { name: "日本語" }), { key: "ArrowUp" });
    expect(screen.getByRole("option", { name: "English" })).toHaveFocus();
  });

  it("Home/End jump to the first/last option", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.keyDown(screen.getByRole("option", { name: "English" }), { key: "End" });
    expect(screen.getByRole("option", { name: "日本語" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("option", { name: "日本語" }), { key: "Home" });
    expect(screen.getByRole("option", { name: "None" })).toHaveFocus();
  });

  it("Enter selects the focused option: onChange fires, list closes, focus restores to the trigger", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.keyDown(screen.getByRole("option", { name: "English" }), { key: "ArrowUp" });
    fireEvent.keyDown(screen.getByRole("option", { name: "None" }), { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith("none");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Subtitle track" })).toHaveFocus();
  });

  it("Escape closes and restores focus to the trigger", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.keyDown(screen.getByRole("option", { name: "English" }), { key: "Escape" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Subtitle track" })).toHaveFocus();
  });

  it("typeahead jumps to the next option matching the typed prefix", () => {
    render(
      <Dropdown
        options={[
          { value: "none", labelKey: "track.none" },
          { value: "en", labelKey: "track.1" },
          { value: "fr", labelKey: "track.2" },
        ]}
        value="none"
        onChange={onChange}
        label="Subtitle track"
        resolveLabel={resolve}
      />,
    );
    openList();
    fireEvent.keyDown(screen.getByRole("option", { name: "None" }), { key: "f" });
    expect(screen.getByRole("option", { name: "Français" })).toHaveFocus();
  });

  it("clicking a disabled option does not select; clicking an enabled one does", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.click(screen.getByRole("option", { name: "Français" }));
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("option", { name: "日本語" }));
    expect(onChange).toHaveBeenCalledWith("ja");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("outside pointer press dismisses the open list", () => {
    render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    openList();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("applies the W26 density class to the root (default comfortable; compact for settings)", () => {
    const { rerender } = render(
      <Dropdown options={options} value="en" onChange={onChange} label="Subtitle track" resolveLabel={resolve} />,
    );
    const root = screen.getByRole("button", { name: "Subtitle track" }).parentElement;
    expect(root).toHaveClass("lumen-dropdown--comfortable");
    rerender(
      <Dropdown
        options={options}
        value="en"
        onChange={onChange}
        label="Subtitle track"
        resolveLabel={resolve}
        density="compact"
      />,
    );
    expect(root).toHaveClass("lumen-dropdown--compact");
  });

  it("renders option icons in a decorative slot (W26 Setting Preset grammar)", () => {
    const iconOptions: DropdownOption<string>[] = [{ value: "none", labelKey: "track.none", icon: "volume-mute" }];
    render(
      <Dropdown
        options={iconOptions}
        value="none"
        onChange={onChange}
        label="Subtitle track"
        resolveLabel={resolve}
      />,
    );
    openList();
    const option = screen.getByRole("option", { name: "None" });
    const icon = option.querySelector(".lumen-dropdown__option-icon svg");
    expect(icon).not.toBeNull();
    // Decorative: the resolved label stays the accessible name.
    expect(icon?.getAttribute("aria-hidden")).toBe("true");
  });
});
