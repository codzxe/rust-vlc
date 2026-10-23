// Test seam for the audio area (card 4.E3): a configurable command
// transport. Kept out of audio.tsx so the component module only exports
// components (react-refresh) — and shared with the video/subtitles area
// tests, which assert those areas never touch the transport at all.

import type { CommandRequest } from "../../../client/transport";
import type { CommandResponse } from "../../../client/transport";
import type { CommandTransport } from "../../../client/transport";

export type FixtureResponder = (request: CommandRequest) => CommandResponse["result"];

export function makeFixtureTransport(
  respond: FixtureResponder,
): CommandTransport & { readonly sent: readonly CommandRequest[] } {
  const sent: CommandRequest[] = [];
  let generation = 0;
  return {
    sent,
    subscribe: () => () => undefined,
    async send(request) {
      sent.push(request);
      generation += 1;
      const gen: CommandResponse["gen"] = generation;
      return { gen, requestId: request.requestId, result: respond(request) };
    },
  };
}
