// SpatializerTool tests (w26.c2) — pins the spatializer card against its
// frame ("Audio - Spatializer Settings.svg"): collapsed-by-default card +
// toggle, the five frame rows with plain values, degree ticks on EVERY row
// (10 + 4×9, data-driven), local-state slider interaction, collapse, and the
// honest staged markers.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SpatializerTool } from "./SpatializerTool";

/** The frame's label → default value column, in frame order. */
const FRAME_ROWS: [label: string, value: string][] = [
  ["Size", "8.0"],
  ["Width", "10.0"],
  ["Wet", "4.0"],
  ["Dry", "5.0"],
  ["Damp", "5.0"],
];

function expand(): HTMLElement {
  const toggle = screen.getByRole("button", { name: "Expand Settings" });
  fireEvent.click(toggle);
  return toggle;
}

describe("SpatializerTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<SpatializerTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Damp" })).not.toBeInTheDocument();
  });

  it("expands on toggle click and collapses back", () => {
    render(<SpatializerTool />);
    const toggle = expand();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toHaveAttribute("hidden");
    expect(screen.getByText("Size")).toBeInTheDocument();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Size" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<SpatializerTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("SpatializerTool — frame rows", () => {
  it("renders the five frame rows: label left, plain value right, slider below", () => {
    render(<SpatializerTool />);
    expand();
    for (const [label] of FRAME_ROWS) {
      expect(screen.getByText(label)).toBeInTheDocument();
      expect(screen.getByRole("slider", { name: label })).toBeInTheDocument();
    }
    // Value column (frame defaults): Dry and Damp share "5.0".
    expect(screen.getByText("8.0")).toBeInTheDocument();
    expect(screen.getByText("10.0")).toBeInTheDocument();
    expect(screen.getByText("4.0")).toBeInTheDocument();
    expect(screen.getAllByText("5.0")).toHaveLength(2);
    expect(screen.getAllByRole("slider")).toHaveLength(FRAME_ROWS.length);
  });

  it("renders the frame's data-driven degree ticks (10 + 4×9)", () => {
    const { container } = render(<SpatializerTool defaultOpen />);
    expect(container.querySelectorAll(".dynamics-slider__tick")).toHaveLength(46);
    // Each default sits exactly on its tick — covered by the thumb (Width
    // sits at the max end, past the last tick, so 4 of 5 rows cover one).
    expect(
      container.querySelectorAll(".dynamics-slider__tick--covered"),
    ).toHaveLength(4);
  });

  it("sliders carry valuetext in the frame format", () => {
    render(<SpatializerTool />);
    expand();
    const wet = screen.getByRole("slider", { name: "Wet" });
    expect(wet).toHaveAttribute("aria-valuemin", "0");
    expect(wet).toHaveAttribute("aria-valuemax", "10");
    expect(wet).toHaveAttribute("aria-valuetext", "4.0");
  });
});

describe("SpatializerTool — local slider state", () => {
  it("keyboard commit updates the value read-out", () => {
    render(<SpatializerTool />);
    expand();
    const dry = screen.getByRole("slider", { name: "Dry" });
    fireEvent.keyDown(dry, { key: "ArrowRight" });
    expect(screen.getByText("5.5")).toBeInTheDocument();
  });

  it("clamps at the range minimum", () => {
    render(<SpatializerTool />);
    expand();
    const size = screen.getByRole("slider", { name: "Size" });
    fireEvent.keyDown(size, { key: "Home" });
    expect(screen.getByText("0.0")).toBeInTheDocument();
  });
});

describe("SpatializerTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<SpatializerTool defaultOpen />);
    const card = screen.getByLabelText("Spatializer");
    expect(card).toHaveAttribute("data-staged-tool", "spatializer");
    expect(card).toHaveAttribute("data-pending-command", "audio.set-spatializer");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
