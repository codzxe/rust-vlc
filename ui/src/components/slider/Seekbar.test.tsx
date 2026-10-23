import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Seekbar, formatSeekTime } from "./Seekbar";
import type { SeekbarBufferRange } from "./Seekbar";

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

const pointer = (el: Element, type: "pointerDown" | "pointerMove" | "pointerUp", x: number, y = 14) =>
  fireEvent[type](el, { pointerId: 1, buttons: 1, clientX: x, clientY: y });

const vars = (el: Element) => ({
  fill: (el as HTMLElement).style.getPropertyValue("--lumen-slider-fill"),
  thumb: (el as HTMLElement).style.getPropertyValue("--lumen-slider-thumb"),
});

describe("Seekbar (doc 08 §5.2 truthful seek)", () => {
  it("exposes slider ARIA with time-formatted valuetext", () => {
    render(<Seekbar value={65} duration={600} />);
    const bar = screen.getByRole("slider");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "600");
    expect(bar).toHaveAttribute("aria-valuenow", "65");
    expect(bar).toHaveAttribute("aria-valuetext", "1:05");
    expect(bar).toHaveAttribute("aria-orientation", "horizontal");
  });

  it("renders engine-reported buffer ranges honestly (positions + data)", () => {
    const buffered: readonly SeekbarBufferRange[] = [
      { start: 0, end: 180 },
      { start: 240, end: 300 },
    ];
    const { container } = render(<Seekbar value={60} duration={600} buffered={buffered} />);
    const ranges = container.querySelectorAll(".lumen-seekbar__buffer");
    expect(ranges).toHaveLength(2);
    expect(ranges[0]).toHaveAttribute("data-range-start", "0");
    expect(ranges[0]).toHaveAttribute("data-range-end", "180");
    expect((ranges[0] as HTMLElement).style.left).toBe("0%");
    expect((ranges[0] as HTMLElement).style.width).toBe("30%");
    expect((ranges[1] as HTMLElement).style.left).toBe("40%");
    expect((ranges[1] as HTMLElement).style.width).toBe("10%");
  });

  it("renders no buffer sub-regions when the engine reports none", () => {
    const { container } = render(<Seekbar value={60} duration={600} />);
    expect(container.querySelectorAll(".lumen-seekbar__buffer")).toHaveLength(0);
  });

  it("scrub: knob follows the preview while the fill stays at engine truth (§5.2.1)", () => {
    const onChangeLive = vi.fn();
    render(<Seekbar value={100} duration={1000} onChangeLive={onChangeLive} />);
    const bar = screen.getByRole("slider");
    pinRect(bar, 1000, 28);
    pointer(bar, "pointerDown", 500);
    expect(onChangeLive).toHaveBeenLastCalledWith(500);
    expect(vars(bar)).toEqual({ fill: "10", thumb: "50" }); // fill=engine 100s, thumb=preview 500s
    expect(bar).toHaveAttribute("data-dragging", "true");
    expect(bar).toHaveAttribute("aria-valuenow", "500");
    pointer(bar, "pointerUp", 500);
    expect(vars(bar)).toEqual({ fill: "10", thumb: "10" }); // back to controlled engine value
  });

  it("commit fires once with the target seconds; live fired during the drag", () => {
    const order: string[] = [];
    render(
      <Seekbar
        value={0}
        duration={1000}
        onChangeLive={(s) => order.push(`live:${s}`)}
        onCommit={(s) => order.push(`commit:${s}`)}
      />,
    );
    const bar = screen.getByRole("slider");
    pinRect(bar, 1000, 28);
    pointer(bar, "pointerDown", 250);
    pointer(bar, "pointerMove", 750);
    pointer(bar, "pointerUp", 750);
    expect(order).toEqual(["live:250", "live:750", "commit:750"]);
  });

  it("pending: knob locks at the requested target, fill stays at engine position (§5.2.2)", () => {
    render(<Seekbar value={100} duration={1000} pending={500} />);
    const bar = screen.getByRole("slider");
    expect(bar).toHaveAttribute("data-pending", "true");
    expect(vars(bar)).toEqual({ fill: "10", thumb: "50" });
    expect(bar).toHaveAttribute("aria-valuenow", "500");
    expect(bar).toHaveAttribute("aria-valuetext", "8:20");
  });

  it("pending resolves to engine truth when the caller confirms (§5.2.3)", () => {
    const { rerender } = render(<Seekbar value={100} duration={1000} pending={500} />);
    rerender(<Seekbar value={500} duration={1000} />);
    const bar = screen.getByRole("slider");
    expect(bar).not.toHaveAttribute("data-pending");
    expect(vars(bar)).toEqual({ fill: "50", thumb: "50" });
    expect(bar).toHaveAttribute("aria-valuenow", "500");
  });

  it("keyboard: arrows seek by a sub-second step and commit", () => {
    const onCommit = vi.fn();
    const onChangeLive = vi.fn();
    render(<Seekbar value={100} duration={1000} onChangeLive={onChangeLive} onCommit={onCommit} />);
    const bar = screen.getByRole("slider");
    fireEvent.keyDown(bar, { key: "ArrowRight" });
    expect(onCommit).toHaveBeenCalledWith(101); // step = duration/1000 = 1s
    expect(onChangeLive).toHaveBeenCalledWith(101);
    fireEvent.keyDown(bar, { key: "End" });
    expect(onCommit).toHaveBeenLastCalledWith(1000);
  });

  it("no duration (no media): inert and truthfully disabled, no events", () => {
    const onCommit = vi.fn();
    render(<Seekbar value={0} duration={0} onCommit={onCommit} />);
    const bar = screen.getByRole("slider");
    expect(bar).toHaveAttribute("aria-disabled", "true");
    expect(bar).toHaveAttribute("aria-valuemax", "0");
    pinRect(bar, 1000, 28);
    pointer(bar, "pointerDown", 500);
    fireEvent.keyDown(bar, { key: "ArrowRight" });
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("accepts a custom time formatter for aria-valuetext", () => {
    render(<Seekbar value={12} duration={600} formatTime={(s) => `${Math.round(s)}s`} />);
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "12s");
  });

  it("formatSeekTime: M:SS and H:MM:SS forms, negative-safe", () => {
    expect(formatSeekTime(0)).toBe("0:00");
    expect(formatSeekTime(65)).toBe("1:05");
    expect(formatSeekTime(600)).toBe("10:00");
    expect(formatSeekTime(3725)).toBe("1:02:05");
    expect(formatSeekTime(-3)).toBe("0:00");
  });
});
