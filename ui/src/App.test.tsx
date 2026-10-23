import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { App } from "./App";

function renderAt(path: string): void {
  render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}

// Route smoke over the doc 08 §1.6 skeleton: each design route renders its
// honest placeholder heading.
describe("App routing skeleton (doc 08 §1.6)", () => {
  it("redirects / to the player screen", () => {
    renderAt("/");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
  });

  it("renders the music-player route", () => {
    renderAt("/player/music");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Music player");
  });

  it("renders the settings route with its area segment", () => {
    renderAt("/settings/audio");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    expect(screen.getByText("audio")).toBeInTheDocument();
  });

  it("renders the onboarding route", () => {
    renderAt("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Onboarding");
  });

  it("renders unknown paths as the not-found screen", () => {
    renderAt("/definitely-not-a-route");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("not found");
  });
});
