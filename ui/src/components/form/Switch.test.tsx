import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { Switch } from "./Switch";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

/** Switch — the form family's Toggle.svg port (wave 26; see form.css). */
describe("Switch (Toggle.svg, doc 08 §4.5)", () => {
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

  it("carries both decorative ON/OFF micro labels; state rides on aria-checked", () => {
    // Both labels stay in the DOM (CSS shows exactly one per state — jsdom
    // does not load the stylesheet, so the DOM contract is what we pin).
    render(<Switch checked onChange={() => {}} />);
    expect(screen.getByText("ON")).toHaveClass("lumen-switch__text--on");
    expect(screen.getByText("OFF")).toHaveClass("lumen-switch__text--off");
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("DOM snapshot", () => {
    const { container } = render(<Switch checked onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("Switch skin consumes the Toggle.svg language (semantic tokens)", () => {
  it("track/segments/fill map onto surface, accent-tinted, accent-strong, stroke-border", () => {
    expect(formCss).toMatch(/\.lumen-switch\s*\{[^}]*--color-surface[^}]*\}/);
    expect(formCss).toMatch(/\.lumen-switch__text--on\s*\{[^}]*--color-accent-tinted[^}]*--color-accent[^}]*\}/);
    expect(formCss).toMatch(/\.lumen-switch__text--off\s*\{[^}]*--color-surface-panel[^}]*--color-content-tertiary[^}]*\}/);
    expect(formCss).toMatch(/\.lumen-switch__fill\s*\{[^}]*--color-stroke-border/);
    expect(formCss).toMatch(/\.lumen-switch\[aria-checked="true"\] \.lumen-switch__fill\s*\{[^}]*--color-accent-strong/);
  });

  it("state matrix: hover, focus-visible, disabled via tokens", () => {
    expect(formCss).toMatch(/\.lumen-switch:hover\s*\{[^}]*--color-surface-hover/);
    expect(formCss).toMatch(/\.lumen-switch:focus-visible\s*\{[^}]*--color-accent/);
    expect(formCss).toMatch(/\.lumen-switch:disabled\s*\{[^}]*--opacity-disabled/);
  });

  it("frame geometry: 68x32 track, 33x26/26x26 segments (Toggle.svg)", () => {
    expect(formCss).toMatch(/\.lumen-switch\s*\{[^}]*width: 68px;[^}]*height: 32px;/s);
    expect(formCss).toMatch(/\.lumen-switch__text\s*\{[^}]*width: 33px;[^}]*height: 26px;/s);
    expect(formCss).toMatch(/\.lumen-switch__fill\s*\{[^}]*width: 26px;[^}]*height: 26px;/s);
  });
});
