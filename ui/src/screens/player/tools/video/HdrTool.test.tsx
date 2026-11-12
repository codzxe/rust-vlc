// HdrTool tests (w26.c4) — pins the HDR section against its frame
// ("HDR Options.svg"): the mode dropdown (trigger "Auto", six plain options
// — no leading icons, "Auto" selected), selection via pointer + keyboard,
// and the honest staged markers.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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

describe("HdrTool — clip-aware list placement (w26.i08)", () => {
  it("jsdom has no layout: the shared default holds, no escape/cap markers", async () => {
    // Zero-geometry triggers degrade to the shared Dropdown's own in-flow
    // placement (no data-list-escape, no data-list-direction, no inline
    // coordinates). The live behavior — defer to the shared flip while the
    // list fits the dialog body, position:fixed escape at the HDR rest
    // state, cap as last resort — is pinned by resolveListPlacement unit
    // tests (useDropdownScrollportFlip.test.ts) + the lane's Playwright
    // probe (/tmp/cap-w26i08/).
    render(<HdrTool />);
    const section = screen.getByLabelText("HDR options");
    expect(section).not.toHaveAttribute("data-list-direction");
    expect(section).not.toHaveAttribute("data-list-escape");
    fireEvent.click(screen.getByRole("button", { name: "HDR mode" }));
    // The hook observes the shared Dropdown's list mount
    // (MutationObserver — delivered on a microtask, hence waitFor).
    await waitFor(() =>
      expect(screen.getByRole("listbox", { name: "HDR mode" })).toBeInTheDocument(),
    );
    await waitFor(() => {
      expect(section).not.toHaveAttribute("data-list-direction");
      expect(section).not.toHaveAttribute("data-list-escape");
    });
    const list = screen.getByRole("listbox", { name: "HDR mode" });
    expect(list.getAttribute("style") ?? "").not.toMatch(/position|top|left|width/);
    // Closing unmounts the list (the shared primitive's own behavior).
    fireEvent.keyDown(list, { key: "Escape" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
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
