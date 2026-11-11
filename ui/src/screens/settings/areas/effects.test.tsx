// Effects & Tools area tests (w26.c5 integration) — the Settings → Audio
// Effects section composed to `Settings - audio-1.svg`: the embedded
// wave-c EqualizerTool (frame "EQ" block), the staged rows around it, and
// the honest-surface rules:
//   • the embedded EQ + every row control is STAGED (no audio-effects
//     catalog command exists yet) — markers name the awaited command, the
//     note badge gives the honest reason, nothing ever sends;
//   • the retired v0 widgets (master volume, mute, balance, video adjust)
//     are gone per the frame — live playback control stays in the player
//     ControlBar (see the effects.tsx module header + the w26.c5 lane log).

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { keys as toolsKeys } from "../../../i18n/keys/w26.a18";
import { EffectsArea } from "./effects";
import { makeFixtureTransport } from "./audio-fixtures";

const COLLAPSE = toolsKeys["player.tools.collapse-settings"];

const renderArea = () => {
  const transport = makeFixtureTransport(() => ({ completed: { kind: "unit" } }));
  const view = render(
    <ClientProvider transport={transport}>
      <EffectsArea />
    </ClientProvider>,
  );
  return { transport, ...view };
};

describe("EffectsArea — the frame's Effects section (Settings - audio-1)", () => {
  it("renders the Effects section header", () => {
    renderArea();
    expect(screen.getByRole("heading", { name: "Effects" })).toBeInTheDocument();
  });

  it("embeds the EqualizerTool EXPANDED (frame EQ block, 'Close Equalizer')", () => {
    renderArea();
    // The tool's expanded toggle + all 11 frame rows (Preamp + 10 bands).
    expect(screen.getByRole("button", { name: COLLAPSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(screen.getAllByRole("slider")).toHaveLength(11);
    expect(screen.getByRole("slider", { name: "Preamp" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "60 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "16 KHz" })).toBeInTheDocument();
  });

  it("the EQ presets panel is embedded with the tool (frame 'Equalizer preset' row)", () => {
    renderArea();
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    // The frame's 8 presets (EQ presets.svg) open from the embedded tool.
    for (const name of [
      "None",
      "Flat",
      "Classical",
      "Club",
      "Dance",
      "Full bass",
      "Full bass & treble",
      "Full treble",
    ]) {
      expect(screen.getByRole("option", { name })).toBeInTheDocument();
    }
  });
});

describe("EffectsArea — staged rows are inert WITH REASON (the old pattern stays)", () => {
  it("the Equalizer enable toggle is staged OFF for audio.set-eq", () => {
    renderArea();
    const eq = screen.getByRole("switch", { name: "Equalizer" });
    expect(eq).toBeDisabled();
    expect(eq).toHaveAttribute("aria-checked", "false");
    expect(eq.closest("[data-pending-command]")?.getAttribute("data-pending-command")).toBe(
      "audio.set-eq",
    );
  });

  it("the embedded EQ panel carries the staged markers (audio.set-eq)", () => {
    renderArea();
    const tool = screen.getByRole("region", { name: "Equalizer" });
    const staged = tool.closest("[data-staged-tool]");
    expect(staged).not.toBeNull();
    expect(staged).toHaveAttribute("data-staged-tool", "equalizer");
    expect(staged).toHaveAttribute("data-pending-command", "audio.set-eq");
    // The honest reason renders (old pattern: visible, never silent).
    expect(screen.getAllByRole("note").length).toBeGreaterThan(0);
  });

  it("Two-pass / Spatial audio / Stereo widening toggles are staged, naming their commands", () => {
    renderArea();
    const twoPass = screen.getByRole("switch", { name: "Two-pass" });
    expect(twoPass).toBeDisabled();
    expect(
      twoPass.closest("[data-pending-command]")?.getAttribute("data-pending-command"),
    ).toBe("audio.set-eq");

    const spatial = screen.getByRole("switch", { name: "Spatial audio" });
    expect(spatial).toBeDisabled();
    expect(
      spatial.closest("[data-pending-command]")?.getAttribute("data-pending-command"),
    ).toBe("audio.set-spatializer");

    const widening = screen.getByRole("switch", { name: "Stereo widening" });
    expect(widening).toBeDisabled();
    expect(
      widening.closest("[data-pending-command]")?.getAttribute("data-pending-command"),
    ).toBe("audio.set-stereo-widener");
  });

  it("the Spatial audio mode dropdown is staged on the frame's option", () => {
    renderArea();
    const mode = screen.getByRole("button", { name: "Spatial audio mode" });
    expect(mode).toBeDisabled();
    expect(mode.textContent).toContain("Headphone virtualization");
    expect(mode.closest("[data-pending-command]")?.getAttribute("data-pending-command")).toBe(
      "audio.set-spatializer",
    );
  });

  it("staged controls never send anything (no live commands left in the area)", () => {
    const { transport } = renderArea();
    // Interact with what renders: expand/collapse the EQ, open the presets.
    fireEvent.click(screen.getByRole("button", { name: COLLAPSE }));
    fireEvent.click(screen.getByRole("button", { name: toolsKeys["player.tools.expand-settings"] }));
    // Only the area's own hydration-free surface — no config.set-setting, no
    // invented writes for commands the engine does not own.
    expect(transport.sent).toEqual([]);
  });
});
