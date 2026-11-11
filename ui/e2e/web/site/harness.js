/*
 * Lumen web browser-matrix harness (card 7.I1, issue #49; doc 05 §7
 * + doc 09 §8/§11.2) — the scripted consumer.
 *
 * Drives the engine Worker through the SAME MessagePort contract the
 * product page will (apps/web/worker.js + the wasm ABI behind it):
 * real Worker, real wasm instantiation, real fetch transport, real
 * postMessage ordering, real setTimeout re-arm. The scenario set is
 * the doc 05 §7 conformance categories this host serves today:
 *
 *   - open → play → pause → stop happy path (class-A truth rule:
 *     success is asserted through EVENTS, never the Accepted echo)
 *   - play to end (position ≈ duration, busy release, raw frame lane)
 *   - media info / tracks / duration queries (class S)
 *   - error paths with exact stage/code (rtsp, file, http, garbage
 *     blob, unserved seek/config — typed, never a crash)
 *   - idempotent self-transition echo (pause-while-paused)
 *   - event-envelope invariants (seq strictly increasing; pipeline-
 *     scoped events carry session + gen — doc 05 §4.3)
 *   - autoplay gate (gestureRequired surfaced once, position frozen,
 *     resume on the running report — doc 09 §5.3's never-fake-play)
 *   - transport probes (fetch Range behavior with/without server
 *     support — the seekability inputs of card 7.H4, exercised
 *     against the REAL browser fetch)
 *
 * Zero npm dependencies (hand-written, like worker.js). The page
 * exposes window.__lumenWebMatrix.run(runtimeConfig); the Playwright
 * driver calls it per isolation mode and collects the result object.
 * Every scenario gets a FRESH Worker so a failure cannot cascade.
 */
"use strict";

// Default wait budget for every waitUntil call — the page-side twin of
// pump.sliceTimeoutMs. The literal is ONLY the pre-config fallback (a human
// debugging the page by hand, outside the driver); the Playwright driver
// always overwrites it at run start from the runtime config's waits.timeoutMs
// (matrix.config.json owns the knob — W24-R8 P3-1), and runSuite asserts the
// knob is present before any scenario runs, so a config regression can never
// silently fall back to the literal in a real lane.
let harnessTimeoutMs = 30000;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** All events observed so far (flattened across `events` batches). */
function observedEvents(log) {
  const events = [];
  for (const message of log) {
    if (message.type === "events") {
      events.push(...message.events);
    }
  }
  return events;
}

/** The `session.state-changed` trail, in arrival order. */
function stateTrail(log) {
  return observedEvents(log)
    .filter((event) => event.kind === "session.state-changed")
    .map((event) => event.to);
}

/**
 * One engine lane: a fresh module Worker plus the message log and
 * waiters. `waitUntil` matches against the log too, so a reply that
 * races ahead of the waiter registration is never missed.
 */
class EngineLane {
  constructor() {
    this.log = [];
    this.waiters = [];
    this.worker = new Worker("worker.js", { type: "module" });
    this.worker.onmessage = (event) => {
      this.log.push(event.data);
      this.waiters = this.waiters.filter((waiter) => !waiter.check(event.data));
    };
    this.worker.onerror = (event) => {
      this.log.push({ type: "workerError", message: event.message });
    };
  }

  post(message) {
    this.worker.postMessage(message);
    return this;
  }

  waitUntil(predicate, timeoutMs = harnessTimeoutMs) {
    const existing = this.log.find(predicate);
    if (existing) {
      return Promise.resolve(existing);
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiters = this.waiters.filter((entry) => entry !== waiter);
        const tail = JSON.stringify(this.log.slice(-6));
        reject(new Error(`timed out after ${timeoutMs}ms; log tail: ${tail}`));
      }, timeoutMs);
      const waiter = {
        check: (message) => {
          if (predicate(message)) {
            clearTimeout(timer);
            resolve(message);
            return true;
          }
          return false;
        },
      };
      this.waiters.push(waiter);
    });
  }

  /** Send one catalog command envelope; resolve on its response. */
  request(requestId, cmd, args) {
    this.post({ type: "command", requestId, cmd, args });
    return this.waitUntil(
      (message) => message.type === "response" && message.requestId === requestId,
    );
  }

  /** The `pumped` reports observed so far. */
  pumps() {
    return this.log.filter((message) => message.type === "pumped");
  }

  /** `presentFrame` messages observed so far. */
  frames() {
    return this.log.filter((message) => message.type === "presentFrame");
  }

  terminate() {
    this.worker.terminate();
  }
}

/** The page's REAL renderer probe (doc 09 §4.4: probe, then boot). */
async function probeRenderer() {
  const facts = { webgpuApi: false, webgpuAdapter: false, webgl2Context: false };
  facts.webgpuApi = typeof navigator.gpu !== "undefined";
  if (facts.webgpuApi) {
    // requestAdapter can hang on some builds; bound it.
    facts.webgpuAdapter = await Promise.race([
      navigator.gpu.requestAdapter().then((adapter) => adapter !== null),
      sleep(2000).then(() => false),
    ]).catch(() => false);
  }
  try {
    const canvas = document.createElement("canvas");
    facts.webgl2Context = canvas.getContext("webgl2") !== null;
  } catch {
    facts.webgl2Context = false;
  }
  return facts;
}

/** Boot a fresh lane with the real probe facts; resolve the handshake. */
async function bootLane() {
  const lane = new EngineLane();
  const renderer = await probeRenderer();
  lane.post({ type: "boot", renderer });
  const booted = await lane.waitUntil((message) => message.type === "booted");
  return { lane, renderer, booted };
}

/** Fetch the served fixture bytes (the I/O pillar's real HTTP path). */
async function fetchFixture(path) {
  const response = await fetch(path);
  assert(response.ok, `fixture fetch failed: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** Boot + load the fixture blob + open the session (the happy-path prefix). */
async function openFixtureLane(requestIdBase, config) {
  const ctx = await bootLane();
  const bytes = await fetchFixture(config.server.servedFixture);
  ctx.lane.post({ type: "loadBlob", name: "fixture", bytes: Array.from(bytes) });
  await ctx.lane.waitUntil(
    (message) => message.type === "blobLoaded" && message.name === "fixture",
  );
  const opened = await ctx.lane.request(requestIdBase, "session.open", {
    source: { kind: "url", url: "blob:fixture" },
    options: {},
  });
  return { ...ctx, opened, fixtureBytes: bytes };
}

/**
 * Drive the pump loop. The page starts it (one pump slice); the
 * worker's own setTimeout re-arm continues it while `busy`
 * (apps/web/worker.js). If the loop releases before `done` matches
 * (an autoplay-park stall, a paused settle), re-arm a bounded number
 * of further slices. Resolves on the first matching pump report.
 */
async function drivePump(lane, config, done = () => false) {
  const deadline = Date.now() + config.pump.sliceTimeoutMs;
  let slices = 0;
  const arm = () => {
    lane.post({ type: "pump", ticks: config.pump.ticksPerSlice });
    slices += 1;
  };
  arm();
  for (;;) {
    const settled = lane.pumps().find(done);
    if (settled) {
      return settled;
    }
    const last = lane.pumps()[lane.pumps().length - 1];
    if (last && !last.report.busy && slices < config.pump.maxSlices) {
      arm();
    }
    if (Date.now() > deadline) {
      throw new Error(
        `pump loop exceeded ${config.pump.sliceTimeoutMs}ms; last report: ${JSON.stringify(last)}`,
      );
    }
    await sleep(5);
  }
}

// -- the scenario table ----------------------------------------------------

/**
 * Run the full suite. `config` is the runtime configuration the
 * Playwright driver passes in (matrix.config.json + mode facts).
 * Returns { scenarios: [...], probes: {...} } — scenario failures never
 * throw (they become fail rows); a malformed runtime CONFIG throws loudly
 * instead (the driver validates the same knobs before launch, so that
 * only fires when the page is driven outside the driver).
 */
async function runSuite(config) {
  assert(
    Number.isInteger(config.waits?.timeoutMs) && config.waits.timeoutMs > 0,
    "runtime config is missing waits.timeoutMs (matrix.config.json owns it — W24-R8 P3-1): " +
      JSON.stringify(config.waits),
  );
  harnessTimeoutMs = config.waits.timeoutMs;
  const results = [];
  // Per-scenario progress on the page's status element: a conformance
  // run is minutes-long in a real browser, and a stuck scenario must
  // be visible from the OUTSIDE (the Playwright timeout snapshot and
  // any human watching the lane both need "which scenario is running"
  // — the previous shape only wrote a line at the very end, so a hang
  // was indistinguishable from silence).
  const statusElement = document.getElementById("status");
  const report = (text) => {
    if (statusElement) {
      statusElement.textContent = text;
    }
  };
  const record = (name, category, fn) => {
    const started = performance.now();
    report(`running ${results.length + 1}: ${name} (${category})`);
    return Promise.resolve()
      .then(fn)
      .then((evidence) => {
        results.push({
          name,
          category,
          status: "pass",
          ms: Math.round(performance.now() - started),
          evidence,
        });
      })
      .catch((error) => {
        results.push({
          name,
          category,
          status: "fail",
          ms: Math.round(performance.now() - started),
          error: String(error && error.message ? error.message : error),
        });
      });
  };

  let lane = null;
  const cleanup = () => {
    if (lane) {
      lane.terminate();
      lane = null;
    }
  };

  // -- boot / capability scenarios (doc 09 §3.3/§4.4) --------------------

  await record("boot-handshake", "capability", async () => {
    const ctx = await bootLane();
    lane = ctx.lane;
    // Wire truth (protocol.rs): the enum's `rename_all = "camelCase"`
    // renames the VARIANT TAGS ("booted", "blobLoaded"), not the
    // struct-variant FIELDS — the handshake reply carries
    // `engine_info` (snake), while EngineInfo's own serde layer
    // camelCases its contents (engineVersion, …). The earlier
    // `engineInfo` read was a harness bug: 0 evidence rows, the
    // property does not exist on the wire (found by the first full
    // Chromium run, issue #49).
    const { engine_info: engineInfo, capabilities } = ctx.booted;
    assertEq(engineInfo.host, "web", "engine info host");
    assertEq(capabilities.threading, "single", "threading truth (doc 09 §3.2)");
    assert(
      Array.isArray(capabilities.blocked) && capabilities.blocked.length > 0,
      "the snapshot carries typed blocked rows",
    );
    assert(Array.isArray(capabilities.decoders), "decoders is a list");
    return {
      engineVersion: engineInfo.engineVersion,
      catalogVersion: engineInfo.catalogVersion,
      features: engineInfo.features,
      threading: capabilities.threading,
      renderer: capabilities.renderer,
      audioOutput: capabilities.audioOutput,
      decoders: capabilities.decoders,
      blockedRows: capabilities.blocked.map((row) => row.capability),
    };
  });

  await record("renderer-ladder-consistency", "capability", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    const active = ctx.booted.capabilities.renderer;
    if (ctx.renderer.webgl2Context || ctx.renderer.webgpuApi) {
      assert(
        active.startsWith("webgl2") || active.startsWith("webgpu"),
        `an available probe must yield an active backend, got "${active}"`,
      );
    } else {
      assert(
        active.startsWith("unavailable"),
        `no probe facts must report unavailable, got "${active}"`,
      );
    }
    return { probed: ctx.renderer, activeBackend: active };
  });

  // -- happy path (doc 05 §7.1 categories) -------------------------------

  await record("open-play-pause-stop", "happy-path", async () => {
    cleanup();
    const ctx = await openFixtureLane(100, config);
    lane = ctx.lane;
    const opened = ctx.opened;
    // Class A: Accepted echo, never a completed claim (the truth rule).
    assert(
      opened.result && opened.result.accepted,
      `open must be class-A accepted, got ${JSON.stringify(opened.result)}`,
    );
    assertEq(opened.result.accepted.session, 1, "accepted session id");
    // Play to a Playing observation (event-driven, not echo-driven).
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    const trailAtPlaying = stateTrail(lane.log);
    assert(trailAtPlaying.includes("playing"), "the engine reached Playing");
    // Pause (class A) — wait for the paused observation.
    const pauseResponse = await lane.request(101, "playback.pause");
    assert(pauseResponse.result && pauseResponse.result.accepted, "pause accepted");
    await drivePump(lane, config, () => stateTrail(lane.log).includes("paused"));
    const trailAtPause = stateTrail(lane.log);
    assert(trailAtPause.includes("paused"), "the engine reached Paused");
    assert(
      trailAtPause.indexOf("playing") < trailAtPause.indexOf("paused"),
      "playing precedes paused in the trail",
    );
    // Position advanced while playing (the clock is real).
    const timeResponse = await lane.request(102, "media.get-current-time");
    const timeUs = timeResponse.result.completed.time;
    assert(timeUs > 0, `position must be > 0 after playing, got ${timeUs}`);
    // Stop (class A) — the stopped observation.
    const stopResponse = await lane.request(103, "playback.stop");
    assert(stopResponse.result && stopResponse.result.accepted, "stop accepted");
    await drivePump(lane, config, () => stateTrail(lane.log).includes("stopped"));
    assert(stateTrail(lane.log).includes("stopped"), "the engine reached Stopped");
    // Close + shutdown (graceful teardown).
    const closeResponse = await lane.request(104, "session.close");
    assert(closeResponse.result && closeResponse.result.accepted, "close accepted");
    lane.post({ type: "shutdown" });
    await lane.waitUntil((message) => message.type === "shutdownComplete");
    return {
      classACommands: ["session.open", "playback.pause", "playback.stop", "session.close"],
      positionAfterPauseUs: timeUs,
      stateTrail: stateTrail(lane.log),
    };
  });

  await record("play-to-end", "happy-path", async () => {
    cleanup();
    const ctx = await openFixtureLane(200, config);
    lane = ctx.lane;
    const ended = await drivePump(
      lane,
      config,
      (message) => message.report.state === "ended" && !message.report.busy,
    );
    assertEq(ended.report.state, "ended", "final pump state");
    assert(!ended.report.busy, "busy released at the end (the re-arm truth)");
    const durationResponse = await lane.request(201, "media.get-duration");
    const timeResponse = await lane.request(202, "media.get-current-time");
    const durationUs = durationResponse.result.completed.duration.time;
    const positionUs = timeResponse.result.completed.time;
    const delta = Math.abs(positionUs - durationUs);
    assert(
      delta <= config.playToEnd.positionToleranceUs,
      `position ${positionUs} must reach duration ${durationUs} (delta ${delta} > ${config.playToEnd.positionToleranceUs})`,
    );
    // The video limb delivered: the raw frame lane shipped RGBA frames.
    const frames = lane.frames();
    assert(frames.length > 0, "presentFrame messages arrived (the render lane)");
    for (const frame of frames) {
      assertEq(
        frame.rgba.byteLength,
        frame.width * frame.height * 4,
        `frame rgba payload is w*h*4 (${frame.width}x${frame.height})`,
      );
    }
    assert(
      observedEvents(lane.log).some((event) => event.kind === "playback.end-of-media"),
      "the end-of-media event was flushed",
    );
    return {
      durationUs,
      positionUs,
      deltaUs: delta,
      framesPresented: frames.length,
      frameGeometry: frames.length > 0
        ? { width: frames[0].width, height: frames[0].height }
        : null,
      stateTrail: stateTrail(lane.log),
    };
  });

  await record("media-info-and-tracks", "happy-path", async () => {
    cleanup();
    const ctx = await openFixtureLane(300, config);
    lane = ctx.lane;
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    const durationResponse = await lane.request(301, "media.get-duration");
    const durationUs = durationResponse.result.completed.duration.time;
    assert(durationUs > 0, `duration must be positive, got ${durationUs}`);
    const infoResponse = await lane.request(302, "media.get-info");
    const info = infoResponse.result.completed.info;
    assert(info && typeof info === "object", "media info is an object");
    const tracksResponse = await lane.request(303, "tracks.list");
    const tracks = tracksResponse.result.completed.tracks;
    assert(
      Array.isArray(tracks) && tracks.length === 2,
      `the first vertical carries a video + an audio track, got ${JSON.stringify(tracks)}`,
    );
    const kinds = tracks.map((track) => track.kind).sort();
    assertEq(JSON.stringify(kinds), JSON.stringify(["audio", "video"]), "track kinds");
    return {
      durationUs,
      trackCount: tracks.length,
      kinds,
      codecs: tracks.map((track) => track.codec),
      infoKeys: Object.keys(info),
    };
  });

  // -- error paths (typed, exact stage/code — doc 05 §7.1) ---------------

  await record("open-rtsp-refused", "error-path", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    const response = await lane.request(400, "session.open", {
      source: { kind: "url", url: "rtsp://media.example/stream" },
      options: {},
    });
    const error = response.result.error;
    assert(error, "rtsp open must refuse");
    assertEq(error.stage, "input", "rtsp refusal stage");
    assertEq(error.code, "unsupported-for-host", "rtsp refusal code");
    assertEq(error.details.capability, "rtsp-rtp-udp-access", "cites the blocked row");
    // Redaction (doc 05 §7.1): no URL echo in the message.
    assert(!error.message.includes("media.example"), "the message does not leak the URL");
    return { stage: error.stage, code: error.code, capability: error.details.capability };
  });

  await record("open-file-refused", "error-path", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    const response = await lane.request(401, "session.open", {
      source: { kind: "file", path: "/tmp/x.avi" },
      options: {},
    });
    const error = response.result.error;
    assert(error, "file open must refuse");
    assertEq(error.code, "unsupported-for-host", "file refusal code");
    assertEq(error.details.capability, "local-filesystem", "cites the no-filesystem row");
    return { stage: error.stage, code: error.code, capability: error.details.capability };
  });

  await record("open-http-pending-lane", "error-path", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    const response = await lane.request(402, "session.open", {
      source: { kind: "url", url: "http://media.example/a.avi" },
      options: {},
    });
    const error = response.result.error;
    assert(error, "http open must refuse (the staged fetch lane is not wired to open)");
    assertEq(error.code, "unsupported-for-host", "http refusal code");
    assertEq(error.details.capability, "http-streaming", "cites the pending lane");
    return { stage: error.stage, code: error.code, capability: error.details.capability };
  });

  await record("open-garbage-blob-refused", "error-path", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    const zeros = new Array(4096).fill(0);
    lane.post({ type: "loadBlob", name: "garbage", bytes: zeros });
    await lane.waitUntil(
      (message) => message.type === "blobLoaded" && message.name === "garbage",
    );
    const response = await lane.request(403, "session.open", {
      source: { kind: "url", url: "blob:garbage" },
      options: {},
    });
    const error = response.result.error;
    assert(error, "garbage must refuse");
    assertEq(error.stage, "probe", "the probe's own refusal stage");
    return { stage: error.stage, code: error.code };
  });

  await record("seek-not-served", "error-path", async () => {
    cleanup();
    const ctx = await openFixtureLane(500, config);
    lane = ctx.lane;
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    const response = await lane.request(501, "playback.seek", {
      target: { kind: "absolute", time: 1000000 },
    });
    const error = response.result.error;
    assert(error, "seek must refuse");
    assertEq(error.stage, "control", "unserved commands refuse at control");
    assertEq(error.code, "unsupported-for-host", "seek refusal code");
    assert(error.details.owner, "the refusal names the owning card");
    return { stage: error.stage, code: error.code, owner: error.details.owner };
  });

  await record("unserved-config-command", "error-path", async () => {
    cleanup();
    const ctx = await bootLane();
    lane = ctx.lane;
    // The catalog's args shape for `config.get-setting` is
    // `{area, key}` (commands catalog §2.11 — the native web-host
    // test uses area "engine", key "log-level"); the earlier
    // key-only envelope never DECODED (controlError web.abi.parse,
    // "missing field `area`") so the typed control refusal was
    // never even reached. Same vector as the native test.
    const response = await lane.request(502, "config.get-setting", {
      area: "engine",
      key: "log-level",
    });
    const error = response.result.error;
    assert(error, "config must refuse");
    assertEq(error.code, "unsupported-for-host", "config refusal code");
    assert(error.message.includes("not served by this build"), "the honest wording");
    return { stage: error.stage, code: error.code };
  });

  // -- envelope + lifecycle invariants (doc 05 §4.3) ----------------------

  await record("event-envelope-invariants", "invariants", async () => {
    cleanup();
    const ctx = await openFixtureLane(600, config);
    lane = ctx.lane;
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    await lane.request(601, "playback.pause");
    await drivePump(lane, config, () => stateTrail(lane.log).includes("paused"));
    const events = observedEvents(lane.log);
    assert(events.length > 0, "events were flushed");
    let lastSeq = 0;
    for (const event of events) {
      assert(event.seq > lastSeq, `seq must strictly increase (at ${event.seq})`);
      lastSeq = event.seq;
      const pipelineFamily =
        event.kind.startsWith("session.") ||
        event.kind.startsWith("timing.") ||
        event.kind.startsWith("playback.") ||
        event.kind.startsWith("tracks.") ||
        event.kind.startsWith("media.");
      if (pipelineFamily) {
        // Pipeline-scoped families carry session + gen (the staleness
        // rule's applicability, doc 05 §4.2).
        assertEq(event.session, 1, `pipeline event ${event.kind} carries the session`);
        assert(
          typeof event.gen === "number" && event.gen >= 1,
          `pipeline event ${event.kind} carries a generation`,
        );
      }
    }
    const responses = lane.log.filter((message) => message.type === "response");
    for (const response of responses) {
      assert(response.requestId > 0, "responses echo their requestId");
    }
    return { eventsFlushed: events.length, maxSeq: lastSeq, responses: responses.length };
  });

  await record("idempotent-pause-echo", "invariants", async () => {
    cleanup();
    const ctx = await openFixtureLane(700, config);
    lane = ctx.lane;
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    const first = await lane.request(701, "playback.pause");
    assert(first.result && first.result.accepted, "the first pause is accepted");
    await drivePump(lane, config, () => stateTrail(lane.log).includes("paused"));
    // Pause while already paused: an echo acceptance, no state churn.
    const before = stateTrail(lane.log).join(",");
    const echo = await lane.request(702, "playback.pause");
    assert(echo.result && echo.result.accepted, "the self-echo is accepted");
    await sleep(150);
    const after = stateTrail(lane.log).join(",");
    assertEq(after, before, "no spurious state transitions on the self-echo");
    return { accepted: true, trail: after.split(",") };
  });

  // -- autoplay gate (doc 09 §5.3 — never fake playing) --------------------

  await record("autoplay-gate-parks-and-resumes", "autoplay", async () => {
    cleanup();
    const ctx = await openFixtureLane(800, config);
    lane = ctx.lane;
    await drivePump(lane, config, () => stateTrail(lane.log).includes("playing"));
    // Report a suspended, not-gesture-authorized context.
    lane.post({ type: "audioContext", running: false });
    await lane.waitUntil((message) => message.type === "gestureRequired");
    // The park freezes the clock: two sampled positions identical.
    const sampleA = lane.pumps()[lane.pumps().length - 1];
    await sleep(200);
    const sampleB = lane.pumps()[lane.pumps().length - 1];
    assert(
      sampleB.report.positionUs === sampleA.report.positionUs,
      `position frozen while parked (got ${sampleA.report.positionUs} then ${sampleB.report.positionUs} — never fake playing)`,
    );
    // Exactly one event per parking cycle (the once-per-cycle latch).
    const gestureCount = lane.log.filter(
      (message) => message.type === "gestureRequired",
    ).length;
    assertEq(gestureCount, 1, "gestureRequired fires once per cycle");
    // The gesture satisfied: playback truly flows again.
    lane.post({ type: "audioContext", running: true });
    const ended = await drivePump(
      lane,
      config,
      (message) => message.report.state === "ended" && !message.report.busy,
    );
    assertEq(ended.report.state, "ended", "resumed playback reaches the end");
    return {
      parkedPositionUs: sampleA.report.positionUs,
      resumedToEnd: true,
      gestureEvents: gestureCount,
    };
  });

  // -- transport probes (real fetch — the 7.H4 seekability inputs) --------

  await record("fetch-range-supported-server", "transport", async () => {
    const response = await fetch(config.server.rangeSupportingPath, {
      headers: { Range: "bytes=0-1023" },
    });
    assertEq(response.status, 206, "a Range-aware server answers 206");
    const contentRange = response.headers.get("content-range");
    assert(
      contentRange && contentRange.startsWith("bytes 0-1023/"),
      `content-range echoes the window, got "${contentRange}"`,
    );
    const body = new Uint8Array(await response.arrayBuffer());
    assertEq(body.byteLength, 1024, "the window body is exactly 1024 bytes");
    return {
      status: response.status,
      contentRange,
      acceptRanges: response.headers.get("accept-ranges"),
    };
  });

  await record("fetch-range-blind-server", "transport", async () => {
    const response = await fetch(config.server.rangeBlindPath, {
      headers: { Range: "bytes=0-1023" },
    });
    assertEq(response.status, 200, "a range-blind server ignores the window (200)");
    assert(response.headers.get("accept-ranges") === null, "no accept-ranges advertisement");
    const body = new Uint8Array(await response.arrayBuffer());
    assertEq(body.byteLength, config.fixtureBytes, "the full body arrives (the unseekable shape)");
    return { status: response.status, bodyBytes: body.byteLength };
  });

  await record("audio-context-and-worklet", "transport", async () => {
    const audio = {
      contextConstructible: false,
      workletModuleLoaded: false,
      contextState: null,
      sampleRate: null,
    };
    try {
      const context = new AudioContext();
      audio.contextConstructible = true;
      audio.contextState = context.state;
      audio.sampleRate = context.sampleRate;
      await context.audioWorklet.addModule("audio-worklet.js");
      audio.workletModuleLoaded = true;
      await context.close();
    } catch (error) {
      audio.error = String(error);
    }
    // Honest posture: record what the browser gave; a suspended context
    // without a gesture is EXPECTED (doc 09 §5.3) — never a failure.
    assert(audio.contextConstructible, "AudioContext is constructible");
    assert(audio.workletModuleLoaded, "the worklet module parses + loads");
    return audio;
  });

  cleanup();

  return {
    scenarios: results,
    probes: {
      crossOriginIsolated,
      isolationMode: config.mode.id,
    },
  };
}

// The driver-facing entry point (Playwright calls this via page.evaluate).
window.__lumenWebMatrix = {
  run: (config) =>
    runSuite(config).then((result) => {
      const status = document.getElementById("status");
      if (status) {
        const failed = result.scenarios.filter((scenario) => scenario.status === "fail");
        status.textContent =
          `done: ${result.scenarios.length - failed.length}/${result.scenarios.length} passed` +
          (failed.length > 0 ? ` (failed: ${failed.map((entry) => entry.name).join(", ")})` : "");
      }
      return result;
    }),
};
