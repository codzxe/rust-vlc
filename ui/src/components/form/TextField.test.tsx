import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { TextField } from "./TextField";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

describe("TextField", () => {
  it("wires label, input and typing", () => {
    const onChange = vi.fn();
    render(<TextField label="Network URL" onChange={onChange} />);
    const input = screen.getByLabelText("Network URL");
    expect(input).toHaveAttribute("type", "text");
    fireEvent.change(input, { target: { value: "https://example.org/stream" } });
    expect(onChange).toHaveBeenCalledOnce();
  });

  it("error state: aria-invalid + aria-describedby point at the message", () => {
    render(<TextField label="Name" error="Enter a folder name" />);
    const input = screen.getByLabelText("Name");
    expect(input).toHaveAttribute("aria-invalid", "true");
    const describedBy = input.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();
    expect(screen.getByText("Enter a folder name")).toHaveAttribute("id", describedBy);
  });

  it("no error state by default", () => {
    render(<TextField label="Name" />);
    expect(screen.getByLabelText("Name")).not.toHaveAttribute("aria-invalid");
    expect(screen.queryByText(/error/i)).toBeNull();
  });

  it("disabled means disabled — native disabled blocks user input (browser-gated)", () => {
    render(<TextField label="Name" disabled onChange={() => {}} />);
    // The disabled attribute IS the contract: browsers never deliver user
    // events to disabled controls. (fireEvent bypasses that gate, so we
    // assert the attribute rather than fake a dispatch.)
    expect(screen.getByLabelText("Name")).toBeDisabled();
  });

  it("hideLabel keeps the accessible name", () => {
    render(<TextField label="Search" hideLabel />);
    // w26.f02: hidden = aria-label on the input, no label text node in the
    // DOM (row-grammar hosts print their own visible label — one text node).
    expect(screen.getByRole("textbox", { name: "Search" })).toBeInTheDocument();
    expect(screen.queryByText("Search")).toBeNull();
  });

  it("forwards the ref + DOM snapshot", () => {
    const ref = createRef<HTMLInputElement>();
    const { container } = render(<TextField ref={ref} label="Snapshot" error="boom" />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("Form-control state matrix consumes semantic tokens (doc 08 §4.4)", () => {
  // §4.4 audit closure (states.md): rest/hover/pressed/focus-visible/disabled
  // all present on every interactive form control; pending NOT required —
  // §4.4 scopes it "where relevant (§5)" and §5's pending states belong to
  // command-dispatching components, not value-entry primitives.
  it("control: hover, pressed, focus-visible, disabled all styled via tokens", () => {
    expect(formCss).toMatch(/\.lumen-field__control:hover\s*\{[^}]*--color-surface-hover/);
    expect(formCss).toMatch(/\.lumen-field__control:active\s*\{[^}]*--color-surface-selected/);
    expect(formCss).toMatch(/\.lumen-field__control:focus-visible\s*\{[^}]*--color-accent/);
    expect(formCss).toMatch(/\.lumen-field__control:disabled\s*\{[^}]*--opacity-disabled/);
  });

  it("steppers and choice rows carry pressed too (buttons/labels are interactive)", () => {
    expect(formCss).toMatch(/\.lumen-number__stepper:active\s*\{[^}]*--color-surface-selected/);
    expect(formCss).toMatch(/\.lumen-choice:active\s*\{[^}]*--color-accent-icon/);
  });
});
