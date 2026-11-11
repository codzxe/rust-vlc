import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { IconButton } from "./IconButton";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const buttonCss = readFileSync(`${dirname}/button.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = buttonCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};

const Glyph = (): ReactElement => (
  <svg data-testid="glyph" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M8 5v14l11-7z" />
  </svg>
);

describe("IconButton", () => {
  it("exposes its label as the accessible name (doc 08 §4.5)", () => {
    render(
      <IconButton label="Play">
        <Glyph />
      </IconButton>,
    );
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
  });

  it("applies size modifiers, the icon-host protocol class and composes className", () => {
    render(
      <IconButton label="Stop" size="lg" className="extra">
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Stop" });
    expect(button).toHaveClass("lumen-icon-button", "lumen-icon-button--lg", "lumen-icon-host", "extra");
  });

  it("renders the glyph and fires clicks", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(onClick).toHaveBeenCalledOnce();
    expect(screen.getByTestId("glyph")).toBeInTheDocument();
  });

  it("disabled means disabled — no handlers fire", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" disabled onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Next" });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("loading means inert: aria-busy + disabled", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" loading onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Next" });
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("DOM snapshot", () => {
    const { container } = render(
      <IconButton label="Snapshot">
        <Glyph />
      </IconButton>,
    );
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("IconButton state styling consumes semantic tokens (Icon Wrapper.svg, W26)", () => {
  it("rest: transparent + 1px transparent border + control radius, 32px hit area", () => {
    const rule = cssRule(".lumen-icon-button");
    expect(rule).toContain("background: transparent");
    expect(rule).toContain("border: 1px solid transparent");
    expect(rule).toContain("border-radius: var(--radius-control)");
    expect(rule).toMatch(/height: 32px/);
  });

  it("hit areas stay >= 28px (sm 28 / md 32 / lg 48)", () => {
    expect(cssRule(".lumen-icon-button--sm")).toContain("height: 28px");
    expect(cssRule(".lumen-icon-button--sm")).toContain("width: 28px");
    expect(cssRule(".lumen-icon-button--lg")).toContain("width: 48px");
    expect(cssRule(".lumen-icon-button--lg")).toContain("height: 48px");
  });

  it("hover: surface-hover chip (Icon Wrapper.svg) + bright glyph", () => {
    const rule = cssRule(".lumen-icon-button:hover");
    expect(rule).toContain("background: var(--color-surface-hover)");
    expect(rule).toContain("color: var(--color-content-bright)");
  });

  it("focus-visible: 2px accent ring at 2px offset", () => {
    const rule = cssRule(".lumen-icon-button:focus-visible");
    expect(rule).toContain("outline: 2px solid var(--color-accent)");
    expect(rule).toContain("outline-offset: 2px");
  });

  it("disabled: tertiary glyph (token remap reaches .lumen-icon children) + inert", () => {
    const rule = cssRule(".lumen-icon-button:disabled");
    expect(rule).toContain("color: var(--color-content-tertiary)");
    expect(rule).toContain("--color-stroke-icon: var(--color-content-tertiary)");
    expect(rule).toContain("pointer-events: none");
  });
});
