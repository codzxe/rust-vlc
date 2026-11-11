// EssentialTool tests (w26.c4) — pins the Essential card against its frame
// ("Video - Essential.svg"): collapsed-by-default card + toggle, the eight
// LABEL-ONLY slider rows (no value column — video.css hides the shared
// value span; the sliders' aria-valuetext still reports every value), the
// per-row degree-tick counts, local-state commits, and the staged markers.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EssentialTool } from "./EssentialTool";

/** The frame's rows, in frame order: label → degree-tick count. */
const FRAME_ROWS: [label: string, ticks: number][] = [
  ["Hue", 1],
  ["Brightness", 1],
  ["Contrast", 1],
  ["Saturation", 2],
  ["Gamma", 9],
  ["Sharpen Sigma", 3],
  ["Banding Removal Radius", 6],
  ["Film Grain Variance", 9],
];

describe("EssentialTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<EssentialTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Hue" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<EssentialTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("EssentialTool — frame rows", () => {
  it("renders the eight label-only slider rows, in frame order", () => {
    const { container } = render(<EssentialTool defaultOpen />);
    for (const [label] of FRAME_ROWS) {
      expect(screen.getByRole("slider", { name: label })).toBeInTheDocument();
    }
    expect(screen.getAllByRole("slider")).toHaveLength(FRAME_ROWS.length);
    // Rows land in frame order (data-row markers).
    const ids = Array.from(container.querySelectorAll(".dynamics-row")).map(
      (row) => row.getAttribute("data-row"),
    );
    expect(ids).toEqual([
      "hue",
      "brightness",
      "contrast",
      "saturation",
      "gamma",
      "sharpen-sigma",
      "banding-radius",
      "grain-variance",
    ]);
  });

  it("carries the frame's per-row degree-tick counts", () => {
    const { container } = render(<EssentialTool defaultOpen />);
    const byId: Record<string, number> = {
      hue: 1,
      brightness: 1,
      contrast: 1,
      saturation: 2,
      gamma: 9,
      "sharpen-sigma": 3,
      "banding-radius": 6,
      "grain-variance": 9,
    };
    for (const [id, ticks] of Object.entries(byId)) {
      expect(
        container.querySelectorAll(`[data-row="${id}"] .dynamics-slider__tick`),
      ).toHaveLength(ticks);
    }
  });

  it("sliders carry the staged display ranges and valuetext", () => {
    render(<EssentialTool defaultOpen />);
    const hue = screen.getByRole("slider", { name: "Hue" });
    expect(hue).toHaveAttribute("aria-valuemin", "-180");
    expect(hue).toHaveAttribute("aria-valuemax", "180");
    expect(hue).toHaveAttribute("aria-valuenow", "0");
    expect(hue).toHaveAttribute("aria-valuetext", "0°");
    expect(screen.getByRole("slider", { name: "Gamma" })).toHaveAttribute(
      "aria-valuetext",
      "1.1",
    );
    expect(
      screen.getByRole("slider", { name: "Banding Removal Radius" }),
    ).toHaveAttribute("aria-valuetext", "7 px");
    expect(
      screen.getByRole("slider", { name: "Film Grain Variance" }),
    ).toHaveAttribute("aria-valuetext", "21 %");
  });

  it("is the label-only family: the card scopes the value column away (frame)", () => {
    render(<EssentialTool defaultOpen />);
    const card = screen.getByLabelText("Essential");
    // video.css hides .dynamics-row__value inside .video-tool — the frame
    // carries no value text; the span stays for the DOM contract.
    expect(card.className).toContain("video-tool--essential");
    expect(card.querySelectorAll(".dynamics-row__value")).toHaveLength(8);
  });
});

describe("EssentialTool — local slider state", () => {
  it("keyboard commit updates the valuetext", () => {
    render(<EssentialTool defaultOpen />);
    const hue = screen.getByRole("slider", { name: "Hue" });
    fireEvent.keyDown(hue, { key: "ArrowRight" });
    expect(hue).toHaveAttribute("aria-valuetext", "1°");
  });

  it("clamps at the range maximum", () => {
    render(<EssentialTool defaultOpen />);
    const grain = screen.getByRole("slider", { name: "Film Grain Variance" });
    fireEvent.keyDown(grain, { key: "End" });
    expect(grain).toHaveAttribute("aria-valuetext", "100 %");
  });
});

describe("EssentialTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<EssentialTool defaultOpen />);
    const card = screen.getByLabelText("Essential");
    expect(card).toHaveAttribute("data-staged-tool", "essential");
    expect(card).toHaveAttribute("data-pending-command", "video.set-essential");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
