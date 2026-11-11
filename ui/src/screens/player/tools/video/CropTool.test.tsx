// CropTool tests (w26.c4) — pins the Crop card against its frame
// ("Video - Crop.svg"): collapsed-by-default card + toggle, the four
// labeled number inputs (Top/Bottom/Left/Right, "0 px"), the per-row link
// toggles coupling each pair, digit sanitization, and the staged markers.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CropTool } from "./CropTool";

describe("CropTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<CropTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("textbox", { name: "Top" })).not.toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<CropTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("CropTool — frame rows", () => {
  it("renders the four labeled inputs with the frame's 0 px values", () => {
    render(<CropTool defaultOpen />);
    for (const label of ["Top", "Bottom", "Left", "Right"]) {
      expect(screen.getByRole("textbox", { name: label })).toHaveValue("0");
    }
    expect(screen.getAllByText("px")).toHaveLength(4);
  });

  it("renders the two link toggles (chain = pair locked, frame state)", () => {
    render(<CropTool defaultOpen />);
    const vertical = screen.getByRole("button", { name: "Link top and bottom" });
    const horizontal = screen.getByRole("button", { name: "Link left and right" });
    expect(vertical).toHaveAttribute("aria-pressed", "true");
    expect(horizontal).toHaveAttribute("aria-pressed", "true");
  });

  it("carries no separators (the frame sections by rows, not lines)", () => {
    const { container } = render(<CropTool defaultOpen />);
    expect(container.querySelectorAll(".video-sep")).toHaveLength(0);
  });
});

describe("CropTool — link coupling", () => {
  it("a linked pair mirrors edits across both fields", () => {
    render(<CropTool defaultOpen />);
    fireEvent.change(screen.getByRole("textbox", { name: "Top" }), {
      target: { value: "40" },
    });
    expect(screen.getByRole("textbox", { name: "Top" })).toHaveValue("40");
    expect(screen.getByRole("textbox", { name: "Bottom" })).toHaveValue("40");
    // The other pair is untouched.
    expect(screen.getByRole("textbox", { name: "Left" })).toHaveValue("0");
  });

  it("unlinking a pair makes its fields independent", () => {
    render(<CropTool defaultOpen />);
    const link = screen.getByRole("button", { name: "Link top and bottom" });
    fireEvent.click(link);
    expect(link).toHaveAttribute("aria-pressed", "false");
    fireEvent.change(screen.getByRole("textbox", { name: "Bottom" }), {
      target: { value: "12" },
    });
    expect(screen.getByRole("textbox", { name: "Top" })).toHaveValue("0");
    expect(screen.getByRole("textbox", { name: "Bottom" })).toHaveValue("12");
  });

  it("sanitizes non-digit input (crop pixels are non-negative integers)", () => {
    render(<CropTool defaultOpen />);
    fireEvent.change(screen.getByRole("textbox", { name: "Left" }), {
      target: { value: "1a2b" },
    });
    expect(screen.getByRole("textbox", { name: "Left" })).toHaveValue("12");
    expect(screen.getByRole("textbox", { name: "Right" })).toHaveValue("12"); // linked
  });
});

describe("CropTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<CropTool defaultOpen />);
    const card = screen.getByLabelText("Crop");
    expect(card).toHaveAttribute("data-staged-tool", "crop");
    expect(card).toHaveAttribute("data-pending-command", "video.set-crop");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
