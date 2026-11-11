import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Tooltip } from "./Tooltip";
import { TOOLTIP_HIDE_DELAY_MS, TOOLTIP_SHOW_DELAY_MS } from "./timings";

/**
 * Card 4.B3 — Tooltip tests: hover-intent show/hide delays (fake timers),
 * immediate keyboard show, aria-describedby wiring, Escape dismiss.
 * Timer callbacks setState outside React's event flow, so advances are
 * wrapped in `act` to flush the scheduled render.
 */

describe("Tooltip", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows the bubble after the show delay (hover intent)", async () => {
    vi.useFakeTimers();
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.mouseOver(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(TOOLTIP_SHOW_DELAY_MS - 1);
    });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.getByRole("tooltip")).toHaveTextContent("Play/Pause");
  });

  it("hides the bubble after the hide delay when the pointer leaves", async () => {
    vi.useFakeTimers();
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.mouseOver(trigger);
    await act(async () => {
      vi.advanceTimersByTime(TOOLTIP_SHOW_DELAY_MS);
    });
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.mouseOut(trigger);
    await act(async () => {
      vi.advanceTimersByTime(TOOLTIP_HIDE_DELAY_MS - 1);
    });
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("shows immediately on keyboard focus (no delay) and hides on blur", () => {
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.focus(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.blur(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("links trigger and bubble via aria-describedby while open", () => {
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    const wrapper = trigger.parentElement as HTMLElement;
    expect(wrapper).not.toHaveAttribute("aria-describedby");
    fireEvent.focus(trigger);
    const bubble = screen.getByRole("tooltip");
    expect(wrapper).toHaveAttribute("aria-describedby", bubble.id);
  });

  it("Escape hides the tooltip while the trigger is focused", () => {
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.focus(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.keyDown(trigger, { key: "Escape" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("honors custom show/hide delays from props", async () => {
    vi.useFakeTimers();
    render(
      <Tooltip content="tip" showDelay={100} hideDelay={50}>
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.mouseOver(trigger);
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.mouseOut(trigger);
    await act(async () => {
      vi.advanceTimersByTime(50);
    });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });
});
