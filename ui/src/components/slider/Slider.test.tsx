import { fireEvent, render, screen } from "@testing-library/react";
// @types/node is a devDependency since F4/G1; directive removed.
import { readFileSync } from "node:fs";
import { createRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { Slider } from "./Slider";
import type { SliderProps } from "./Slider";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const sliderCss = readFileSync(`${dirname}/slider.css`, "utf8");

/** jsdom rects are all-zero; pin a deterministic geometry on the control. */
const pinRect = (el: Element, width: number, height: number): void => {
  vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    top: 0,
    left: 0,
    right: width,
    bottom: height,
    width,
    height,
    toJSON: () => ({}),
  } as DOMRect);
};

const pointer = (el: Element, type: "pointerDown" | "pointerMove" | "pointerUp", x: number, y = 8) =>
  fireEvent[type](el, { pointerId: 1, buttons: 1, clientX: x, clientY: y });

const progressVar = (el: Element, name: "--lumen-slider-fill" | "--lumen-slider-thumb"): string =>
  (el as HTMLElement).style.getPropertyValue(name);

/** Simulates a real caller: feeds every commit back into `value` (doc 08 §5). */
const StatefulSlider = ({ initial, ...props }: { initial: number } & Omit<SliderProps, "value">) => {
  const [value, setValue] = useState(initial);
  return (
    <Slider
      {...props}
      value={value}
      onCommit={(next) => {
        setValue(next);
        props.onCommit?.(next);
      }}
    />
  );
};

describe("Slider (doc 08 §4/§4.5: role=slider)", () => {
  it("exposes slider ARIA with value range and focusability", () => {
    render(<Slider value={42} min={0} max={100} aria-label="Gain" />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-valuemin", "0");
    expect(slider).toHaveAttribute("aria-valuemax", "100");
    expect(slider).toHaveAttribute("aria-valuenow", "42");
    expect(slider).toHaveAttribute("aria-orientation", "horizontal");
    expect(slider).toHaveAttribute("tabindex", "0");
  });

  it("formats aria-valuetext through formatValue", () => {
    render(<Slider value={30} min={0} max={100} formatValue={(v) => `${v} dB`} />);
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "30 dB");
  });

  it("scrubs: onChangeLive fires continuously, onCommit only on release (call order)", () => {
    const order: string[] = [];
    render(
      <Slider
        value={0}
        min={0}
        max={100}
        onChangeLive={(v) => order.push(`live:${v}`)}
        onCommit={(v) => order.push(`commit:${v}`)}
      />,
    );
    const slider = screen.getByRole("slider");
    pinRect(slider, 100, 16);
    pointer(slider, "pointerDown", 25);
    pointer(slider, "pointerMove", 50);
    pointer(slider, "pointerMove", 75);
    pointer(slider, "pointerUp", 75);
    expect(order).toEqual(["live:25", "live:50", "live:75", "commit:75"]);
  });

  it("a plain click (down+up, no move) commits the clicked position once", () => {
    const onCommit = vi.fn();
    const onChangeLive = vi.fn();
    render(<Slider value={0} min={0} max={100} onChangeLive={onChangeLive} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    pinRect(slider, 100, 16);
    pointer(slider, "pointerDown", 40);
    pointer(slider, "pointerUp", 40);
    expect(onChangeLive).toHaveBeenCalledTimes(1);
    expect(onChangeLive).toHaveBeenCalledWith(40);
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(40);
  });

  it("quantizes pointer positions to the step lattice", () => {
    const onChangeLive = vi.fn();
    render(<Slider value={0} min={0} max={100} step={25} onChangeLive={onChangeLive} />);
    const slider = screen.getByRole("slider");
    pinRect(slider, 100, 16);
    pointer(slider, "pointerDown", 33);
    expect(onChangeLive).toHaveBeenCalledWith(25);
    pointer(slider, "pointerUp", 33);
  });

  it("clamps positions outside the track into [min, max]", () => {
    const onChangeLive = vi.fn();
    render(<Slider value={50} min={0} max={100} onChangeLive={onChangeLive} />);
    const slider = screen.getByRole("slider");
    pinRect(slider, 100, 16);
    pointer(slider, "pointerDown", -20);
    expect(onChangeLive).toHaveBeenLastCalledWith(0);
    pointer(slider, "pointerMove", 120);
    expect(onChangeLive).toHaveBeenLastCalledWith(100);
    pointer(slider, "pointerUp", 120);
  });

  it("keyboard: arrows step, live fires before commit", () => {
    const order: string[] = [];
    render(
      <StatefulSlider
        initial={10}
        min={0}
        max={100}
        onChangeLive={(v) => order.push(`live:${v}`)}
        onCommit={(v) => order.push(`commit:${v}`)}
      />,
    );
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(order).toEqual(["live:11", "commit:11"]);
    fireEvent.keyDown(slider, { key: "ArrowLeft" });
    expect(order).toEqual(["live:11", "commit:11", "live:10", "commit:10"]);
    expect(slider).toHaveAttribute("aria-valuenow", "10"); // fed back through value
  });

  it("keyboard: Shift+arrows are coarse (×10 step)", () => {
    const onCommit = vi.fn();
    render(<StatefulSlider initial={10} min={0} max={100} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "ArrowRight", shiftKey: true });
    expect(onCommit).toHaveBeenCalledWith(20);
    fireEvent.keyDown(slider, { key: "ArrowLeft", shiftKey: true });
    expect(onCommit).toHaveBeenLastCalledWith(10);
  });

  it("keyboard: PageUp/PageDown coarse, Home/End jump to bounds", () => {
    const onCommit = vi.fn();
    render(<StatefulSlider initial={50} min={0} max={100} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "PageUp" });
    expect(onCommit).toHaveBeenLastCalledWith(60);
    fireEvent.keyDown(slider, { key: "PageDown" });
    expect(onCommit).toHaveBeenLastCalledWith(50);
    fireEvent.keyDown(slider, { key: "Home" });
    expect(onCommit).toHaveBeenLastCalledWith(0);
    fireEvent.keyDown(slider, { key: "End" });
    expect(onCommit).toHaveBeenLastCalledWith(100);
  });

  it("keyboard: unhandled keys pass through without events", () => {
    const onCommit = vi.fn();
    render(<Slider value={50} min={0} max={100} onCommit={onCommit} />);
    fireEvent.keyDown(screen.getByRole("slider"), { key: "a" });
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("disabled means disabled — aria-disabled, inert, unfocusable", () => {
    const onCommit = vi.fn();
    const onChangeLive = vi.fn();
    render(<Slider value={7} min={0} max={100} disabled onChangeLive={onChangeLive} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-disabled", "true");
    expect(slider).toHaveAttribute("tabindex", "-1");
    pinRect(slider, 100, 16);
    pointer(slider, "pointerDown", 50);
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(onChangeLive).not.toHaveBeenCalled();
    expect(onCommit).not.toHaveBeenCalled();
    expect(slider).toHaveAttribute("aria-valuenow", "7"); // state stays truthful
  });

  it("forwards the ref and renders track/fill/thumb structure", () => {
    const ref = createRef<HTMLDivElement>();
    const { container } = render(<Slider ref={ref} value={25} min={0} max={100} />);
    expect(ref.current).toBeInstanceOf(HTMLDivElement);
    expect(container.querySelector(".lumen-slider__track")).toBeInTheDocument();
    const fill = container.querySelector(".lumen-slider__fill");
    const thumb = container.querySelector(".lumen-slider__thumb");
    expect(fill).toBeInTheDocument();
    expect(thumb).toBeInTheDocument();
    expect(progressVar(ref.current!, "--lumen-slider-fill")).toBe("25");
    expect(progressVar(ref.current!, "--lumen-slider-thumb")).toBe("25");
  });

  it("vertical orientation maps bottom→min, top→max, ArrowUp increases", () => {
    const onChangeLive = vi.fn();
    const onCommit = vi.fn();
    render(
      <StatefulSlider
        initial={0}
        min={0}
        max={100}
        orientation="vertical"
        onChangeLive={onChangeLive}
        onCommit={onCommit}
      />,
    );
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-orientation", "vertical");
    pinRect(slider, 16, 100);
    pointer(slider, "pointerDown", 8, 90); // near the bottom -> small value
    expect(onChangeLive).toHaveBeenLastCalledWith(10);
    pointer(slider, "pointerUp", 8, 90);
    fireEvent.keyDown(slider, { key: "ArrowUp" });
    expect(onCommit).toHaveBeenCalledWith(11);
  });

  it("css: semantic tokens only (no raw colors) and structural selectors present", () => {
    const withoutComments = sliderCss.replace(/\/\*[\s\S]*?\*\//g, "");
    expect(withoutComments).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    expect(withoutComments).not.toMatch(/rgba?\(/i);
    for (const selector of [
      ".lumen-slider__fill",
      ".lumen-slider__thumb",
      ".lumen-slider--vertical",
      ".lumen-seekbar__buffer",
      '[data-pending="true"]',
      ".lumen-volume",
      ".lumen-eq-slider",
      ".lumen-seekbar[data-empty]", // calm unplayed track guard (caller hook)
    ]) {
      expect(withoutComments).toContain(selector);
    }
  });
});

/* ------------------------------------------------------------------------- */
/* w26.k03 (j03 P1-1/P1-2/P2-8) — L8 flush geometry pinned by PIXEL MATH.     */
/*                                                                            */
/* The g16-P2-3 thumb formula shipped without its fill counterpart on the base */
/* scope (P1-1) and never vertically (P1-2); both survived three fix waves     */
/* because nothing evaluated the formulas. These tests parse the CSS source    */
/* (B1 pattern) and COMPUTE the geometry at real track sizes, asserting the   */
/* flush identity at every probed p — a reverted formula fails the math, not   */
/* just a string compare.                                                     */
/* ------------------------------------------------------------------------- */

const sliderCssCode = sliderCss.replace(/\/\*[\s\S]*?\*\//g, "");

/** One rule body for `selector` (comment-stripped source, first match). */
const cssBody = (selector: string): string =>
  sliderCssCode.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s+")}\\s*\\{([^}]*)\\}`))?.[1] ?? "";

/** One declaration value out of a rule body. */
const decl = (selector: string, prop: string): string =>
  cssBody(selector).match(new RegExp(`(?:^|;)\\s*${prop}\\s*:\\s*([^;]+);`))?.[1]?.trim() ?? "";

/** The px number of an `Npx` declaration. */
const px = (selector: string, prop: string): number => {
  const value = decl(selector, prop);
  const m = value.match(/^(-?\d+(?:\.\d+)?)px$/);
  expect(m, `${selector} { ${prop}: ${value} } is not a plain px length`).not.toBeNull();
  return Number(m![1]);
};

/**
 * The L8 travel grammar `calc((100% - Npx) * var(--lumen-slider-*, 0) / 100 [- Mpx])`
 * parsed into its exact pixel constants: the travel extent is `(track - N)`,
 * the position offset is `post` (negative = inset-shifted start).
 */
const travel = (selector: string, prop: string): { extentLess: number; post: number } => {
  const value = decl(selector, prop);
  const m = value.match(/^calc\(\(100% - (\d+)px\) \* var\(--lumen-slider-(?:fill|thumb), 0\) \/ 100(?: - (\d+)px)?\)$/);
  expect(m, `${selector} { ${prop}: ${value} } is not the L8 travel grammar`).not.toBeNull();
  return { extentLess: Number(m![1]), post: m![2] === undefined ? 0 : -Number(m![2]) };
};

/** The bottom component of a vertical `inset: auto [sides]px [bottom]px`
 * (a bare `0` is valid unitless CSS for the sides slot). */
const insetBottom = (selector: string): number => {
  const value = decl(selector, "inset");
  const m = value.match(/^auto\s+(?:(\d+)(?:px)?\s+)?(\d+)px$/);
  expect(m, `${selector} { inset: ${value} } is not the vertical anchor form`).not.toBeNull();
  return Number(m![2]);
};

describe("L8 flush geometry — pixel math over the CSS source (w26.k03, j03 P1-1)", () => {
  it("base horizontal: fill inset-2 + L8 thumb are flush at every p (settings §C0 track)", () => {
    const FILL = ".lumen-slider__fill";
    const THUMB = ".lumen-slider__thumb";
    const fillInset = px(FILL, "left");
    const fill = travel(FILL, "width");
    const thumb = travel(THUMB, "left");
    const thumbWidth = px(THUMB, "width");

    // The anatomy the review demanded: inset-2 fill, 8px handle, L8 offsets.
    expect(fillInset).toBe(2);
    expect(fill.extentLess).toBe(4);
    expect(thumb.extentLess).toBe(4);
    expect(thumbWidth).toBe(8);

    // Live-measured pre-fix on /settings/video (j03 §Dimension 2): thumb
    // right edge = fill end + 2px at p=0, −2px at p=100 (err = 2 − 4p/100),
    // thumb left edge 6px outside the track at Home. The inset-2 fill makes
    // both edges the same number at EVERY p:
    const W = 320; // settings §C0 "Slider" track width
    for (const p of [0, 25, 50, 75, 100]) {
      const fillEnd = fillInset + (W - fill.extentLess) * (p / 100);
      const thumbRight = thumb.post + (W - thumb.extentLess) * (p / 100) + thumbWidth;
      expect(thumbRight, `p=${p}`).toBe(fillEnd);
    }
    // p=0: the handle's left edge sits 6px outside the well — the frame
    // anatomy (EQ.svg handles overhang; the hit area covers it).
    expect(thumb.post).toBe(-6);
    // p=100: both edges land exactly 2px inside the track's right end.
    expect(2 + (W - fill.extentLess)).toBe(W - 2);
  });

  it("base vertical: fill bottom-inset-2 + L8 mirror thumb are flush at every p (j03 P1-2)", () => {
    const FILL = ".lumen-slider--vertical .lumen-slider__fill";
    const THUMB = ".lumen-slider--vertical .lumen-slider__thumb";
    const fillBottom = insetBottom(FILL);
    const fill = travel(FILL, "height");
    const thumb = travel(THUMB, "bottom");
    const thumbHeight = px(THUMB, "height");

    expect(fillBottom).toBe(2);
    expect(fill.extentLess).toBe(4);
    expect(thumb.extentLess).toBe(4);
    expect(thumbHeight).toBe(8);

    // Vertical mirror of the flush contract: the handle's TOP edge sits on
    // the fill's TOP edge at every p. (The pre-fix `(100% - 8px)·p` travel
    // overhung the fill top by 6px → 2px across the range.)
    const H = 160; // the structural default extent
    for (const p of [0, 20, 50, 80, 100]) {
      const fillTop = H - fillBottom - (H - fill.extentLess) * (p / 100);
      const thumbTop = H - (thumb.post + (H - thumb.extentLess) * (p / 100)) - thumbHeight;
      // sub-pixel tolerance (1e-9 px): p=20/80 are not binary-exact fractions
      // — a real regression is ≥0.5px, never nanopixel noise.
      expect(thumbTop, `p=${p}`).toBeCloseTo(fillTop, 9);
    }
  });

  it("base vertical: the stale `(100% - 8px)·p` grammar is gone from BOTH vertical thumbs", () => {
    // String-level pin (belt and braces): the exact grammar g16-P2-3 killed.
    expect(sliderCssCode).not.toContain("calc((100% - 8px)");
  });
});
