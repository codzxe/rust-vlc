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
