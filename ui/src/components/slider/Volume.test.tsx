import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { VolumeSlider } from "./Volume";
import type { VolumeSliderProps } from "./Volume";

/** Simulates a real caller: feeds every commit back into `value` (doc 08 §5). */
const StatefulVolume = ({ initial, ...props }: { initial: number } & Omit<VolumeSliderProps, "value">) => {
  const [value, setValue] = useState(initial);
  return (
    <VolumeSlider
      {...props}
      value={value}
      onCommit={(next) => {
        setValue(next);
        props.onCommit?.(next);
      }}
    />
  );
};

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

describe("VolumeSlider (w26.a04: volume.svg — compact horizontal, level fill)", () => {
  it("exposes horizontal slider ARIA over the 0…1 volume range with percent text", () => {
    render(<VolumeSlider value={0.5} aria-label="Volume" />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-orientation", "horizontal");
    expect(slider).toHaveAttribute("aria-valuemin", "0");
    expect(slider).toHaveAttribute("aria-valuemax", "1");
    expect(slider).toHaveAttribute("aria-valuenow", "0.5");
    expect(slider).toHaveAttribute("aria-valuetext", "50%");
    expect(slider).toHaveAttribute("aria-label", "Volume");
  });

  it("keyboard: ArrowUp/Right raise volume in 0.01 steps, live before commit", () => {
    const order: string[] = [];
    render(
      <StatefulVolume
        initial={0.5}
        onChangeLive={(v) => order.push(`live:${v}`)}
        onCommit={(v) => order.push(`commit:${v}`)}
      />,
    );
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "ArrowUp" });
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(order).toEqual(["live:0.51", "commit:0.51", "live:0.52", "commit:0.52"]);
    fireEvent.keyDown(slider, { key: "ArrowDown" });
    expect(order).toEqual(["live:0.51", "commit:0.51", "live:0.52", "commit:0.52", "live:0.51", "commit:0.51"]);
  });

  it("keyboard: Home mutes (0), End is full (1), clamped either way", () => {
    const onCommit = vi.fn();
    render(<VolumeSlider value={0.5} onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    fireEvent.keyDown(slider, { key: "Home" });
    expect(onCommit).toHaveBeenLastCalledWith(0);
    fireEvent.keyDown(slider, { key: "End" });
    expect(onCommit).toHaveBeenLastCalledWith(1);
  });

  it("pointer scrub maps left→0, right→1 (horizontal level axis)", () => {
    const onChangeLive = vi.fn();
    render(<VolumeSlider value={0.5} onChangeLive={onChangeLive} />);
    const slider = screen.getByRole("slider");
    pinRect(slider, 100, 20);
    fireEvent.pointerDown(slider, { pointerId: 1, buttons: 1, clientX: 10, clientY: 10 });
    expect(onChangeLive).toHaveBeenLastCalledWith(0.1);
    fireEvent.pointerMove(slider, { pointerId: 1, buttons: 1, clientX: 90, clientY: 10 });
    expect(onChangeLive).toHaveBeenLastCalledWith(0.9);
    fireEvent.pointerUp(slider, { pointerId: 1, buttons: 1, clientX: 90, clientY: 10 });
  });

  it("renders the compact horizontal anatomy (lumen-volume + circular thumb)", () => {
    const { container } = render(<VolumeSlider value={0.25} />);
    expect(container.querySelector(".lumen-volume")).toBeInTheDocument();
    expect(container.querySelector(".lumen-volume.lumen-slider--vertical")).not.toBeInTheDocument();
    const thumb = container.querySelector(".lumen-slider__thumb");
    expect(thumb).toBeInTheDocument();
    expect((screen.getByRole("slider") as HTMLElement).style.getPropertyValue("--lumen-slider-fill")).toBe("25");
  });

  it("disabled stays truthful and inert", () => {
    const onCommit = vi.fn();
    render(<VolumeSlider value={0.3} disabled onCommit={onCommit} />);
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-disabled", "true");
    expect(slider).toHaveAttribute("aria-valuenow", "0.3");
    fireEvent.keyDown(slider, { key: "ArrowUp" });
    expect(onCommit).not.toHaveBeenCalled();
  });
});
