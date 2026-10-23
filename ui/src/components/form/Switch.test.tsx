import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Switch } from "./Switch";

/** Switch is the button-family Toggle under its ARIA name (doc 08 §4.5). */
describe("Switch (= Toggle, doc 08 §4.5)", () => {
  it("exposes switch semantics with checked state", () => {
    render(<Switch checked onChange={() => {}} />);
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("changes through onChange; disabled means disabled", () => {
    const onChange = vi.fn();
    const { rerender } = render(<Switch checked={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
    rerender(<Switch checked disabled onChange={onChange} />);
    const control = screen.getByRole("switch");
    expect(control).toBeDisabled();
    fireEvent.click(control);
    expect(onChange).not.toHaveBeenCalledWith(false);
  });

  it("DOM snapshot", () => {
    const { container } = render(<Switch checked onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});
