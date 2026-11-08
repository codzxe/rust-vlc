import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { NotFoundScreen } from "./NotFoundScreen";

// Solstice v2 stage 404 (brief §4.8): a theatrical full-sunset scene with
// ONE clear CTA home. Regression tests for the rebuilt composition's
// contracts: the vivid backdrop is a mount effect (set while mounted,
// cleaned up on unmount — never leaked onto the rest of the app), the
// "404" page number is decorative (aria-hidden; the real heading carries
// the accessible title), and the primary CTA routes back to the player.

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

describe("NotFoundScreen (Solstice v2 stage)", () => {
  it("renders the lost-note scene with a clear title and one primary CTA", () => {
    render404();
    expect(screen.getByRole("heading", { level: 1, name: "Lost the thread" })).toBeInTheDocument();
    const cta = screen.getByRole("button", { name: "Back to the player" });
    expect(cta).toHaveClass("lumen-button--primary");
  });

  it("the big 404 page number is decorative — never announced", () => {
    render404();
    expect(screen.getByText("404")).toHaveAttribute("aria-hidden", "true");
  });

  it("flips the backdrop to the vivid stage tier only while mounted", () => {
    const utils = render404();
    expect(document.documentElement.dataset.backdrop).toBe("vivid");
    utils.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("the CTA returns to the player", async () => {
    render404();
    fireEvent.click(screen.getByRole("button", { name: "Back to the player" }));
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/player"));
  });
});
