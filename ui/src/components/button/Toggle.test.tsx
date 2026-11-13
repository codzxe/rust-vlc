import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { keys as laneKeys } from "../../i18n/keys/w26.k01";
import { Toggle } from "./Toggle";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const buttonCss = readFileSync(`${dirname}/button.css`, "utf8");

// j01-4 regression proof: the ON/OFF micro labels must resolve through
// useMsg() (i18n seam), not through JSX literals. A hardcoded "ON" fails
// here — the mocked catalog serves "AN"/"AUS" for the switch keys (values
// that exist NOWHERE else, so a bypassed seam cannot fake a pass).
vi.mock("../../i18n/messages", () => ({
  useMsg: () => (key: string): string | undefined =>
    key === "switch.on" ? "AN" : key === "switch.off" ? "AUS" : undefined,
}));

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
    // decorative per states.md: checked state is conveyed by aria-checked
    // alone. (Labels render as the mocked catalog's "AN"/"AUS" — see the
    // j01-4 test below; the wrapping contract is what's pinned here.)
    expect(screen.getByText("AN").closest("[aria-hidden='true']")).toBeTruthy();
    expect(screen.getByText("AUS").closest("[aria-hidden='true']")).toBeTruthy();
  });

  it("j01-4: the micro labels flow through useMsg() — a localized catalog wins over the lane fallback", () => {
    render(<Toggle checked={false} onChange={() => {}} />);
    // "AN"/"AUS" exist only in the mocked catalog — literals or a bypassed
    // seam would still render "ON"/"OFF" and fail here.
    expect(screen.getByText("AN")).toBeInTheDocument();
    expect(screen.getByText("AUS")).toBeInTheDocument();
    expect(screen.queryByText(laneKeys["switch.on"])).toBeNull();
  });

  it("DOM snapshot (on and off)", () => {
    const off = render(<Toggle checked={false} onChange={() => {}} />);
    expect(off.container.firstChild).toMatchSnapshot("toggle-off");
    const on = render(<Toggle checked onChange={() => {}} />);
    expect(on.container.firstChild).toMatchSnapshot("toggle-on");
  });
});

describe("Toggle state styling consumes semantic tokens (Toggle.svg, W26)", () => {
  /** Body of the first rule whose selector list matches `pattern` (the
   * shared switch skin ships selector lists — one rule, both class
   * grammars — so plain first-occurrence lookups are ambiguous). */
  const ruleOf = (pattern: RegExp): string => buttonCss.match(pattern)?.[1] ?? "";

  it("track: surface + control radius (68x32 rx6, #090909)", () => {
    const rule = ruleOf(/\.lumen-toggle,\n\.lumen-switch\s*\{([^}]*)\}/);
    expect(rule).toContain("background: var(--color-surface)");
    expect(rule).toContain("border-radius: var(--radius-control)");
    expect(rule).toMatch(/width: 68px/);
    expect(rule).toMatch(/height: 32px/);
  });

  it("segment base: 33x26 rx3 micro-label wells, caption type (frame ~12px)", () => {
    const base = ruleOf(/\.lumen-toggle__fill,\n\.lumen-toggle__off,\n\.lumen-switch__text\s*\{([^}]*)\}/);
    expect(base).toContain("width: 33px");
    expect(base).toContain("height: 26px");
    expect(base).toContain("font-size: var(--font-size-caption)");
    expect(base).toContain("font-weight: var(--font-weight-medium)");
  });

  it("on: accent-tinted fill + accent label, shown when checked + accent-strong knob", () => {
    const on = ruleOf(/\.lumen-toggle__fill,\n\.lumen-switch__text--on\s*\{([^}]*)\}/);
    expect(on).toContain("display: none"); // hidden while off (frame)
    expect(on).toContain("background: var(--color-accent-tinted)");
    expect(on).toContain("color: var(--color-accent)");
    expect(ruleOf(/\.lumen-toggle\[aria-checked="true"\] \.lumen-toggle__fill,\n\.lumen-switch\[aria-checked="true"\] \.lumen-switch__text--on\s*\{([^}]*)\}/)).toContain(
      "display: flex",
    );
    expect(
      ruleOf(/\.lumen-toggle\[aria-checked="true"\] \.lumen-toggle__knob,\n\.lumen-switch\[aria-checked="true"\] \.lumen-switch__fill\s*\{([^}]*)\}/),
    ).toContain("background: var(--color-accent-strong)");
  });

  it("off: stroke-border knob (#404040) + surface-panel segment (#191919), hidden when checked", () => {
    expect(ruleOf(/\.lumen-toggle__knob,\n\.lumen-switch__fill\s*\{([^}]*)\}/)).toContain(
      "background: var(--color-stroke-border)",
    );
    const off = ruleOf(/\.lumen-toggle__off,\n\.lumen-switch__text--off\s*\{([^}]*)\}/);
    expect(off).toContain("background: var(--color-surface-panel)");
    expect(
      ruleOf(/\.lumen-toggle\[aria-checked="true"\] \.lumen-toggle__off,\n\.lumen-switch\[aria-checked="true"\] \.lumen-switch__text--off\s*\{([^}]*)\}/),
    ).toContain("display: none");
  });

  // 4.F4 a11y: the off label stays secondary — the frame's #6B6B6B tertiary
  // can never clear 4.5:1 on the #191919 segment in either theme (axe
  // wcag2aa color-contrast). Secondary keeps "dimmer than the accent
  // on-state" while being compliant; deviation documented in states.md.
  // Label size: Toggle.svg ON/OFF cap-height 9.0px => ~12px type; caption
  // (11px) is the nearest semantic step (w26.a03 relaunch measurement).
  it("micro labels: caption size (frame ~12px), on=accent, off=content-secondary (doc 08 §1.3/§1.5, 4.F4 contrast fix)", () => {
    expect(ruleOf(/\.lumen-toggle__fill,\n\.lumen-toggle__off,\n\.lumen-switch__text\s*\{([^}]*)\}/)).toContain(
      "font-size: var(--font-size-caption)",
    );
    expect(ruleOf(/\.lumen-toggle__fill,\n\.lumen-switch__text--on\s*\{([^}]*)\}/)).toContain(
      "color: var(--color-accent)",
    );
    expect(ruleOf(/\.lumen-toggle__off,\n\.lumen-switch__text--off\s*\{([^}]*)\}/)).toContain(
      "color: var(--color-content-secondary)",
    );
  });

  it("focus-visible: accent ring from tokens", () => {
    expect(ruleOf(/\.lumen-toggle:focus-visible,\n\.lumen-switch:focus-visible\s*\{([^}]*)\}/)).toContain(
      "outline: 2px solid var(--color-accent)",
    );
  });

  it("disabled: opacity token + pointer-events none", () => {
    expect(ruleOf(/\.lumen-toggle:disabled,\n\.lumen-switch:disabled\s*\{([^}]*)\}/)).toContain(
      "opacity: var(--opacity-disabled)",
    );
  });

  // j01-3: ONE shared skin behind both exported names — every rule covers
  // the `.lumen-toggle*` and `.lumen-switch*` grammars in the same selector
  // list, so the two can never drift apart again (they had: j01-9).
  it("j01-3: the switch skin is ONE selector-list block covering both class grammars (shared core)", () => {
    expect(buttonCss).toMatch(/\.lumen-toggle,\n\.lumen-switch \{/);
    expect(buttonCss).toMatch(/\.lumen-toggle__knob,\n\.lumen-switch__fill \{/);
    expect(buttonCss).toMatch(/\.lumen-toggle__off,\n\.lumen-switch__text--off \{/);
    // j01-9: the shared OFF label is secondary in BOTH grammars (the form
    // copy had drifted to tertiary 4.57:1).
    const offRule = ruleOf(/\.lumen-toggle__off,\n\.lumen-switch__text--off\s*\{([^}]*)\}/);
    expect(offRule).toContain("color: var(--color-content-secondary)");
    expect(offRule).not.toContain("--color-content-tertiary");
  });
});
