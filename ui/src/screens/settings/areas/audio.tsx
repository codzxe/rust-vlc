// Settings area: Audio — card 4.E3, doc 07 §5.4 screen "audio".
//
// HONEST SURFACE RULES (card AC: config-never-invents):
//   • Output device is a CAPABILITY-BACKED control: the command catalog
//     documents `audio.list-devices` / `audio.set-device`, so the control is
//     real product surface fed by engine enumeration — never invented
//     options. When the engine reports the capability absent (product.toml
//     [features] `audio-device-enumeration = false`; MockTransport answers
//     the typed `feature-not-exposed` envelope) the control renders
//     DISABLED WITH THE HONEST REASON — visible, inert, never enabled.
//   • Audio exclusivity mode, volume-default/ceiling, normalize and
//     visualizer have NO entries in schemas/settings.toml (doc 07 §5.4
//     seed names none of them for this screen yet) → rendered as NOTHING
//     here, with a handoff note in ui/NEXT-STEPS.md for the schema owner.
//     Rule: never a dead control that looks alive.

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { useClient } from "../../../client/ClientProvider";
import type { CommandResponse } from "../../../client/transport";
import { RadioGroup } from "../../../components/form/RadioGroup";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";

// Wire types derived through the transport seam (transport.ts is the only
// module that reaches into src/generated — we keep it that way).
type Completed = Extract<CommandResponse["result"], { completed: unknown }>["completed"];
type AudioDevicesResult = Extract<Completed, { kind: "audio-devices" }>;
export type AudioDevice = AudioDevicesResult["devices"][number];

/** Enumeration query state machine — `unavailable` always carries a reason. */
type DeviceQuery =
  | { readonly status: "loading" }
  | { readonly status: "unavailable"; readonly reason: MessageKey }
  | { readonly status: "ready"; readonly devices: readonly AudioDevice[]; readonly selected: string };

const defaultDevice = (devices: readonly AudioDevice[]): string =>
  devices.find((device) => device.isDefault)?.id ?? devices[0]?.id ?? "";

const reasonFor = (code: string): MessageKey =>
  // The card's key honest rule: the exact absent-capability reason.
  code === "feature-not-exposed"
    ? "settings.audio.device-enumeration-unavailable"
    : "settings.capability-unavailable";

/** Engine-enumerated output-device picker (audio.list-devices / .set-device). */
export function AudioDeviceControl(): ReactElement {
  const m = useMsg();
  const client = useClient();
  const [query, setQuery] = useState<DeviceQuery>({ status: "loading" });
  const nextRequestId = useRef(1);

  useEffect(() => {
    let cancelled = false;
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.list-devices" })
      .then((response: CommandResponse) => {
        if (cancelled) return;
        const result = response.result;
        if ("completed" in result && result.completed.kind === "audio-devices") {
          const devices = result.completed.devices;
          setQuery(
            devices.length === 0
              ? { status: "unavailable", reason: "settings.audio.no-devices" }
              : { status: "ready", devices, selected: defaultDevice(devices) },
          );
        } else if ("error" in result) {
          setQuery({ status: "unavailable", reason: reasonFor(result.error.code) });
        } else {
          // `accepted` with no payload yet (doc 05 §1 async completion) — the
          // honest stand-in until events deliver the listing.
          setQuery({ status: "unavailable", reason: "settings.capability-unavailable" });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  const chooseDevice = (id: string): void => {
    if (query.status !== "ready" || id === query.selected) return;
    const previous = query.selected;
    // Optimistic accept; a failed apply reverts (accepted-vs-completed, doc 05 §1).
    setQuery({ ...query, selected: id });
    client
      .send({ requestId: nextRequestId.current++, cmd: "audio.set-device", args: { id } })
      .then((response: CommandResponse) => {
        if ("error" in response.result) {
          setQuery((current) => (current.status === "ready" ? { ...current, selected: previous } : current));
        }
      });
  };

  const options =
    query.status === "ready"
      ? query.devices.map((device) => ({
          value: device.id,
          label: device.isDefault ? `${device.name} — ${m("settings.audio.device-default")}` : device.name,
        }))
      : [];
  const selectedValue = query.status === "ready" ? query.selected : "";
  const reason: MessageKey | null =
    query.status === "loading"
      ? "settings.audio.device-list-loading"
      : query.status === "unavailable"
        ? query.reason
        : null;

  return (
    <div
      className="settings-field"
      data-setting-capability="audio.list-devices"
      data-capability={query.status}
    >
      <div className="settings-field__head">
        <span className="settings-field__label">{m("settings.audio.output-device")}</span>
      </div>
      <RadioGroup
        label={m("settings.audio.output-device")}
        options={options}
        value={selectedValue}
        onChange={chooseDevice}
        disabled={query.status !== "ready"}
      />
      {reason !== null ? (
        <p className="settings-field__badges" role="note" data-capability-reason={query.status}>
          <span className="settings-badge">{m(reason)}</span>
        </p>
      ) : null}
    </div>
  );
}

/**
 * Audio settings area. Integration note: schemas/settings.toml [areas].known
 * lists "audio", so once entries land the generator emits this screen; until
 * then this module is the hand-authored surface (one capability-backed
 * control, zero schema-invented controls).
 */
export function AudioArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="audio">
      <section className="settings-section" data-section="output">
        <h2 className="settings-section__title">{m("settings.section.audio.output")}</h2>
        <AudioDeviceControl />
      </section>
      {/* audio exclusivity mode: no schema key → nothing rendered (see header). */}
    </div>
  );
}

/** Test seam lives in ./audio-fixtures (shared with sibling area tests). */