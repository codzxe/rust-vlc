// w26.gfix-B — Video area tests (g12-P0-1 rewrite): the area composes the
// frame's 4 sections (Settings - video.svg §C5) from the schema seed — Output
// (4 toggles), Picture (5 inline sliders + the Reset picture action row),
// Deinterlacing (2 dropdowns), Framing (2 dropdowns) — every row bound to its
// real schema key, the Reset action wired to the settings store, and no
// invented engine data (the area never queries the transport).

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { makeFixtureTransport } from "./audio-fixtures";
import { VideoArea, ResetPictureRow } from "./video";
import { SettingsProvider } from "../SettingsProvider";
import { SETTINGS_ENTRIES } from "../generated/schema";

const VIDEO_ENTRIES = SETTINGS_ENTRIES.filter((entry) => entry.screen === "video");

const mountArea = (transport = makeFixtureTransport(() => ({ completed: { kind: "unit" } }))) =>
  render(
    <ClientProvider transport={transport}>
      <SettingsProvider>
        <VideoArea />
      </SettingsProvider>
    </ClientProvider>,
  );

describe("VideoArea — the frame's 4 sections (Settings - video.svg §C5)", () => {
  it("renders every schema row bound to its key, in the frame's sections", () => {
    const { container } = mountArea();
    expect(document.querySelector('[data-settings-area="video"]')).not.toBeNull();
    for (const entry of VIDEO_ENTRIES) {
      expect(entry.featureGate, `video entry ${entry.key} is ungated (frame-annotated seed)`).toBe("");
      const field = container.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `video entry ${entry.key} must render bound to its schema key`).not.toBeNull();
    }
    // The frame's Output toggles render in their frame states.
    expect(screen.getByRole("switch", { name: "Enable video" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("switch", { name: "Start videos in fullscreen" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(screen.getByRole("switch", { name: "Stop the screensaver during playback" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("switch", { name: "Play video as desktop wallpaper" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    // Deinterlacing / Framing — closed dropdowns on the frame's values.
    expect(screen.getByRole("button", { name: "Deinterlacing" })).toHaveTextContent("Automatic");
    expect(screen.getByRole("button", { name: "Method" })).toHaveTextContent("Automatic");
    expect(screen.getByRole("button", { name: "Aspect ratio" })).toHaveTextContent("Automatic");
    expect(screen.getByRole("button", { name: "Crop" })).toHaveTextContent("None");
  });

  it("Aspect ratio / Crop options carry real labels — never blank (w26.i14, h09 defect #1)", () => {
    mountArea();
    // The ship-blocker: 10 of 12 non-default options rendered EMPTY labels
    // (unregistered msgids) and picking one blanked the trigger. Open both
    // lists and pin every option's accessible name + a real pick.
    fireEvent.click(screen.getByRole("button", { name: "Aspect ratio" }));
    for (const label of ["Automatic", "1:1", "4:3", "16:9", "16:10", "2.35:1", "2.39:1"]) {
      expect(screen.getByRole("option", { name: label }), `aspect option "${label}"`).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("option", { name: "16:9" }));
    // The trigger reads the picked value back — the blank-trigger bug is dead.
    expect(screen.getByRole("button", { name: "Aspect ratio" })).toHaveTextContent("16:9");
    expect(document.querySelector('[data-setting="video.aspect-ratio"]')).toHaveAttribute("data-dirty");

    fireEvent.click(screen.getByRole("button", { name: "Crop" }));
    for (const label of ["None", "1:1", "4:3", "16:9", "16:10"]) {
      expect(screen.getByRole("option", { name: label }), `crop option "${label}"`).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("option", { name: "4:3" }));
    expect(screen.getByRole("button", { name: "Crop" })).toHaveTextContent("4:3");
  });

  it("Picture: the five frame rows are inline sliders, never number wells (g12-P1-3)", () => {
    mountArea();
    const pictureKeys = ["video.hue", "video.brightness", "video.contrast", "video.saturation", "video.gamma"];
    for (const key of pictureKeys) {
      const row = document.querySelector(`[data-setting="${key}"]`);
      expect(row, `${key} renders`).not.toBeNull();
      expect(row?.getAttribute("data-kind")).toBe("slider");
      const slider = row?.querySelector('[role="slider"]');
      expect(slider, `${key} is a slider`).not.toBeNull();
      expect(row?.querySelector("input")).toBeNull(); // never a bare input well
    }
    // Frame anatomy: hue spans −180…180, the scale-less inline layout.
    const hue = screen.getByRole("slider", { name: "Hue" });
    expect(hue).toHaveAttribute("aria-valuemin", "-180");
    expect(hue).toHaveAttribute("aria-valuemax", "180");
    expect(hue).toHaveAttribute("aria-valuenow", "0"); // frame default
    // w26.gfix-B probe fix (frame thumb x806/x730, fills 105/32 of 320): the
    // saturation/gamma ranges are the ffmpeg eq domains — 0..300 / 0..1000 —
    // so the defaults sit at the frame positions, not 50%.
    const saturation = screen.getByRole("slider", { name: "Saturation" });
    expect(saturation).toHaveAttribute("aria-valuemin", "0");
    expect(saturation).toHaveAttribute("aria-valuemax", "300");
    const gamma = screen.getByRole("slider", { name: "Gamma" });
    expect(gamma).toHaveAttribute("aria-valuemin", "0");
    expect(gamma).toHaveAttribute("aria-valuemax", "1000");
  });

  it("Reset picture restores the five picture keys through the store (g12-P2-7)", () => {
    mountArea();
    // Make two picture rows dirty via the sliders' keyboard commits.
    fireEvent.keyDown(screen.getByRole("slider", { name: "Hue" }), { key: "ArrowUp" });
    fireEvent.keyDown(screen.getByRole("slider", { name: "Gamma" }), { key: "ArrowDown" });
    expect(document.querySelector('[data-setting="video.hue"]')).toHaveAttribute("data-dirty");
    expect(document.querySelector('[data-setting="video.gamma"]')).toHaveAttribute("data-dirty");
    // The frame's action row: label left, Reset 100×32 right (§C5 y551).
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(document.querySelector('[data-setting="video.hue"]')).not.toHaveAttribute("data-dirty");
    expect(document.querySelector('[data-setting="video.gamma"]')).not.toHaveAttribute("data-dirty");
    expect(screen.getByRole("slider", { name: "Hue" })).toHaveAttribute("aria-valuenow", "0");
  });

  it("renders the Reset picture action row in the frame's row grammar", () => {
    render(
      <SettingsProvider>
        <ResetPictureRow />
      </SettingsProvider>,
    );
    const row = document.querySelector('[data-video-action="reset-picture"]');
    expect(row).not.toBeNull();
    expect(row?.querySelector(".settings-field__label")).toHaveTextContent("Reset picture");
    expect(screen.getByRole("button", { name: "Reset" })).toBeEnabled(); // real behavior, never dead
  });

  it("never queries the engine (no display/codec enumeration exists — config-never-invents)", () => {
    const transport = makeFixtureTransport(() => ({ completed: { kind: "unit" } }));
    mountArea(transport);
    expect(transport.sent).toHaveLength(0);
  });
});
