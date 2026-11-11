// StereoWidenerTool tests (w26.c2) — pins the stereo-widener card against
// its frame ("Audio - Stereo Widener.svg"): collapsed-by-default card +
// toggle, the four frame rows (ms + percent units), tick pattern 0/8/7/9
// (Delay Time has NONE), local-state slider interaction, collapse, and the
// honest staged markers.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StereoWidenerTool } from "./StereoWidenerTool";

/** The frame's label → default value column, in frame order. */
const FRAME_ROWS: [label: string, value: string][] = [
  ["Delay Time", "20.0 ms"],
  ["Feedback Gain", "0.2 %"],
  ["Crossfeed", "0.3 %"],
  ["Dry Mix", "0.8 %"],
];

function expand(): HTMLElement {
  const toggle = screen.getByRole("button", { name: "Expand Settings" });
  fireEvent.click(toggle);
  return toggle;
}

describe("StereoWidenerTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<StereoWidenerTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Crossfeed" })).not.toBeInTheDocument();
  });

  it("expands on toggle click and collapses back", () => {
    render(<StereoWidenerTool />);
    const toggle = expand();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toHaveAttribute("hidden");
    expect(screen.getByText("Delay Time")).toBeInTheDocument();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Delay Time" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<StereoWidenerTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("StereoWidenerTool — frame rows", () => {
  it("renders the four frame rows: label left, plain value right, slider below", () => {
    render(<StereoWidenerTool />);
    expand();
    for (const [label, value] of FRAME_ROWS) {
      expect(screen.getByText(label)).toBeInTheDocument();
      expect(screen.getByText(value)).toBeInTheDocument();
      expect(screen.getByRole("slider", { name: label })).toBeInTheDocument();
    }
    expect(screen.getAllByRole("slider")).toHaveLength(FRAME_ROWS.length);
  });

  it("renders the frame's tick pattern (none / 8 / 7 / 9)", () => {
    const { container } = render(<StereoWidenerTool defaultOpen />);
    expect(container.querySelectorAll(".dynamics-slider__tick")).toHaveLength(24);
    // Delay Time row carries no tick overlay at all.
    const delayRow = container.querySelector('[data-row="delay"]');
    expect(delayRow).not.toBeNull();
    expect(delayRow?.querySelectorAll(".dynamics-slider__tick")).toHaveLength(0);
  });

  it("sliders carry the staged display ranges and valuetext", () => {
    render(<StereoWidenerTool />);
    expand();
    const crossfeed = screen.getByRole("slider", { name: "Crossfeed" });
    expect(crossfeed).toHaveAttribute("aria-valuemin", "0");
    expect(crossfeed).toHaveAttribute("aria-valuemax", "0.8");
    expect(crossfeed).toHaveAttribute("aria-valuetext", "0.3 %");
  });
});

describe("StereoWidenerTool — local slider state", () => {
  it("keyboard commit updates the value read-out", () => {
    render(<StereoWidenerTool />);
    expand();
    const delay = screen.getByRole("slider", { name: "Delay Time" });
    fireEvent.keyDown(delay, { key: "ArrowRight" });
    expect(screen.getByText("20.5 ms")).toBeInTheDocument();
    expect(delay).toHaveAttribute("aria-valuetext", "20.5 ms");
  });

  it("shift+arrow takes the coarse step (feedback 0.2 → 0.3 %)", () => {
    render(<StereoWidenerTool />);
    expand();
    const feedback = screen.getByRole("slider", { name: "Feedback Gain" });
    fireEvent.keyDown(feedback, { key: "ArrowRight", shiftKey: true });
    expect(feedback).toHaveAttribute("aria-valuetext", "0.3 %");
    // The old read-out is gone (Crossfeed keeps its own "0.3 %").
    expect(screen.queryByText("0.2 %")).not.toBeInTheDocument();
  });
});

describe("StereoWidenerTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<StereoWidenerTool defaultOpen />);
    const card = screen.getByLabelText("Stereo Widener");
    expect(card).toHaveAttribute("data-staged-tool", "stereo-widener");
    expect(card).toHaveAttribute("data-pending-command", "audio.set-stereo-widener");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
