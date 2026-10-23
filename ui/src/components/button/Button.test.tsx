import { fireEvent, render, screen } from "@testing-library/react";
// @ts-expect-error node:fs is untyped in ui/ (no @types/node; zero-new-deps rule, card 4.B1) — vitest runs on Node, so the runtime import is sound.
import { readFileSync } from "node:fs";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Button } from "./Button";

/**
 * Card 4.B1 — Button primitive tests (doc 08 §4.4 state matrix).
 * Visual-snapshot adaptation (recorded in states.md): 4.G1's screenshot
 * harness does not exist yet, and vitest stubs `.css` imports so jsdom never
 * cascades the real stylesheet. The "visual snapshot" is therefore (a) a
 * vitest DOM snapshot plus (b) computed-style-proxy assertions against the
 * CSS source mapped through semantic token variables (read via `node:fs` —
 * vitest stubs `.css` imports, including `?raw` queries).
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

describe("Button state styling consumes semantic tokens (doc 08 §3.2)", () => {
  it("rest: surface-deep + radius-inset per Button.svg", () => {
    const rule = cssRule(".lumen-button");
    expect(rule).toContain("background: var(--color-surface-deep)");
    expect(rule).toContain("border-radius: var(--radius-inset)");
    expect(rule).toContain("font-size: var(--font-size-body)");
  });

  it("hover: surface-hover token", () => {
    expect(cssRule(".lumen-button:hover")).toContain("background: var(--color-surface-hover)");
  });

  it("pressed: surface-selected token", () => {
    expect(cssRule(".lumen-button:active")).toContain("background: var(--color-surface-selected)");
  });

  it("focus-visible: 2px accent ring at 2px offset (doc 08 §2 interim)", () => {
    const rule = cssRule(".lumen-button:focus-visible");
    expect(rule).toContain("outline: 2px solid var(--color-accent)");
    expect(rule).toContain("outline-offset: 2px");
  });

  it("disabled: opacity token + pointer-events none (doc 08 §2 interim)", () => {
    const rule = cssRule(".lumen-button:disabled");
    expect(rule).toContain("opacity: var(--opacity-disabled)");
    expect(rule).toContain("pointer-events: none");
  });

  it("loading: emphasis opacity + spinner glyph via CSS", () => {
    expect(cssRule('.lumen-button[aria-busy="true"]')).toContain("opacity: var(--opacity-emphasis)");
    expect(cssRule('.lumen-button[aria-busy="true"]::before')).toContain("animation: lumen-button-spin");
    expect(buttonCss).toContain("prefers-reduced-motion"); // §4.5: motion off
  });

  it("primary variant: accent tokens per onboarding button", () => {
    expect(cssRule(".lumen-button--primary")).toContain("background: var(--color-accent)");
    expect(cssRule(".lumen-button--primary")).toContain("color: var(--color-content-on-accent)");
  });

  it("contains no raw colors (semantic tokens only)", () => {
    const code = buttonCss.replace(/\/\*[\s\S]*?\*\//g, ""); // strip provenance comments
    expect(code).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(code).not.toMatch(/\brgb(a)?\(/i);
  });
});
