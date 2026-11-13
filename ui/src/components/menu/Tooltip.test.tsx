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

  it("links the FOCUSABLE TRIGGER (not the wrapper) to the bubble via aria-describedby while open (w26.k02, j02 J02-1)", () => {
    render(
      <Tooltip content="Play/Pause">
        <button type="button">Go</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    // Closed: no tooltip wiring on the trigger (the a11y bug was a
    // wrapper-level describedby that no SR ever announced — focus lands on
    // the child, so the wiring must ride the child).
    expect(trigger).not.toHaveAttribute("aria-describedby");
    fireEvent.focus(trigger);
    const bubble = screen.getByRole("tooltip");
    expect(trigger).toHaveAttribute("aria-describedby", bubble.id);
  });

  it("merges the tooltip id into an aria-describedby the trigger already carries (never clobbers host wiring)", () => {
    render(
      <Tooltip content="Play/Pause">
        <button type="button" aria-describedby="external-hint">
          Go
        </button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    // Closed: the child's own description survives untouched.
    expect(trigger).toHaveAttribute("aria-describedby", "external-hint");
    fireEvent.focus(trigger);
    const bubble = screen.getByRole("tooltip");
    const described = trigger.getAttribute("aria-describedby") ?? "";
    expect(described.split(" ")).toContain("external-hint");
    expect(described.split(" ")).toContain(bubble.id);
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

  it("w26.k02 (j02 J02-13): Escape on an OPEN tooltip stays scoped — a listening host dialog does not close; closed-state Escape passes through", () => {
    const hostKeyDown = vi.fn();
    render(
      <div onKeyDown={hostKeyDown}>
        <Tooltip content="Play/Pause">
          <button type="button">Go</button>
        </Tooltip>
      </div>,
    );
    const trigger = screen.getByRole("button", { name: "Go" });
    fireEvent.focus(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    // The tooltip consumed this Escape — the host (e.g. a dialog scrim
    // closing on Escape) must not also fire.
    fireEvent.keyDown(trigger, { key: "Escape" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    expect(hostKeyDown).not.toHaveBeenCalled();
    // Closed tooltip: Escape is not consumed — the host still receives it.
    fireEvent.keyDown(trigger, { key: "Escape" });
    expect(hostKeyDown).toHaveBeenCalledTimes(1);
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
