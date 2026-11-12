// Card 4.D2 — fixture-driven panel tests: every section renders from
// fixtures shaped exactly like the `media.get-info` result type; the
// unknown-field AC (absent → honest "Unknown", never invented); multi-stream
// cases; empty + not-exposed states; event-driven updates.

import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
  MediaInfo,
} from "../../client/transport";
import { MediaInfoPanel } from "./MediaInfoPanel";
import { MediaInfoProvider } from "./MediaInfoProvider";
import {
  fullMediaInfo,
  liveAudioMediaInfo,
  sparseMediaInfo,
} from "./fixtures";

type ErrorCode = "feature-not-exposed" | "not-found";

/** Test double: answers `media.get-info` from a fixture (or an error). */
class FakeTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  private generation = 0;

  constructor(
    private readonly info: MediaInfo | null,
    private readonly errorCode: ErrorCode = "not-found",
  ) {}

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    const requestId = request.requestId;
    const gen = ++this.generation;
    if (request.cmd === "media.get-info") {
      if (this.info !== null) {
        return {
          gen,
          requestId,
          result: { completed: { kind: "media-info", info: this.info } },
        };
      }
      return {
        gen,
        requestId,
        result: {
          error: {
            code: this.errorCode,
            details: { cmd: "media.get-info" },
            message: "no media open",
            stage: "session",
          },
        },
      };
    }
    return { gen, requestId, result: { completed: { kind: "unit" } } };
  }
}

function renderPanel(transport: CommandTransport) {
  return render(
    <ClientProvider transport={transport}>
      <MediaInfoProvider>
        <MediaInfoPanel />
      </MediaInfoProvider>
    </ClientProvider>,
  );
}

function fieldValue(container: HTMLElement, field: string): string {
  const row = container.querySelector(`[data-field="${field}"]`);
  expect(row, `field row ${field} must render`).not.toBeNull();
  return row!.textContent ?? "";
}

function section(container: HTMLElement, name: string): HTMLElement {
  const el = container.querySelector(`[data-mi-section="${name}"]`);
  expect(el, `section ${name} must render`).not.toBeNull();
  return el as HTMLElement;
}

describe("MediaInfoPanel — fixture rendering", () => {
  it("queries the client seam with media.get-info and renders all sections", async () => {
    const transport = new FakeTransport(fullMediaInfo);
    const { container } = renderPanel(transport);

    await waitFor(() => {
      expect(screen.getByText("Matroska (MKV)")).toBeTruthy();
    });
    expect(transport.sent[0]?.cmd).toBe("media.get-info");

    for (const heading of ["General", "Video", "Audio", "Subtitles", "Metadata"]) {
      expect(screen.getByRole("heading", { name: heading })).toBeTruthy();
    }
    expect(fieldValue(container, "title")).toBe("TitleBig Buck Bunny");
    expect(fieldValue(container, "source")).toBe("Source/media/big-buck-bunny.mkv");
    expect(fieldValue(container, "duration")).toBe("Duration10:35");
    expect(fieldValue(container, "chapters")).toBe("Chapters1");
  });

  it("renders video stream descriptors (codec, profile, dimensions, fps)", async () => {
    const { container } = renderPanel(new FakeTransport(fullMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("1920 × 1080")).toBeTruthy();
    });
    const video = section(container, "video");
    expect(video.querySelector('[data-track-id="1"]')?.textContent).toContain(
      "H.264 (AVC) (avc1)",
    );
    expect(video.textContent).toContain("High");
    expect(video.textContent).toContain("4.1");
    expect(video.textContent).toContain("23.976 fps");
    expect(video.textContent).toContain("8-bit");
  });

  it("renders audio stream descriptors (channels, sample rate, language)", async () => {
    const { container } = renderPanel(new FakeTransport(fullMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("48000 Hz")).toBeTruthy();
    });
    const audio = section(container, "audio");
    expect(audio.querySelector('[data-track-id="3"]')?.textContent).toContain("6");
    expect(audio.textContent).toContain("eng");
    expect(audio.textContent).toContain("E-AC-3");
    expect(audio.textContent).toContain("Surround 5.1");
  });

  it("renders subtitle streams including external-file origin", async () => {
    const { container } = renderPanel(new FakeTransport(fullMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("SubRip (SRT)")).toBeTruthy();
    });
    const subs = section(container, "subtitle");
    expect(subs.querySelector('[data-track-id="5"]')?.textContent).toContain(
      "Container stream",
    );
    const external = subs.querySelector('[data-track-id="6"]');
    expect(external?.textContent).toContain("forced.de.srt");
    expect(external?.textContent).toContain("External file");
  });

  it("renders metadata tags from the record", async () => {
    const { container } = renderPanel(new FakeTransport(fullMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("Blender Foundation")).toBeTruthy();
    });
    expect(fieldValue(container, "metadata.ARTIST")).toBe("ARTISTBlender Foundation");
    expect(fieldValue(container, "metadata.ENCODER")).toBe("ENCODERLavf61.1.100");
  });

  it("multi-stream: every stream of a kind renders as its own entry", async () => {
    const { container } = renderPanel(new FakeTransport(fullMediaInfo));
    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Video stream 1" })).toBeTruthy();
    });
    const video = section(container, "video");
    expect(video.querySelectorAll('[data-mi-stream="video"]')).toHaveLength(2);
    expect(screen.getByRole("heading", { name: "Video stream 2" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Audio stream 2" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Subtitle stream 2" })).toBeTruthy();
  });

  it("live stream: infinite duration, live yes, seekable no, url source", async () => {
    const { container } = renderPanel(new FakeTransport(liveAudioMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("∞ (live)")).toBeTruthy();
    });
    expect(fieldValue(container, "live")).toBe("LiveYes");
    expect(fieldValue(container, "seekable")).toBe("SeekableNo");
    expect(fieldValue(container, "source")).toContain("ice1.somafm.com");
  });
});

describe("MediaInfoPanel — the unknown-field AC", () => {
  it("renders the honest Unknown for every absent field, never an invented value", async () => {
    const { container } = renderPanel(new FakeTransport(sparseMediaInfo));
    await waitFor(() => {
      expect(container.querySelector('[data-mi-section="general"]')).not.toBeNull();
    });
    // General: title/container/duration all absent in the fixture.
    expect(fieldValue(container, "title")).toBe("TitleUnknown");
    expect(fieldValue(container, "container")).toBe("ContainerUnknown");
    expect(fieldValue(container, "duration")).toBe("DurationUnknown");
    // CATALOG GAP rows: size/bitrate are not in the wire type — Unknown, never invented.
    expect(fieldValue(container, "size")).toBe("SizeUnknown");
    expect(fieldValue(container, "bitrate")).toBe("BitrateUnknown");
    // Video stream: codec descriptor empty, fps/bit-depth null.
    const video = section(container, "video");
    expect(video.querySelector('[data-track-id="1"]')?.textContent).toContain("Unknown");
    expect(fieldValue(video, "codec")).toBe("CodecUnknown");
    expect(fieldValue(video, "frame-rate")).toBe("Frame rateUnknown");
    expect(fieldValue(video, "bit-depth")).toBe("Bit depthUnknown");
    // Audio stream: sample rate absent.
    const audio = section(container, "audio");
    expect(fieldValue(audio, "sample-rate")).toBe("Sample rateUnknown");
    expect(fieldValue(audio, "language")).toBe("LanguageUnknown");
  });

  it("sparse metadata renders the honest none-reported note", async () => {
    renderPanel(new FakeTransport(sparseMediaInfo));
    await waitFor(() => {
      expect(screen.getByText("No metadata tags reported.")).toBeTruthy();
    });
  });
});

describe("MediaInfoPanel — data flow + honest states", () => {
  it("empty state when nothing is open (command errors: no media)", async () => {
    const { container } = renderPanel(new FakeTransport(null, "not-found"));
    await waitFor(() => {
      expect(container.querySelector('[data-state="empty"]')).not.toBeNull();
    });
    expect(screen.getByText("No media open")).toBeTruthy();
    expect(screen.getByText(/Media information appears once a media/i)).toBeTruthy();
    // Never a fabricated section:
    expect(container.querySelector('[data-mi-section="general"]')).toBeNull();
    // w26.i18 (h19 §5): the honest states ride the section-card grammar and
    // the empty state carries the one real path to a populated body.
    expect(container.querySelector('[data-testid="media-info-empty-card"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Open media" })).toBeTruthy();
  });

  it("not-exposed note with the scaffold MockTransport (honest, no fake data)", async () => {
    const { container } = renderPanel(new MockTransport());
    await waitFor(() => {
      expect(container.querySelector('[data-state="not-exposed"]')).not.toBeNull();
    });
    expect(
      screen.getByText(/not available — the command layer did not expose/i),
    ).toBeTruthy();
    expect(container.querySelector('[data-mi-section="general"]')).toBeNull();
    // w26.i18: the not-exposed verdict rides the same card grammar.
    expect(container.querySelector('[data-testid="media-info-not-exposed"]')).not.toBeNull();
  });

  it("media.info-ready events keep the panel authoritative", async () => {
    const transport = new FakeTransport(null, "not-found");
    const { container } = renderPanel(transport);
    await waitFor(() => {
      expect(container.querySelector('[data-state="empty"]')).not.toBeNull();
    });

    transport.emit({ kind: "media.info-ready", info: fullMediaInfo, seq: 1 });
    await waitFor(() => {
      expect(container.querySelector('[data-state="ready"]')).not.toBeNull();
    });
    expect(screen.getByText("Matroska (MKV)")).toBeTruthy();

    // A later event replaces it (new media opened).
    transport.emit({ kind: "media.info-ready", info: sparseMediaInfo, seq: 2 });
    await waitFor(() => {
      expect(fieldValue(container, "container")).toBe("ContainerUnknown");
    });
  });
});
