import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { BrowseField } from "./BrowseField";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

describe("BrowseField (settings path combo, wave 26)", () => {
  it("wires the label to the input and typing flows through", () => {
    const onChange = vi.fn();
    render(<BrowseField label="Skin file" actionLabel="Browse" onChange={onChange} />);
    const input = screen.getByLabelText("Skin file");
    expect(input).toHaveAttribute("type", "text");
    fireEvent.change(input, { target: { value: "/skins/lumen.vlt" } });
    expect(onChange).toHaveBeenCalledOnce();
  });

  it("the action button fires onAction and carries its caption", () => {
    const onAction = vi.fn();
    render(<BrowseField label="Log file" actionLabel="Browse" onAction={onAction} />);
    fireEvent.click(screen.getByRole("button", { name: "Browse" }));
    expect(onAction).toHaveBeenCalledOnce();
  });

  it("disabled disables both the input and the action; no action fires", () => {
    const onAction = vi.fn();
    render(<BrowseField label="Log file" actionLabel="Browse" onAction={onAction} disabled />);
    expect(screen.getByLabelText("Log file")).toBeDisabled();
    const action = screen.getByRole("button", { name: "Browse" });
    expect(action).toBeDisabled();
    fireEvent.click(action);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("error state: aria-invalid + aria-describedby point at the message", () => {
    render(<BrowseField label="Skin file" actionLabel="Browse" error="Pick a valid skin" />);
    const input = screen.getByLabelText("Skin file");
    expect(input).toHaveAttribute("aria-invalid", "true");
    const describedBy = input.getAttribute("aria-describedby");
    expect(screen.getByText("Pick a valid skin")).toHaveAttribute("id", describedBy);
  });

  // j01-8: one hideLabel mechanism across every field — TextField's
  // aria-label contract (no second label text node; row-grammar hosts print
  // their own visible label and expect exactly one text node).
  it("j01-8: hideLabel uses the TextField mechanism — aria-label, no label node (aligned, not two mechanisms)", () => {
    render(<BrowseField label="Skin file" actionLabel="Browse" hideLabel />);
    const input = screen.getByRole("textbox", { name: "Skin file" });
    expect(input).toHaveAttribute("aria-label", "Skin file");
    expect(screen.queryByText("Skin file")).toBeNull(); // no label text node
  });

  it("forwards the ref to the input", () => {
    const ref = createRef<HTMLInputElement>();
    render(<BrowseField ref={ref} label="Skin file" actionLabel="Browse" />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });

  it("DOM snapshot", () => {
    const { container } = render(<BrowseField label="Skin file" actionLabel="Browse" />);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("BrowseField skin consumes the settings-frame language", () => {
  it("panel button with control radius + secondary text; token states", () => {
    expect(formCss).toMatch(
      /\.lumen-browse__action\s*\{[^}]*--color-content-secondary[^}]*--color-surface-panel[^}]*--radius-control/,
    );
    expect(formCss).toMatch(/\.lumen-browse__action:hover\s*\{[^}]*--color-surface-raised/);
    expect(formCss).toMatch(/\.lumen-browse__action:active\s*\{[^}]*--color-surface-selected/);
    expect(formCss).toMatch(/\.lumen-browse__action:focus-visible\s*\{[^}]*--color-accent/);
    expect(formCss).toMatch(/\.lumen-browse__action:disabled\s*\{[^}]*--opacity-disabled/);
  });

  it("frame geometry: 32px button, 100px min-width (Settings - interface.svg)", () => {
    expect(formCss).toMatch(/\.lumen-browse__action\s*\{[^}]*min-width: 100px;[^}]*height: 32px;/s);
  });
});
