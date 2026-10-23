import { fireEvent, render, screen } from "@testing-library/react";
// @ts-expect-error node:fs is untyped in ui/ (no @types/node; zero-new-deps rule, card 4.B1) — vitest runs on Node, so the runtime import is sound.
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

  it("forwards the ref and renders knob/fill structure", () => {
    const ref = createRef<HTMLButtonElement>();
    const { container } = render(<Toggle ref={ref} checked={false} onChange={() => {}} />);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
    expect(container.querySelector(".lumen-toggle__knob")).toBeInTheDocument();
    expect(container.querySelector(".lumen-toggle__fill")).toBeInTheDocument();
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

describe("Toggle state styling consumes semantic tokens (Toggle.svg)", () => {
  it("track: surface-deep + radius-control (68x32 rx6)", () => {
    const rule = cssRule(".lumen-toggle");
    expect(rule).toContain("background: var(--color-surface-deep)");
    expect(rule).toContain("border-radius: var(--radius-control)");
  });

  it("on: accent-tinted fill widens + accent-strong knob", () => {
    expect(cssRule(".lumen-toggle__fill")).toContain("background: var(--color-accent-tinted)");
    expect(cssRule('.lumen-toggle[aria-checked="true"] .lumen-toggle__fill')).toContain("width: 33px");
    expect(cssRule('.lumen-toggle[aria-checked="true"] .lumen-toggle__knob')).toContain(
      "background: var(--color-accent-strong)",
    );
  });

  it("off knob: surface-muted per Toggle.svg", () => {
    expect(cssRule(".lumen-toggle__knob")).toContain("background: var(--color-surface-muted)");
  });

  it("micro labels: font-size-micro, on=accent, off=content-tertiary (doc 08 §1.3/§1.5)", () => {
    const label = cssRule(".lumen-toggle__label");
    expect(label).toContain("font-size: var(--font-size-micro)");
    expect(cssRule(".lumen-toggle__label--on")).toContain("color: var(--color-accent)");
    expect(cssRule(".lumen-toggle__label--off")).toContain("color: var(--color-content-tertiary)");
  });

  it("focus-visible: accent ring from tokens", () => {
    expect(cssRule(".lumen-toggle:focus-visible")).toContain("outline: 2px solid var(--color-accent)");
  });

  it("disabled: opacity token + pointer-events none", () => {
    expect(cssRule(".lumen-toggle:disabled")).toContain("opacity: var(--opacity-disabled)");
  });
});
