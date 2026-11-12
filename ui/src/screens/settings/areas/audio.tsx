// Settings area: Audio — "Audio Defaults" (Settings - audio.svg, analysis
// tools-settings.md §C6; w26.g12-P0-2 composition fix).
//
// w26.gfix-B COMPOSITION (frame truth, top → bottom; the schema seed carries
// every schema row — the generated section data stays the source of truth):
//   Output (y83) — Enable audio ON (schema) · Output device → dropdown
//     "System default" 320×36 (capability-backed hand row, spliced between
//     the two schema rows — frame order y114/y156/y202) · Speaker setup
//     "Automatic" (schema).
//   Volume (y276) — Maximum volume slider (scale 100 %/200 %/300 %) ·
//     Startup volume = the Values-1 radio rows (a19 B5: "Remember last
//     volume" / "Always starts at: 200%", SettingWidgets) · Volume step
//     input "1" (schema).
//   Loudness (y548) — Normalize loudness OFF · Target level "1" · Night
//     mode OFF · Replay gain "Off" · Replay gain preamp "0.0" (schema).
// The Effects fold (audio-1 variant) renders BELOW this area — the registry
// (areas/index.tsx) composes <AudioArea/> + <EffectsArea/>.
//
// HONEST SURFACE RULES (card AC: config-never-invents):
//   • Output device is a CAPABILITY-BACKED control: `audio.list-devices` /
//     `audio.set-device` feed it from engine enumeration — never invented
//     options. When the capability is absent (product.toml
//     [features] audio-device-enumeration = false) the control renders the
//     CLOSED 320×36 trigger (value "System default" — the honest fallback
//     output), DISABLED WITH THE HONEST REASON below the row — visible,
//     inert, never a 4×4 stub (g12-P1-5).

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { Dropdown } from "../../../components/menu/Dropdown";
import { useClient } from "../../../client/ClientProvider";
import type { CommandResponse } from "../../../client/transport";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { SettingsSections } from "../SettingsSections";
import { schemaSectionsOf } from "../SettingsSections";
import { withRows } from "../SettingsSections";

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

/** The frame's fallback trigger value (C6: dropdown "System default"). */
const SYSTEM_DEFAULT = "system-default";

type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam (w26.gfixB: the fallback value copy). */
const laneCatalog: Readonly<Record<string, string>> = fixerKeys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | FixerKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * Engine-enumerated output-device picker (audio.list-devices /
 * audio.set-device) in the settings row grammar: a closed compact Dropdown
 * (320×36) — the frame's control, never a stacked radio list or a collapsed
 * stub (g12-P1-5).
 */
export function AudioDeviceControl(): ReactElement {
  const m = useAreaMsg();
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

  const label = m("settings.audio.output-device");
  const ready = query.status === "ready";
  const devices = ready ? query.devices : [];
  const options =
    devices.length > 0
      ? devices.map((device) => ({
          value: device.id,
          labelKey: device.isDefault ? `${device.name} — ${m("settings.audio.device-default")}` : device.name,
        }))
      : // Honest inert value while the enumeration is absent/loading: the
        // engine's actual fallback output (never an invented device name).
        [{ value: SYSTEM_DEFAULT, labelKey: m("settings.audio.device-system-default") }];
  const selectedValue = ready ? query.selected : SYSTEM_DEFAULT;
  const reason: MessageKey | null =
    query.status === "loading"
      ? "settings.audio.device-list-loading"
      : query.status === "unavailable"
        ? query.reason
        : null;

  return (
    <div
      className="settings-field"
      data-kind="enum"
      data-setting-capability="audio.list-devices"
      data-capability={query.status}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
        </div>
        <div className="settings-field__control">
          <Dropdown
            className="settings-field__dropdown"
            density="compact"
            label={label}
            options={options}
            value={selectedValue}
            onChange={chooseDevice}
            disabled={!ready}
          />
        </div>
      </div>
      {reason !== null ? (
        <p className="settings-field__badges" role="note" data-capability-reason={query.status}>
          <span className="settings-badge">{m(reason)}</span>
        </p>
      ) : null}
    </div>
  );
}

/**
 * Audio settings area (route /settings/audio; Media group) — the frame's 3
 * schema sections with the capability-backed Output device row interleaved
 * (frame order: Enable audio · Output device · Speaker setup).
 */
export function AudioArea(): ReactElement {
  return (
    <SettingsSections
      area="audio"
      sections={withRows(schemaSectionsOf("audio"), "output", [
        {
          after: "audio.enable",
          row: {
            rowId: "output-device",
            keywords: ["Output device", "System default"],
            node: <AudioDeviceControl />,
          },
        },
      ])}
    />
  );
}

/** Test seam lives in ./audio-fixtures (shared with sibling area tests). */
