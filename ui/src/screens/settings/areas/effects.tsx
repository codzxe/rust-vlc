// Settings area: Effects & Tools — issue #33 / D-012 (the ui-screen-av-tools
// row's settings-side v0: audio DSP + video adjust controls).
//
// HONEST SURFACE RULES (same discipline as audio.tsx):
//   • LIVE controls are the ones with a REAL catalog command AND engine
//     support today: master volume (`audio.set-volume`, engine `Volume`),
//     mute (`audio.set-mute`, engine `Volume::MUTE`). They hydrate from
//     `audio.get-volume` and send on change.
//   • PENDING controls are the engine stages that exist WITHOUT a catalog
//     command yet: stereo balance (audio-engine `Balance`, constant-power)
//     and the video adjust family (video-engine `AdjustParams`). They
//     render DISABLED with the honest reason and a data-pending-command
//     marker naming the command they await — visible, inert, never a dead
//     control that looks alive. They light up when the D-012 command-layer
//     follow-up lands (`audio.set-balance`, `video.set-adjust`).
//   • No schemas/settings.toml entries exist for effects yet — when the
//     schema owner seeds them, this area's schema-backed fields move to the
//     generator (same migration note as audio.tsx).

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { useClient } from "../../../client/ClientProvider";
import type { CommandResponse } from "../../../client/transport";
import { Switch } from "../../../components/form";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";

/** Volume wire ceiling (doc 05 §2.2 `audio.max-volume` default 1.25). */
const MAX_VOLUME = 1.25;

type VolumeQuery =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly volume: number };

/** Master volume slider — `audio.get-volume` hydrate, `audio.set-volume` apply. */
export function VolumeControl(): ReactElement {
  const m = useMsg();
  const client = useClient();
  const [query, setQuery] = useState<VolumeQuery>({ status: "loading" });
  const nextRequestId = useRef(1);

  useEffect(() => {
    let cancelled = false;
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.get-volume" })
      .then((response: CommandResponse) => {
        if (cancelled) return;
        const result = response.result;
        if ("completed" in result && result.completed.kind === "audio-volume") {
          setQuery({ status: "ready", volume: result.completed.volume });
        } else {
          // No engine truth yet — start from unity rather than invent one.
          setQuery({ status: "ready", volume: 1.0 });
        }
      });
    return () => {
      cancelled = false;
    };
  }, [client]);

  const change = (volume: number): void => {
    setQuery({ status: "ready", volume });
    client.send({
      requestId: nextRequestId.current++,
      cmd: "audio.set-volume",
      args: { volume },
    });
  };

  return (
    <div className="settings-field" data-setting-key="effects.audio.volume">
      <div className="settings-field__head">
        <span className="settings-field__label">{m("settings.effects.volume")}</span>
        {query.status === "ready" ? (
          <span className="settings-badge">{Math.round(query.volume * 100)}%</span>
        ) : null}
      </div>
      <input
        type="range"
        min={0}
        max={MAX_VOLUME}
        step={0.05}
        value={query.status === "ready" ? query.volume : 1}
        onChange={(event) => change(Number(event.target.value))}
        disabled={query.status !== "ready"}
        aria-label={m("settings.effects.volume")}
      />
    </div>
  );
}

/** Mute switch — `audio.set-mute`, independent of volume (doc 05 §2.6). */
export function MuteControl(): ReactElement {
  const m = useMsg();
  const client = useClient();
  const [muted, setMuted] = useState(false);
  const nextRequestId = useRef(1);

  const change = (next: boolean): void => {
    setMuted(next);
    client.send({
      requestId: nextRequestId.current++,
      cmd: "audio.set-mute",
      args: { muted: next },
    });
  };

  return (
    <div className="settings-field" data-setting-key="effects.audio.mute">
      <div className="settings-field__head">
        <span className="settings-field__label">{m("settings.effects.mute")}</span>
      </div>
      <Switch checked={muted} onChange={change} aria-label={m("settings.effects.mute")} />
    </div>
  );
}

/**
 * A pending control: engine stage exists, catalog command does not yet.
 * Rendered inert with the honest reason (never a dead-but-alive control).
 */
function PendingSlider(props: {
  label: MessageKey;
  min: number;
  max: number;
  step: number;
  pendingCommand: string;
}): ReactElement {
  const m = useMsg();
  return (
    <div
      className="settings-field"
      data-setting-key={`pending.${props.pendingCommand}`}
      data-pending-command={props.pendingCommand}
    >
      <div className="settings-field__head">
        <span className="settings-field__label">{m(props.label)}</span>
      </div>
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step}
        value={(props.min + props.max) / 2}
        disabled
        aria-label={m(props.label)}
      />
      <p className="settings-field__badges" role="note">
        <span className="settings-badge">{m("settings.effects.pending-command")}</span>
      </p>
    </div>
  );
}

/** Effects & Tools settings area (hand-authored; see module header). */
export function EffectsArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="effects">
      <section className="settings-section" data-section="audio">
        <h2 className="settings-section__title">{m("settings.section.effects.audio")}</h2>
        <VolumeControl />
        <MuteControl />
        <PendingSlider
          label="settings.effects.balance"
          min={-1}
          max={1}
          step={0.05}
          pendingCommand="audio.set-balance"
        />
      </section>
      <section className="settings-section" data-section="video">
        <h2 className="settings-section__title">{m("settings.section.effects.video")}</h2>
        <PendingSlider
          label="settings.effects.brightness"
          min={0}
          max={2}
          step={0.05}
          pendingCommand="video.set-adjust"
        />
        <PendingSlider
          label="settings.effects.contrast"
          min={0}
          max={2}
          step={0.05}
          pendingCommand="video.set-adjust"
        />
        <PendingSlider
          label="settings.effects.saturation"
          min={0}
          max={3}
          step={0.05}
          pendingCommand="video.set-adjust"
        />
        <PendingSlider
          label="settings.effects.hue"
          min={-180}
          max={180}
          step={1}
          pendingCommand="video.set-adjust"
        />
      </section>
    </div>
  );
}
