// VisualizerTool tests (w26.c3) — pins the visualizer card against its
// frame ("Audio Visualizer.svg", w26.a20 §A7): the family card shell
// (collapsed by default, staged markers), the five-mode dropdown (labels,
// two-tone icons, selected check, keyboard model), mode switching through
// the onChange seam, and the HONEST display contract — resting state when
// no `levels` exist (never simulated), live bars when they do, nothing at
// all while Disabled.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { VisualizerTool } from "./VisualizerTool";

afterEach(cleanup);

/** The frame's five options, in frame order (correct "Spectrometer"). */
const FRAME_MODES = ["Disabled", "Spectrometer", "Scope", "Spectrum", "Waveform"];

/** The Nth display bar (fails loudly when missing). */
function bar(container: HTMLElement, index: number): HTMLElement {
  const found = container.querySelectorAll<HTMLElement>(".viz-display__bar")[index];
  if (found === undefined) throw new Error(`display bar ${index} missing`);
  return found;
}

function expand(): HTMLElement {
  const toggle = screen.getByRole("button", { name: "Expand Settings" });
  fireEvent.click(toggle);
  return toggle;
}

function openSelector(): void {
  fireEvent.click(screen.getByRole("button", { name: "Visualizer" }));
}

describe("VisualizerTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<VisualizerTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
  });

  it("expands on toggle click and collapses back", () => {
    render(<VisualizerTool />);
    const toggle = expand();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toBe(toggle);
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("button", { name: "Visualizer" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<VisualizerTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });

  it("marks the card staged with the pending command marker", () => {
    render(<VisualizerTool defaultOpen />);
    const card = screen.getByLabelText("Audio Visualizer");
    expect(card).toHaveAttribute("data-staged-tool", "visualizer");
    expect(card).toHaveAttribute("data-pending-command", "audio.set-visualizer");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});

describe("VisualizerTool — the frame's mode dropdown (§A7)", () => {
  it("defaults to Disabled with no display — the frame's closed state", () => {
    const { container } = render(<VisualizerTool defaultOpen />);
    expect(screen.getByRole("button", { name: "Visualizer" })).toHaveTextContent(
      "Disabled",
    );
    expect(container.querySelector(".viz-display")).toBeNull();
  });

  it("opens the five frame options in order with two-tone icons", () => {
    const { container } = render(<VisualizerTool defaultOpen />);
    openSelector();
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual(FRAME_MODES);
    // The four non-Disabled rows carry the frame's two-tone leading icons.
    expect(container.querySelectorAll(".viz-select__icon")).toHaveLength(4);
    // Disabled (row 1) is selected: #1F1F1F band + ONE orange check.
    const selected = screen.getByRole("option", { selected: true });
    expect(selected).toHaveAttribute("data-mode", "disabled");
    expect(container.querySelectorAll(".viz-select__check")).toHaveLength(1);
    expect(selected.querySelector(".viz-select__check")).not.toBeNull();
  });

  it("selects a mode on click: trigger label, display, onChange seam", () => {
    const onChange = vi.fn();
    render(<VisualizerTool defaultOpen onChange={onChange} />);
    openSelector();
    fireEvent.click(screen.getByRole("option", { name: "Spectrum" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("spectrum");
    // List closed, trigger now names the picked mode + its icon.
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
    const trigger = screen.getByRole("button", { name: "Visualizer" });
    expect(trigger).toHaveTextContent("Spectrum");
    expect(trigger.querySelector(".viz-select__selected-icon")).not.toBeNull();
  });

  it("keyboard model: arrows move, Enter commits, Escape closes + refocuses", () => {
    const onChange = vi.fn();
    render(<VisualizerTool defaultOpen onChange={onChange} />);
    openSelector();
    const list = screen.getByRole("listbox", { name: "Visualizer" });
    // The selected option (Disabled, row 1) is the roving start.
    expect(screen.getByRole("option", { selected: true })).toHaveFocus();
    fireEvent.keyDown(list, { key: "ArrowDown" });
    expect(screen.getByRole("option", { name: "Spectrometer" })).toHaveFocus();
    fireEvent.keyDown(list, { key: "ArrowDown" });
    fireEvent.keyDown(list, { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith("scope");
    expect(screen.queryByRole("option")).not.toBeInTheDocument();

    openSelector();
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Visualizer" })).toHaveFocus();
  });

  it("defaultMode seeds the selection (waveform)", () => {
    render(<VisualizerTool defaultOpen defaultMode="waveform" />);
    expect(screen.getByRole("button", { name: "Visualizer" })).toHaveTextContent(
      "Waveform",
    );
    expect(screen.getByRole("img")).toHaveAttribute("data-mode", "waveform");
  });
});

describe("VisualizerTool — honest display (levels contract)", () => {
  it("renders the mode's RESTING frame depiction when levels are absent", () => {
    const { container } = render(
      <VisualizerTool defaultOpen defaultMode="spectrum" />,
    );
    const display = screen.getByRole("img");
    expect(display).toHaveAttribute("data-state", "resting");
    expect(display).toHaveAttribute(
      "aria-label",
      "Spectrum — Waiting for audio data",
    );
    // The frame icon's six bars (5/14/12/19/13/8 of 19).
    expect(container.querySelectorAll(".viz-display__bar")).toHaveLength(6);
    expect(bar(container, 0)).toHaveAttribute("data-level", "0.263");
    expect(bar(container, 3)).toHaveAttribute("data-level", "1.000");
    // Spectrum carries a peak dot per bar (frame Vector 91–97).
    expect(container.querySelectorAll(".viz-display__peak")).toHaveLength(6);
  });

  it("renders NOTHING while Disabled — even with levels (frame default)", () => {
    const { container } = render(<VisualizerTool defaultOpen levels={[0.5, 1]} />);
    expect(container.querySelector(".viz-display")).toBeNull();
  });

  it("live levels drive one bar per entry, clamped to the band", () => {
    const { container } = render(
      <VisualizerTool defaultOpen defaultMode="spectrum" levels={[1, 0.5, 2, -1]} />,
    );
    const display = screen.getByRole("img");
    expect(display).toHaveAttribute("data-state", "live");
    expect(display).toHaveAttribute("aria-label", "Spectrum — Live");
    expect(container.querySelectorAll(".viz-display__bar")).toHaveLength(4);
    expect(bar(container, 0).style.height).toBe("41px"); // 1.0 × 41px band
    expect(bar(container, 1).style.height).toBe("21px"); // 0.5 × 41
    expect(bar(container, 2).style.height).toBe("41px"); // 2 → clamped to 1
    expect(bar(container, 3).style.height).toBe("3px"); // -1 → pill floor
    // Live spectrum bars cycle the eq token group (mission pin).
    expect(bar(container, 0)).toHaveAttribute("data-accent", "1");
    expect(bar(container, 1)).toHaveAttribute("data-accent", "2");
    expect(bar(container, 0).getAttribute("data-tone")).toBeNull();
  });

  it("an empty levels array stays RESTING (honest: no data, no fake bars)", () => {
    const { container } = render(
      <VisualizerTool defaultOpen defaultMode="spectrum" levels={[]} />,
    );
    expect(screen.getByRole("img")).toHaveAttribute("data-state", "resting");
    expect(container.querySelectorAll(".viz-display__bar")).toHaveLength(6);
  });

  it("scope keeps the frame's two-tone bars from the baseline", () => {
    const { container } = render(
      <VisualizerTool defaultOpen defaultMode="scope" levels={[1, 1]} />,
    );
    expect(container.querySelectorAll(".viz-display__bar")).toHaveLength(2);
    expect(bar(container, 0)).toHaveAttribute("data-tone", "gray"); // gray-first
    expect(bar(container, 1)).toHaveAttribute("data-tone", "accent");
    expect(bar(container, 0).style.height).toBe("48px"); // full band
  });

  it("waveform centers its bars on the midline", () => {
    const { container } = render(
      <VisualizerTool defaultOpen defaultMode="waveform" levels={[1, 0.5]} />,
    );
    expect(container.querySelector(".viz-display__bars")).toHaveAttribute(
      "data-align",
      "center",
    );
    expect(bar(container, 0)).toHaveAttribute("data-tone", "accent"); // accent-first
    expect(bar(container, 0).style.height).toBe("48px");
    expect(bar(container, 1).style.height).toBe("24px");
  });

  it("spectrometer renders the concentric dial (levels = rings + arcs)", () => {
    const live = render(
      <VisualizerTool defaultOpen defaultMode="spectrometer" levels={[1, 0.5]} />,
    );
    expect(screen.getByRole("img")).toHaveAttribute("data-state", "live");
    const dial = live.container.querySelector(".viz-display__dial");
    expect(dial).not.toBeNull();
    // 2 level rings + the frame's outer arc ring (Ellipse 18 grammar).
    expect(dial?.querySelectorAll("circle")).toHaveLength(3);
    cleanup();
    // Resting: the frame icon's two circles (r4/r7 scaled) + the arc ring.
    const resting = render(
      <VisualizerTool defaultOpen defaultMode="spectrometer" />,
    );
    const restingDial = resting.container.querySelector(".viz-display__dial");
    expect(restingDial?.querySelectorAll("circle")).toHaveLength(3);
    expect(resting.container.querySelector(".viz-display__bar")).toBeNull();
  });
});
