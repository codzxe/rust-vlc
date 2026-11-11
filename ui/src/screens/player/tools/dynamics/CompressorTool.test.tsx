// CompressorTool tests (w26.c2) — pins the compressor card against its frame
// ("Audio - Compressor Settings.svg"): collapsed-by-default card + toggle,
// the seven frame rows (labels, plain default values, sliders), local-state
// slider interaction, collapse, and the honest staged markers.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CompressorTool } from "./CompressorTool";

/** The frame's label → default value column, in frame order. */
const FRAME_ROWS: [label: string, value: string][] = [
  ["RMS / Peak", "0.2"],
  ["Attack", "25.0 ms"],
  ["Release", "100.0 ms"],
  ["Threshold", "-11.0 dB"],
  ["Ratio", "4.0 : 1"],
  ["Knee Radius", "5.0 dB"],
  ["Makeup Gain", "7.0 dB"],
];

function expand(): HTMLElement {
  const toggle = screen.getByRole("button", { name: "Expand Settings" });
  fireEvent.click(toggle);
  return toggle;
}

describe("CompressorTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<CompressorTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    // No row content leaks while collapsed (role queries respect `hidden`).
    expect(screen.queryByRole("slider", { name: "Attack" })).not.toBeInTheDocument();
  });

  it("expands on toggle click and collapses back", () => {
    render(<CompressorTool />);
    const toggle = expand();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toBe(toggle);
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toHaveAttribute("hidden");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Attack" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<CompressorTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("CompressorTool — frame rows", () => {
  it("renders the seven frame rows: label left, plain value right, slider below", () => {
    render(<CompressorTool />);
    expand();
    for (const [label, value] of FRAME_ROWS) {
      expect(screen.getByText(label)).toBeInTheDocument();
      expect(screen.getByText(value)).toBeInTheDocument();
      expect(screen.getByRole("slider", { name: label })).toBeInTheDocument();
    }
    const sliders = screen.getAllByRole("slider");
    expect(sliders).toHaveLength(FRAME_ROWS.length);
  });

  it("sliders carry the staged display ranges and valuetext", () => {
    render(<CompressorTool />);
    expand();
    const threshold = screen.getByRole("slider", { name: "Threshold" });
    expect(threshold).toHaveAttribute("aria-valuemin", "-30");
    expect(threshold).toHaveAttribute("aria-valuemax", "0");
    expect(threshold).toHaveAttribute("aria-valuenow", "-11");
    expect(threshold).toHaveAttribute("aria-valuetext", "-11.0 dB");
  });

  it("has no degree ticks (compressor frame carries none)", () => {
    const { container } = render(<CompressorTool defaultOpen />);
    expect(container.querySelectorAll(".dynamics-slider__tick")).toHaveLength(0);
  });
});

describe("CompressorTool — local slider state", () => {
  it("keyboard commit updates the value read-out", () => {
    render(<CompressorTool />);
    expand();
    const attack = screen.getByRole("slider", { name: "Attack" });
    fireEvent.keyDown(attack, { key: "ArrowRight" });
    expect(screen.getByText("25.5 ms")).toBeInTheDocument();
    expect(attack).toHaveAttribute("aria-valuetext", "25.5 ms");
  });

  it("clamps at the range maximum", () => {
    render(<CompressorTool />);
    expand();
    const ratio = screen.getByRole("slider", { name: "Ratio" });
    fireEvent.keyDown(ratio, { key: "End" });
    expect(screen.getByText("10.0 : 1")).toBeInTheDocument();
  });
});

describe("CompressorTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<CompressorTool defaultOpen />);
    const card = screen.getByLabelText("Compressor");
    expect(card).toHaveAttribute("data-staged-tool", "compressor");
    expect(card).toHaveAttribute("data-pending-command", "audio.set-compressor");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
