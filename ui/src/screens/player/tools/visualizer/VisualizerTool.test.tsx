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
import { RESTING_LEVELS } from "./types";

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
    // The four non-Disabled rows carry the frame's two-tone leading icons
    // (the shared Dropdown's option-icon slot; w26.k10 migrated the
    // selector onto the primitive — the icons are the lane's own svgs).
    expect(container.querySelectorAll(".lumen-dropdown__option-icon svg")).toHaveLength(4);
    // Disabled (row 1) is selected (structural data-mode marker through the
    // primitive's `data` option field); the #1F1F1F band + orange check are
    // the shared menu.css selected grammar (CSS pseudo — not a DOM node).
    const selected = screen.getByRole("option", { selected: true });
    expect(selected).toHaveAttribute("data-mode", "disabled");
    expect(selected).toHaveAttribute("aria-selected", "true");
  });

  it("reveals the whole open list to scrolling hosts (w26.i07 fold fix)", () => {
    // h14 defect 4: inside the tools-dialog body the list opened with its
    // tail 164px below the fold — only the focused option auto-scrolled.
    // w26.k10: the fold-reveal (list.scrollIntoView {block:"nearest"}) now
    // lives in the shared Dropdown's open effect — this pins it there.
    // jsdom has no scrollIntoView; install a spy and pin the reveal call.
    const spy = vi.fn();
    const original = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = spy as unknown as typeof original;
    try {
      render(<VisualizerTool defaultOpen />);
      openSelector();
      const list = screen.getByRole("listbox", { name: "Visualizer" });
      expect(spy).toHaveBeenCalledTimes(1);
      expect(spy).toHaveBeenCalledWith({ block: "nearest" });
      expect(spy.mock.instances[0]).toBe(list);
    } finally {
      if (original === undefined) {
        delete (Element.prototype as Partial<{ scrollIntoView?: unknown }>)
          .scrollIntoView;
      } else {
        Element.prototype.scrollIntoView = original;
      }
    }
  });

  it("selects a mode on click: trigger label, display, onChange seam", () => {
    const onChange = vi.fn();
    render(<VisualizerTool defaultOpen onChange={onChange} />);
    openSelector();
    fireEvent.click(screen.getByRole("option", { name: "Spectrum" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("spectrum");
    // List closed, trigger now names the picked mode + its icon (the
    // primitive mirrors the selected option's iconNode in the trigger).
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
    const trigger = screen.getByRole("button", { name: "Visualizer" });
    expect(trigger).toHaveTextContent("Spectrum");
    expect(trigger.querySelector(".lumen-dropdown__option-icon svg")).not.toBeNull();
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
    // The frame icon's six bars (5/14/12/16/13/8 of 16 — w26.i07 corrected
    // bar 4 from the 19 typo to the frame's 16, Vector 87_2).
    expect(container.querySelectorAll(".viz-display__bar")).toHaveLength(6);
    expect(bar(container, 0)).toHaveAttribute("data-level", "0.313");
    expect(bar(container, 3)).toHaveAttribute("data-level", "1.000");
    // Frame proportions render at the h14-measured heights (13/36/31/41/
    // 33/21 — every non-max bar was ~16% short before the fix).
    const heights = Array.from(
      container.querySelectorAll<HTMLElement>(".viz-display__bar"),
      (el) => el.style.height,
    );
    expect(heights).toEqual(["13px", "36px", "31px", "41px", "33px", "21px"]);
    // Spectrum carries a peak dot per bar (frame Vector 91–97).
    expect(container.querySelectorAll(".viz-display__peak")).toHaveLength(6);
  });

  it("RESTING_LEVELS transcribes the frame icon exactly (bar 4 = 16)", () => {
    // Data pin (h14 defect 5 / g16-P3-4): the frame's Spectrum icon bars.
    expect(RESTING_LEVELS.spectrum).toEqual([5, 14, 12, 16, 13, 8]);
    expect(RESTING_LEVELS.scope).toEqual([5, 14, 12, 16, 13, 8]);
    expect(RESTING_LEVELS.waveform).toEqual([2, 7, 5, 9, 6, 4]);
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
