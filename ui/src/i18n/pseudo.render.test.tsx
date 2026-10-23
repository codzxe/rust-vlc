// Pseudo-locale render proof (card 4.H3, doc 07 §6.3).
//
// Renders the key screens with the deterministic pseudo locale active:
//   * no crash,
//   * no missing-key fallback visible (no `undefined`, no `missing:` debug
//     fallback text — doc 07 §6.3 chain),
//   * headings carry the pseudo brackets (the wiring actually reached the
//     screen, not just the hook).
//
// The pixel-level clipping half of the proof belongs to the visual harness
// (G1/G2): `?locale=pseudo` is the supported override (main.tsx bootstrap),
// and harness runs append it to the URL without touching harness internals.
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../client/ClientProvider";
import { MockTransport } from "../client/transport";
import { App } from "../App";
import { PSEUDO_LOCALE_TAG } from "./pseudo";
import { setActiveLocale } from "./messages";

function renderAt(path: string): void {
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </ClientProvider>,
  );
}

beforeEach(() => {
  setActiveLocale(PSEUDO_LOCALE_TAG);
});
afterEach(() => {
  setActiveLocale("en-US");
});

// Key screens per the doc 08 §1.6 route skeleton (mirrors App.test.tsx).
const ROUTES: ReadonlyArray<[name: string, path: string]> = [
  ["player", "/player"],
  ["music mode", "/player/music"],
  ["playlist", "/playlist"],
  ["settings", "/settings/global"],
  ["onboarding", "/onboarding"],
  ["media info", "/media-info"],
  ["not-found", "/definitely-not-a-route"],
];

describe("key screens render under the pseudo locale (card 4.H3)", () => {
  for (const [name, path] of ROUTES) {
    it(`${name} (${path}): no crash, no fallback, pseudo copy visible`, () => {
      expect(() => renderAt(path)).not.toThrow();
      const text = document.body.textContent ?? "";
      expect(text).not.toContain("undefined");
      expect(text).not.toContain("missing:");
      // The pseudo catalog brackets every string — an h1 carrying them proves
      // the locale seam reached the rendered copy.
      const heading = screen.getByRole("heading", { level: 1 });
      expect(heading.textContent ?? "").toMatch(/\[/u);
    });
  }
});
