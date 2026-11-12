// w26.f04 — Subtitles area tests: the `Settings - subtitles.svg` frame's 3
// sections / 14 rows render as STAGED controls (visible, inert, honestly
// marked — the effects-area pattern), the frame-verbatim values are shown,
// and the config-never-invents rules hold: zero transport calls, no schema
// bindings, and no duplication of the playback screen's subtitle options
// (playback.preferred-subtitle-languages / playback.subtitle-mode, doc 07
// §5.2 D-rules).

import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { makeFixtureTransport } from "./audio-fixtures";
import { SubtitlesArea } from "./subtitles";

const renderArea = () => {
  const transport = makeFixtureTransport(() => ({ completed: { kind: "unit" } }));
  const view = render(
    <ClientProvider transport={transport}>
      <SubtitlesArea />
    </ClientProvider>,
  );
  return { ...view, transport, sections: view.container.querySelectorAll<HTMLElement>("section.settings-section") };
};

const ROW_LABELS: readonly string[] = [
  "Enable subtitles",
  "Preferred language",
  "Text encoding",
  "Load subtitle files automatically",
  "File name matching",
  "Font",
  "Size values",
  "Font size",
  "Bold",
  "Text color",
  "Text opacity",
  "Outline thickness",
  "Outline color",
  "Shadow",
];

describe("SubtitlesArea — the frame's three sections (Settings - subtitles.svg)", () => {
  it("renders the sections in frame order under the area root", () => {
    const { container, sections } = renderArea();
    expect(container.querySelector('[data-settings-area="subtitles"]')).not.toBeNull();
    expect(Array.from(sections).map((section) => section.dataset.section)).toEqual([
      "tracks-languages",
      "loading",
      "appearance",
    ]);
    expect(
      within(sections[0]!).getByRole("heading", { name: "Tracks & Languages" }),
    ).toBeInTheDocument();
    expect(within(sections[1]!).getByRole("heading", { name: "Loading Subtitles" })).toBeInTheDocument();
    expect(within(sections[2]!).getByRole("heading", { name: "Appearance" })).toBeInTheDocument();
  });

  it("renders all 14 frame rows", () => {
    renderArea();
    for (const label of ROW_LABELS) {
      expect(screen.getByText(label), `frame row "${label}"`).toBeInTheDocument();
    }
  });

  it("Tracks & Languages: frame toggles/dropdowns with the frame's values", () => {
    const { sections } = renderArea();
    const section = within(sections[0]!);
    const enable = section.getByRole("switch", { name: "Enable subtitles" });
    expect(enable).toBeDisabled();
    expect(enable).toHaveAttribute("aria-checked", "true"); // frame: ON
    const language = section.getByRole("button", { name: "Preferred language" });
    expect(language).toBeDisabled();
    expect(section.getByText("System language")).toBeInTheDocument();
    const encoding = section.getByRole("button", { name: "Text encoding" });
    expect(encoding).toBeDisabled();
    expect(section.getByText("Automatic")).toBeInTheDocument();
  });

  it("Loading Subtitles: frame toggle + the reused match-strategy value", () => {
    const { sections } = renderArea();
    const section = within(sections[1]!);
    const autoLoad = section.getByRole("switch", { name: "Load subtitle files automatically" });
    expect(autoLoad).toBeDisabled();
    expect(autoLoad).toHaveAttribute("aria-checked", "true"); // frame: ON
    const matching = section.getByRole("button", { name: "File name matching" });
    expect(matching).toBeDisabled();
    // frame-verbatim option; the copy reuses the registered
    // playlist.subtitles.match.extra (sub filename.svg) — no re-declared key.
    expect(section.getByText("Title with extra characters")).toBeInTheDocument();
  });

  it("Appearance: font/size/values/colors/opacity/shadow per the frame", () => {
    const { sections } = renderArea();
    const section = within(sections[2]!);
    // Font — the frame's TEXT input well (318×34, no chevron; g13-P2-2).
    const font = section.getByRole("textbox", { name: "Font" }) as HTMLInputElement;
    expect(font).toBeDisabled();
    expect(font).toHaveValue("System, sans-serif");
    // Size values — segmented Relative (active) | Pixels.
    const segmented = section.getByRole("radiogroup", { name: "Size values" });
    expect(segmented).toBeInTheDocument();
    const relative = within(segmented).getByRole("radio", { name: "Relative" });
    const pixels = within(segmented).getByRole("radio", { name: "Pixels" });
    expect(relative).toHaveAttribute("aria-checked", "true");
    expect(relative).toBeDisabled();
    expect(pixels).toHaveAttribute("aria-checked", "false");
    expect(pixels).toBeDisabled();
    // Font size — dropdown "Automatic".
    expect(section.getByRole("button", { name: "Font size" })).toBeDisabled();
    // Bold — frame OFF.
    const bold = section.getByRole("switch", { name: "Bold" });
    expect(bold).toBeDisabled();
    expect(bold).toHaveAttribute("aria-checked", "false");
    // Text / Outline colors — hex fields + 22×22 swatches (value = data).
    expect(section.getByText("#FFFFFF")).toBeInTheDocument();
    expect(section.getByText("#000000")).toBeInTheDocument();
    const textSwatch = section
      .getByText("#FFFFFF")
      .closest(".settings-area-subtitles-color")
      ?.querySelector<HTMLElement>(".settings-area-subtitles-color-swatch");
    expect(textSwatch?.style.getPropertyValue("--settings-area-subtitles-swatch")).toBe(
      "#FFFFFF",
    );
    // Text opacity — slider at the frame's 50% + the 0/50/100% scale.
    const opacity = section.getByRole("slider", { name: "Text opacity" });
    expect(opacity).toHaveAttribute("aria-valuenow", "50");
    expect(opacity).toHaveAttribute("aria-valuetext", "50%");
    expect(opacity).toHaveAttribute("aria-disabled", "true");
    expect(section.getByText("0%")).toBeInTheDocument();
    expect(section.getByText("50%")).toBeInTheDocument();
    expect(section.getByText("100%")).toBeInTheDocument();
    // Outline thickness — dropdown "Normal"; Shadow — frame ON.
    expect(section.getByRole("button", { name: "Outline thickness" })).toBeDisabled();
    expect(section.getByText("Normal")).toBeInTheDocument();
    const shadow = section.getByRole("switch", { name: "Shadow" });
    expect(shadow).toBeDisabled();
    expect(shadow).toHaveAttribute("aria-checked", "true");
  });

  it("carries the frame's 9 degree ticks on the opacity track", () => {
    const { container } = renderArea();
    expect(container.querySelectorAll(".settings-area-subtitles-opacity-tick")).toHaveLength(9);
  });

  it("pins the ⓘ info glyph after the 7 labels the frame marks", () => {
    const { container } = renderArea();
    // Tracks & Languages ×3, Loading Subtitles ×2, Font + Size values.
    expect(container.querySelectorAll(".settings-area-subtitles-info")).toHaveLength(7);
  });
});

describe("SubtitlesArea — honest staged surface (config-never-invents)", () => {
  it("marks every row staged with its pending schema key — no live bindings", () => {
    const { container } = renderArea();
    const staged = container.querySelectorAll('[data-staged="subtitles-settings-seed"]');
    expect(staged).toHaveLength(14);
    for (const row of staged) {
      expect(row.getAttribute("data-setting-key")).toMatch(/^pending\.subtitles\./);
    }
    // w26.gfix-B (g13-P1-1): NO in-flow note lines — the frame carries none
    // (they pushed 3 rows below the 865px fold); the honesty lives in the 7
    // ⓘ titles (the staged note as their accessible name).
    expect(screen.queryAllByRole("note")).toHaveLength(0);
    expect(
      screen.getAllByRole("img", {
        name: "Staged — controls activate when the subtitles settings seed lands",
      }),
    ).toHaveLength(7);
  });

  it("never touches the transport (no engine seam exists for these rows)", () => {
    const { transport } = renderArea();
    expect(transport.sent).toHaveLength(0);
  });

  it("binds no schema setting and does not duplicate the playback screen's options (D-rules)", () => {
    const { container } = renderArea();
    // playback.preferred-subtitle-languages / playback.subtitle-mode are
    // the generated PlaybackScreen's rows — never defined twice.
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
    expect(screen.queryByText("Preferred subtitle languages")).toBeNull();
    expect(screen.queryByText("Subtitle mode")).toBeNull();
  });
});
