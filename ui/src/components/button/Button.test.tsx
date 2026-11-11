import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Button } from "./Button";

/**
 * Card 4.B1 — Button primitive tests (doc 08 §4.4 state matrix; W26 Figma
 * restyle, w26.a03). Visual-snapshot adaptation (recorded in states.md):
 * 4.G1's screenshot harness does not exist yet, and vitest stubs `.css`
 * imports so jsdom never cascades the real stylesheet. The "visual snapshot"
 * is therefore (a) a vitest DOM snapshot plus (b) computed-style-proxy
 * assertions against the CSS source mapped through semantic token variables
 * (read via `node:fs` — vitest stubs `.css` imports, including `?raw`
 * queries).
 */

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const buttonCss = readFileSync(`${dirname}/button.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = buttonCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};

describe("Button", () => {
  it("renders a native button (role + default type)", () => {
    render(<Button>Save</Button>);
    const button = screen.getByRole("button", { name: "Save" });
    expect(button).toHaveAttribute("type", "button");
    expect(button.tagName).toBe("BUTTON"); // native semantics => Enter/Space activation
  });

  it("applies variant and size modifier classes, composing className", () => {
    render(
      <Button variant="primary" size="lg" className="extra">
        Go
      </Button>,
    );
    const button = screen.getByRole("button");
    expect(button).toHaveClass("lumen-button", "lumen-button--primary", "lumen-button--lg", "extra");
  });

  it("base variants (default/secondary) render without a modifier class — one secondary look", () => {
    const { rerender } = render(<Button>Cancel</Button>);
    expect(screen.getByRole("button")).toHaveClass("lumen-button");
    expect(screen.getByRole("button").className).not.toMatch(/lumen-button--/);
    rerender(<Button variant="secondary">Cancel</Button>);
    expect(screen.getByRole("button")).toHaveClass("lumen-button");
    expect(screen.getByRole("button").className).not.toMatch(/lumen-button--/);
  });

  it("fires click handlers (mouse)", () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    fireEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("is keyboard operable: native <button> + click wiring (Enter/Space activation is browser-native)", () => {
    // jsdom does not synthesize keydown->click, so we assert the contract that
    // guarantees it: a native <button> element with the handler on click.
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    const button = screen.getByRole("button");
    expect(button.tagName).toBe("BUTTON");
    expect(button).not.toHaveAttribute("tabindex", "-1");
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("disabled means disabled — no handlers fire", () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Go
      </Button>,
    );
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("loading means inert: aria-busy + disabled, no handlers fire", () => {
    const onClick = vi.fn();
    render(
      <Button loading onClick={onClick}>
        Go
      </Button>,
    );
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("forwards the ref", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Go</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
  });

  it("DOM snapshot", () => {
    const { container } = render(
      <Button variant="primary" size="sm">
        Snapshot me
      </Button>,
    );
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("Button state styling consumes semantic tokens (doc 08 §3.2; W26 frames)", () => {
  it("rest (secondary): raised fill + 1px border + control radius per dialog frames", () => {
    const rule = cssRule(".lumen-button");
    expect(rule).toContain("background: var(--color-surface-raised)");
    expect(rule).toContain("border: 1px solid var(--color-stroke-border)");
    expect(rule).toContain("border-radius: var(--radius-control)");
    expect(rule).toContain("font-size: var(--font-size-body)");
    expect(rule).toContain("font-weight: var(--font-weight-medium)");
  });

  it("compact heights per frames: 28/32/36 (sm/md/lg)", () => {
    expect(cssRule(".lumen-button--sm")).toContain("height: 28px");
    expect(cssRule(".lumen-button")).toMatch(/height: 32px/);
    expect(cssRule(".lumen-button--lg")).toContain("height: 36px");
  });

  it("hover: surface-hover token + bright label", () => {
    const rule = cssRule(".lumen-button:hover");
    expect(rule).toContain("background: var(--color-surface-hover)");
    expect(rule).toContain("color: var(--color-content-bright)");
  });

  it("pressed: surface-selected token", () => {
    expect(cssRule(".lumen-button:active")).toContain("background: var(--color-surface-selected)");
  });

  it("focus-visible: 2px accent ring at 2px offset", () => {
    const rule = cssRule(".lumen-button:focus-visible");
    expect(rule).toContain("outline: 2px solid var(--color-accent)");
    expect(rule).toContain("outline-offset: 2px");
  });

  it("disabled: tertiary content on a muted well + pointer-events none (w26.a03)", () => {
    const rule = cssRule(".lumen-button:disabled");
    expect(rule).toContain("color: var(--color-content-tertiary)");
    expect(rule).toContain("background: var(--color-surface-muted)");
    expect(rule).toContain("pointer-events: none");
  });

  it("loading: emphasis opacity + spinner glyph via CSS", () => {
    expect(cssRule('.lumen-button[aria-busy="true"]')).toContain("opacity: var(--opacity-emphasis)");
    expect(cssRule('.lumen-button[aria-busy="true"]::before')).toContain("animation: lumen-button-spin");
    expect(buttonCss).toContain("prefers-reduced-motion"); // §4.5: motion off
  });

  it("primary variant: accent fill + on-accent glyph, r6 (Onboarding.svg Next / play.svg pill)", () => {
    const rule = cssRule(".lumen-button--primary");
    expect(rule).toContain("background: var(--color-accent)");
    expect(rule).toContain("color: var(--color-content-on-accent)");
    expect(rule).toContain("border-color: transparent");
  });

  it("primary hover/pressed walk accent-bright/strong (spec §1 accent roles)", () => {
    expect(cssRule(".lumen-button--primary:hover")).toContain("background: var(--color-accent-bright)");
    expect(cssRule(".lumen-button--primary:active")).toContain("background: var(--color-accent-strong)");
  });

  it("tinted variant: dialog confirm = accent-tinted fill + accent label (Save buttons)", () => {
    const rule = cssRule(".lumen-button--tinted");
    expect(rule).toContain("background: var(--color-accent-tinted)");
    expect(rule).toContain("color: var(--color-accent)");
  });

  it("ghost variant: transparent rest + 1px transparent border, hover surface-hover", () => {
    const rule = cssRule(".lumen-button--ghost");
    expect(rule).toContain("background: transparent");
    expect(rule).toContain("border-color: transparent");
    expect(cssRule(".lumen-button--ghost:hover")).toContain("background: var(--color-surface-hover)");
  });

  it("nested .lumen-icon glyphs follow the button label token at every state (play.svg dark glyph on accent)", () => {
    // icon.css paints .lumen-icon from --color-stroke-icon, so the button must
    // remap that token in-subtree or primary pills render GRAY glyphs.
    expect(cssRule(".lumen-button")).toContain("--color-stroke-icon: var(--color-content-secondary)");
    expect(cssRule(".lumen-button:hover")).toContain("--color-stroke-icon: var(--color-content-bright)");
    expect(cssRule(".lumen-button:focus-visible")).toContain("--color-stroke-icon: var(--color-content-bright)");
    expect(cssRule(".lumen-button--primary")).toContain("--color-stroke-icon: var(--color-content-on-accent)");
    expect(buttonCss).toContain(
      ".lumen-button--primary:hover,\n.lumen-button--primary:active,\n.lumen-button--primary:focus-visible",
    );
    expect(cssRule(".lumen-button--tinted")).toContain("--color-stroke-icon: var(--color-accent)");
    expect(cssRule(".lumen-button:disabled")).toContain("--color-stroke-icon: var(--color-content-tertiary)");
  });

  it("contains no raw colors (semantic tokens only)", () => {
    const code = buttonCss.replace(/\/\*[\s\S]*?\*\//g, ""); // strip provenance comments
    expect(code).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(code).not.toMatch(/\brgb(a)?\(/i);
  });
});
