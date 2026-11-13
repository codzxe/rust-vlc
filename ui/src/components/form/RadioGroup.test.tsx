import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RadioGroup } from "./RadioGroup";

const options = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

describe("RadioGroup (doc 08 §4.5)", () => {
  it("exposes radiogroup semantics and labelled options", () => {
    render(<RadioGroup label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(screen.getByRole("radiogroup", { name: "Theme" })).toBeInTheDocument();
    expect(screen.getByLabelText("Dark")).toBeChecked();
    expect(screen.getByLabelText("Light")).not.toBeChecked();
  });

  it("selects an option through onChange on click", () => {
    const onChange = vi.fn();
    render(<RadioGroup label="Theme" options={options} value="dark" onChange={onChange} />);
    fireEvent.click(screen.getByLabelText("System"));
    expect(onChange).toHaveBeenCalledWith("system");
  });

  it("roving tabindex: only the selected option is tabbable", () => {
    render(<RadioGroup label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(screen.getByLabelText("Dark")).toHaveAttribute("tabindex", "0");
    expect(screen.getByLabelText("Light")).toHaveAttribute("tabindex", "-1");
    expect(screen.getByLabelText("System")).toHaveAttribute("tabindex", "-1");
  });

  it("horizontal arrows move the selection too", () => {
    const onChange = vi.fn();
    render(<RadioGroup label="Theme" options={options} value="dark" onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("system");
  });

  it("arrow keys wrap around the option list", () => {
    const onChange = vi.fn();
    render(<RadioGroup label="Theme" options={options} value="system" onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowDown" });
    expect(onChange).toHaveBeenCalledWith("light");
  });

  it("disabled group — every option carries the disabled attribute", () => {
    render(<RadioGroup label="Theme" options={options} value="dark" disabled onChange={() => {}} />);
    // Native disabled on each input is the contract (browser-gated).
    expect(screen.getByLabelText("Light")).toBeDisabled();
    expect(screen.getByLabelText("Dark")).toBeDisabled();
    expect(screen.getByLabelText("System")).toBeDisabled();
  });

  // j01-14: per-option disabled (only group-level was covered before).
  it("per-option disabled — the option is inert and the arrow keys skip it", () => {
    const onChange = vi.fn();
    render(
      <RadioGroup
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
    expect(screen.getByLabelText("Light")).toBeDisabled();
    expect(screen.getByLabelText("Dark")).toBeEnabled();
    // Enabled set = [dark, system]. ArrowDown from "dark" → "system".
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowDown" });
    expect(onChange).toHaveBeenCalledWith("system");
    onChange.mockClear();
    // The discriminating case: ArrowUp from "dark" (index 0 of the enabled
    // set) wraps to "system" — if the disabled "light" were NOT skipped,
    // ArrowUp would select "light" instead.
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowUp" });
    expect(onChange).toHaveBeenCalledWith("system");
    expect(onChange).not.toHaveBeenCalledWith("light");
  });

  // j01-7: the id prop lands on the GROUP element (consumers wire
  // aria-describedby/aria-labelledby against it).
  it("id lands on the radiogroup host (aria-describedby wiring)", () => {
    render(<RadioGroup id="theme-group" label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(screen.getByRole("radiogroup")).toHaveAttribute("id", "theme-group");
    // child ids still derive from the group id (no collision)
    expect(screen.getByLabelText("Dark")).toHaveAttribute("id", "theme-group-dark");
  });

  it("invalid flag sets aria-invalid on the group", () => {
    render(<RadioGroup label="Theme" options={options} value="dark" invalid onChange={() => {}} />);
    expect(screen.getByRole("radiogroup")).toHaveAttribute("aria-invalid", "true");
  });

  it("DOM snapshot", () => {
    const { container } = render(<RadioGroup label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});
