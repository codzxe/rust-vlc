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
//     output), DISABLED WITH THE HONEST REASON — visible, inert, never a 4×4
//     stub (g12-P1-5).
//   • w26.i14 (h10 defects #2/#3/#5): the not-ready states reuse the
//     staged-inert grammar — FULL-STRENGTH trigger (never the 0.5 opacity
//     dimming that contradicted the staged siblings 700px below), the reason
//     rides the frame's ⓘ affordance (r7.5 circle, Settings - audio.svg
//     Output rows) + a screen-reader note — NO in-flow badge line (the
//     "Device enumeration is not available" pill injected +22px no frame
//     carries: Speaker setup y224 vs frame 202, Effects header +39).
//   • w26.i14 (h10 defect #1): the frame hangs a ⓘ circle on 10 of the 11
//     rows (Output×3 · Startup volume · Volume step · the 5 Loudness rows;
//     only the Maximum-volume slider carries none) — so this module
//     hand-renders its sections (the InterfaceArea/PrivacyArea precedent) to
//     pass per-row `hint` props through SettingField, with this lane's hint
//     copy (ui/src/i18n/keys/w26.i14.ts). The screen stays schema-driven:
//     sections/entries/order come from the generated seed.

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { Dropdown } from "../../../components/menu/Dropdown";
import { useClient } from "../../../client/ClientProvider";
import type { CommandResponse } from "../../../client/transport";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { keys as i14Keys } from "../../../i18n/keys/w26.i14";
import type { SettingEntry } from "../generated/schema";
import { schemaSectionsOf } from "../SettingsSections";
import { SettingField } from "../SettingField";
import { SettingInfoDot } from "../SettingField";
import { useSettingsSearch } from "../search";
import "./audio.css";

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
type I14Key = keyof typeof i14Keys;
/** Pre-registration seam (w26.gfixB: the fallback value copy). */
const laneCatalog: Readonly<Record<string, string>> = fixerKeys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | FixerKey | I14Key): string => m(key as MessageKey) ?? laneCatalog[key];
}

/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;

/** ⓘ hint msgids per schema row (w26.i14, h10 #1 — the frame's 10 circles;
 * the Maximum-volume slider row carries none, reproduced honestly). */
const AUDIO_HINTS: Readonly<Record<string, MessageKey>> = {
  "audio.enable": hintMsg("settings.hint.audio.enable"),
  "audio.speaker-setup": hintMsg("settings.hint.audio.speaker-setup"),
  "audio.startup-volume": hintMsg("settings.hint.audio.startup-volume"),
  "audio.volume-step": hintMsg("settings.hint.audio.volume-step"),
  "audio.normalize-loudness": hintMsg("settings.hint.audio.normalize-loudness"),
  "audio.target-level": hintMsg("settings.hint.audio.target-level"),
  "audio.night-mode": hintMsg("settings.hint.audio.night-mode"),
  "audio.replay-gain": hintMsg("settings.hint.audio.replay-gain"),
  "audio.replay-gain-preamp": hintMsg("settings.hint.audio.replay-gain-preamp"),
};

/** The hand row's search terms (SettingsSections' SettingRowNode contract). */
const OUTPUT_DEVICE_KEYWORDS = ["Output device", "System default"] as const;

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
  // Resolved ONCE — the ⓘ title, the trigger state and the note all read
  // the same honest reason string (no layout footprint, w26.i14). When the
  // enumeration is live the ⓘ carries the row's plain hint, exactly like
  // every other ⓘ on the screen (the frame's circle is unconditional).
  const reasonText = reason !== null ? m(reason) : null;
  const infoNote = reasonText ?? m("settings.hint.audio.output-device");

  return (
    <div
      className="settings-field"
      data-kind="enum"
      data-setting-capability="audio.list-devices"
      data-capability={query.status}
      data-staged={ready ? undefined : "audio-device-enumeration"}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
          <SettingInfoDot note={infoNote} label={m("settings.field.more-info")} />
          {reasonText !== null ? (
            <span className="settings-area-audio-reason" role="note" data-capability-reason={query.status}>
              {reasonText}
            </span>
          ) : null}
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
    </div>
  );
}

/**
 * Audio settings area (route /settings/audio; Media group) — the frame's 3
 * schema sections with the capability-backed Output device row interleaved
 * (frame order: Enable audio · Output device · Speaker setup), hand-rendered
 * with per-row ⓘ hints (h10 #1).
 */
export function AudioArea(): ReactElement {
  const m = useAreaMsg();
  const { query, matches } = useSettingsSearch();
  const needle = query.trim().toLowerCase();
  const deviceRowMatches =
    needle === "" ||
    OUTPUT_DEVICE_KEYWORDS.some((keyword) => keyword.toLowerCase().includes(needle));

  return (
    <div className="settings-sections" data-settings-area="audio">
      {schemaSectionsOf("audio").map((section) => {
        // Pure-schema sections (the Output device hand row is rendered by
        // this module, not spliced into the section data): narrow the row
        // union so the schema-entry fields type-check below.
        const entries = section.entries
          .filter((row): row is SettingEntry => "key" in row)
          .filter((entry) => entry.featureExposed && matches(entry));
        // The Output device row (frame C6 y156) rides the Output section's
        // own visibility while a query is live (its keywords are its filter)
        // and sits right after Enable audio in frame order — or first when a
        // live query has filtered the schema sibling out.
        const showDeviceRow = section.id === "output" && deviceRowMatches;
        if (entries.length === 0 && !showDeviceRow) return null;
        const deviceAt = showDeviceRow
          ? entries.findIndex((entry) => entry.key === "audio.enable") + 1
          : entries.length;
        const head = entries.slice(0, deviceAt);
        const tail = entries.slice(deviceAt);
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
            {head.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={AUDIO_HINTS[entry.key]} />
            ))}
            {showDeviceRow ? (
              <div data-setting-row="output-device">
                <AudioDeviceControl />
              </div>
            ) : null}
            {tail.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={AUDIO_HINTS[entry.key]} />
            ))}
          </section>
        );
      })}
    </div>
  );
}

/** Test seam lives in ./audio-fixtures (shared with sibling area tests). */
