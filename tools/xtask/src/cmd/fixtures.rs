//! `xtask fixtures` — deterministic demo-fixture generation (card 2.A3).
//!
//! Owned by card **2.A3** (doc 13 §5, wave 2.A). Registered by the Lead
//! (frozen-registry ownership grant, worklog 1.F1-V); the stub that lived
//! here until now only reported that the owning card had not landed.
//!
//! Generates the Phase-2 first-vertical fixtures of doc 03 §7/§8 —
//! **decision-independent**: both candidate container pairs are produced
//! (the Lead's AVI-vs-MP4 pick at wave-2.B launch blocks nothing):
//!
//! * `AVI + MJPEG + PCM` — RIFF/AVI 1.0: `RIFF('AVI '` → `LIST('hdrl')`
//!   (`avih` + two `strl`/`strh`/`strf` streams) → `LIST('movi')` with
//!   interleaved `00dc` MJPEG frames and `01wb` PCM chunks → `idx1`.
//! * `MP4 + MJPEG + PCM` — ISO/IEC 14496-12/-14 progressive: `ftyp` +
//!   `moov` (two `trak`s with `stsd` (`jpeg` video / `sowt` audio),
//!   `stts`/`stss`/`stsc`/`stsz`/`stco` sample tables) + `mdat`.
//!
//! plus the malformed/truncated derivative set of doc 03 §7 and
//! docs/inventory/corpus-plan.md §1 class B (structure-boundary
//! truncations, garbage-append, seeded bit-flips, one spec-violating
//! variant per container: missing `idx1` for AVI, `stsz` count lie for
//! MP4).
//!
//! # Determinism contract (card AC: byte-stable across runs)
//!
//! The whole corpus is a pure function of this source file:
//!
//! * **No wall clock** — MP4 `mvhd`/`tkhd`/`mdhd` creation times are 0.
//! * **Integer math only** (corpus-plan §2): fixed-point DCT, integer
//!   RGB→YCbCr, a 256-point LUT sine NCO with phase accumulator. No
//!   float op executes anywhere in the generation path, so no platform
//!   libm can drift a byte.
//! * **No unordered containers** — the file plan is a fixed `Vec`;
//!   no `HashMap`/`HashSet` iteration can leak ordering.
//! * **Seeded PRNG only** — xorshift64*, seeded per corpus-plan §1 B2
//!   with `sha256(parent)[0..8]` (the hasher below), never from entropy.
//! * Seed root `lumen-2a3-v1` (compile-time constant, corpus-plan §2
//!   proposal instantiated here; changing it regenerates every hash and
//!   is regression paperwork).
//!
//! # Clean-room position (AGENTS.md §3.1)
//!
//! Every byte is authored by this in-tree generator from public
//! specifications (ISO/IEC 10918-1 baseline JPEG incl. Annex K tables,
//! the AVI RIFF file-format reference, ISO/IEC 14496-12/-14) — no VLC
//! code or assets, no third-party media, no external encoder runs.
//! Output is CC0 generated test data (doc 10 §5.3, corpus-plan §5).
//! `ffmpeg`/`ffprobe` are used only as *local cross-check oracles*
//! (corpus-plan §2 case a) during development — never in the build.
//!
//! # Layout produced (corpus-plan §4)
//!
//! ```text
//! tests/fixtures/
//!   CORPUS.md                        # licensing ledger (generated, CC0)
//!   generated/MANIFEST.toml          # generator + sha256 per file
//!   generated/valid/*.avi|*.mp4      # class A parents
//!   generated/valid/*.structure.json # structure maps (corpus-plan §2)
//!   generated/malformed/*__*.avi|mp4 # class B derivatives
//!   expectations/*.toml              # one expectation file per fixture
//! ```
//!
//! # Exit codes (frozen xtask contract, main.rs)
//!
//! * `0` — success (write mode, or `--check` found no drift).
//! * `1` — `--check` ran to a verdict and found drift/missing/stale
//!   files (mirrors `schema-gen --check` / `codegen --check`).
//! * `2` — xtask error (usage, I/O) via `XtaskError`.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::XtaskError;

/// Workspace root, derived from this crate's manifest location
/// (`<root>/tools/xtask`), the same compile-time anchor the sibling
/// subcommands use.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Corpus seed root (corpus-plan §2). The entire corpus is a pure
/// function of the code plus this constant; derivatives additionally
/// derive their PRNG seeds from their parent's sha256.
const SEED_ROOT: &str = "lumen-2a3-v1";

/// The committed fixture tree root (relative to the workspace root).
const FIXTURES_SUBDIR: &str = "tests/fixtures";

// ---------------------------------------------------------------------------
// Command-line surface
// ---------------------------------------------------------------------------

const USAGE: &str = "usage: cargo xtask fixtures [--check] [--dest <dir>] [--list]

Deterministic demo-fixture generation (card 2.A3, doc 03 §7/§8 +
docs/inventory/corpus-plan.md). Produces the decision-independent
first-vertical corpus: AVI+MJPEG+PCM and MP4+MJPEG+PCM valid parents,
malformed/truncated derivatives, expectations, structure maps,
generated/MANIFEST.toml and CORPUS.md — all CC0, all byte-stable.

  (no flags)    regenerate every fixture under tests/fixtures/ and print
                the report (committed artifacts: refresh + commit on
                generator change, same workflow as schema-gen/codegen)
  --check       verify the tree matches regeneration byte-for-byte
                (exit 0 = no drift, 1 = drift, 2 = error)
  --dest <dir>  write/verify under <dir> instead of tests/fixtures/
                (scratch runs, CI regeneration, tests)
  --list        print the fixture table (class, size, sha256) without
                writing anything

Determinism: the corpus is a pure function of the generator source —
fixed seed root, integer-only math, no wall clock, no HashMaps.
";

/// Parsed command line for `cargo xtask fixtures`.
#[derive(Debug)]
enum Invocation {
    /// `--help`/`-h`: print usage, exit 0.
    Help,
    /// Generate (`--check` verifies instead of writing; `--list` prints
    /// the table only; `dest` overrides the committed tree location).
    Run {
        check: bool,
        list: bool,
        dest: Option<PathBuf>,
    },
}

fn parse_args(args: &[String]) -> Result<Invocation, XtaskError> {
    let mut check = false;
    let mut list = false;
    let mut dest: Option<String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Invocation::Help),
            "--check" => {
                if check {
                    return Err(XtaskError::Usage(
                        "`--check` given more than once".to_string(),
                    ));
                }
                check = true;
            }
            "--list" => {
                if list {
                    return Err(XtaskError::Usage(
                        "`--list` given more than once".to_string(),
                    ));
                }
                list = true;
            }
            "--dest" => {
                if dest.is_some() {
                    return Err(XtaskError::Usage(
                        "`--dest` given more than once".to_string(),
                    ));
                }
                dest = Some(
                    rest.next()
                        .ok_or_else(|| XtaskError::Usage("`--dest` requires a path".to_string()))?
                        .clone(),
                );
            }
            other => {
                return Err(XtaskError::Usage(format!("unexpected argument `{other}`")));
            }
        }
    }
    if check && list {
        return Err(XtaskError::Usage(
            "`--check` and `--list` are mutually exclusive".to_string(),
        ));
    }
    Ok(Invocation::Run {
        check,
        list,
        dest: dest.map(PathBuf::from),
    })
}

/// Workspace root at runtime: `<root>/tools/xtask` minus two components.
fn workspace_root() -> &'static Path {
    Path::new(WORKSPACE_ROOT)
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(WORKSPACE_ROOT))
}

/// Entry point (frozen-dispatch shape, card 1.A3).
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    match parse_args(args)? {
        Invocation::Help => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Invocation::Run { check, list, dest } => {
            let base = match dest {
                Some(dest) => dest,
                None => workspace_root().join(FIXTURES_SUBDIR),
            };
            let files = generate_all().map_err(XtaskError::Internal)?;
            if list {
                print_file_table(&files);
                return Ok(ExitCode::SUCCESS);
            }
            if check {
                let drift = verify_tree(&base, &files)?;
                return Ok(ExitCode::from(u8::from(drift)));
            }
            write_tree(&base, &files)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic primitives (corpus-plan §2: reproducible or it didn't happen)
// ---------------------------------------------------------------------------

/// SHA-256 (FIPS 180-4), std-only by hand: xtask is dependency-free by
/// C5 and the manifest/corpus hash contract (doc 10 §5.3, TB14) needs
/// one hasher shared by the generator, the manifest and the derivative
/// seeds. All arithmetic is wrapping — the hash path must not panic on
/// overflow in debug builds either.
struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_bytes: u64,
}

/// Round constants: first 32 bits of the fractional parts of the cube
/// roots of the first 64 primes (FIPS 180-4 §4.2.2, public spec table).
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    fn new() -> Sha256 {
        Sha256 {
            // First 32 bits of the fractional parts of the square roots
            // of the first 8 primes (FIPS 180-4 §5.3.3).
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0u8; 64],
            buf_len: 0,
            total_bytes: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_bytes = self.total_bytes.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let want = 64 - self.buf_len;
            let take = want.min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_len = self.total_bytes.wrapping_mul(8);
        // Pad: 0x80, zeros, 8-byte big-endian bit length.
        self.update(&[0x80]);
        while self.buf_len != 56 {
            self.update(&[0]);
        }
        // Length must be appended without counting it in total_bytes:
        // push it straight into the buffer, then compress.
        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buf;
        self.compress(&block);
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// SHA-256 of `data`, hex-encoded lowercase (manifest/CORPUS format).
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_lower(&hasher.finish())
}

/// Lowercase hex (fixed-width, no `fmt` machinery in the hot path).
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Deterministic PRNG for the class-B derivatives (corpus-plan §1 B2:
/// xorshift64*, seeded from the parent's sha256 — never from entropy).
struct Xorshift64Star {
    state: u64,
}

impl Xorshift64Star {
    /// Seed from the first 8 bytes of a digest (corpus-plan's
    /// `sha256(parent)[:8]`), big-endian. A zero seed is xorshift's
    /// fixed point, so any zero byte pattern is nudged to 1 — the
    /// parents are fixed known files, this branch never fires today,
    /// but the generator must not have a latent hang.
    fn from_digest(seed: &[u8]) -> Xorshift64Star {
        let mut value: u64 = 0;
        for &byte in &seed[..8] {
            value = (value << 8) | byte as u64;
        }
        Xorshift64Star {
            state: if value == 0 { 1 } else { value },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Pseudo-uniform value below `bound` (modulo bias is irrelevant for
    /// fixture mutation; determinism is the contract).
    fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}

/// Integer sine oscillator: 256-point full-wave table (built from the
/// 65-entry quarter-wave constant below) with linear interpolation and
/// a u32 phase accumulator. Pure integer math — the PCM tone must be
/// byte-identical on every platform (corpus-plan §2 bans float paths).
struct SineOsc {
    phase: u32,
    step: u32,
}

/// `round(sin(i * pi / 128) * 32767)` for i in 0..=64 (quarter wave,
/// endpoint 32767 included) — fixed at authoring time, public math.
const SIN_QUARTER: [u16; 65] = [
    0, 804, 1608, 2410, 3212, 4011, 4808, 5602, //
    6393, 7179, 7962, 8739, 9512, 10278, 11039, 11793, //
    12539, 13279, 14010, 14732, 15446, 16151, 16846, 17530, //
    18204, 18868, 19519, 20159, 20787, 21403, 22005, 22594, //
    23170, 23731, 24279, 24811, 25329, 25832, 26319, 26790, //
    27245, 27683, 28105, 28510, 28898, 29268, 29621, 29956, //
    30273, 30571, 30852, 31113, 31356, 31580, 31785, 31971, //
    32137, 32285, 32412, 32521, 32609, 32678, 32728, 32757, //
    32767,
];

/// Full-wave table: `TABLE[j]` = sine at phase `j/256`, j in 0..=256
/// (wrapping point duplicated so interpolation never indexes out).
fn sine_full_table() -> [i32; 257] {
    let mut table = [0i32; 257];
    for j in 0..=256 {
        let t = j & 0xff;
        let magnitude = if t <= 64 {
            SIN_QUARTER[t] as i32
        } else if t <= 128 {
            SIN_QUARTER[128 - t] as i32
        } else if t <= 192 {
            -(SIN_QUARTER[t - 128] as i32)
        } else {
            -(SIN_QUARTER[256 - t] as i32)
        };
        table[j] = magnitude;
    }
    table
}

impl SineOsc {
    /// `step = round(freq * 2^32 / rate)` — u64 intermediate, no float.
    fn new(freq: u32, rate: u32) -> SineOsc {
        let numerator = (freq as u64) << 32;
        let step = (numerator + (rate as u64) / 2) / rate as u64;
        SineOsc {
            phase: 0,
            step: u32::try_from(step).unwrap_or(u32::MAX),
        }
    }

    /// Next sample in [-32767, 32767] (15-bit scale).
    fn sample(&mut self, table: &[i32; 257]) -> i32 {
        let index = (self.phase >> 24) as usize;
        let frac = ((self.phase >> 8) & 0xffff) as i32;
        let a = table[index];
        let b = table[index + 1];
        self.phase = self.phase.wrapping_add(self.step);
        a + (((b - a) * frac) >> 16)
    }
}

// ---------------------------------------------------------------------------
// Fixture media essence — geometry, cadence and tone (fixed constants)
// ---------------------------------------------------------------------------

/// Video geometry: 320×240 is an exact multiple of the 16×16 4:2:0 MCU
/// (20×15 MCUs — no edge padding anywhere in the corpus).
const FRAME_WIDTH: usize = 320;
const FRAME_HEIGHT: usize = 240;

/// Cadence: 30 fps over 2.0 s — 60 frames; 44100/30 = 1470 samples per
/// frame tick, so audio and video interleave with no fractional sample
/// in either container.
const FPS: u32 = 30;
const FRAME_COUNT: usize = 60;
const AUDIO_RATE: u32 = 44100;
const AUDIO_CHANNELS: usize = 2;
/// Samples per frame tick, per channel (`AUDIO_RATE / FPS`).
const SAMPLES_PER_TICK: usize = 1470;
/// Total samples per channel (`FRAME_COUNT * SAMPLES_PER_TICK`).
const TOTAL_SAMPLES: usize = 88_200;

/// Tone plan: 440 Hz left / 660 Hz right, distinct amplitudes, so a
/// future decoder can assert channel mapping, not just sample values.
const TONE_LEFT_HZ: u32 = 440;
const TONE_RIGHT_HZ: u32 = 660;
const TONE_LEFT_AMP: i32 = 12_000;
const TONE_RIGHT_AMP: i32 = 9_000;

/// Generate the interleaved PCM payload (s16le, stereo): 4 bytes per
/// sample pair. Pure integer NCO — see [`SineOsc`].
fn generate_pcm() -> Vec<u8> {
    let table = sine_full_table();
    let mut left = SineOsc::new(TONE_LEFT_HZ, AUDIO_RATE);
    let mut right = SineOsc::new(TONE_RIGHT_HZ, AUDIO_RATE);
    let mut pcm = Vec::with_capacity(TOTAL_SAMPLES * 4);
    for _ in 0..TOTAL_SAMPLES {
        let l = ((left.sample(&table) * TONE_LEFT_AMP) >> 15) as i16;
        let r = ((right.sample(&table) * TONE_RIGHT_AMP) >> 15) as i16;
        pcm.extend_from_slice(&l.to_le_bytes());
        pcm.extend_from_slice(&r.to_le_bytes());
    }
    pcm
}

// ---------------------------------------------------------------------------
// Procedural frame content (corpus-plan §1: colorbars + a frame counter
// drawn in the pixels, so seek accuracy is assertable from decoded data)
// ---------------------------------------------------------------------------

/// Minimal RGB raster, 3 bytes per pixel.
struct RgbImage {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

/// Background of the counter area (dark, far from every bar color).
const COLOR_BG: [u8; 3] = [32, 32, 32];
/// Counter digit color (full white — maximum Y contrast).
const COLOR_DIGIT: [u8; 3] = [255, 255, 255];

impl RgbImage {
    fn new(width: usize, height: usize, fill: [u8; 3]) -> RgbImage {
        let mut pixels = Vec::with_capacity(width * height * 3);
        for _ in 0..width * height {
            pixels.extend_from_slice(&fill);
        }
        RgbImage {
            width,
            height,
            pixels,
        }
    }

    /// Fill an axis-aligned rectangle (x0/y0 inclusive, w/h extent).
    fn fill_rect(&mut self, x0: usize, y0: usize, w: usize, h: usize, color: [u8; 3]) {
        for y in y0..(y0 + h).min(self.height) {
            for x in x0..(x0 + w).min(self.width) {
                let base = (y * self.width + x) * 3;
                self.pixels[base..base + 3].copy_from_slice(&color);
            }
        }
    }

    /// One pixel (test access).
    #[cfg(test)]
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let base = (y * self.width + x) * 3;
        [
            self.pixels[base],
            self.pixels[base + 1],
            self.pixels[base + 2],
        ]
    }
}

/// The eight 100%-amplitude color bars across the top strip.
const COLOR_BARS: [[u8; 3]; 8] = [
    [255, 255, 255],
    [255, 255, 0],
    [0, 255, 255],
    [0, 255, 0],
    [255, 0, 255],
    [255, 0, 0],
    [0, 0, 255],
    [0, 0, 0],
];

/// 7-segment digit map: A (top), B (top-right), C (bottom-right),
/// D (bottom), E (bottom-left), F (top-left), G (middle).
const DIGIT_SEGS: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],     // 0
    [false, true, true, false, false, false, false], // 1
    [true, true, false, true, true, false, true],    // 2
    [true, true, true, true, false, false, true],    // 3
    [false, true, true, false, false, true, true],   // 4
    [true, false, true, true, false, true, true],    // 5
    [true, false, true, true, true, true, true],     // 6
    [true, true, true, false, false, false, false],  // 7
    [true, true, true, true, true, true, true],      // 8
    [true, true, true, true, false, true, true],     // 9
];

/// Digit cell geometry: cells are 64 px wide (horizontal segments 32×16,
/// vertical segments 16×32); the counter band spans y 56..168 (112 px).
/// All offsets are multiples of 8 so the drawing is stable under the
/// 8×8 DCT grid.
const DIGIT_W: usize = 64;

/// Draw one 7-segment digit with its top-left corner at (x0, y0).
fn draw_digit(img: &mut RgbImage, x0: usize, y0: usize, digit: usize) {
    let segments: [(usize, usize, usize, usize); 7] = [
        (16, 0, 32, 16),  // A
        (48, 16, 16, 32), // B
        (48, 64, 16, 32), // C
        (16, 96, 32, 16), // D
        (0, 64, 16, 32),  // E
        (0, 16, 16, 32),  // F
        (16, 48, 32, 16), // G
    ];
    for (on, &(sx, sy, sw, sh)) in DIGIT_SEGS[digit].iter().zip(segments.iter()) {
        let color = if *on { COLOR_DIGIT } else { COLOR_BG };
        img.fill_rect(x0 + sx, y0 + sy, sw, sh, color);
    }
}

/// Draw fixture frame `index` (0..FRAME_COUNT): color bars on top, the
/// two-digit frame counter center, a 10-step gray ramp at the bottom.
fn draw_frame(index: usize) -> Result<RgbImage, String> {
    if index >= FRAME_COUNT {
        return Err(format!(
            "frame index {index} out of range (0..{FRAME_COUNT})"
        ));
    }
    let mut img = RgbImage::new(FRAME_WIDTH, FRAME_HEIGHT, COLOR_BG);
    for (i, color) in COLOR_BARS.iter().enumerate() {
        img.fill_rect(i * 40, 0, 40, 24, *color);
    }
    for i in 0..10usize {
        let level = (25 * (i + 1)) as u8;
        img.fill_rect(i * 32, 216, 32, 24, [level, level, level]);
    }
    // Two-digit counter, centered horizontally in the middle band.
    let total_w = 2 * DIGIT_W + 16;
    let x0 = (FRAME_WIDTH - total_w) / 2;
    let y0 = 56;
    draw_digit(&mut img, x0, y0, index / 10);
    draw_digit(&mut img, x0 + DIGIT_W + 16, y0, index % 10);
    Ok(img)
}

// ---------------------------------------------------------------------------
// Baseline JPEG encoder (ISO/IEC 10918-1, sequential DCT, Huffman)
//
// The "tiny in-tree baseline-JPEG writer" of doc 03 §7: keeps the MJPEG
// payloads license-clean and byte-reproducible. 4:2:0, one interleaved
// scan, per-image Huffman tables built from actual symbol frequencies.
// Integer fixed-point DCT only (corpus-plan §2: no float anywhere).
// ---------------------------------------------------------------------------

/// Forward-DCT basis in Q16: `A[u][x] = c(u)/2 * cos((2x+1) u pi / 16)`,
/// `c(0) = 1/sqrt(2)` — rounded at authoring time, row-major `[u*8+x]`.
const DCT_A: [i32; 64] = [
    23170, 23170, 23170, 23170, 23170, 23170, 23170, 23170, //
    32138, 27246, 18205, 6393, -6393, -18205, -27246, -32138, //
    30274, 12540, -12540, -30274, -30274, -12540, 12540, 30274, //
    27246, -6393, -32138, -18205, 18205, 32138, 6393, -27246, //
    23170, -23170, -23170, 23170, 23170, -23170, -23170, 23170, //
    18205, -32138, 6393, 27246, -27246, -6393, 32138, -18205, //
    12540, -30274, 30274, -12540, -12540, 30274, -30274, 12540, //
    6393, -18205, 27246, -32138, 32138, -27246, 18205, -6393,
];

/// Zigzag order: natural (row-major) position of the i-th coefficient
/// in scan order (ISO/IEC 10918-1 Figure A.6).
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, //
    17, 24, 32, 25, 18, 11, 4, 5, //
    12, 19, 26, 33, 40, 48, 41, 34, //
    27, 20, 13, 6, 7, 14, 21, 28, //
    35, 42, 49, 56, 57, 50, 43, 36, //
    29, 22, 15, 23, 30, 37, 44, 51, //
    58, 59, 52, 45, 38, 31, 39, 46, //
    53, 60, 61, 54, 47, 55, 62, 63,
];

/// Luminance quantization table, natural order (Annex K.3, public
/// specification table — cited, not copied from any implementation).
const Q_LUMA: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, //
    12, 12, 14, 19, 26, 58, 60, 55, //
    14, 13, 16, 24, 40, 57, 69, 56, //
    14, 17, 22, 29, 51, 87, 80, 62, //
    18, 22, 37, 56, 68, 109, 103, 77, //
    24, 35, 55, 64, 81, 104, 113, 92, //
    49, 64, 78, 87, 103, 121, 120, 101, //
    72, 92, 95, 98, 112, 100, 103, 99,
];

/// Chrominance quantization table, natural order (Annex K.3).
const Q_CHROMA: [u16; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, //
    18, 21, 26, 66, 99, 99, 99, 99, //
    24, 26, 56, 99, 99, 99, 99, 99, //
    47, 66, 99, 99, 99, 99, 99, 99, //
    99, 99, 99, 99, 99, 99, 99, 99, //
    99, 99, 99, 99, 99, 99, 99, 99, //
    99, 99, 99, 99, 99, 99, 99, 99, //
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Forward DCT + quantization of one raw-[0,255] block. Returns
/// natural-order quantized coefficients (`F[v*8+u]`, u horizontal).
///
/// Math: `F_q = Σ A[u][x]·f · A[v][y]` accumulates a 2^32 scale, then
/// `q = round(F_q / (Q << 32))`, round-half-away-from-zero — pure i64.
fn fdct_quantize(block: &[u8; 64], qtab: &[u16; 64]) -> [i16; 64] {
    let mut mid = [0i64; 64]; // mid[u*8 + y]
    for y in 0..8 {
        for u in 0..8 {
            let mut acc: i64 = 0;
            for x in 0..8 {
                acc += DCT_A[u * 8 + x] as i64 * (block[y * 8 + x] as i32 - 128) as i64;
            }
            mid[u * 8 + y] = acc;
        }
    }
    let mut out = [0i16; 64];
    for u in 0..8 {
        for v in 0..8 {
            let mut acc: i64 = 0;
            for y in 0..8 {
                acc += DCT_A[v * 8 + y] as i64 * mid[u * 8 + y];
            }
            let denom = (qtab[v * 8 + u] as i64) << 32;
            let half = denom >> 1;
            let q = if acc >= 0 {
                (acc + half) / denom
            } else {
                -((-acc + half) / denom)
            };
            out[v * 8 + u] = q as i16;
        }
    }
    out
}

/// Magnitude category of a DC/AC value: bit count of `|v|` (spec C.2).
fn magnitude_category(v: i32) -> u32 {
    let magnitude = v.unsigned_abs();
    if magnitude == 0 {
        0
    } else {
        32 - magnitude.leading_zeros()
    }
}

/// The `s` amplitude bits that follow a DC-diff/AC-size symbol: `v` for
/// non-negative values, `v + 2^s - 1` for negative (spec F.1.2.1).
fn amplitude_bits(v: i32, s: u32) -> u32 {
    if v >= 0 {
        v as u32
    } else {
        (v + ((1i32 << s) - 1)) as u32
    }
}

/// A canonical Huffman table (spec Annex C): per-length code counts,
/// the HUFFVAL symbol list, and the derived per-symbol code/length.
struct HuffmanTable {
    /// `bits[l]` = number of codes of length `l` (index 0 unused).
    bits: [u8; 17],
    /// Symbols in code order (the DHT HUFFVAL payload).
    symbols: Vec<u8>,
    /// Canonical code per symbol (valid where `lens[sym] > 0`).
    codes: [u16; 256],
    /// Code length per symbol (0 = symbol absent from the table).
    lens: [u8; 256],
}

/// Huffman tree node for the (deterministic) optimal-code construction.
struct HuffNode {
    weight: u64,
    /// Total-order tiebreak: leaves use the symbol value, internal
    /// nodes 256+creation index — identical trees on every run.
    ord: u32,
    left: Option<usize>,
    right: Option<usize>,
    sym: Option<u8>,
}

/// Build one Huffman table from symbol frequencies (deterministic:
/// smallest `(weight, ord)` pairs combine first). Code lengths are
/// capped at 16 bits with the spec's own count-adjustment procedure
/// (Annex K.3, the "shift a code from length i to i-2" loop).
fn build_huffman_table(freqs: &[u32; 256]) -> Result<HuffmanTable, String> {
    let symbols: Vec<u8> = (0..256)
        .filter(|&s| freqs[s] > 0)
        .map(|s| s as u8)
        .collect();
    if symbols.is_empty() {
        return Err("cannot build a Huffman table with no symbols".to_string());
    }

    // Tree depths per symbol (single-symbol tables use a 1-bit code).
    let mut depths = [0u8; 256];
    if symbols.len() == 1 {
        depths[symbols[0] as usize] = 1;
    } else {
        let mut nodes: Vec<HuffNode> = symbols
            .iter()
            .map(|&sym| HuffNode {
                weight: freqs[sym as usize] as u64,
                ord: sym as u32,
                left: None,
                right: None,
                sym: Some(sym),
            })
            .collect();
        let mut active: Vec<usize> = (0..nodes.len()).collect();
        while active.len() > 1 {
            active.sort_by_key(|&i| (nodes[i].weight, nodes[i].ord));
            let a = active.remove(0);
            let b = active.remove(0);
            let ord = 256 + nodes.len() as u32;
            nodes.push(HuffNode {
                weight: nodes[a].weight + nodes[b].weight,
                ord,
                left: Some(a),
                right: Some(b),
                sym: None,
            });
            active.push(nodes.len() - 1);
        }
        // Iterative depth walk from the root.
        let mut stack = vec![(active[0], 0u32)];
        while let Some((index, depth)) = stack.pop() {
            let node = &nodes[index];
            match node.sym {
                Some(sym) => {
                    let d = u8::try_from(depth).map_err(|_| {
                        format!("huffman depth {depth} overflows u8 for symbol {sym}")
                    })?;
                    depths[sym as usize] = d;
                }
                None => {
                    if let (Some(l), Some(r)) = (node.left, node.right) {
                        stack.push((l, depth + 1));
                        stack.push((r, depth + 1));
                    }
                }
            }
        }
    }

    // Length histogram (up to 32 before the cap), then the 16-bit cap
    // (Annex K.3 adjustment).
    let mut long_bits = [0u8; 33];
    for &sym in &symbols {
        let depth = depths[sym as usize];
        if depth == 0 || depth > 32 {
            return Err(format!(
                "huffman code length {depth} for symbol {sym} outside 1..=32 \
                 (content too skewed for the 16-bit cap procedure)"
            ));
        }
        long_bits[depth as usize] += 1;
    }
    for i in (17..=32).rev() {
        while long_bits[i] > 0 {
            // Find the next-shorter occupied length below i-1.
            let mut j = i - 2;
            while j >= 1 && long_bits[j] == 0 {
                j -= 1;
            }
            if j == 0 {
                return Err(format!("huffman length adjustment failed at length {i}"));
            }
            long_bits[i] -= 2;
            long_bits[i - 1] += 1;
            long_bits[j + 1] += 2;
            long_bits[j] -= 1;
        }
    }
    let mut bits = [0u8; 17];
    bits.copy_from_slice(&long_bits[..17]);

    // Canonical assignment (spec C.2): symbols ordered by tree depth
    // (ties by symbol value) receive the codes of each length in turn.
    let mut ordered: Vec<u8> = symbols.clone();
    ordered.sort_by_key(|&sym| (depths[sym as usize], sym));
    let total: usize = bits[1..].iter().map(|&c| c as usize).sum();
    if total != ordered.len() {
        return Err(format!(
            "huffman bits sum {total} != symbol count {}",
            ordered.len()
        ));
    }
    let mut table = HuffmanTable {
        bits,
        symbols: Vec::with_capacity(ordered.len()),
        codes: [0u16; 256],
        lens: [0u8; 256],
    };
    let mut code: u32 = 0;
    let mut next = 0usize;
    for length in 1..=16usize {
        for _ in 0..bits[length] {
            let sym = ordered[next];
            next += 1;
            if code >= (1u32 << length) {
                return Err(format!(
                    "huffman canonical code overflow at length {length}"
                ));
            }
            table.codes[sym as usize] = code as u16;
            table.lens[sym as usize] = length as u8;
            table.symbols.push(sym);
            code += 1;
        }
        code <<= 1;
    }
    Ok(table)
}

/// MSB-first bit writer with 0xFF byte stuffing (spec F.1.2.3).
struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    fn new() -> BitWriter {
        BitWriter {
            out: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    fn push(&mut self, code: u16, len: u8) {
        for i in (0..len).rev() {
            self.acc = (self.acc << 1) | ((code >> i) & 1) as u32;
            self.nbits += 1;
            if self.nbits == 8 {
                let byte = (self.acc & 0xff) as u8;
                self.out.push(byte);
                if byte == 0xff {
                    self.out.push(0x00);
                }
                self.acc >>= 8;
                self.nbits = 0;
            }
        }
    }

    /// Flush with 1-padding to the byte boundary (spec F.1.2.3).
    fn finish(mut self) -> Vec<u8> {
        while self.nbits != 0 {
            self.push(1, 1);
        }
        self.out
    }
}

/// RGB → YCbCr planes with 2×2 chroma subsampling (JFIF integer math).
/// Returns `(y, cb, cr)` with cb/cr at half resolution.
fn rgb_to_ycc420(img: &RgbImage) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    if img.width % 16 != 0 || img.height % 16 != 0 {
        return Err(format!(
            "frame {}x{} is not MCU-aligned (multiples of 16 required)",
            img.width, img.height
        ));
    }
    let mut y = vec![0u8; img.width * img.height];
    let mut cb_full = vec![0i32; img.width * img.height];
    let mut cr_full = vec![0i32; img.width * img.height];
    for idx in 0..img.width * img.height {
        let base = idx * 3;
        let r = img.pixels[base] as i32;
        let g = img.pixels[base + 1] as i32;
        let b = img.pixels[base + 2] as i32;
        y[idx] = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
        cb_full[idx] = ((-43 * r - 85 * g + 128 * b) >> 8) + 128;
        cr_full[idx] = ((128 * r - 107 * g - 21 * b) >> 8) + 128;
    }
    let cw = img.width / 2;
    let ch = img.height / 2;
    let mut cb = vec![0u8; cw * ch];
    let mut cr = vec![0u8; cw * ch];
    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_cb = 0i32;
            let mut sum_cr = 0i32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let idx = (cy * 2 + dy) * img.width + cx * 2 + dx;
                    sum_cb += cb_full[idx];
                    sum_cr += cr_full[idx];
                }
            }
            cb[cy * cw + cx] = ((sum_cb + 2) >> 2) as u8;
            cr[cy * cw + cx] = ((sum_cr + 2) >> 2) as u8;
        }
    }
    Ok((y, cb, cr))
}

/// Count the Huffman symbols one block will emit (pass 1 of 2), walking
/// the same zigzag path the emitter walks.
fn count_block_symbols(
    block: &[i16; 64],
    prev_dc: &mut i32,
    dc_freqs: &mut [u32; 256],
    ac_freqs: &mut [u32; 256],
) {
    let diff = block[0] as i32 - *prev_dc;
    *prev_dc = block[0] as i32;
    dc_freqs[magnitude_category(diff) as usize] += 1;
    let mut run: u32 = 0;
    for &zig in ZIGZAG.iter().skip(1) {
        let value = block[zig] as i32;
        if value == 0 {
            run += 1;
        } else {
            while run > 15 {
                ac_freqs[0xf0] += 1;
                run -= 16;
            }
            let size = magnitude_category(value);
            ac_freqs[((run << 4) | size) as usize] += 1;
            run = 0;
        }
    }
    if run > 0 {
        ac_freqs[0x00] += 1;
    }
}

/// Emit one block's bits (pass 2 of 2).
fn emit_block(
    writer: &mut BitWriter,
    block: &[i16; 64],
    prev_dc: &mut i32,
    dc_table: &HuffmanTable,
    ac_table: &HuffmanTable,
) {
    let diff = block[0] as i32 - *prev_dc;
    *prev_dc = block[0] as i32;
    let size = magnitude_category(diff);
    writer.push(dc_table.codes[size as usize], dc_table.lens[size as usize]);
    if size > 0 {
        writer.push(amplitude_bits(diff, size) as u16, size as u8);
    }
    let mut run: u32 = 0;
    for &zig in ZIGZAG.iter().skip(1) {
        let value = block[zig] as i32;
        if value == 0 {
            run += 1;
        } else {
            while run > 15 {
                writer.push(ac_table.codes[0xf0], ac_table.lens[0xf0]);
                run -= 16;
            }
            let sz = magnitude_category(value);
            let symbol = ((run << 4) | sz) as usize;
            writer.push(ac_table.codes[symbol], ac_table.lens[symbol]);
            writer.push(amplitude_bits(value, sz) as u16, sz as u8);
            run = 0;
        }
    }
    if run > 0 {
        writer.push(ac_table.codes[0x00], ac_table.lens[0x00]);
    }
}

/// Append one marker segment (`FF marker len payload`).
fn jpeg_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    out.extend_from_slice(&[0xff, marker]);
    let len = u16::try_from(payload.len() + 2)
        .unwrap_or(u16::MAX)
        .to_be_bytes();
    out.extend_from_slice(&len);
    out.extend_from_slice(payload);
}

/// One quantized MCU: 4 luma blocks + Cb + Cr (blocks in scan order).
struct Mcu {
    y: [[i16; 64]; 4],
    cb: [i16; 64],
    cr: [i16; 64],
}

/// Encode one RGB frame as a complete baseline JFIF JPEG (SOI…EOI).
///
/// Structure: APP0(JFIF) · DQT(2 tables) · SOF0 · DHT(4 tables) ·
/// SOS(one interleaved 4:2:0 scan) · entropy · EOI.
fn encode_baseline_jpeg(img: &RgbImage) -> Result<Vec<u8>, String> {
    let (y_plane, cb_plane, cr_plane) = rgb_to_ycc420(img)?;
    let mcu_w = img.width / 16;
    let mcu_h = img.height / 16;
    let cw = img.width / 2;

    let extract = |plane: &[u8], px: usize, py: usize, stride: usize| -> [u8; 64] {
        let mut block = [0u8; 64];
        for row in 0..8 {
            for col in 0..8 {
                block[row * 8 + col] = plane[(py + row) * stride + px + col];
            }
        }
        block
    };

    // Pass 1: quantize every MCU, count Huffman symbols.
    let mut mcus: Vec<Mcu> = Vec::with_capacity(mcu_w * mcu_h);
    let mut dc_freqs_y = [0u32; 256];
    let mut ac_freqs_y = [0u32; 256];
    let mut dc_freqs_c = [0u32; 256];
    let mut ac_freqs_c = [0u32; 256];
    let mut prev_dc_y = 0i32;
    let mut prev_dc_cb = 0i32;
    let mut prev_dc_cr = 0i32;
    for mcu_y in 0..mcu_h {
        for mcu_x in 0..mcu_w {
            let mut mcu = Mcu {
                y: [[0; 64]; 4],
                cb: [0; 64],
                cr: [0; 64],
            };
            for by in 0..2 {
                for bx in 0..2 {
                    let raw = extract(
                        &y_plane,
                        mcu_x * 16 + bx * 8,
                        mcu_y * 16 + by * 8,
                        img.width,
                    );
                    let block = fdct_quantize(&raw, &Q_LUMA);
                    count_block_symbols(&block, &mut prev_dc_y, &mut dc_freqs_y, &mut ac_freqs_y);
                    mcu.y[by * 2 + bx] = block;
                }
            }
            let raw = extract(&cb_plane, mcu_x * 8, mcu_y * 8, cw);
            let block = fdct_quantize(&raw, &Q_CHROMA);
            count_block_symbols(&block, &mut prev_dc_cb, &mut dc_freqs_c, &mut ac_freqs_c);
            mcu.cb = block;
            let raw = extract(&cr_plane, mcu_x * 8, mcu_y * 8, cw);
            let block = fdct_quantize(&raw, &Q_CHROMA);
            count_block_symbols(&block, &mut prev_dc_cr, &mut dc_freqs_c, &mut ac_freqs_c);
            mcu.cr = block;
            mcus.push(mcu);
        }
    }

    let dc_y = build_huffman_table(&dc_freqs_y)?;
    let ac_y = build_huffman_table(&ac_freqs_y)?;
    let dc_c = build_huffman_table(&dc_freqs_c)?;
    let ac_c = build_huffman_table(&ac_freqs_c)?;

    // Marker stream.
    let mut out = Vec::with_capacity(4096);
    out.extend_from_slice(&[0xff, 0xd8]); // SOI

    // APP0: JFIF v1.1, no thumbnail.
    let mut app0 = Vec::with_capacity(14);
    app0.extend_from_slice(b"JFIF\0");
    app0.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0]);
    jpeg_segment(&mut out, 0xe0, &app0);

    // DQT: both tables, zigzag order (spec B.2.4.1).
    let mut dqt = Vec::with_capacity(130);
    for (tq, table) in [(0u8, &Q_LUMA), (1u8, &Q_CHROMA)] {
        dqt.push(tq); // Pq = 0 (8-bit), Tq
        for &zig in ZIGZAG.iter() {
            dqt.push(table[zig] as u8);
        }
    }
    jpeg_segment(&mut out, 0xdb, &dqt);

    // SOF0: baseline, 8-bit, 3 components, 4:2:0.
    let mut sof0 = Vec::with_capacity(17);
    sof0.push(8);
    sof0.extend_from_slice(&(img.height as u16).to_be_bytes());
    sof0.extend_from_slice(&(img.width as u16).to_be_bytes());
    sof0.push(3);
    sof0.extend_from_slice(&[1, 0x22, 0]); // Y: id 1, h2v2, Tq 0
    sof0.extend_from_slice(&[2, 0x11, 1]); // Cb: id 2, h1v1, Tq 1
    sof0.extend_from_slice(&[3, 0x11, 1]); // Cr: id 3, h1v1, Tq 1
    jpeg_segment(&mut out, 0xc0, &sof0);

    // DHT: DC0(luma) AC0(luma) DC1(chroma) AC1(chroma).
    for (class, id, table) in [
        (0u8, 0u8, &dc_y),
        (1u8, 0u8, &ac_y),
        (0u8, 1u8, &dc_c),
        (1u8, 1u8, &ac_c),
    ] {
        let mut dht = Vec::with_capacity(1 + 16 + table.symbols.len());
        dht.push((class << 4) | id);
        for length in 1..=16 {
            dht.push(table.bits[length]);
        }
        dht.extend_from_slice(&table.symbols);
        jpeg_segment(&mut out, 0xc4, &dht);
    }

    // SOS: one interleaved scan over all three components.
    let mut sos = Vec::with_capacity(12);
    sos.push(3);
    sos.extend_from_slice(&[1, 0x00]); // Y: Td 0, Ta 0
    sos.extend_from_slice(&[2, 0x11]); // Cb: Td 1, Ta 1
    sos.extend_from_slice(&[3, 0x11]); // Cr: Td 1, Ta 1
    sos.extend_from_slice(&[0, 63, 0]); // Ss, Se, Ah/Al
    jpeg_segment(&mut out, 0xda, &sos);

    // Entropy-coded scan (pass 2).
    let mut writer = BitWriter::new();
    let mut prev_dc_y = 0i32;
    let mut prev_dc_cb = 0i32;
    let mut prev_dc_cr = 0i32;
    for mcu in &mcus {
        for block in &mcu.y {
            emit_block(&mut writer, block, &mut prev_dc_y, &dc_y, &ac_y);
        }
        emit_block(&mut writer, &mcu.cb, &mut prev_dc_cb, &dc_c, &ac_c);
        emit_block(&mut writer, &mcu.cr, &mut prev_dc_cr, &dc_c, &ac_c);
    }
    out.extend_from_slice(&writer.finish());
    out.extend_from_slice(&[0xff, 0xd9]); // EOI
    Ok(out)
}

// ---------------------------------------------------------------------------
// AVI writer (RIFF/AVI 1.0 — doc 03 §7 candidate 2; generated regardless of
// the Lead's container pick so the decision blocks nothing)
// ---------------------------------------------------------------------------

/// Little-endian scalar emitters (RIFF is little-endian throughout).
fn put_u16le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u32le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Recorded AVI structure: every offset the class-B derivatives, the
/// structure map and the demuxer tests need (corpus-plan §2 — the class-A
/// writer emits the map that makes B1 truncation principled).
struct AviLayout {
    /// File offset of the `RIFF` size field (the no-idx1 derivative
    /// patches it after dropping the index).
    riff_size_pos: usize,
    /// File offset of the `avih` chunk header.
    avih_offset: usize,
    /// File offsets of the two `LIST strl` headers (video, audio).
    strl_offsets: [usize; 2],
    /// File offset of the `LIST movi` header.
    movi_offset: usize,
    /// File offset of the first `movi` chunk header. `idx1` `dwOffset`
    /// values are relative to the `movi` FourCC, so chunk 0 sits at +4.
    movi_data_offset: usize,
    /// File offset of the `idx1` chunk header (== file length).
    idx1_offset: usize,
    /// Every `movi` chunk in order: (fourcc, header offset, data size).
    chunks: Vec<([u8; 4], usize, usize)>,
}

struct AviBuilt {
    bytes: Vec<u8>,
    layout: AviLayout,
}

/// Append one RIFF chunk (`fourcc <u32le size> payload [pad]`; chunks are
/// WORD-aligned and the pad byte is not counted in the size). Returns the
/// chunk-header offset.
fn avi_chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> usize {
    let offset = out.len();
    out.extend_from_slice(fourcc);
    put_u32le(out, payload.len() as u32);
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
    offset
}

/// Open a sized chunk (RIFF/LIST) with a zero size placeholder; returns
/// the offset of the size field for [`avi_end_sized`].
fn avi_begin_sized(out: &mut Vec<u8>, fourcc: &[u8; 4]) -> usize {
    out.extend_from_slice(fourcc);
    let size_pos = out.len();
    put_u32le(out, 0);
    size_pos
}

/// Patch the size field opened by [`avi_begin_sized`] to cover everything
/// written since (a RIFF/LIST size includes its form FourCC).
fn avi_end_sized(out: &mut Vec<u8>, size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}

/// Build the AVI+MJPEG+PCM parent (doc 03 §7): `vids`/`MJPG` video (one
/// `00dc` chunk per frame) and `auds` PCM s16le stereo audio (one `01wb`
/// chunk per frame tick), interleaved inside `LIST movi` and fully
/// indexed by `idx1` (every chunk flagged `AVIIF_KEYFRAME` — MJPEG and
/// PCM have only keyframes).
fn build_avi(frames: &[Vec<u8>], pcm: &[u8]) -> Result<AviBuilt, String> {
    if frames.len() != FRAME_COUNT {
        return Err(format!(
            "avi writer needs exactly {FRAME_COUNT} frames, got {}",
            frames.len()
        ));
    }
    let pcm_need = TOTAL_SAMPLES * AUDIO_CHANNELS * 2;
    if pcm.len() != pcm_need {
        return Err(format!(
            "avi writer needs {pcm_need} PCM bytes, got {}",
            pcm.len()
        ));
    }
    let tick_bytes = SAMPLES_PER_TICK * AUDIO_CHANNELS * 2;
    let max_video = frames.iter().map(Vec::len).max().unwrap_or(0);
    let suggested = (max_video.max(tick_bytes)) as u32;

    // `movi` payload size, for dwMaxBytesPerSec (integer ceil-division —
    // no float anywhere in the writer, corpus-plan §2).
    let mut movi_payload = 4usize; // the 'movi' FourCC
    for frame in frames {
        movi_payload += 8 + frame.len() + (frame.len() & 1);
    }
    movi_payload += FRAME_COUNT * (8 + tick_bytes);
    let max_bytes_per_sec =
        (movi_payload as u64 * u64::from(FPS) + FRAME_COUNT as u64 - 1) / FRAME_COUNT as u64;

    // avih — main AVI header (56-byte payload, 14 u32 fields).
    let micros_per_frame = ((1_000_000u64 + u64::from(FPS) / 2) / u64::from(FPS)) as u32; // 33333
    let mut avih = Vec::with_capacity(56);
    put_u32le(&mut avih, micros_per_frame); // dwMicroSecPerFrame
    put_u32le(
        &mut avih,
        u32::try_from(max_bytes_per_sec).unwrap_or(u32::MAX), // dwMaxBytesPerSec
    );
    put_u32le(&mut avih, 0); // dwPaddingGranularity
    put_u32le(&mut avih, 0x0000_0010 | 0x0000_0100); // AVIF_HASINDEX | AVIF_ISINTERLEAVED
    put_u32le(&mut avih, FRAME_COUNT as u32); // dwTotalFrames
    put_u32le(&mut avih, 0); // dwInitialFrames
    put_u32le(&mut avih, 2); // dwStreams
    put_u32le(&mut avih, suggested); // dwSuggestedBufferSize
    put_u32le(&mut avih, FRAME_WIDTH as u32); // dwWidth
    put_u32le(&mut avih, FRAME_HEIGHT as u32); // dwHeight
    for _ in 0..4 {
        put_u32le(&mut avih, 0); // dwReserved[4]
    }

    // strh — video stream header (56-byte payload).
    let mut strh_video = Vec::with_capacity(56);
    strh_video.extend_from_slice(b"vids");
    strh_video.extend_from_slice(b"MJPG");
    put_u32le(&mut strh_video, 0); // dwFlags
    put_u16le(&mut strh_video, 0); // wPriority
    put_u16le(&mut strh_video, 0); // wLanguage
    put_u32le(&mut strh_video, 0); // dwInitialFrames
    put_u32le(&mut strh_video, 1); // dwScale
    put_u32le(&mut strh_video, FPS); // dwRate -> rate/scale = 30 fps
    put_u32le(&mut strh_video, 0); // dwStart
    put_u32le(&mut strh_video, FRAME_COUNT as u32); // dwLength (frames)
    put_u32le(&mut strh_video, max_video as u32); // dwSuggestedBufferSize
    put_u32le(&mut strh_video, u32::MAX); // dwQuality = -1 (default)
    put_u32le(&mut strh_video, 0); // dwSampleSize (variable-size chunks)
    put_u16le(&mut strh_video, 0); // rcFrame left
    put_u16le(&mut strh_video, 0); // rcFrame top
    put_u16le(&mut strh_video, FRAME_WIDTH as u16); // rcFrame right
    put_u16le(&mut strh_video, FRAME_HEIGHT as u16); // rcFrame bottom

    // strf — video format: BITMAPINFOHEADER (40 bytes).
    let mut strf_video = Vec::with_capacity(40);
    put_u32le(&mut strf_video, 40); // biSize
    put_u32le(&mut strf_video, FRAME_WIDTH as u32); // biWidth
    put_u32le(&mut strf_video, FRAME_HEIGHT as u32); // biHeight
    put_u16le(&mut strf_video, 1); // biPlanes
    put_u16le(&mut strf_video, 24); // biBitCount
    strf_video.extend_from_slice(b"MJPG"); // biCompression
    put_u32le(&mut strf_video, (FRAME_WIDTH * FRAME_HEIGHT * 3) as u32); // biSizeImage
    put_u32le(&mut strf_video, 0); // biXPelsPerMeter
    put_u32le(&mut strf_video, 0); // biYPelsPerMeter
    put_u32le(&mut strf_video, 0); // biClrUsed
    put_u32le(&mut strf_video, 0); // biClrImportant

    // strh — audio stream header (PCM convention: dwScale 1, dwRate =
    // sample rate, dwLength = sample frames, dwSampleSize = block align).
    let mut strh_audio = Vec::with_capacity(56);
    strh_audio.extend_from_slice(b"auds");
    strh_audio.extend_from_slice(&[0u8; 4]); // fccHandler: none
    put_u32le(&mut strh_audio, 0); // dwFlags
    put_u16le(&mut strh_audio, 0); // wPriority
    put_u16le(&mut strh_audio, 0); // wLanguage
    put_u32le(&mut strh_audio, 0); // dwInitialFrames
    put_u32le(&mut strh_audio, 1); // dwScale
    put_u32le(&mut strh_audio, AUDIO_RATE); // dwRate
    put_u32le(&mut strh_audio, 0); // dwStart
    put_u32le(&mut strh_audio, TOTAL_SAMPLES as u32); // dwLength
    put_u32le(&mut strh_audio, tick_bytes as u32); // dwSuggestedBufferSize
    put_u32le(&mut strh_audio, u32::MAX); // dwQuality
    put_u32le(&mut strh_audio, (AUDIO_CHANNELS * 2) as u32); // dwSampleSize
    put_u16le(&mut strh_audio, 0); // rcFrame left
    put_u16le(&mut strh_audio, 0); // rcFrame top
    put_u16le(&mut strh_audio, 0); // rcFrame right
    put_u16le(&mut strh_audio, 0); // rcFrame bottom

    // strf — audio format: PCMWAVEFORMAT (16 bytes; no cbSize needed).
    let mut strf_audio = Vec::with_capacity(16);
    put_u16le(&mut strf_audio, 1); // wFormatTag = PCM
    put_u16le(&mut strf_audio, AUDIO_CHANNELS as u16); // nChannels
    put_u32le(&mut strf_audio, AUDIO_RATE); // nSamplesPerSec
    put_u32le(
        &mut strf_audio,
        AUDIO_RATE * (AUDIO_CHANNELS as u32) * 2, // nAvgBytesPerSec
    );
    put_u16le(&mut strf_audio, (AUDIO_CHANNELS * 2) as u16); // nBlockAlign
    put_u16le(&mut strf_audio, 16); // wBitsPerSample

    // Assembly. All sizes are patched in place, so no float or size
    // pre-computation beyond dwMaxBytesPerSec is needed.
    let mut out: Vec<u8> = Vec::with_capacity(movi_payload + 1024);
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    put_u32le(&mut out, 0);
    out.extend_from_slice(b"AVI ");

    let hdrl_pos = avi_begin_sized(&mut out, b"LIST");
    out.extend_from_slice(b"hdrl");
    let avih_offset = avi_chunk(&mut out, b"avih", &avih);
    let mut strl_offsets = [0usize; 2];
    for (slot, (strh, strf)) in [
        (0, (&strh_video, &strf_video)),
        (1, (&strh_audio, &strf_audio)),
    ] {
        let pos = avi_begin_sized(&mut out, b"LIST");
        out.extend_from_slice(b"strl");
        avi_chunk(&mut out, b"strh", strh);
        avi_chunk(&mut out, b"strf", strf);
        avi_end_sized(&mut out, pos);
        strl_offsets[slot] = pos;
    }
    avi_end_sized(&mut out, hdrl_pos);

    let movi_offset = out.len();
    let movi_pos = avi_begin_sized(&mut out, b"LIST");
    out.extend_from_slice(b"movi");
    let movi_fourcc_pos = movi_offset + 8;
    let mut chunks: Vec<([u8; 4], usize, usize)> = Vec::with_capacity(2 * FRAME_COUNT);
    for (index, frame) in frames.iter().enumerate() {
        let offset = avi_chunk(&mut out, b"00dc", frame);
        chunks.push((*b"00dc", offset, frame.len()));
        let from = index * tick_bytes;
        let tick = &pcm[from..from + tick_bytes];
        let offset = avi_chunk(&mut out, b"01wb", tick);
        chunks.push((*b"01wb", offset, tick.len()));
    }
    avi_end_sized(&mut out, movi_pos);
    let movi_data_offset = chunks
        .first()
        .map(|&(_, offset, _)| offset)
        .ok_or_else(|| "movi list has no chunks".to_string())?;

    let idx1_offset = out.len();
    let mut idx1: Vec<u8> = Vec::with_capacity(16 * chunks.len());
    for &(fourcc, offset, size) in &chunks {
        idx1.extend_from_slice(&fourcc);
        put_u32le(&mut idx1, 0x10); // AVIIF_KEYFRAME
        put_u32le(&mut idx1, (offset - movi_fourcc_pos) as u32); // dwOffset
        put_u32le(&mut idx1, size as u32); // dwSize
    }
    avi_chunk(&mut out, b"idx1", &idx1);
    avi_end_sized(&mut out, riff_size_pos);

    Ok(AviBuilt {
        bytes: out,
        layout: AviLayout {
            riff_size_pos,
            avih_offset,
            strl_offsets,
            movi_offset,
            movi_data_offset,
            idx1_offset,
            chunks,
        },
    })
}

// ---------------------------------------------------------------------------
// Structure maps (corpus-plan §2: "<fixture>.structure.json" — the offsets
// class-B truncation and field patches derive from)
// ---------------------------------------------------------------------------

/// Append a JSON string literal. The corpus strings are fixed ASCII, but
/// the escaping covers the general cases (never `format!` with floats —
/// there are none in the corpus by construction).
fn json_put_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                out.push_str("\\u");
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let value = ch as u32;
                for shift in [12u32, 8, 4, 0] {
                    out.push(HEX[((value >> shift) & 0xf) as usize] as char);
                }
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// Append a JSON integer (u64 Display — deterministic).
fn json_put_uint(out: &mut String, value: u64) {
    out.push_str(&value.to_string());
}

/// FourCC as a JSON string (`"00dc"` — printable ASCII by construction).
fn fourcc_str(tag: &[u8; 4]) -> String {
    tag.iter().map(|&b| b as char).collect()
}

/// Serialize the AVI structure map (compact single-line JSON — a machine
/// input, not prose).
fn avi_structure_json(avi: &AviBuilt) -> Vec<u8> {
    let layout = &avi.layout;
    let len = avi.bytes.len();
    let mut s = String::with_capacity(2048 + 48 * layout.chunks.len());
    s.push_str("{\"schema\":\"structure-map/1\",\"fixture\":");
    json_put_string(&mut s, "generated/valid/avi_mjpeg_pcm.avi");
    s.push_str(",\"size\":");
    json_put_uint(&mut s, len as u64);
    s.push_str(",\"note\":\"offsets of top-level chunks and movi entries; class-B derivatives derive their cut/patch positions from here (corpus-plan s1 B1/B3)\"");
    s.push_str(",\"top_level\":[");
    s.push_str("{\"fourcc\":\"RIFF\",\"form\":\"AVI \",\"offset\":0,\"size\":");
    json_put_uint(&mut s, (len - 8) as u64);
    s.push_str("},{\"fourcc\":\"LIST\",\"form\":\"hdrl\",\"offset\":12,\"size\":");
    json_put_uint(&mut s, (layout.movi_offset - 12) as u64);
    s.push_str("},{\"fourcc\":\"LIST\",\"form\":\"movi\",\"offset\":");
    json_put_uint(&mut s, layout.movi_offset as u64);
    s.push_str(",\"size\":");
    json_put_uint(&mut s, (layout.idx1_offset - layout.movi_offset) as u64);
    s.push_str("},{\"fourcc\":\"idx1\",\"form\":\"\",\"offset\":");
    json_put_uint(&mut s, layout.idx1_offset as u64);
    s.push_str(",\"size\":");
    json_put_uint(&mut s, (len - layout.idx1_offset) as u64);
    s.push_str("}]");
    s.push_str(",\"avi\":{\"riff_size_field_offset\":");
    json_put_uint(&mut s, layout.riff_size_pos as u64);
    s.push_str(",\"avih_offset\":");
    json_put_uint(&mut s, layout.avih_offset as u64);
    s.push_str(",\"strl_offsets\":[");
    json_put_uint(&mut s, layout.strl_offsets[0] as u64);
    s.push(',');
    json_put_uint(&mut s, layout.strl_offsets[1] as u64);
    s.push_str("],\"movi_offset\":");
    json_put_uint(&mut s, layout.movi_offset as u64);
    s.push_str(",\"movi_data_offset\":");
    json_put_uint(&mut s, layout.movi_data_offset as u64);
    s.push_str(",\"idx1_offset\":");
    json_put_uint(&mut s, layout.idx1_offset as u64);
    s.push_str(",\"chunk_count\":");
    json_put_uint(&mut s, layout.chunks.len() as u64);
    s.push_str(",\"chunks\":[");
    for (index, &(fourcc, offset, size)) in layout.chunks.iter().enumerate() {
        if index > 0 {
            s.push(',');
        }
        s.push_str("{\"fourcc\":");
        json_put_string(&mut s, &fourcc_str(&fourcc));
        s.push_str(",\"offset\":");
        json_put_uint(&mut s, offset as u64);
        s.push_str(",\"size\":");
        json_put_uint(&mut s, size as u64);
        s.push('}');
    }
    s.push_str("]}}");
    s.into_bytes()
}

// ---------------------------------------------------------------------------
// Generation pipeline
// ---------------------------------------------------------------------------

/// One generated file: path relative to the fixture tree root, bytes,
/// and the metadata the manifest/CORPUS rows carry.
struct GeneratedFile {
    path: String,
    bytes: Vec<u8>,
    class: &'static str,
    op: &'static str,
    parent: Option<String>,
}

/// Build the full fixture set in memory, in a fixed order. Pure
/// function of the code (see the module header's determinism contract).
fn generate_all() -> Result<Vec<GeneratedFile>, String> {
    // Shared video essence: every frame is encoded once and reused by
    // every container (identical JPEG bytes in the AVI and MP4 parents).
    let mut frames: Vec<Vec<u8>> = Vec::with_capacity(FRAME_COUNT);
    for index in 0..FRAME_COUNT {
        let img = draw_frame(index)?;
        frames.push(encode_baseline_jpeg(&img)?);
    }
    let pcm = generate_pcm();

    let avi = build_avi(&frames, &pcm)?;
    let mut files: Vec<GeneratedFile> = vec![GeneratedFile {
        path: "generated/valid/avi_mjpeg_pcm.avi".to_string(),
        bytes: avi.bytes.clone(),
        class: "A",
        op: "parent",
        parent: None,
    }];
    files.push(GeneratedFile {
        path: "generated/valid/avi_mjpeg_pcm.avi.structure.json".to_string(),
        bytes: avi_structure_json(&avi),
        class: "A",
        op: "structure-map",
        parent: Some("generated/valid/avi_mjpeg_pcm.avi".to_string()),
    });
    Ok(files)
}

/// Print the `--list` table (no writes).
fn print_file_table(files: &[GeneratedFile]) {
    println!(
        "{:<5} {:<13} {:>9}  {:<64}  path",
        "class", "op", "bytes", "sha256"
    );
    for file in files {
        println!(
            "{:<5} {:<13} {:>9}  {:<64}  {}",
            file.class,
            file.op,
            file.bytes.len(),
            sha256_hex(&file.bytes),
            file.path
        );
    }
    let total: usize = files.iter().map(|f| f.bytes.len()).sum();
    let media = files
        .iter()
        .filter(|f| f.path.ends_with(".avi") || f.path.ends_with(".mp4"))
        .count();
    println!(
        "result: {} file(s) ({} media fixture(s), {} bytes total)",
        files.len(),
        media,
        total
    );
}

/// Write every file under `base`, creating directories as needed.
fn write_tree(base: &Path, files: &[GeneratedFile]) -> Result<(), XtaskError> {
    for file in files {
        let path = base.join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
                context: format!("creating directory {}", parent.display()),
                source,
            })?;
        }
        fs::write(&path, &file.bytes).map_err(|source| XtaskError::Io {
            context: format!("writing fixture {}", path.display()),
            source,
        })?;
        println!("wrote {} ({} bytes)", file.path, file.bytes.len());
    }
    println!(
        "result: {} file(s) written under {}",
        files.len(),
        base.display()
    );
    Ok(())
}

/// Verify the tree under `base` matches regeneration byte-for-byte
/// (drift gate, `--check`). Returns `true` when drift was found.
fn verify_tree(base: &Path, files: &[GeneratedFile]) -> Result<bool, XtaskError> {
    let mut drift = false;
    for file in files {
        let path = base.join(&file.path);
        match fs::read(&path) {
            Ok(bytes) => {
                if bytes != file.bytes {
                    println!(
                        "drift: {} differs from the regenerated output — run `cargo xtask fixtures` and commit the refreshed tree",
                        file.path
                    );
                    drift = true;
                }
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "missing: {} — run `cargo xtask fixtures` and commit the generated tree",
                    file.path
                );
                drift = true;
            }
            Err(source) => {
                return Err(XtaskError::Io {
                    context: format!("reading fixture {}", path.display()),
                    source,
                });
            }
        }
    }
    if drift {
        println!("result: DRIFT — regenerate with `cargo xtask fixtures` and commit");
    } else {
        println!(
            "result: no drift — {} file(s) verified under {}",
            files.len(),
            base.display()
        );
    }
    Ok(drift)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- CLI ---------------------------------------------------------------

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_flag_prints_usage() {
        match parse_args(&args(&["--help"])) {
            Ok(Invocation::Help) => {}
            other => unreachable!("--help should parse to Help, got {other:?}"),
        }
        assert!(USAGE.contains("--check"));
        assert!(USAGE.contains("--dest"));
        assert!(USAGE.contains("--list"));
    }

    #[test]
    fn default_invocation_is_write_mode() {
        match parse_args(&args(&[])) {
            Ok(Invocation::Run { check, list, dest }) => {
                assert!(!check);
                assert!(!list);
                assert!(dest.is_none());
            }
            other => unreachable!("no args should parse to Run, got {other:?}"),
        }
    }

    #[test]
    fn flags_parse_and_reject_bad_usage() {
        match parse_args(&args(&["--check", "--dest", "/tmp/x"])) {
            Ok(Invocation::Run { check, list, dest }) => {
                assert!(check);
                assert!(!list);
                assert_eq!(dest, Some(PathBuf::from("/tmp/x")));
            }
            other => unreachable!("--check --dest should parse, got {other:?}"),
        }
        assert!(parse_args(&args(&["--list"])).is_ok());
        // Bad usage exits 2 through the shared XtaskError path.
        assert!(parse_args(&args(&["--no-such-flag"])).is_err());
        assert!(parse_args(&args(&["--dest"])).is_err());
        assert!(parse_args(&args(&["--check", "--list"])).is_err());
        assert!(parse_args(&args(&["--check", "--check"])).is_err());
    }

    // -- SHA-256 (FIPS 180-4 / NIST example vectors, public) --------------

    #[test]
    fn sha256_matches_nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&million_a),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn sha256_is_update_split_invariant() {
        // One-shot vs chunked updates must agree (the derivative seeds
        // hash parents in one shot, the manifest hashes the same bytes).
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let one_shot = {
            let mut h = Sha256::new();
            h.update(&data);
            hex_lower(&h.finish())
        };
        let chunked = {
            let mut h = Sha256::new();
            for chunk in data.chunks(7) {
                h.update(chunk);
            }
            hex_lower(&h.finish())
        };
        assert_eq!(one_shot, chunked);
    }

    // -- PRNG ----------------------------------------------------------------

    #[test]
    fn prng_is_deterministic_and_seed_sensitive() {
        let mut a = Xorshift64Star::from_digest(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let mut b = Xorshift64Star::from_digest(&[1, 2, 3, 4, 5, 6, 7, 8]);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = Xorshift64Star::from_digest(&[1, 2, 3, 4, 5, 6, 7, 9]);
        assert_ne!(a.next_u64(), c.next_u64());
    }

    #[test]
    fn prng_stays_in_bounds() {
        let mut rng = Xorshift64Star::from_digest(&[9; 8]);
        for bound in [2u64, 64, 512, 1_000_003] {
            for _ in 0..1000 {
                let value = rng.below(bound);
                assert!(value < bound);
            }
        }
    }

    // -- Sine NCO -------------------------------------------------------------

    #[test]
    fn sine_table_has_correct_shape() {
        let table = sine_full_table();
        assert_eq!(table[0], 0);
        assert_eq!(table[64], 32767);
        assert_eq!(table[128], 0);
        assert_eq!(table[192], -32767);
        assert_eq!(table[256], 0);
        // Quarter-wave symmetry must be exact by construction.
        for j in 0..=256 {
            assert_eq!(table[j], -table[(j + 128) & 0xff]);
        }
        // Monotone rise across the first quarter.
        for j in 1..=64 {
            assert!(table[j] >= table[j - 1], "not monotone at {j}");
        }
    }

    #[test]
    fn sine_oscillator_is_bounded_and_starts_at_zero() {
        let table = sine_full_table();
        let mut osc = SineOsc::new(440, 44100);
        assert_eq!(osc.sample(&table), 0);
        let mut peak = 0i32;
        let mut previous = 0i32;
        for _ in 0..44_100 {
            let value = osc.sample(&table);
            assert!((-(1 << 15)) < value && value < (1 << 15));
            peak = peak.max(value.abs());
            previous = value;
        }
        // After a full second at 440 Hz the peak must be close to full
        // scale (the LUT tops out at 32767; phase quantization costs a
        // little) — a broken oscillator (stuck/decaying) fails this.
        assert!(peak > 32_000, "peak too small: {peak} (last {previous})");
    }

    // -- PCM ----------------------------------------------------------------

    #[test]
    fn pcm_payload_has_exact_shape_and_starts_silent() {
        let pcm = generate_pcm();
        assert_eq!(pcm.len(), TOTAL_SAMPLES * 4);
        // Sample 0 of both channels is sin(0) = 0.
        assert_eq!(&pcm[..4], &[0, 0, 0, 0]);
        // Channel interleaving: left amplitude exceeds right amplitude
        // somewhere in the first quarter second (440 vs 660 Hz, 12000
        // vs 9000 amplitude — a swapped-channel bug fails this).
        let mut max_l = 0i32;
        let mut max_r = 0i32;
        for pair in pcm.chunks_exact(4).take(11_025) {
            let l = i16::from_le_bytes([pair[0], pair[1]]) as i32;
            let r = i16::from_le_bytes([pair[2], pair[3]]) as i32;
            max_l = max_l.max(l.abs());
            max_r = max_r.max(r.abs());
        }
        assert!(max_l > 10_000, "left peak too small: {max_l}");
        assert!(max_r > 7_000, "right peak too small: {max_r}");
        assert!(max_l > max_r, "channel plan violated: {max_l} vs {max_r}");
    }

    // -- Frame content ---------------------------------------------------------

    #[test]
    fn drawn_frame_matches_the_planned_geometry() {
        let img = match draw_frame(23) {
            Ok(img) => img,
            Err(message) => unreachable!("draw_frame(23) failed: {message}"),
        };
        assert_eq!(img.width, FRAME_WIDTH);
        assert_eq!(img.height, FRAME_HEIGHT);
        assert_eq!(img.pixels.len(), FRAME_WIDTH * FRAME_HEIGHT * 3);
        // Color bars: one probe pixel per 40px bar.
        for (i, expected) in COLOR_BARS.iter().enumerate() {
            assert_eq!(&img.pixel(i * 40 + 20, 10), expected, "bar {i}");
        }
        // Gray ramp: 10 steps of 32px across the bottom.
        for i in 0..10 {
            let level = (25 * (i + 1)) as u8;
            assert_eq!(img.pixel(i * 32 + 16, 228), [level, level, level]);
        }
        // Counter "23": digit geometry (64+16+64 = 144 wide at x0 = 88,
        // y 56..168). Segment G of the tens digit (2) is on; segment B
        // of the units digit (3) is on.
        assert_eq!(img.pixel(88 + 32, 56 + 56), COLOR_DIGIT, "tens G");
        assert_eq!(img.pixel(88 + 80 + 48 + 8, 56 + 32), COLOR_DIGIT, "units B");
        // Off-segment of the tens digit stays background (digit 2 has
        // no F segment).
        assert_eq!(img.pixel(88 + 8, 56 + 32), COLOR_BG, "tens F off");
        // Background between the digits.
        assert_eq!(img.pixel(88 + DIGIT_W + 8, 56 + 8), COLOR_BG);
    }

    #[test]
    fn frames_differ_and_reject_out_of_range() {
        let a = match draw_frame(0) {
            Ok(img) => img,
            Err(message) => unreachable!("draw_frame(0) failed: {message}"),
        };
        let b = match draw_frame(1) {
            Ok(img) => img,
            Err(message) => unreachable!("draw_frame(1) failed: {message}"),
        };
        assert_ne!(a.pixels, b.pixels);
        assert!(draw_frame(FRAME_COUNT).is_err());
    }

    // -- DCT / quantization ------------------------------------------------

    #[test]
    fn fdct_of_constant_block_is_dc_only() {
        // A constant c block has F(0,0) = 8·(c−128) exactly; with the
        // Annex-K Q tables the quantized DC is round(8(c−128)/Q00).
        for &(value, expected) in &[(128u8, 0i16), (100, -14), (156, 14), (0, -64), (255, 63)] {
            let block = [value; 64];
            let out = fdct_quantize(&block, &Q_LUMA);
            assert_eq!(out[0], expected, "DC of constant {value}");
            for (i, &coefficient) in out.iter().enumerate().skip(1) {
                assert_eq!(coefficient, 0, "AC {i} of constant block must vanish");
            }
        }
    }

    #[test]
    fn fdct_matches_float_reference_on_a_gradient_block() {
        // Reference computed independently (Python, float64 DCT + the
        // same round-half-away-from-zero quantization): block values
        // (3·x + 5·y) mod 256.
        let mut block = [0u8; 64];
        for y in 0..8 {
            for x in 0..8 {
                block[y * 8 + x] = ((3 * x + 5 * y) % 256) as u8;
            }
        }
        let expected: [i16; 64] = [
            -50, -5, 0, 0, 0, 0, 0, 0, //
            -8, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
            -1, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let out = fdct_quantize(&block, &Q_LUMA);
        // The float reference and the Q16 fixed-point path can differ
        // by one on knife-edge roundings; anything more is a bug.
        for (i, (&got, &want)) in out.iter().zip(expected.iter()).enumerate() {
            let delta = (got - want).abs();
            assert!(delta <= 1, "coefficient {i}: got {got}, want {want}");
        }
    }

    // -- Huffman ----------------------------------------------------------------

    #[test]
    fn huffman_table_is_prefix_free_and_complete() {
        // Fibonacci-ish frequencies force a deep tree (depth > 16
        // exercises the Annex-K.3 length adjustment).
        let mut freqs = [0u32; 256];
        let mut weight = 1u32;
        let mut next = 1u32;
        for slot in freqs.iter_mut().take(24) {
            *slot = weight;
            let sum = weight.wrapping_add(next);
            weight = next;
            next = sum;
        }
        let table = match build_huffman_table(&freqs) {
            Ok(table) => table,
            Err(message) => unreachable!("fibonacci table failed: {message}"),
        };
        // All lengths within the JPEG cap.
        for &sym in &table.symbols {
            assert!(table.lens[sym as usize] <= 16);
        }
        // Kraft sum must be exactly 1 (complete canonical code).
        let mut kraft = 0u64;
        for &sym in &table.symbols {
            kraft += 1u64 << (16 - table.lens[sym as usize]);
        }
        assert_eq!(kraft, 1 << 16);
        // No code is a prefix of another (prefix-free).
        let mut codes: Vec<(u16, u8)> = table
            .symbols
            .iter()
            .map(|&sym| (table.codes[sym as usize], table.lens[sym as usize]))
            .collect();
        codes.sort();
        for pair in codes.windows(2) {
            let (a, la) = pair[0];
            let (b, lb) = pair[1];
            let shift = 16 - la.min(lb);
            assert_ne!(
                ((a as u32) << 16) >> shift,
                ((b as u32) << 16) >> shift,
                "code {a:b}/{la} prefixes {b:b}/{lb}"
            );
        }
    }

    #[test]
    fn huffman_single_symbol_uses_a_one_bit_code() {
        let mut freqs = [0u32; 256];
        freqs[0x2a] = 7;
        let table = match build_huffman_table(&freqs) {
            Ok(table) => table,
            Err(message) => unreachable!("single-symbol table failed: {message}"),
        };
        assert_eq!(table.symbols, vec![0x2a]);
        assert_eq!(table.lens[0x2a], 1);
        assert_eq!(table.codes[0x2a], 0);
        assert!(build_huffman_table(&[0u32; 256]).is_err());
    }

    #[test]
    fn huffman_round_trips_through_a_mini_decoder() {
        // Frequencies resembling a real AC table: EOB dominant, a few
        // run/size symbols, one ZRL.
        let mut freqs = [0u32; 256];
        freqs[0x00] = 900;
        freqs[0x01] = 200;
        freqs[0x11] = 60;
        freqs[0x21] = 30;
        freqs[0x02] = 15;
        freqs[0xf0] = 3;
        let table = match build_huffman_table(&freqs) {
            Ok(table) => table,
            Err(message) => unreachable!("table failed: {message}"),
        };
        let stream = [0x00u8, 0x01, 0x11, 0x21, 0x02, 0xf0, 0x00, 0x11];
        let mut writer = BitWriter::new();
        for &sym in &stream {
            writer.push(table.codes[sym as usize], table.lens[sym as usize]);
        }
        let bytes = writer.finish();
        // Decode MSB-first: on each step the top `len` bits of the
        // accumulator must equal exactly one symbol's code (the table
        // is prefix-free and complete).
        let mut decoded = Vec::new();
        let mut acc = 0u32;
        let mut have = 0u32;
        'outer: for &byte in &bytes {
            acc = (acc << 8) | byte as u32;
            have += 8;
            while have > 0 && decoded.len() < stream.len() {
                let mut matched = false;
                for &sym in &table.symbols {
                    let len = table.lens[sym as usize] as u32;
                    if len <= have && (acc >> (have - len)) as u16 == table.codes[sym as usize] {
                        decoded.push(sym);
                        have -= len;
                        acc &= (1u32 << have) - 1;
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    break; // need more bytes
                }
            }
            if decoded.len() == stream.len() {
                break 'outer;
            }
        }
        assert_eq!(decoded, stream);
    }

    // -- JPEG structure ----------------------------------------------------------

    /// Walk the marker segments of a JPEG up to and including SOS, then
    /// return the entropy-coded data span (after SOS, before the final
    /// EOI). The scan length itself is only knowable by decoding, so
    /// the span ends at `len - 2` (the writer always ends with EOI).
    fn jpeg_scan_span(jpeg: &[u8]) -> (usize, usize) {
        assert_eq!(&jpeg[0..2], &[0xff, 0xd8], "missing SOI");
        assert_eq!(&jpeg[jpeg.len() - 2..], &[0xff, 0xd9], "missing EOI at end");
        let mut pos = 2usize;
        let mut scan_start = None;
        while pos + 2 <= jpeg.len() {
            assert_eq!(jpeg[pos], 0xff, "expected a marker at {pos}");
            let marker = jpeg[pos + 1];
            assert!(
                pos + 4 <= jpeg.len(),
                "marker {marker:#x} truncated at {pos}"
            );
            let len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
            assert!(len >= 2, "segment length {len} at {pos}");
            assert!(
                pos + 2 + len <= jpeg.len(),
                "segment at {pos} overruns the file"
            );
            if marker == 0xda {
                scan_start = Some(pos + 2 + len);
                break; // entropy-coded data follows; the walk ends here
            }
            pos += 2 + len;
        }
        match scan_start {
            Some(start) => (start, jpeg.len() - 2),
            None => unreachable!("SOS never found"),
        }
    }

    #[test]
    fn encoded_jpeg_has_well_formed_structure() {
        let img = match draw_frame(42) {
            Ok(img) => img,
            Err(message) => unreachable!("draw_frame(42) failed: {message}"),
        };
        let jpeg = match encode_baseline_jpeg(&img) {
            Ok(jpeg) => jpeg,
            Err(message) => unreachable!("encode failed: {message}"),
        };
        assert!(jpeg.len() > 500, "suspiciously small: {}", jpeg.len());
        // All markers walk cleanly and the scan is byte-stuffed.
        let (start, end) = jpeg_scan_span(&jpeg);
        let scan = &jpeg[start..end];
        let mut i = 0usize;
        while i < scan.len() {
            if scan[i] == 0xff {
                assert!(i + 1 < scan.len(), "bare 0xFF at scan end");
                assert_eq!(scan[i + 1], 0x00, "unstuffed 0xFF in scan at {i}");
            }
            i += 1;
        }
        // Determinism: same frame encodes to identical bytes.
        let again = match encode_baseline_jpeg(&img) {
            Ok(jpeg) => jpeg,
            Err(message) => unreachable!("re-encode failed: {message}"),
        };
        assert_eq!(jpeg, again);
        // Different frames encode differently.
        let other = match draw_frame(43).and_then(|img| encode_baseline_jpeg(&img)) {
            Ok(jpeg) => jpeg,
            Err(message) => unreachable!("frame 43 failed: {message}"),
        };
        assert_ne!(jpeg, other);
    }

    #[test]
    fn every_fixture_frame_encodes() {
        for index in 0..FRAME_COUNT {
            let jpeg = match draw_frame(index).and_then(|img| encode_baseline_jpeg(&img)) {
                Ok(jpeg) => jpeg,
                Err(message) => unreachable!("frame {index} failed: {message}"),
            };
            let (_start, end) = jpeg_scan_span(&jpeg);
            assert!(end >= 2, "frame {index} has an empty scan");
        }
    }

    // -- Corpus pipeline + AVI writer ---------------------------------------

    use std::sync::OnceLock;

    /// The corpus, generated once per test binary (determinism makes the
    /// cache sound; the AC's "byte-stable across runs" has its own
    /// dedicated two-run test).
    fn corpus() -> &'static Result<Vec<GeneratedFile>, String> {
        static CORPUS: OnceLock<Result<Vec<GeneratedFile>, String>> = OnceLock::new();
        CORPUS.get_or_init(generate_all)
    }

    fn files() -> &'static [GeneratedFile] {
        match corpus() {
            Ok(files) => files.as_slice(),
            Err(message) => unreachable!("fixture generation failed: {message}"),
        }
    }

    fn find(path: &str) -> &'static GeneratedFile {
        match files().iter().find(|f| f.path == path) {
            Some(file) => file,
            None => unreachable!("fixture {path} missing from the corpus"),
        }
    }

    fn le32(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }
    fn be32(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }
    fn le16(b: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([b[at], b[at + 1]])
    }
    fn be16(b: &[u8], at: usize) -> u16 {
        u16::from_be_bytes([b[at], b[at + 1]])
    }

    #[test]
    fn corpus_layout_starts_with_the_avi_parent() {
        let files = files();
        assert_eq!(files[0].path, "generated/valid/avi_mjpeg_pcm.avi");
        assert_eq!(files[0].class, "A");
        assert_eq!(files[0].op, "parent");
        assert!(files[0].parent.is_none());
        assert!(files[0].bytes.len() > 300_000, "avi parent too small");
        // Unique paths only (the manifest/CORPUS rows key on them).
        let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        paths.sort_unstable();
        let total = paths.len();
        paths.dedup();
        assert_eq!(paths.len(), total, "duplicate fixture path");
    }

    #[test]
    fn avi_parent_is_a_self_consistent_riff_avi() {
        let b = &find("generated/valid/avi_mjpeg_pcm.avi").bytes;
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(le32(b, 4) as usize, b.len() - 8, "RIFF size");
        assert_eq!(&b[8..12], b"AVI ");

        // LIST hdrl at 12; its children are avih + two LIST strl.
        assert_eq!(&b[12..16], b"LIST");
        assert_eq!(&b[20..24], b"hdrl");
        let hdrl_end = 12 + 8 + le32(b, 16) as usize;
        let mut walk = 24usize;
        let mut children: Vec<[u8; 4]> = Vec::new();
        while walk < hdrl_end {
            children.push([b[walk], b[walk + 1], b[walk + 2], b[walk + 3]]);
            let size = le32(b, walk + 4) as usize;
            walk += 8 + size + (size & 1);
        }
        assert_eq!(walk, hdrl_end, "hdrl children overrun the list");
        assert_eq!(children.len(), 3, "hdrl children: {children:?}");
        assert_eq!(&children[0], b"avih");
        assert_eq!(&children[1], b"LIST");
        assert_eq!(&children[2], b"LIST");

        // avih payload at 24 + 8 = 32: 14 u32 fields.
        let avih = 32usize;
        assert_eq!(le32(b, avih), 33_333, "dwMicroSecPerFrame");
        assert!(le32(b, avih + 4) > 0, "dwMaxBytesPerSec");
        assert_eq!(le32(b, avih + 8), 0, "dwPaddingGranularity");
        assert_eq!(
            le32(b, avih + 12),
            0x110,
            "AVIF_HASINDEX|AVIF_ISINTERLEAVED"
        );
        assert_eq!(le32(b, avih + 16), FRAME_COUNT as u32, "dwTotalFrames");
        assert_eq!(le32(b, avih + 20), 0, "dwInitialFrames");
        assert_eq!(le32(b, avih + 24), 2, "dwStreams");
        assert_eq!(le32(b, avih + 32), FRAME_WIDTH as u32, "dwWidth");
        assert_eq!(le32(b, avih + 36), FRAME_HEIGHT as u32, "dwHeight");
        for i in 0..4 {
            assert_eq!(le32(b, avih + 40 + i * 4), 0, "dwReserved[{i}]");
        }

        // Video strl at 24 + 8 + 56 = 88.
        let strl_v = 88usize;
        assert_eq!(&b[strl_v..strl_v + 4], b"LIST");
        assert_eq!(&b[strl_v + 8..strl_v + 12], b"strl");
        assert_eq!(&b[strl_v + 12..strl_v + 16], b"strh");
        assert_eq!(le32(b, strl_v + 16) as usize, 56, "video strh size");
        let vh = strl_v + 20; // strh payload
        assert_eq!(&b[vh..vh + 4], b"vids");
        assert_eq!(&b[vh + 4..vh + 8], b"MJPG");
        assert_eq!(le32(b, vh + 20), 1, "dwScale");
        assert_eq!(le32(b, vh + 24), FPS, "dwRate");
        assert_eq!(le32(b, vh + 32), FRAME_COUNT as u32, "dwLength");
        assert_eq!(le32(b, vh + 44), 0, "dwSampleSize");
        assert_eq!(le16(b, vh + 52), FRAME_WIDTH as u16, "rcFrame right");
        assert_eq!(le16(b, vh + 54), FRAME_HEIGHT as u16, "rcFrame bottom");
        // Video strf at strl_v + 12 + 8 + 56 = 164: BITMAPINFOHEADER
        // (payload at 172; biCompression sits 16 bytes into it).
        assert_eq!(&b[strl_v + 76..strl_v + 80], b"strf");
        assert_eq!(le32(b, strl_v + 80) as usize, 40, "biSize");
        assert_eq!(&b[strl_v + 100..strl_v + 104], b"MJPG");
        assert_eq!(le32(b, strl_v + 88), FRAME_WIDTH as u32, "biWidth");
        assert_eq!(le32(b, strl_v + 92), FRAME_HEIGHT as u32, "biHeight");
        assert_eq!(le16(b, strl_v + 96), 1, "biPlanes");
        assert_eq!(le16(b, strl_v + 98), 24, "biBitCount");

        // Audio strl right after (8 hdr + 4 'strl' + 64 + 48 = 124 bytes).
        let strl_a = strl_v + 124;
        assert_eq!(&b[strl_a..strl_a + 4], b"LIST");
        assert_eq!(&b[strl_a + 12..strl_a + 16], b"strh");
        let ah = strl_a + 20;
        assert_eq!(&b[ah..ah + 4], b"auds");
        assert_eq!(le32(b, ah + 4), 0, "audio fccHandler");
        assert_eq!(le32(b, ah + 20), 1, "dwScale");
        assert_eq!(le32(b, ah + 24), AUDIO_RATE, "dwRate");
        assert_eq!(le32(b, ah + 32), TOTAL_SAMPLES as u32, "dwLength");
        assert_eq!(le32(b, ah + 44), 4, "dwSampleSize");
        // Audio strf: PCMWAVEFORMAT, 16 bytes.
        assert_eq!(&b[strl_a + 76..strl_a + 80], b"strf");
        assert_eq!(le32(b, strl_a + 80) as usize, 16, "audio strf size");
        let af = strl_a + 84;
        assert_eq!(le16(b, af), 1, "wFormatTag PCM");
        assert_eq!(le16(b, af + 2), AUDIO_CHANNELS as u16, "nChannels");
        assert_eq!(le32(b, af + 4), AUDIO_RATE, "nSamplesPerSec");
        assert_eq!(le32(b, af + 8), 176_400, "nAvgBytesPerSec");
        assert_eq!(le16(b, af + 12), 4, "nBlockAlign");
        assert_eq!(le16(b, af + 14), 16, "wBitsPerSample");

        // LIST movi directly follows hdrl.
        let movi = hdrl_end;
        assert_eq!(&b[movi..movi + 4], b"LIST");
        assert_eq!(&b[movi + 8..movi + 12], b"movi");
        let movi_end = movi + 8 + le32(b, movi + 4) as usize;
        let movi_fourcc = movi + 8;
        let tick_bytes = SAMPLES_PER_TICK * AUDIO_CHANNELS * 2;
        let mut pos = movi + 12;
        let mut video_sizes = Vec::with_capacity(FRAME_COUNT);
        let mut audio_offsets = Vec::with_capacity(FRAME_COUNT);
        while pos < movi_end {
            let fourcc = [b[pos], b[pos + 1], b[pos + 2], b[pos + 3]];
            let size = le32(b, pos + 4) as usize;
            let data = pos + 8;
            if &fourcc == b"00dc" {
                assert_eq!(&b[data..data + 2], &[0xff, 0xd8], "chunk SOI at {pos}");
                assert_eq!(
                    &b[data + size - 2..data + size],
                    &[0xff, 0xd9],
                    "chunk EOI at {pos}"
                );
                video_sizes.push(size);
            } else {
                assert_eq!(&fourcc, b"01wb", "unexpected fourcc at {pos}");
                assert_eq!(size, tick_bytes, "audio chunk size at {pos}");
                audio_offsets.push(pos);
            }
            // WORD alignment: the next chunk starts at an even offset
            // (the pad byte is appended when the payload is odd-sized
            // and is not counted in `size`).
            let next = data + size + if (data + size) & 1 == 1 { 1 } else { 0 };
            assert_eq!(next & 1, 0, "chunk at {pos} not padded to even");
            pos = next;
        }
        assert_eq!(pos, movi_end, "movi chunk walk");
        assert_eq!(video_sizes.len(), FRAME_COUNT, "video chunk count");
        assert_eq!(audio_offsets.len(), FRAME_COUNT, "audio chunk count");

        // idx1: the last top-level chunk, one entry per movi chunk.
        let idx1 = movi_end;
        assert_eq!(&b[idx1..idx1 + 4], b"idx1");
        let entry_count = (b.len() - idx1 - 8) / 16;
        assert_eq!(entry_count, 2 * FRAME_COUNT, "idx1 entries");
        for i in 0..entry_count {
            let at = idx1 + 8 + 16 * i;
            assert_eq!(le32(b, at + 4), 0x10, "idx1 flags (keyframe) {i}");
            let offset = le32(b, at + 8) as usize;
            assert_eq!(
                &b[movi_fourcc + offset..movi_fourcc + offset + 4],
                &b[at..at + 4],
                "idx1 offset {i} does not point at its chunk"
            );
            assert_eq!(
                le32(b, at + 12) as usize,
                le32(b, movi_fourcc + offset + 4) as usize,
                "idx1 size {i}"
            );
        }
    }

    #[test]
    fn avi_movi_carries_the_planned_essence() {
        let b = &find("generated/valid/avi_mjpeg_pcm.avi").bytes;
        let pcm = generate_pcm();
        let tick_bytes = SAMPLES_PER_TICK * AUDIO_CHANNELS * 2;
        // Find movi via the fixed layout: hdrl ends where LIST movi starts
        // (avih chunk 64 + video strl 124 + audio strl 100 bytes).
        let movi = 12 + 8 + 4 + 64 + 124 + 100;
        assert_eq!(&b[movi..movi + 4], b"LIST");
        let mut pos = movi + 12;
        let mut saw_first = false;
        let mut saw_last = false;
        let mut tick_index = 0usize;
        while &b[pos..pos + 4] != b"idx1" {
            let fourcc = [b[pos], b[pos + 1], b[pos + 2], b[pos + 3]];
            let size = le32(b, pos + 4) as usize;
            let data = pos + 8;
            if &fourcc == b"00dc" {
                if tick_index == 0 || tick_index == FRAME_COUNT - 1 {
                    // First and last video chunk must be the exact
                    // re-encoded procedural frames (the full 60-frame
                    // equality is the determinism test's job).
                    let jpeg =
                        match draw_frame(tick_index).and_then(|img| encode_baseline_jpeg(&img)) {
                            Ok(jpeg) => jpeg,
                            Err(message) => unreachable!("re-encode failed: {message}"),
                        };
                    assert_eq!(&b[data..data + size], &jpeg[..], "frame {tick_index}");
                    if tick_index == 0 {
                        saw_first = true;
                    } else {
                        saw_last = true;
                    }
                }
            } else {
                let from = tick_index * tick_bytes;
                assert_eq!(&b[data..data + size], &pcm[from..from + tick_bytes]);
            }
            pos = data + size + if (data + size) & 1 == 1 { 1 } else { 0 };
            if &fourcc == b"01wb" {
                tick_index += 1;
            }
        }
        assert!(saw_first && saw_last, "first/last frames not verified");
        assert_eq!(tick_index, FRAME_COUNT, "audio tick count");
    }

    #[test]
    fn avi_structure_map_records_the_walked_layout() {
        let map = find("generated/valid/avi_mjpeg_pcm.avi.structure.json");
        let text = match String::from_utf8(map.bytes.clone()) {
            Ok(text) => text,
            Err(_) => unreachable!("structure map is not valid UTF-8"),
        };
        let b = &find("generated/valid/avi_mjpeg_pcm.avi").bytes;
        assert!(text.contains("\"schema\":\"structure-map/1\""));
        assert!(text.contains("\"size\":"));
        assert_eq!(text.matches("\"fourcc\":\"00dc\"").count(), FRAME_COUNT);
        assert_eq!(text.matches("\"fourcc\":\"01wb\"").count(), FRAME_COUNT);
        // The recorded movi/idx1 offsets must match a fresh walk.
        let movi = 12 + 8 + 4 + 64 + 124 + 100;
        assert!(text.contains(&format!("\"movi_offset\":{movi}")));
        let idx1_at = text.find("\"idx1_offset\":").unwrap_or(0) + "\"idx1_offset\":".len();
        let end = text[idx1_at..].find(',').unwrap_or(text.len() - idx1_at) + idx1_at;
        let recorded: usize = match text[idx1_at..end].parse() {
            Ok(value) => value,
            Err(_) => unreachable!("idx1_offset not an integer"),
        };
        assert_eq!(&b[recorded..recorded + 4], b"idx1");
        assert_eq!(recorded + 8 + 2 * 16 * FRAME_COUNT, b.len());
    }
}
