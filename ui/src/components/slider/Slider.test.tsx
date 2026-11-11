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
