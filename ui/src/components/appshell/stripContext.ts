// w26.g03 P1-2 (Lead): the player control band spans the FULL window width
// under rail + sidebar (Player.svg: strip translate(0,1026) width=1920; rail
// and sidebar end above it). The AppShell renders a strip slot after its
// frame row and exposes it through this context; PlayerScreen portals its
// footer region into it when mounted inside the shell. Outside the shell
// (tests, isolated hosts) the context is null and the footer renders in
// place — zero behavior change for every existing test.

import { createContext, useContext } from "react";

/** The live strip host element (null until the shell footer mounts, or when
 * no shell is present). Value is a STATE-backed element so consumers
 * re-render when the host lands. */
export type AppShellStripHost = HTMLDivElement | null;

export const AppShellStripContext = createContext<AppShellStripHost>(null);

export function useAppShellStrip(): AppShellStripHost {
  return useContext(AppShellStripContext);
}
