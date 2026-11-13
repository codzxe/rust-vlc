import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { keys as laneKeys } from "../../i18n/keys/w26.k01";
import { Switch } from "./Switch";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");
// w26.k01 j01-3: the Switch SKIN lives in button.css now — one selector-list
// block shared with the button family's .lumen-toggle* grammar.
const buttonCss = readFileSync(`${dirname}/../button/button.css`, "utf8");

// j01-4 regression proof (same as Toggle.test): the labels must come from
// useMsg() — the mocked catalog serves values that exist nowhere else.
vi.mock("../../i18n/messages", () => ({
  useMsg: () => (key: string): string | undefined =>
    key === "switch.on" ? "AN" : key === "switch.off" ? "AUS" : undefined,
}));

/** Switch — the form family's export of the shared switch core (w26.k01
 * j01-3: ONE core behind both Switch and button/Toggle; this skin keeps the
 * `.lumen-switch*` class grammar settings CSS hooks onto). */
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
    // Under this suite's mocked catalog the labels render "AN"/"AUS" — the
    // j01-4 test below proves the seam; the class grammar is pinned here.
    render(<Switch checked onChange={() => {}} />);
    expect(screen.getByText("AN")).toHaveClass("lumen-switch__text--on");
    expect(screen.getByText("AUS")).toHaveClass("lumen-switch__text--off");
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("j01-4: the micro labels flow through useMsg() — a localized catalog wins over the lane fallback", () => {
    render(<Switch checked onChange={() => {}} />);
    expect(screen.getByText("AN")).toBeInTheDocument();
    expect(screen.getByText("AUS")).toBeInTheDocument();
    expect(screen.queryByText(laneKeys["switch.on"])).toBeNull();
    expect(screen.queryByText(laneKeys["switch.off"])).toBeNull();
  });

  it("DOM snapshot", () => {
    const { container } = render(<Switch checked onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("Switch skin consumes the Toggle.svg language (semantic tokens)", () => {
  it("track/segments/fill map onto surface, accent-tinted, accent-strong, stroke-border (shared block in button.css)", () => {
    expect(buttonCss).toMatch(/\.lumen-toggle,\n\.lumen-switch\s*\{[^}]*--color-surface[^}]*\}/);
    expect(buttonCss).toMatch(/\.lumen-toggle__fill,\n\.lumen-switch__text--on\s*\{[^}]*--color-accent-tinted[^}]*--color-accent[^}]*\}/);
    // j01-9: the OFF label is content-secondary in BOTH grammars now (the
    // form copy had drifted to tertiary, 4.57:1 on #191919).
    expect(buttonCss).toMatch(
      /\.lumen-toggle__off,\n\.lumen-switch__text--off\s*\{[^}]*--color-surface-panel[^}]*--color-content-secondary[^}]*\}/,
    );
    expect(buttonCss).toMatch(/\.lumen-toggle__knob,\n\.lumen-switch__fill\s*\{[^}]*--color-stroke-border/);
    expect(buttonCss).toMatch(
      /\.lumen-toggle\[aria-checked="true"\] \.lumen-toggle__knob,\n\.lumen-switch\[aria-checked="true"\] \.lumen-switch__fill\s*\{[^}]*--color-accent-strong/,
    );
  });

  it("state matrix: hover, focus-visible, disabled via tokens (shared rules, both grammars)", () => {
    expect(buttonCss).toMatch(/\.lumen-toggle:hover,\n\.lumen-switch:hover\s*\{[^}]*--color-surface-hover/);
    expect(buttonCss).toMatch(/\.lumen-toggle:focus-visible,\n\.lumen-switch:focus-visible\s*\{[^}]*--color-accent/);
    expect(buttonCss).toMatch(/\.lumen-toggle:disabled,\n\.lumen-switch:disabled\s*\{[^}]*--opacity-disabled/);
    // hover walks the knob in both grammars (w26.a06's form-port hover,
    // shared since w26.k01)
    expect(buttonCss).toMatch(
      /\.lumen-switch\[aria-checked="true"\]:hover \.lumen-switch__fill\s*\{[^}]*--color-accent-bright/,
    );
  });

  it("frame geometry: 68x32 track, 33x26/26x26 segments (Toggle.svg)", () => {
    expect(buttonCss).toMatch(/\.lumen-toggle,\n\.lumen-switch\s*\{[^}]*width: 68px;[^}]*height: 32px;/s);
    expect(buttonCss).toMatch(/\.lumen-toggle__fill,\n\.lumen-toggle__off,\n\.lumen-switch__text\s*\{[^}]*width: 33px;[^}]*height: 26px;/s);
    expect(buttonCss).toMatch(/\.lumen-toggle__knob,\n\.lumen-switch__fill\s*\{[^}]*width: 26px;[^}]*height: 26px;/s);
  });

  it("j01-3: no second switch skin — form.css carries no .lumen-switch rules (one core, one stylesheet)", () => {
    expect(formCss).not.toMatch(/^\.lumen-switch[ ,{]/m);
    expect(formCss).not.toMatch(/\.lumen-switch__text--on\s*\{/);
    expect(formCss).not.toMatch(/\.lumen-switch__fill\s*\{/);
  });
});
