import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { CommandTransport } from "./transport";

// React context over the command-layer transport (doc 08 §8 "app/ root
// providers"). Hosts inject the real transport; tests and the scaffold use
// MockTransport. No Tauri/wasm imports are allowed here — hosts configure.
const ClientContext = createContext<CommandTransport | null>(null);

export function ClientProvider({
  transport,
  children,
}: {
  transport: CommandTransport;
  children: ReactNode;
}) {
  const value = useMemo(() => transport, [transport]);
  return <ClientContext.Provider value={value}>{children}</ClientContext.Provider>;
}

export function useClient(): CommandTransport {
  const ctx = useContext(ClientContext);
  if (ctx === null) {
    throw new Error("useClient requires a ClientProvider ancestor");
  }
  return ctx;
}
