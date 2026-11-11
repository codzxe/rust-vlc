// HdrTool tests (w26.c4) — pins the HDR section against its frame
// ("HDR Options.svg"): the mode dropdown (trigger "Auto", six plain options
// — no leading icons, "Auto" selected), selection via pointer + keyboard,
// and the honest staged markers.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { HdrTool } from "./HdrTool";

/** The frame's six modes, in frame order. */
const FRAME_MODES = [
  "Auto",
  "Off",
  "HDR10",
  "HDR10+",
  "Dolby Vision",
  "HLG",
] as const;

describe("HdrTool — trigger", () => {
  it("renders the closed trigger with the frame's selected mode", () => {
    render(<HdrTool />);
    const trigger = screen.getByRole("button", { name: "HDR mode" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveTextContent("Auto");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });
});

describe("HdrTool — open list (frame state)", () => {
  it("opens the six frame modes, Auto selected, no leading icons", () => {
    render(<HdrTool />);
    fireEvent.click(screen.getByRole("button", { name: "HDR mode" }));
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual([...FRAME_MODES]);
    expect(screen.getByRole("option", { name: "Auto" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("option", { name: "HDR10" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    // The frame's option rows are PLAIN (no icon slot content).
    expect(document.querySelectorAll(".lumen-dropdown__option-icon")).toHaveLength(0);
  });

  it("selects a mode by click and moves the selection", () => {
    render(<HdrTool />);
    fireEvent.click(screen.getByRole("button", { name: "HDR mode" }));
    fireEvent.click(screen.getByRole("option", { name: "HDR10+" }));
    const trigger = screen.getByRole("button", { name: "HDR mode" });
    expect(trigger).toHaveTextContent("HDR10+");
    expect(trigger).toHaveAttribute("aria-expanded", "false"); // select closes
  });

  it("selects by keyboard (arrows + Enter) and restores focus on Escape", () => {
    render(<HdrTool />);
    const trigger = screen.getByRole("button", { name: "HDR mode" });
    fireEvent.click(trigger);
    const list = screen.getByRole("listbox", { name: "HDR mode" });
    // The list opens focused on the selected row (Auto); two ArrowDowns
    // rove Auto → Off → HDR10, Enter commits.
    fireEvent.keyDown(list, { key: "ArrowDown" });
    fireEvent.keyDown(list, { key: "ArrowDown" });
    fireEvent.keyDown(list, { key: "Enter" });
    expect(trigger).toHaveTextContent("HDR10");

    fireEvent.click(trigger);
    fireEvent.keyDown(screen.getByRole("listbox", { name: "HDR mode" }), {
      key: "Escape",
    });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveFocus();
  });
});

describe("HdrTool — honest staging (effects-area pattern)", () => {
  it("marks the section staged with the pending command marker", () => {
    render(<HdrTool />);
    const section = screen.getByLabelText("HDR options");
    expect(section).toHaveAttribute("data-staged-tool", "hdr");
    expect(section).toHaveAttribute("data-pending-command", "video.set-hdr");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
