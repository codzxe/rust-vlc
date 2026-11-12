// Effects & Tools area tests (w26.c5 integration; w26.f05 A2 rewrite) —
// the Settings → Audio Effects section composed to `Settings - audio-1.svg`:
// the "Equalizer preset" settings row OUTSIDE the panel (F6), the embedded
// A2 GraphicEQ vertical-fader panel (F5/F1), the ⓘ affordances (F7), and
// the honest-surface rules:
//   • the embedded EQ + every row control is STAGED (no audio-effects
//     catalog command exists yet) — markers name the awaited command, ONE
//     section-level note line gives the honest reason (g12-P3-8: the per-row
//     badges doubled the frame's row pitch), nothing ever sends;
//   • the retired v0 widgets (master volume, mute, balance, video adjust)
//     are gone per the frame — live playback control stays in the player
//     ControlBar (see the effects.tsx module header + the w26.c5 lane log).

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { keys as panelKeys } from "../../../i18n/keys/w26.f05";
import { EffectsArea } from "./effects";
import { makeFixtureTransport } from "./audio-fixtures";

const CLOSE = panelKeys["screen.eq.close-panel"];

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

  it("embeds the A2 GraphicEQ panel EXPANDED (frame EQ block, 'Close Equalizer')", () => {
    renderArea();
    // The panel's expanded toggle + all 11 frame faders (Preamp + 10 bands,
    // vertical per EQ.svg).
    expect(screen.getByRole("button", { name: CLOSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    const faders = screen.getAllByRole("slider");
    expect(faders).toHaveLength(11);
    for (const fader of faders) {
      expect(fader).toHaveAttribute("aria-orientation", "vertical");
    }
    expect(screen.getByRole("slider", { name: "Preamp" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "60 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "16 KHz" })).toBeInTheDocument();
    // The monochrome frame: no band accent dots anywhere (w26.e07 F8).
    expect(document.querySelector(".eq-tool__accent")).toBeNull();
  });

  it("'Equalizer preset' is a separate settings row OUTSIDE the panel (F6)", () => {
    const { container } = renderArea();
    const presetButton = screen.getByRole("button", { name: "Equalizer preset" });
    // The row lives in a settings-field, NOT inside the EQ panel region.
    const row = presetButton.closest(".settings-field");
    expect(row).not.toBeNull();
    const panel = screen.getByRole("region", { name: "Equalizer" });
    expect(row?.contains(panel)).toBe(false);
    expect(panel.contains(presetButton)).toBe(false);
    // The staged marker names the awaited command, like the panel wrapper.
    expect(row?.getAttribute("data-pending-command")).toBe("audio.set-eq");
    expect(container.querySelectorAll(".eq-panel__fader--zero")).toHaveLength(10);
  });

  it("the preset row opens the frame's 8 presets (EQ presets.svg)", () => {
    renderArea();
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
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

  it("a preset pick re-shapes the embedded panel's fader chips (shared state)", () => {
    renderArea();
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Club" }));
    // Club = [0, 0, 4.8, 3.6, 3.6, 3.6, 2.4, 0, 0, 0] — the panel chips follow.
    expect(screen.getAllByText("4.8 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.6 dB")).toHaveLength(3);
    // The trigger reads the applied preset.
    expect(
      screen.getByRole("button", { name: "Equalizer preset" }).textContent,
    ).toContain("Club");
  });

  it("carries the frame's ⓘ affordances on all six marked rows (F7 + w26.i14)", () => {
    renderArea();
    // The frame's 6 circles (y118.5/163.5/707.5/749.5/793.5/837.5): the
    // Equalizer + preset rows AND the four rows below the EQ panel that
    // carried dead `note` props before w26.i14 (h10 defect #3).
    const infoDots = document.querySelectorAll(".effects-info");
    expect(infoDots).toHaveLength(6);
    // Accessible (role img, never aria-hidden) on every marked row.
    const infos = screen.getAllByRole("img", { name: /Engine stage ready/ });
    expect(infos).toHaveLength(6);
    // One on the Equalizer row, one on the Equalizer preset row.
    const eqRow = screen.getByRole("switch", { name: "Equalizer" }).closest(
      ".settings-field",
    );
    const presetRow = screen
      .getByRole("button", { name: "Equalizer preset" })
      .closest(".settings-field");
    expect(eqRow?.querySelector(".effects-info")).not.toBeNull();
    expect(presetRow?.querySelector(".effects-info")).not.toBeNull();
    // The four rows below the panel: Two-pass / Spatial audio / Spatial
    // audio mode / Stereo widening (h10 defect #3 — the frame's ⓘ now
    // renders where the note prop used to be dead).
    for (const label of ["Two-pass", "Spatial audio", "Spatial audio mode", "Stereo widening"]) {
      const row = screen.getByRole(
        label === "Spatial audio mode" ? "button" : "switch",
        { name: label },
      ).closest(".settings-field");
      expect(row?.querySelector(".effects-info"), `${label} carries the frame's ⓘ`).not.toBeNull();
      expect(row?.querySelector(".effects-info")?.getAttribute("title")).toContain(
        "Engine stage ready",
      );
    }
    // g12-P3-8: the staged note rides the ⓘ tooltips (no in-row note lines).
    expect(eqRow?.querySelector(".effects-info")?.getAttribute("title")).toContain(
      "Engine stage ready",
    );
    expect(presetRow?.querySelector(".effects-info")?.getAttribute("title")).toContain(
      "Engine stage ready",
    );
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
    // The honest reason renders — as ONE section-level note line after the
    // last row (g12-P3-8), never as per-row badges between rows.
    const notes = screen.getAllByRole("note");
    expect(notes).toHaveLength(1);
    expect(notes[0]!.className).toContain("settings-area-staged-note");
    expect(notes[0]!.textContent).toContain("Engine stage ready");
  });

  it("Two-pass / Spatial audio / Stereo widening toggles are staged, naming their commands", () => {
    renderArea();
    const twoPass = screen.getByRole("switch", { name: "Two-pass" });
    expect(twoPass).toBeDisabled();
    expect(
      twoPass.closest("[data-pending-command]")?.getAttribute("data-pending-command"),
    ).toBe("audio.set-eq");
    // w26.i14: the frame's 42px row pitch (the 46px default accumulated
    // +16px vs the frame over the four rows below the panel).
    expect(twoPass.closest(".settings-field")).toHaveAttribute("data-kind", "switch");
    expect(
      screen.getByRole("switch", { name: "Stereo widening" }).closest(".settings-field"),
    ).toHaveAttribute("data-kind", "switch");
    expect(
      screen.getByRole("button", { name: "Spatial audio mode" }).closest(".settings-field"),
    ).toHaveAttribute("data-kind", "enum");

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
    // Interact with what renders: pick a preset, scrub a fader by keyboard,
    // collapse + reopen the panel.
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Club" }));
    fireEvent.keyDown(screen.getByRole("slider", { name: "60 Hz" }), {
      key: "ArrowUp",
    });
    fireEvent.click(screen.getByRole("button", { name: CLOSE }));
    fireEvent.click(screen.getByRole("button", { name: "Open Equalizer" }));
    // Only the area's own hydration-free surface — no config.set-setting, no
    // invented writes for commands the engine does not own.
    expect(transport.sent).toEqual([]);
  });
});
