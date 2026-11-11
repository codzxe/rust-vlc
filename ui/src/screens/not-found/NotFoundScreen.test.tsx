import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { NotFoundScreen } from "./NotFoundScreen";

// The W26 quiet-hero 404 (w26.b13): a centered cone-mark hero with ONE
// clear CTA home — the onboarding grammar, never a theatrical stage.
// Regression tests for the rebuild's contracts: the Solstice data-backdrop
// veil stays retired (mounting touches no tier — W25 §0), the heading is
// the honest "Page not found" copy, and the primary CTA routes back to the
// player.

/** Location probe for navigation assertions. */
function Where(): ReactElement {
  const location = useLocation();
  return <span data-testid="where">{location.pathname}</span>;
}

const render404 = (): ReturnType<typeof render> =>
  render(
    <MemoryRouter initialEntries={["/qwerty"]}>
      <Where />
      <Routes>
        <Route path="/qwerty" element={<NotFoundScreen />} />
        <Route path="/player" element={<span data-testid="player-stub" />} />
      </Routes>
    </MemoryRouter>,
  );

describe("NotFoundScreen (W26 quiet hero)", () => {
  it("renders the centered quiet hero with a clear title and one primary CTA", () => {
    render404();
    expect(
      screen.getByRole("heading", { level: 1, name: "Page not found" }),
    ).toBeInTheDocument();
    const cta = screen.getByRole("button", { name: "Back to the player" });
    expect(cta).toHaveClass("lumen-button--primary");
  });

  it("the cone mark is decorative — never announced", () => {
    render404();
    const mark = document.querySelector(".not-found__cone");
    expect(mark).not.toBeNull();
    expect(mark).toHaveAttribute("aria-hidden", "true");
  });

  it("retires the Solstice stage — mounting touches no data-backdrop tier", () => {
    const utils = render404();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
    utils.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("the CTA returns to the player", async () => {
    render404();
    fireEvent.click(screen.getByRole("button", { name: "Back to the player" }));
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/player"));
  });
});
