// Card 4.E3 — Video area tests: absent schema entries render NOTHING
// (never a dead control that looks alive), and the area never invents
// engine data (no transport calls at all).

import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { makeFixtureTransport } from "./audio-fixtures";
import { VideoArea } from "./video";

describe("VideoArea — honest empty surface (config-never-invents)", () => {
  it("renders the honest empty state and zero controls", () => {
    const { container } = render(
      <ClientProvider transport={makeFixtureTransport(() => ({ completed: { kind: "unit" } }))}>
        <VideoArea />
      </ClientProvider>,
    );
    expect(container.querySelector('[data-settings-area="video"]')).not.toBeNull();
    expect(screen.getByTestId("settings-area-empty")).toBeVisible();
    expect(container.querySelectorAll(".settings-field")).toHaveLength(0);
  });

  it("renders no deinterlace control — schema entry absent → nothing, not a disabled dead control", () => {
    const { container } = render(
      <ClientProvider transport={makeFixtureTransport(() => ({ completed: { kind: "unit" } }))}>
        <VideoArea />
      </ClientProvider>,
    );
    expect(screen.queryByText(/deinterlace/i)).toBeNull();
    expect(container.querySelectorAll('[data-setting], [data-setting-capability]')).toHaveLength(0);
  });

  it("renders no display selection and never queries the engine (no display enumeration exists)", async () => {
    const transport = makeFixtureTransport(() => ({ completed: { kind: "unit" } }));
    render(
      <ClientProvider transport={transport}>
        <VideoArea />
      </ClientProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("video-area")).toBeInTheDocument());
    expect(transport.sent).toHaveLength(0); // no invented capability probing
  });
});
