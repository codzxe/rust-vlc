// w26.f04 — Subtitles area tests: the `Settings - subtitles.svg` frame's 3
// sections / 14 rows render as STAGED controls (visible, inert, honestly
// marked — the effects-area pattern), the frame-verbatim values are shown,
// and the config-never-invents rules hold: zero transport calls, no schema
// bindings, and no duplication of the playback screen's subtitle options
// (playback.preferred-subtitle-languages / playback.subtitle-mode, doc 07
// §5.2 D-rules).

import { fireEvent, render, screen, within } from "@testing-library/react";
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
    // w26.k13 (j13 P2-3): the shared SettingInfoDot button grammar — the
    // local 14px twin class is gone.
    expect(container.querySelectorAll(".settings-field__info")).toHaveLength(7);
  });

  it("carries the staged reason on the ⓘ-less rows as a visually hidden note (w26.k13, j13 P2-2)", () => {
    const { container } = renderArea();
    // The 7 rows the frame marks with no circle: the reason is still
    // reachable for keyboard/AT users (sr-only twin in the row lead) —
    // never a `title` on a non-focusable div alone. (The ⓘ rows reach it
    // through their own operable reveal + the dot's description twin.)
    const stagedRows = container.querySelectorAll("[data-staged-row]");
    expect(stagedRows).toHaveLength(14);
    expect(container.querySelectorAll(".settings-field__info")).toHaveLength(7);
    const bare = [...stagedRows].filter(
      (row) => !row.querySelector(".settings-field__info"),
    );
    expect(bare).toHaveLength(7);
    for (const row of bare) {
      expect(row.querySelector(".settings-vh")?.textContent).toContain(
        "controls activate when the subtitles settings seed lands",
      );
    }
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
    // w26.i14 (h10 defect #4): the honesty is keyboard-reachable — the 7 ⓘ
    // are operable BUTTONS carrying the staged note (aria-expanded + reveal
    // on activation), never mouse-only aria-hidden glyphs.
    const infoButtons = screen.getAllByRole("button", {
      name: "Staged — controls activate when the subtitles settings seed lands",
    });
    expect(infoButtons).toHaveLength(7);
    for (const button of infoButtons) {
      expect(button).toHaveAttribute("aria-expanded", "false");
    }
    // Activation reveals the honest note under the row (real behavior).
    // w26.k13: the caption is queried directly — the shared SettingInfoDot
    // also carries the note as its sr-only description twin (w26.k12,
    // j12 P2-3), so a bare getByText would match both.
    fireEvent.click(infoButtons[0]!);
    expect(infoButtons[0]).toHaveAttribute("aria-expanded", "true");
    const caption = infoButtons[0]!.closest(".settings-field")?.querySelector(
      ".settings-field__hinttext",
    );
    expect(caption?.textContent).toContain(
      "controls activate when the subtitles settings seed lands",
    );
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
