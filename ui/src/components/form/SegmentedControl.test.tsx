import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { SegmentedControl } from "./SegmentedControl";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

const options = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

describe("SegmentedControl (Values.svg, wave 26)", () => {
  it("exposes radiogroup semantics with one checked segment", () => {
    render(<SegmentedControl label="Theme" options={options} value="dark" onChange={() => {}} />);
    const group = screen.getByRole("radiogroup", { name: "Theme" });
    expect(group).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("aria-checked", "false");
  });

  it("selects a segment through onChange on click", () => {
    const onChange = vi.fn();
    render(<SegmentedControl label="Theme" options={options} value="dark" onChange={onChange} />);
    fireEvent.click(screen.getByRole("radio", { name: "System" }));
    expect(onChange).toHaveBeenCalledWith("system");
  });

  it("roving tabindex: only the selected segment is tabbable", () => {
    render(<SegmentedControl label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("tabindex", "-1");
  });

  it("arrow keys move the selection with wrap-around", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <SegmentedControl label="Theme" options={options} value="dark" onChange={onChange} />,
    );
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("system");
    onChange.mockClear();
    rerender(<SegmentedControl label="Theme" options={options} value="system" onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowDown" });
    expect(onChange).toHaveBeenCalledWith("light");
  });

  it("disabled group — every segment inert", () => {
    render(<SegmentedControl label="Theme" options={options} value="dark" disabled onChange={() => {}} />);
    for (const option of options) {
      expect(screen.getByRole("radio", { name: option.label })).toBeDisabled();
    }
  });

  // j01-14: per-option disabled (only group-level was covered before).
  it("per-option disabled — the segment is inert and the arrow keys skip it", () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        label="Theme"
        value="dark"
        onChange={onChange}
        options={[
          { value: "light", label: "Light", disabled: true },
          { value: "dark", label: "Dark" },
          { value: "system", label: "System" },
        ]}
      />,
    );
    expect(screen.getByRole("radio", { name: "Light" })).toBeDisabled();
    expect(screen.getByRole("radio", { name: "Dark" })).toBeEnabled();
    // Enabled set = [dark, system]; ArrowUp from "dark" wraps to "system"
    // — if disabled "light" were not skipped, it would be selected.
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowUp" });
    expect(onChange).toHaveBeenCalledWith("system");
    expect(onChange).not.toHaveBeenCalledWith("light");
  });

  // j01-7: the id prop lands on the GROUP element.
  it("id lands on the radiogroup host (aria-describedby wiring)", () => {
    render(<SegmentedControl id="theme-seg" label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(screen.getByRole("radiogroup")).toHaveAttribute("id", "theme-seg");
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("id", "theme-seg-dark");
  });

  // j01-6: `invalid` now carries a visual error skin like RadioGroup, not
  // just aria-invalid.
  it("invalid flag sets aria-invalid AND the lumen-choice--error modifier on the group", () => {
    render(<SegmentedControl label="Theme" options={options} value="dark" invalid onChange={() => {}} />);
    const group = screen.getByRole("radiogroup");
    expect(group).toHaveAttribute("aria-invalid", "true");
    expect(group).toHaveClass("lumen-choice--error");
  });

  it("DOM snapshot", () => {
    const { container } = render(
      <SegmentedControl label="Theme" options={options} value="dark" onChange={() => {}} />,
    );
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("SegmentedControl skin consumes the Values.svg language", () => {
  it("track on surface + control radius; active segment accent-tinted with accent text", () => {
    expect(formCss).toMatch(/\.lumen-segmented\s*\{[^}]*--color-surface[^}]*--radius-control/);
    expect(formCss).toMatch(
      /\.lumen-segmented__segment\[aria-checked="true"\]\s*\{[^}]*--color-accent-tinted[^}]*--color-accent/,
    );
  });

  it("inactive segments: tertiary text on panel fill; hover/focus/disabled states", () => {
    expect(formCss).toMatch(/\.lumen-segmented__segment\s*\{[^}]*--color-content-tertiary[^}]*--color-surface-panel/);
    expect(formCss).toMatch(/\.lumen-segmented__segment\[aria-checked="false"\]:hover\s*\{[^}]*--color-surface-raised/);
    expect(formCss).toMatch(/\.lumen-segmented__segment:active\s*\{[^}]*--color-surface-selected/);
    expect(formCss).toMatch(/\.lumen-segmented__segment:focus-visible\s*\{[^}]*--color-accent/);
    expect(formCss).toMatch(/\.lumen-segmented__segment:disabled\s*\{[^}]*--opacity-disabled/);
  });

  // j01-6: the invalid skin — the error modifier swaps the segment focus
  // ring to --color-record, mirroring RadioGroup's `.lumen-choice--error`
  // fidelity (same prop, same grammar).
  it("j01-6: invalid carries an error skin — record focus ring via the shared modifier", () => {
    expect(formCss).toMatch(
      /\.lumen-choice--error \.lumen-segmented__segment:focus-visible\s*\{[^}]*--color-record/,
    );
  });
});
