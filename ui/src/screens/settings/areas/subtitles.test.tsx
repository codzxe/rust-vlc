// Card 4.E3 — Subtitles/OSD area tests: absent schema entries render
// NOTHING, and options owned by other screens (playback track preferences,
// controls OSD duration) are not duplicated here (no option defined twice).

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { makeFixtureTransport } from "./audio-fixtures";
import { SubtitlesArea } from "./subtitles";

const renderArea = () =>
  render(
    <ClientProvider transport={makeFixtureTransport(() => ({ completed: { kind: "unit" } }))}>
      <SubtitlesArea />
    </ClientProvider>,
  );

describe("SubtitlesArea — honest empty surface (config-never-invents)", () => {
  it("renders the honest empty state and zero controls", () => {
    const { container } = renderArea();
    expect(container.querySelector('[data-settings-area="subtitles"]')).not.toBeNull();
    expect(screen.getByTestId("settings-area-empty")).toBeVisible();
    expect(container.querySelectorAll(".settings-field")).toHaveLength(0);
  });

  it("does not duplicate the playback screen's subtitle-language option (D-rules: no double definition)", () => {
    renderArea();
    // "Preferred subtitle languages" is playback.preferred-subtitle-languages,
    // already rendered by the generated PlaybackScreen — never twice.
    expect(screen.queryByText("Preferred subtitle languages")).toBeNull();
    expect(screen.queryByText("Subtitle mode")).toBeNull();
  });

  it("renders no OSD controls — controls.osd-duration-ms belongs to the controls screen", () => {
    const { container } = renderArea();
    expect(screen.queryByText(/OSD/i)).toBeNull();
    expect(container.querySelectorAll('[data-setting], [data-setting-capability]')).toHaveLength(0);
  });
});
