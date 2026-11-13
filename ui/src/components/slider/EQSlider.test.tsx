import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { EQSlider } from "./EQSlider";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const eqCss = readFileSync(`${dirname}/slider.css`, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

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

describe("EQSlider (w26.a04 additive: EQ.png pill slider)", () => {
  it("exposes horizontal slider ARIA over the band range", () => {
    render(<EQSlider value={3} min={-12} max={12} step={1} aria-label="60 Hz" />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-orientation", "horizontal");
    expect(slider).toHaveAttribute("aria-valuemin", "-12");
    expect(slider).toHaveAttribute("aria-valuemax", "12");
    expect(slider).toHaveAttribute("aria-valuenow", "3");
    expect(slider).toHaveAttribute("tabindex", "0");
  });

  it("formats aria-valuetext through formatValue", () => {
    render(<EQSlider value={4.5} min={-20} max={20} formatValue={(v) => `${v} dB`} />);
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "4.5 dB");
  });

  it("renders the pill anatomy (lumen-eq-slider + track/fill/thumb), chip stays outside", () => {
    const { container } = render(<EQSlider value={50} min={0} max={100} />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveClass("lumen-eq-slider");
    expect(container.querySelector(".lumen-slider__track")).toBeInTheDocument();
    expect(container.querySelector(".lumen-slider__fill")).toBeInTheDocument();
    expect(container.querySelector(".lumen-slider__thumb")).toBeInTheDocument();
    // value chip is a CONSUMER concern — the slider renders no text of its own
    expect(slider.textContent).toBe("");
    expect((slider as HTMLElement).style.getPropertyValue("--lumen-slider-fill")).toBe("50");
  });

  it("scrubs live and commits once on release (doc 08 §5 call order)", () => {
    const order: string[] = [];
    render(
      <EQSlider
        value={0}
        min={0}
        max={20}
        onChangeLive={(v) => order.push(`live:${v}`)}
        onCommit={(v) => order.push(`commit:${v}`)}
      />,
    );
    const slider = screen.getByRole("slider");
    pinRect(slider, 200, 24);
    fireEvent.pointerDown(slider, { pointerId: 1, buttons: 1, clientX: 50, clientY: 12 });
    fireEvent.pointerMove(slider, { pointerId: 1, buttons: 1, clientX: 150, clientY: 12 });
    expect(slider).toHaveAttribute("data-dragging", "true");
    fireEvent.pointerUp(slider, { pointerId: 1, buttons: 1, clientX: 150, clientY: 12 });
    expect(order).toEqual(["live:5", "live:15", "commit:15"]);
    expect(slider).not.toHaveAttribute("data-dragging"); // released → engine truth
  });

  it("keyboard: arrows step, Shift+arrows coarse (×10), Home/End bounds", () => {
    const onCommit = vi.fn();
    render(<EQSlider value={0} min={-12} max={12} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(onCommit).toHaveBeenLastCalledWith(1);
    fireEvent.keyDown(slider, { key: "ArrowRight", shiftKey: true });
    expect(onCommit).toHaveBeenLastCalledWith(10);
    fireEvent.keyDown(slider, { key: "Home" });
    expect(onCommit).toHaveBeenLastCalledWith(-12);
    fireEvent.keyDown(slider, { key: "End" });
    expect(onCommit).toHaveBeenLastCalledWith(12);
  });

  it("disabled means disabled — aria-disabled, inert, unfocusable", () => {
    const onCommit = vi.fn();
    render(<EQSlider value={3} min={-12} max={12} disabled onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-disabled", "true");
    expect(slider).toHaveAttribute("tabindex", "-1");
    pinRect(slider, 200, 24);
    fireEvent.pointerDown(slider, { pointerId: 1, buttons: 1, clientX: 100, clientY: 12 });
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(onCommit).not.toHaveBeenCalled();
    expect(slider).toHaveAttribute("aria-valuenow", "3");
  });

  it("w26.f05: orientation=\"vertical\" renders the A2 fader axis (EQ.svg)", () => {
    render(<EQSlider orientation="vertical" value={0} min={-20} max={20} aria-label="60 Hz" />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-orientation", "vertical");
    expect(slider).toHaveAttribute("aria-valuenow", "0");
    expect(slider).toHaveClass("lumen-slider--vertical");
    expect(slider).toHaveClass("lumen-eq-slider");
  });

  it("w26.f05: vertical faders scrub on the Y axis (drag up = up in value)", () => {
    const onCommit = vi.fn();
    render(
      <EQSlider
        orientation="vertical"
        value={0}
        min={-20}
        max={20}
        onCommit={onCommit}
        aria-label="60 Hz"
      />,
    );
    const slider = screen.getByRole("slider");
    pinRect(slider, 68, 328);
    // Vertical hit geometry: y=300 of 328 → ≈8.5% from the bottom → ≈ -16.6
    // snapped onto the 1 dB lattice → -17; y=28 → ≈91.5% → ≈ +16.6 → +17.
    fireEvent.pointerDown(slider, { pointerId: 1, buttons: 1, clientX: 34, clientY: 300 });
    fireEvent.pointerUp(slider, { pointerId: 1, buttons: 1, clientX: 34, clientY: 300 });
    expect(onCommit).toHaveBeenLastCalledWith(-17);
    fireEvent.pointerDown(slider, { pointerId: 2, buttons: 1, clientX: 34, clientY: 28 });
    fireEvent.pointerUp(slider, { pointerId: 2, buttons: 1, clientX: 34, clientY: 28 });
    expect(onCommit).toHaveBeenLastCalledWith(17);
  });

  /* w26.k03 (j03 P1-2/P2-8): the vertical fader never got the L8 fix —
   * live-measured overhang 2.8px @p=80, 4px @p=50 (6px → 2px across the
   * range). Pixel math over the CSS source pins the flush identity. */
  it("w26.k03 (j03 P1-2): EQ vertical fader fill/handle are flush at every p (pixel math, EQ.svg 320px track)", () => {
    const rule = (selector: string): string =>
      eqCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s+")}\\s*\\{([^}]*)\\}`))?.[1] ?? "";
    const declOf = (selector: string, prop: string): string =>
      rule(selector).match(new RegExp(`(?:^|;)\\s*${prop}\\s*:\\s*([^;]+);`))?.[1]?.trim() ?? "";
    const parseTravel = (value: string): { extentLess: number; post: number } => {
      const m = value.match(/^calc\(\(100% - (\d+)px\) \* var\(--lumen-slider-(?:fill|thumb), 0\) \/ 100(?: - (\d+)px)?\)$/);
      expect(m, `not the L8 travel grammar: ${value}`).not.toBeNull();
      return { extentLess: Number(m![1]), post: m![2] === undefined ? 0 : -Number(m![2]) };
    };

    const FILL = ".lumen-eq-slider.lumen-slider--vertical .lumen-slider__fill";
    const THUMB = ".lumen-eq-slider.lumen-slider--vertical .lumen-slider__thumb";
    const inset = declOf(FILL, "inset"); // "auto 2px 2px" → sides 2, bottom 2
    const bottom = Number(inset.match(/^auto\s+(?:(\d+)px\s+)?(\d+)px$/)?.[2]);
    expect(bottom).toBe(2); // frame: fill inset 2 from the bottom
    const fill = parseTravel(declOf(FILL, "height"));
    const thumb = parseTravel(declOf(THUMB, "bottom"));
    const thumbHeight = Number(declOf(THUMB, "height").replace("px", ""));
    expect(thumbHeight).toBe(8); // frame: 24×8 handle

    const H = 320; // EQ.svg fader track height
    for (const p of [0, 20, 50, 80, 100]) {
      const fillTop = H - bottom - (H - fill.extentLess) * (p / 100);
      const thumbTop = H - (thumb.post + (H - thumb.extentLess) * (p / 100)) - thumbHeight;
      // sub-pixel tolerance (1e-9 px) for the non-binary fractions 20/80 —
      // the live-measured regression this pins was 2.8px, not nanopixels.
      expect(thumbTop, `p=${p}`).toBeCloseTo(fillTop, 9);
    }
  });

  it("w26.k03 (j03 P1-1 belt-and-braces): horizontal EQ scope keeps the inset-2 fill + L8 thumb pair", () => {
    const rule = (selector: string): string =>
      eqCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s+")}\\s*\\{([^}]*)\\}`))?.[1] ?? "";
    const declOf = (selector: string, prop: string): string =>
      rule(selector).match(new RegExp(`(?:^|;)\\s*${prop}\\s*:\\s*([^;]+);`))?.[1]?.trim() ?? "";
    const FILL = ".lumen-eq-slider .lumen-slider__fill";
    const THUMB = ".lumen-eq-slider .lumen-slider__thumb";
    // fill: left 2px + (100% - 4px)·p width; thumb: L8 left. Same travel
    // extent ⇒ flush at every p (the flush identity itself is pinned in
    // Slider.test for the base scope with the full pixel-math loop).
    expect(declOf(FILL, "left")).toBe("2px");
    expect(declOf(FILL, "width")).toBe("calc((100% - 4px) * var(--lumen-slider-fill, 0) / 100)");
    expect(declOf(THUMB, "left")).toBe("calc((100% - 4px) * var(--lumen-slider-thumb, 0) / 100 - 6px)");
  });
});
