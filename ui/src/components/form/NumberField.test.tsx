import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { NumberField } from "./NumberField";

describe("NumberField", () => {
  it("renders a labelled number input with steppers", () => {
    render(<NumberField label="Delay" value={5} min={0} max={10} onChange={() => {}} />);
    const input = screen.getByLabelText("Delay");
    expect(input).toHaveAttribute("type", "number");
    expect(input).toHaveValue(5);
    expect(screen.getByTestId("stepper-up")).toBeInTheDocument();
    expect(screen.getByTestId("stepper-down")).toBeInTheDocument();
  });

  it("steppers step through onChange and clamp to bounds", () => {
    const onChange = vi.fn();
    render(<NumberField label="Delay" value={9} min={0} max={10} step={2} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("stepper-up"));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ target: { value: "10", valueAsNumber: 10 } }));
    onChange.mockClear();
    fireEvent.click(screen.getByTestId("stepper-down"));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ target: { value: "7", valueAsNumber: 7 } }));
  });

  it("steppers disable at the bounds (no fabricated out-of-range values)", () => {
    render(<NumberField label="Delay" value={10} max={10} onChange={() => {}} />);
    expect(screen.getByTestId("stepper-up")).toBeDisabled();
    expect(screen.getByTestId("stepper-down")).toBeEnabled();
  });

  // j01-1 regression: min/max/step must reach the NATIVE input — without
  // them, keyboard users (native arrows) step past the bounds / ignore the
  // step while pointer steppers clamp: two contracts for one control.
  it("j01-1: min/max/step land on the native input (keyboard parity with the pointer steppers)", () => {
    render(<NumberField label="Delay" value={4} min={0} max={10} step={2} onChange={() => {}} />);
    const input = screen.getByLabelText("Delay");
    expect(input).toHaveAttribute("min", "0");
    expect(input).toHaveAttribute("max", "10");
    expect(input).toHaveAttribute("step", "2");
  });

  it("j01-1: bounds are omitted from the DOM when not provided (no fabricated constraints)", () => {
    render(<NumberField label="Delay" value={4} onChange={() => {}} />);
    const input = screen.getByLabelText("Delay");
    expect(input).not.toHaveAttribute("min");
    expect(input).not.toHaveAttribute("max");
    // step defaults to 1 (the native default — same behavior, now explicit)
    expect(input).toHaveAttribute("step", "1");
  });

  // j01-11 regression: numeric-string values used to make the steppers
  // silently no-op (`value` is string | number per InputHTMLAttributes).
  it("j01-11: numeric string values step too; empty/non-numeric text still no-ops (no fabricated step)", () => {
    const onChange = vi.fn();
    const { rerender } = render(<NumberField label="Delay" value="7" min={0} max={10} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("stepper-up"));
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ target: { value: "8", valueAsNumber: 8 } }),
    );
    onChange.mockClear();
    rerender(<NumberField label="Delay" value="" min={0} max={10} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("stepper-up"));
    expect(onChange).not.toHaveBeenCalled(); // no value → no fabricated step
  });

  it("error state wires aria-invalid + aria-describedby", () => {
    render(<NumberField label="Delay" value={0} error="Must be at least 1" onChange={() => {}} />);
    const input = screen.getByLabelText("Delay");
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByText("Must be at least 1")).toHaveAttribute(
      "id",
      input.getAttribute("aria-describedby") ?? "",
    );
  });

  it("disabled group: input + steppers inert", () => {
    const onChange = vi.fn();
    render(<NumberField label="Delay" value={5} disabled onChange={onChange} />);
    expect(screen.getByLabelText("Delay")).toBeDisabled();
    const stepper = screen.getByTestId("stepper-up");
    expect(stepper).toBeDisabled();
    fireEvent.click(stepper);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("DOM snapshot", () => {
    const { container } = render(<NumberField label="Snapshot" value={3} min={0} onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});
