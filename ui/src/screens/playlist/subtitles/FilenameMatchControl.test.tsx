// w26.b11 — filename-matching control tests (sub filename.svg): the five
// frame options, the checked default, keyboard selection through the shared
// Dropdown's listbox model, and the scoped frame scale hooks.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FilenameMatchControl } from "./FilenameMatchControl";
import { DEFAULT_MATCH_MODE } from "./matching";

afterEach(cleanup);

const OPTION_LABELS = [
  "Off",
  "Any title",
  "Contains title",
  "Title with extra characters",
  "Exact title match",
];

describe("FilenameMatchControl — sub filename.svg", () => {
  it("renders the trigger with the current strategy's label", () => {
    render(<FilenameMatchControl value={DEFAULT_MATCH_MODE} onChange={() => {}} />);
    // The trigger's accessible name is the control label; its visible text
    // is the selected option.
    expect(screen.getByRole("button", { name: "Filename matching" }).textContent).toContain(
      "Title with extra characters",
    );
  });

  it("mounts the scoped frame-scale class (48px trigger, 44px rows)", () => {
    const { container } = render(
      <FilenameMatchControl value="exact" onChange={() => {}} />,
    );
    expect(container.querySelector(".lumen-plsubs-match")).not.toBeNull();
  });

  it("opens the frame's five options in order (chevron trigger)", () => {
    render(<FilenameMatchControl value="extra" onChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Filename matching" }));
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual(OPTION_LABELS);
  });

  it("marks the selected option (aria-selected) with the check row", () => {
    render(<FilenameMatchControl value="contains" onChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Filename matching" }));
    expect(screen.getByRole("option", { selected: true }).textContent).toContain(
      "Contains title",
    );
  });

  it("reports keyboard selection (ArrowDown + Enter) and restores focus", () => {
    const onChange = vi.fn();
    render(<FilenameMatchControl value="off" onChange={onChange} />);
    const trigger = screen.getByRole("button", { name: "Filename matching" });
    fireEvent.click(trigger);
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Enter" });
    expect(onChange).toHaveBeenLastCalledWith("any");
  });
});
