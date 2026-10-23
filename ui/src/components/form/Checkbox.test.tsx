import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Checkbox } from "./Checkbox";

describe("Checkbox", () => {
  it("renders a labelled checkbox that toggles", () => {
    const onChange = vi.fn();
    render(<Checkbox label="Show thumbnail" onChange={onChange} />);
    const box = screen.getByLabelText("Show thumbnail") as HTMLInputElement;
    expect(box.type).toBe("checkbox");
    expect(box).not.toBeChecked();
    fireEvent.click(box);
    expect(onChange).toHaveBeenCalledOnce();
  });

  it("checked state reflects the controlled prop", () => {
    render(<Checkbox label="Favorite" checked readOnly onChange={() => {}} />);
    expect(screen.getByLabelText("Favorite")).toBeChecked();
  });

  it("invalid flag sets aria-invalid", () => {
    render(<Checkbox label="Accept" invalid />);
    expect(screen.getByLabelText("Accept")).toHaveAttribute("aria-invalid", "true");
  });

  it("disabled means disabled — native attribute (browser-gated)", () => {
    render(<Checkbox label="Accept" disabled onChange={() => {}} />);
    // fireEvent bypasses the browser's disabled gate, so assert the contract.
    expect(screen.getByLabelText("Accept")).toBeDisabled();
  });

  it("forwards the ref + DOM snapshot", () => {
    const ref = createRef<HTMLInputElement>();
    const { container } = render(<Checkbox ref={ref} label="Snapshot" checked onChange={() => {}} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
    expect(container.firstChild).toMatchSnapshot();
  });
});
