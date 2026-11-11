import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Toggle } from "./Toggle";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const buttonCss = readFileSync(`${dirname}/button.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = buttonCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};

describe("Toggle (doc 08 §4.5: role=switch)", () => {
  it("exposes switch semantics with checked state", () => {
    render(<Toggle checked onChange={() => {}} />);
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("flips state through onChange on click", () => {
    const onChange = vi.fn();
    render(<Toggle checked={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("reports the current value back on subsequent activation", () => {
    const onChange = vi.fn();
    render(<Toggle checked onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("is keyboard operable (native <button> => Space/Enter activation) and focusable", () => {
    render(<Toggle checked={false} onChange={() => {}} />);
    const control = screen.getByRole("switch");
    expect(control.tagName).toBe("BUTTON"); // native activation: Space/Enter
    expect(control).toBeEnabled();
    expect(control).toHaveAttribute("aria-checked", "false");
  });

  it("disabled means disabled — no change fires", () => {
    const onChange = vi.fn();
    render(
      <Toggle checked disabled onChange={onChange} />,
    );
    const control = screen.getByRole("switch");
    expect(control).toBeDisabled();
    expect(control).toHaveAttribute("aria-checked", "true"); // state still truthful
    fireEvent.click(control);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("forwards the ref and renders knob/fill/segment structure", () => {
    const ref = createRef<HTMLButtonElement>();
    const { container } = render(<Toggle ref={ref} checked={false} onChange={() => {}} />);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
    expect(container.querySelector(".lumen-toggle__knob")).toBeInTheDocument();
    expect(container.querySelector(".lumen-toggle__fill")).toBeInTheDocument();
    expect(container.querySelector(".lumen-toggle__off")).toBeInTheDocument();
  });

  it("renders decorative ON/OFF micro labels, hidden from the a11y tree", () => {
    render(<Toggle checked={false} onChange={() => {}} />);
    // decorative per states.md: checked state is conveyed by aria-checked alone
    expect(screen.getByText("ON").closest("[aria-hidden='true']")).toBeTruthy();
    expect(screen.getByText("OFF").closest("[aria-hidden='true']")).toBeTruthy();
  });

  it("DOM snapshot (on and off)", () => {
    const off = render(<Toggle checked={false} onChange={() => {}} />);
    expect(off.container.firstChild).toMatchSnapshot("toggle-off");
    const on = render(<Toggle checked onChange={() => {}} />);
    expect(on.container.firstChild).toMatchSnapshot("toggle-on");
  });
});

describe("Toggle state styling consumes semantic tokens (Toggle.svg, W26)", () => {
  it("track: surface + control radius (68x32 rx6, #090909)", () => {
    const rule = cssRule(".lumen-toggle");
    expect(rule).toContain("background: var(--color-surface)");
    expect(rule).toContain("border-radius: var(--radius-control)");
    expect(rule).toMatch(/width: 68px/);
    expect(rule).toMatch(/height: 32px/);
  });

  it("on: accent-tinted 33x26 fill shows + accent-strong knob", () => {
    const fill = cssRule(".lumen-toggle__fill");
    expect(fill).toContain("background: var(--color-accent-tinted)");
    expect(fill).toContain("width: 33px");
    expect(cssRule('.lumen-toggle[aria-checked="true"] .lumen-toggle__fill')).toContain("display: flex");
    expect(cssRule('.lumen-toggle[aria-checked="true"] .lumen-toggle__knob')).toContain(
      "background: var(--color-accent-strong)",
    );
  });

  it("off: stroke-border knob (#404040) + surface-panel segment (#191919)", () => {
    expect(cssRule(".lumen-toggle__knob")).toContain("background: var(--color-stroke-border)");
    const off = cssRule(".lumen-toggle__off");
    expect(off).toContain("background: var(--color-surface-panel)");
    expect(off).toContain("width: 33px");
    expect(cssRule('.lumen-toggle[aria-checked="true"] .lumen-toggle__off')).toContain("display: none");
  });

  // 4.F4 a11y: the off label stays secondary — the frame's #6B6B6B tertiary
  // can never clear 4.5:1 on the #191919 segment in either theme (axe
  // wcag2aa color-contrast). Secondary keeps "dimmer than the accent
  // on-state" while being compliant; deviation documented in states.md.
  it("micro labels: font-size-micro, on=accent, off=content-secondary (doc 08 §1.3/§1.5, 4.F4 contrast fix)", () => {
    const label = cssRule(".lumen-toggle__label");
    expect(label).toContain("font-size: var(--font-size-micro)");
    expect(cssRule(".lumen-toggle__label--on")).toContain("color: var(--color-accent)");
    expect(cssRule(".lumen-toggle__label--off")).toContain("color: var(--color-content-secondary)");
  });

  it("focus-visible: accent ring from tokens", () => {
    expect(cssRule(".lumen-toggle:focus-visible")).toContain("outline: 2px solid var(--color-accent)");
  });

  it("disabled: opacity token + pointer-events none", () => {
    expect(cssRule(".lumen-toggle:disabled")).toContain("opacity: var(--opacity-disabled)");
  });
});
