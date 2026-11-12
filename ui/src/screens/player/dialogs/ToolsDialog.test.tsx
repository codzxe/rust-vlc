// ToolsDialog tests (w26.c5 integration) — pins the player's tools dialog:
// the ToolsDialogShell chrome with the Audio/Video tab pair hosting all
// eight wave-c tools, the EqualizerTool's staged wrapper, and the
// `lumen:open-tools` trigger seam (openToolsDialog + ToolsDialogHost, the
// lumen:open-media pattern).
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { keys as toolsKeys } from "../../../i18n/keys/w26.a18";
import {
  openToolsDialog,
  ToolsDialog,
  ToolsDialogHost,
  TOOLS_DIALOG_EVENT,
} from "./ToolsDialog";

const EXPAND = toolsKeys["player.tools.expand-settings"];

const AUDIO_TOOL_NAMES = [
  "Equalizer",
  "Compressor",
  "Spatializer",
  "Stereo Widener",
  "Audio Visualizer",
] as const;

const VIDEO_TOOL_NAMES = ["Video colors", "Crop", "Essential", "HDR options"] as const;

function renderDialog(props: Partial<Parameters<typeof ToolsDialog>[0]> = {}) {
  const onClose = vi.fn();
  render(<ToolsDialog open onClose={onClose} {...props} />);
  return { onClose };
}

describe("ToolsDialog — shell chrome + the tab pair", () => {
  it("renders the titled dialog with both tabs, Audio selected by default", () => {
    renderDialog();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Tools" })).toBeInTheDocument();
    const audio = screen.getByRole("tab", { name: "Audio" });
    const video = screen.getByRole("tab", { name: "Video" });
    expect(audio).toHaveAttribute("aria-selected", "true");
    expect(video).toHaveAttribute("aria-selected", "false");
  });

  it("Escape closes through the shared Dialog focus trap (shell chrome)", () => {
    const { onClose } = renderDialog();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });
});

describe("ToolsDialog — the AUDIO tab hosts the five audio tools (frame order)", () => {
  it("mounts every audio tool card, collapsed per the frames", () => {
    renderDialog();
    for (const name of AUDIO_TOOL_NAMES) {
      const card = screen.getByRole("region", { name });
      expect(card).toBeInTheDocument();
      expect(card.querySelector('[aria-expanded="false"]')).not.toBeNull();
    }
    // Four collapsed card toggles + the EQ's own frame toggle (EQ.svg: the
    // equalizer is the A2 vertical-fader panel with its "Open/Close
    // Equalizer" morphing header — w26.f05).
    expect(screen.getAllByRole("button", { name: EXPAND })).toHaveLength(4);
    const eqToggle = screen.getByRole("button", { name: "Open Equalizer" });
    expect(eqToggle).toHaveAttribute("aria-expanded", "false");
    // The video panel is hidden — none of its tools reach the a11y tree.
    for (const name of VIDEO_TOOL_NAMES) {
      expect(screen.queryByRole("region", { name })).toBeNull();
    }
  });

  it("expanding the Equalizer reveals the frame rows + the staged note", () => {
    renderDialog();
    fireEvent.click(
      screen.getByRole("region", { name: "Equalizer" }).querySelector(
        '[aria-expanded="false"]',
      ) as HTMLElement,
    );
    expect(screen.getAllByRole("slider")).toHaveLength(11);
    expect(screen.getByRole("slider", { name: "Preamp" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Equalizer preset" })).toBeInTheDocument();
  });

  it("the EqualizerTool wrapper carries the staged markers the tool lacks", () => {
    renderDialog();
    const staged = screen
      .getByRole("region", { name: "Equalizer" })
      .closest("[data-staged-tool]");
    expect(staged).not.toBeNull();
    expect(staged).toHaveAttribute("data-staged-tool", "equalizer");
    expect(staged).toHaveAttribute("data-pending-command", "audio.set-eq");
  });

  it("the sibling tools carry their own staged markers (spot check)", () => {
    renderDialog();
    for (const [name, command] of [
      ["Compressor", "audio.set-compressor"],
      ["Spatializer", "audio.set-spatializer"],
      ["Stereo Widener", "audio.set-stereo-widener"],
      ["Audio Visualizer", "audio.set-visualizer"],
    ] as const) {
      const card = screen.getByRole("region", { name });
      expect(card).toHaveAttribute("data-staged-tool");
      expect(card.getAttribute("data-pending-command")).toBe(command);
    }
  });
});

describe("ToolsDialog — the VIDEO tab hosts the four video tools (frame order)", () => {
  it("switching tabs swaps the tool set", () => {
    renderDialog();
    fireEvent.click(screen.getByRole("tab", { name: "Video" }));
    expect(screen.getByRole("tab", { name: "Video" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    for (const name of VIDEO_TOOL_NAMES) {
      expect(screen.getByRole("region", { name })).toBeInTheDocument();
    }
    for (const name of AUDIO_TOOL_NAMES) {
      expect(screen.queryByRole("region", { name })).toBeNull();
    }
    // Card tools collapsed; the HDR tool is the frame's bare dropdown.
    expect(screen.getAllByRole("button", { name: EXPAND })).toHaveLength(3);
    expect(screen.getByRole("button", { name: "HDR mode" })).toBeInTheDocument();
  });

  it("defaultTab preselects the video tab", () => {
    renderDialog({ defaultTab: "video" });
    expect(screen.getByRole("tab", { name: "Video" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("region", { name: "HDR options" })).toBeInTheDocument();
  });
});

describe("ToolsDialogHost — the lumen:open-tools trigger seam", () => {
  it("renders nothing until the event arrives, then mounts per-open", () => {
    const { container } = render(<ToolsDialogHost />);
    expect(container.firstChild).toBeNull();
    act(() => {
      window.dispatchEvent(new CustomEvent(TOOLS_DIALOG_EVENT, { detail: "audio" }));
    });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Equalizer" })).toBeInTheDocument();
  });

  it("the event detail preselects the tab", () => {
    render(<ToolsDialogHost />);
    act(() => {
      window.dispatchEvent(new CustomEvent(TOOLS_DIALOG_EVENT, { detail: "video" }));
    });
    expect(screen.getByRole("region", { name: "Essential" })).toBeInTheDocument();
  });

  it("closing unmounts the dialog (per-open, fresh tool state next open)", () => {
    render(<ToolsDialogHost />);
    act(() => {
      window.dispatchEvent(new CustomEvent(TOOLS_DIALOG_EVENT));
    });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: toolsKeys["player.tools.close"] }));
    expect(screen.queryByRole("dialog")).toBeNull();
    // Re-open works (fresh mount).
    act(() => {
      window.dispatchEvent(new CustomEvent(TOOLS_DIALOG_EVENT));
    });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("openToolsDialog dispatches the seam event (default tab audio)", () => {
    render(<ToolsDialogHost />);
    act(() => {
      openToolsDialog();
    });
    expect(screen.getByRole("region", { name: "Equalizer" })).toBeInTheDocument();
    act(() => {
      openToolsDialog("video");
    });
    expect(screen.getByRole("region", { name: "Crop" })).toBeInTheDocument();
  });
});
