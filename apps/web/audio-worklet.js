/*
 * Lumen AudioWorklet processor (card 7.H3, issue #48; doc 09 §5.1).
 *
 * The "tiny, fixed, hand-audited JS shim" doc 09 §5.1 mandates: a
 * direct transcription of the tested Rust model in
 * apps/web/src/audio/sink.rs (WorkletQueue). NO engine code runs in
 * here — only ring read, fill, silence-on-underrun, and the played
 * counter bump (doc 03 §4.9(c)'s allowed operations). The model's
 * arithmetic is proven headless-native by this crate's tests; when
 * either half changes, BOTH change (the headers say so).
 *
 * Transcribed constants (pin: apps/web/src/audio/sink.rs):
 *   WORKLET_QUANTUM      = 128   (the platform's fixed render quantum)
 *   DEFAULT_QUEUE_FRAMES = 8192  (the bounded queue, ~170 ms at 48 kHz)
 *   PREBUFFER_TARGET     = 256   (2× the quantum — the ship-more signal)
 *
 * Port protocol (the page routes chunks in; the processor posts
 * evidence out — v1 is `threading: single`, no SharedArrayBuffer, so
 * chunks cross as transferred Float32Array messages):
 *   page -> processor   {type: "chunk", samples: Float32Array}
 *                      (interleaved frames, channels wide — the queue's
 *                      own geometry; a torn chunk is REFUSED whole,
 *                      never a silent drop)
 *                      {type: "flush"} (drop the queue — the seek/park
 *                      path; the played counter is NOT touched: it is
 *                      the clock's truth and stays monotonic)
 *   processor -> page   {type: "report", played, queuedFrames,
 *                        underruns, silentFrames, overflowed}
 *                      (the counter evidence the page pairs with
 *                       AudioContext.currentTime to build the
 *                       wallclock anchor — doc 09 §5.2)
 *                      {type: "queueFull", queued, chunk, capacity}
 *                      (the typed push refusal — the page's routing
 *                      bug, visible, never a silent gap)
 *
 * The report cadence (every 8 quanta by default, ~21 ms at 48 kHz) is
 * the card 7.I1 wiring decision [VERIFY]: frequent enough to anchor
 * the clock, sparse enough not to wake the main thread per quantum.
 *
 * Counter semantics (the model's own — sink.rs):
 *   - played counts RENDERED instants, filled AND silent alike (the
 *     device clock runs through underruns);
 *   - an underrun is any silence in a quantum (one count per quantum);
 *   - a suspended context renders nothing — pause freezes the counter
 *     (the autoplay gate's twin rule; the page parks playback, the
 *     engine's GestureRequired lane surfaces it).
 *
 * Browser-runtime behavior (the real-time audio thread, the browser's
 * watchdog, AudioWorkletNode port routing) is [VERIFY] — card 7.I1's
 * matrix; no browser exists in this sandbox. What this file IS: the
 * audited transcription of the native-tested model.
 */

"use strict";

const QUANTUM = 128;
const DEFAULT_CAPACITY_FRAMES = 8192;
const PREBUFFER_TARGET = 2 * QUANTUM;
const REPORT_EVERY_QUANTA = 8;

class LumenWorkletProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const user = (options && options.processorOptions) || {};
    this.channels = Math.max(1, user.channels | 0);
    this.capacityFrames = Math.max(
      QUANTUM,
      (user.capacityFrames | 0) || DEFAULT_CAPACITY_FRAMES,
    );
    // The queue: chunks of interleaved Float32 frames + a read cursor
    // (chunkIndex, offsetInChunk). Pops are O(1); the head compacts in
    // bounded batches so process() itself never allocates per frame.
    this.queue = [];
    this.head = 0;
    this.offset = 0;
    // The evidence counters (the model's own fields).
    this.played = 0;
    this.underruns = 0;
    this.silentFrames = 0;
    this.overflowed = 0;
    this.quantaSinceReport = 0;
    this.port.onmessage = (event) => this.onPortMessage(event.data);
  }

  /** Frames currently queued (whole frames — pushes validate). */
  queuedFrames() {
    let frames = 0;
    for (let index = this.head; index < this.queue.length; index += 1) {
      frames += this.queue[index].length;
    }
    frames -= this.offset;
    // The cursor can only sit inside the head chunk; the division is
    // exact because every chunk holds whole frames.
    return Math.floor(frames / this.channels);
  }

  /** The producer's ship-more signal (below 2× the quantum). */
  needsFrames() {
    return this.queuedFrames() < PREBUFFER_TARGET;
  }

  /** One page -> processor port message. */
  onPortMessage(message) {
    if (!message || typeof message !== "object") {
      return;
    }
    if (message.type === "chunk") {
      this.push(message.samples);
    } else if (message.type === "flush") {
      // Drop the queued audio; the counter stays monotonic (the clock
      // re-anchors on the page's side — doc 09 §5.2's drift policy).
      this.queue = [];
      this.head = 0;
      this.offset = 0;
    }
  }

  /** Append one interleaved chunk — the bounded-queue discipline. */
  push(samples) {
    if (!(samples instanceof Float32Array) || samples.length === 0) {
      return;
    }
    if (samples.length % this.channels !== 0) {
      // A torn chunk is a routing bug: refuse it whole, visibly.
      this.overflowed += 1;
      this.port.postMessage({
        type: "queueFull",
        queued: this.queuedFrames(),
        chunk: Math.floor(samples.length / this.channels),
        capacity: this.capacityFrames,
      });
      return;
    }
    const frames = samples.length / this.channels;
    if (this.queuedFrames() + frames > this.capacityFrames) {
      // The whole chunk is refused (the page retries after the queue
      // drains; audio is never silently dropped — the typed refusal).
      this.overflowed += 1;
      this.port.postMessage({
        type: "queueFull",
        queued: this.queuedFrames(),
        chunk: frames,
        capacity: this.capacityFrames,
      });
      return;
    }
    this.queue.push(samples);
  }

  /**
   * One render quantum: fill the outputs from the queue, silence
   * whatever the queue cannot supply, and bump the counter by the
   * FULL quantum either way (the device clock runs through underruns).
   * The cursor advances as frames are served — the queue pops
   * continuously across chunk boundaries, exactly the model's
   * VecDeque discipline. Returns true forever — the node stays alive;
   * the page's flush and the context's lifecycle own the teardown.
   */
  process(inputs, outputs) {
    const output = outputs[0];
    if (!output || output.length === 0) {
      return true;
    }
    const channels = Math.min(this.channels, output.length);
    const frames = output[0].length;
    let filled = 0;
    for (let frame = 0; frame < frames; frame += 1) {
      let served = false;
      while (this.head < this.queue.length) {
        const chunk = this.queue[this.head];
        if (this.offset >= chunk.length) {
          // Exhausted chunk: advance the head (the pops continue in
          // the next chunk — a quantum is never silenced by a chunk
          // boundary).
          this.head += 1;
          this.offset = 0;
          continue;
        }
        // A frame's samples never straddle chunks (every chunk holds
        // whole frames; the offsets are multiples of the geometry).
        for (let channel = 0; channel < channels; channel += 1) {
          output[channel][frame] = chunk[this.offset + channel];
        }
        // The full interleaved frame is consumed even when the output
        // exposes fewer channels than the queue's geometry.
        this.offset += this.channels;
        if (this.offset >= chunk.length) {
          this.head += 1;
          this.offset = 0;
        }
        served = true;
        break;
      }
      if (served) {
        filled += 1;
      } else {
        // Silence-on-underrun (the queue could not supply this frame).
        for (let channel = 0; channel < channels; channel += 1) {
          output[channel][frame] = 0;
        }
      }
    }
    // Bounded head compaction (outside the per-frame path).
    if (this.head >= 64) {
      this.queue.splice(0, this.head);
      this.head = 0;
    }
    const silent = frames - filled;
    if (silent > 0) {
      this.underruns += 1;
      this.silentFrames += silent;
    }
    // The counter bump: rendered instants, silence included.
    this.played += frames;
    // The evidence report (throttled — see the header's cadence note).
    this.quantaSinceReport += 1;
    if (this.quantaSinceReport >= REPORT_EVERY_QUANTA) {
      this.quantaSinceReport = 0;
      this.port.postMessage({
        type: "report",
        played: this.played,
        queuedFrames: this.queuedFrames(),
        needsFrames: this.needsFrames(),
        underruns: this.underruns,
        silentFrames: this.silentFrames,
        overflowed: this.overflowed,
      });
    }
    return true;
  }
}

registerProcessor("lumen-worklet-processor", LumenWorkletProcessor);
