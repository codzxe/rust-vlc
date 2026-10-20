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
//!   generated/MANIFEST.toml          # generator + sha256 per fixture
//!   generated/valid/*.avi|*.mp4      # class A parents
//!   generated/valid/*.structure.json # structure maps (corpus-plan §2)
//!   generated/malformed/*__*.avi|mp4 # class B derivatives (10 per parent:
//!                                     #   trunc0..4, garbage0, flip1/4/16,
//!                                     #   noidx0 (AVI) / stszlie0 (MP4))
//!   expectations/valid|malformed/    # expectation/1 TOML per media fixture
//! ```
//!
//! Class column in `--list`: `A`/`B` are corpus classes (corpus-plan §1);
//! `L` marks generated ledger files (manifest, CORPUS, expectations) that
//! are drift-gated by `--check` but are not media fixtures.
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

/// `[first_frame]` digest (doc 10 §5.4): the SHA-256 of the luma
/// 8×8-block-mean descriptor of the FIRST decoded frame — every
/// block of the 320×240 luma plane reduced to its mean rounded
/// half-up (1200 u8 values in raster order), hashed. RECORDED VALUE,
/// not derived here: the writer is the encoder side and cannot
/// decode; this digest was recorded from Lumen's first compliant
/// decode (card 2.B4's `codecs::video::mjpeg::MjpegDecoder`, branch
/// wip/2b4-mjpeg-decoder) and is verified from the decoder side by
/// `crates/codecs/tests/fixtures_parent.rs`
/// (`first_frame_descriptor_matches_the_expectation`). Changing the
/// frame content, the encoder, or the decoder invalidates it — the
/// codecs test fails first, and this constant follows (worklog 2.B4
/// records the regeneration path). Both class-A parents carry the
/// same first frame byte-for-byte, so one value serves both.
const FIRST_FRAME_HASH_Y: &str = "415568c8f5596f7a155c1fc8b99d74245eba7c2ebce4f2084987881018ffabef";

/// `[first_frame]` presentation stamp: frame 0 of a 30 fps stream
/// presents at t = 0 (doc 10 §5.4 records the stamp the first
/// compliant decode actually reported).
const FIRST_FRAME_STAMP_US: u64 = 0;

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

/// MD5 (RFC 1321), std-only by hand: the `[audio_probe]` schema of
/// doc 10 §5.4 names `first_samples_md5`, and this writer is
/// dependency-free by C5. MD5 here is an **identity digest for test
/// comparability, not a security primitive** (RFC 1321 §1: "not
/// suitable as a message authentication / signature algorithm") —
/// exactly what an expectation-file probe needs. All arithmetic is
/// wrapping; the hash path cannot panic on overflow in debug builds.
fn md5_hex(data: &[u8]) -> String {
    // Per-round shift amounts (RFC 1321 §3.4).
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // Sine-derived table (RFC 1321 §3.4, the public spec table).
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    // Pad: 0x80, zeros to 56 mod 64, then 8-byte LITTLE-endian bit
    // length (RFC 1321 §3.1 — the opposite endianness of SHA-256's).
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());

    for block in message.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    // The digest is A,B,C,D serialized LITTLE-endian per word (RFC
    // 1321 §3.5) — the reverse of SHA-256's big-endian words.
    let mut out = String::with_capacity(32);
    for word in state {
        out.push_str(&format!("{:08x}", word.swap_bytes()));
    }
    out
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
        // `.take(8)` rather than `&seed[..8]`: a short seed must pad with
        // zeros, not panic (the no-panic convention applies here too,
        // even though every caller passes a 32-byte digest).
        for &byte in seed.iter().take(8) {
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
    for (j, slot) in table.iter_mut().enumerate() {
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
        *slot = magnitude;
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

/// `[audio_probe]` sample count (doc 10 §5.4 “first 4800 samples”):
/// the first 4800 **sample-instants** — all channels, interleave
/// order — of the decoded stream. At the corpus rate (44100 Hz stereo)
/// that is ≈108.8 ms and spans the first four per-tick audio chunks,
/// so the probe exercises multi-packet accumulation, not just chunk 0
/// (the sample-vs-scalar reading of “4800 samples” is pinned here:
/// instants, the demuxer/decoder vocabulary’s unit).
const AUDIO_PROBE_INSTANTS: usize = 4800;

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

/// The `[audio_probe]` digest (doc 10 §5.4 `first_samples_md5`, card
/// 2.B5): the **pinned conversion** of the first
/// [`AUDIO_PROBE_INSTANTS`] sample-instants of the interleaved s16le
/// stream — each scalar to IEEE-754 f32 as `v as f32 / 32768.0`
/// (exact: every i16 is exact in f32 and the divisor is a power of
/// two, so the conversion is bit-deterministic on every host),
/// serialized little-endian, hashed with MD5 (see [`md5_hex`]).
///
/// Provenance: computed from the generator's own sample stream — for
/// PCM the writer IS the sample source (s16le decode is the identity
/// on little-endian hosts and value-exact everywhere), so unlike
/// `FIRST_FRAME_HASH_Y` this is derived, not recorded-from-decode.
/// The decoder side mirrors the conversion in
/// `crates/codecs/tests/common/mod.rs` (`audio_probe_md5`) and the AC
/// suite recomputes it from the codec-api Decoder path's output —
/// writer and decoder pin each other; changing the tone plan, the
/// conversion or the decoder breaks the pair first.
fn audio_probe_md5(pcm: &[u8]) -> String {
    let need = AUDIO_PROBE_INSTANTS * AUDIO_CHANNELS * 2;
    let available = pcm.len().min(need);
    let mut probe = Vec::with_capacity(available * 2);
    for pair in pcm[..available].chunks_exact(2) {
        let sample = i16::from_le_bytes([pair[0], pair[1]]);
        probe.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
    }
    md5_hex(&probe)
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
    for (length, &count) in bits.iter().enumerate().skip(1) {
        for _ in 0..count {
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

/// The three 4:2:0 planes of one frame (Y full-res, Cb/Cr half-res).
type YccPlanes = (Vec<u8>, Vec<u8>, Vec<u8>);

/// RGB → YCbCr planes with 2×2 chroma subsampling (JFIF integer math).
/// Returns `(y, cb, cr)` with cb/cr at half resolution.
fn rgb_to_ycc420(img: &RgbImage) -> Result<YccPlanes, String> {
    if !img.width.is_multiple_of(16) || !img.height.is_multiple_of(16) {
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
fn avi_end_sized(out: &mut [u8], size_pos: usize) {
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
    let max_bytes_per_sec = (movi_payload as u64 * u64::from(FPS)).div_ceil(FRAME_COUNT as u64);

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
// MP4 writer (ISO/IEC 14496-12/-14 progressive — the second decision-
// independent candidate; ftyp + moov(two traks) + mdat)
// ---------------------------------------------------------------------------

/// Big-endian scalar emitters (ISO boxes are big-endian throughout).
fn put_u16be(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn put_u32be(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Recorded MP4 structure (corpus-plan §2 structure map, typed form).
struct Mp4Layout {
    /// File offsets of the two `trak` boxes (video, audio).
    trak_offsets: [usize; 2],
    /// File offset of the `mvhd` box.
    mvhd_offset: usize,
    /// File offset of the `mdat` box header.
    mdat_offset: usize,
    /// File offset of the first `mdat` payload byte (sample 0 of video).
    mdat_data_offset: usize,
    /// File offset of the video `stsz` `sample_count` field (the B3
    /// count-lie derivative patches exactly this u32).
    stsz_video_count_offset: usize,
    /// Absolute file offset of every video sample (== `stco` values).
    video_offsets: Vec<u32>,
    /// Absolute file offset of every audio sample (== `stco` values).
    audio_offsets: Vec<u32>,
    /// Size of every video sample (== `stsz` values; audio is uniform).
    video_sizes: Vec<u32>,
}

struct Mp4Built {
    bytes: Vec<u8>,
    layout: Mp4Layout,
}

/// Append one complete box (`size` + `kind` + payload). Returns the box
/// offset.
fn mp4_box(out: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) -> usize {
    let offset = out.len();
    let size = 8u32.wrapping_add(payload.len() as u32);
    put_u32be(out, size);
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    offset
}

/// Open a box with a zero size placeholder; returns the offset of the
/// size field (== the box start) for [`mp4_end_box`].
fn mp4_begin_box(out: &mut Vec<u8>, kind: &[u8; 4]) -> usize {
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(kind);
    out.len() - 8
}

/// Patch the size field opened by [`mp4_begin_box`] (full box size,
/// header included).
fn mp4_end_box(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_be_bytes());
}

/// FullBox payload: version(1) + flags(3) + body.
fn mp4_fullbox_body(version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + body.len());
    payload.push(version);
    payload.extend_from_slice(&flags.to_be_bytes()[1..4]);
    payload.extend_from_slice(body);
    payload
}

/// `dinf > dref > url ` with the self-contained flag (both tracks).
/// `dref` is a FullBox: version/flags, entry_count, then the entries.
fn mp4_dinf(out: &mut Vec<u8>) {
    let dinf_pos = mp4_begin_box(out, b"dinf");
    let mut dref_body = Vec::with_capacity(24);
    put_u32be(&mut dref_body, 1); // entry_count
    put_u32be(&mut dref_body, 12); // 'url ' box size
    dref_body.extend_from_slice(b"url ");
    dref_body.extend_from_slice(&mp4_fullbox_body(0, 1, &[]));
    mp4_box(out, b"dref", &mp4_fullbox_body(0, 0, &dref_body));
    mp4_end_box(out, dinf_pos);
}

/// The `'jpeg'` video sample entry (VisualSampleEntry, 78-byte body).
fn mp4_jpeg_sample_entry() -> Vec<u8> {
    let mut body = Vec::with_capacity(78);
    body.extend_from_slice(&[0u8; 6]); // reserved
    put_u16be(&mut body, 1); // data_reference_index
    put_u16be(&mut body, 0); // pre_defined
    put_u16be(&mut body, 0); // reserved
    body.extend_from_slice(&[0u8; 12]); // pre_defined[3]
    put_u16be(&mut body, FRAME_WIDTH as u16);
    put_u16be(&mut body, FRAME_HEIGHT as u16);
    put_u32be(&mut body, 0x0048_0000); // horizresolution 72 dpi
    put_u32be(&mut body, 0x0048_0000); // vertresolution
    put_u32be(&mut body, 0); // reserved
    put_u16be(&mut body, 1); // frame_count
    body.push(5); // compressorname: Pascal string "MJPEG"
    body.extend_from_slice(b"MJPEG");
    body.extend_from_slice(&[0u8; 26]);
    put_u16be(&mut body, 24); // depth
    put_u16be(&mut body, 0xffff); // pre_defined = -1
    let mut entry = Vec::with_capacity(8 + 78);
    put_u32be(&mut entry, 8 + 78);
    entry.extend_from_slice(b"jpeg");
    entry.extend_from_slice(&body);
    entry
}

/// The `'sowt'` audio sample entry (AudioSampleEntry v0, 28-byte body:
/// little-endian s16 PCM in an MP4 file).
fn mp4_sowt_sample_entry() -> Vec<u8> {
    let mut body = Vec::with_capacity(28);
    body.extend_from_slice(&[0u8; 6]); // reserved
    put_u16be(&mut body, 1); // data_reference_index
    put_u16be(&mut body, 0); // version
    put_u16be(&mut body, 0); // revision
    put_u32be(&mut body, 0); // vendor
    put_u16be(&mut body, AUDIO_CHANNELS as u16);
    put_u16be(&mut body, 16); // samplesize
    put_u16be(&mut body, 0); // compression_id
    put_u16be(&mut body, 0); // packet_size
    put_u32be(&mut body, AUDIO_RATE << 16); // samplerate 16.16
    let mut entry = Vec::with_capacity(8 + 28);
    put_u32be(&mut entry, 8 + 28);
    entry.extend_from_slice(b"sowt");
    entry.extend_from_slice(&body);
    entry
}

/// The sample-table box for one track. `sync_all` adds an `stss` listing
/// every sample (MJPEG has only keyframes). Returns the absolute offsets
/// of the `stsz` count field and of the first `stco` entry (the entries
/// are zero placeholders here — `build_mp4` patches them once `mdat` is
/// laid out; the u32 width makes the patch size-neutral).
fn mp4_stbl(
    out: &mut Vec<u8>,
    sample_entry: &[u8],
    sample_count: usize,
    sample_sizes: &[u32],
    stts_delta: u32,
    sync_all: bool,
) -> Result<(usize, usize), String> {
    if sample_sizes.len() != sample_count {
        return Err(format!(
            "stbl needs {sample_count} sizes, got {}",
            sample_sizes.len()
        ));
    }
    let stbl_pos = mp4_begin_box(out, b"stbl");
    // stsd
    let mut stsd_body = Vec::with_capacity(8 + sample_entry.len());
    put_u32be(&mut stsd_body, 1); // entry_count
    stsd_body.extend_from_slice(sample_entry);
    mp4_box(out, b"stsd", &mp4_fullbox_body(0, 0, &stsd_body));
    // stts: one run, uniform delta.
    let mut stts_body = Vec::with_capacity(8);
    put_u32be(&mut stts_body, 1); // entry_count
    put_u32be(&mut stts_body, sample_count as u32);
    put_u32be(&mut stts_body, stts_delta);
    mp4_box(out, b"stts", &mp4_fullbox_body(0, 0, &stts_body));
    // stss: every sample is a sync sample (video only).
    if sync_all {
        let mut body = Vec::with_capacity(4 + 4 * sample_count);
        put_u32be(&mut body, sample_count as u32); // entry_count
        for i in 0..sample_count {
            put_u32be(&mut body, i as u32 + 1);
        }
        mp4_box(out, b"stss", &mp4_fullbox_body(0, 0, &body));
    }
    // stsc: one chunk per sample.
    let mut stsc_body = Vec::with_capacity(16);
    put_u32be(&mut stsc_body, 1); // entry_count
    put_u32be(&mut stsc_body, 1); // first_chunk
    put_u32be(&mut stsc_body, 1); // samples_per_chunk
    put_u32be(&mut stsc_body, 1); // sample_description_index
    mp4_box(out, b"stsc", &mp4_fullbox_body(0, 0, &stsc_body));
    // stsz: per-sample sizes (sample_size 0).
    let mut stsz_body = Vec::with_capacity(12 + 4 * sample_sizes.len());
    put_u32be(&mut stsz_body, 0);
    put_u32be(&mut stsz_body, sample_count as u32);
    for size in sample_sizes {
        put_u32be(&mut stsz_body, *size);
    }
    let stsz_offset = out.len();
    mp4_box(out, b"stsz", &mp4_fullbox_body(0, 0, &stsz_body));
    let stsz_count_offset = stsz_offset + 8 + 4 + 4;
    // stco: placeholder zeros (patched after mdat is laid out).
    let mut stco_body = Vec::with_capacity(8 + 4 * sample_count);
    put_u32be(&mut stco_body, sample_count as u32);
    stco_body.extend_from_slice(&vec![0u8; 4 * sample_count]);
    let stco_offset = out.len();
    mp4_box(out, b"stco", &mp4_fullbox_body(0, 0, &stco_body));
    let stco_entries_offset = stco_offset + 8 + 4 + 4;
    mp4_end_box(out, stbl_pos);
    Ok((stsz_count_offset, stco_entries_offset))
}

/// One `trak` box: `tkhd` + `mdia(mdhd, hdlr, minf(mhdr, dinf, stbl))`.
/// Built in its own Vec so the caller can splice it into `moov` while
/// keeping the returned patch offsets relative to the trak start.
struct TrakBuilt {
    bytes: Vec<u8>,
    stsz_count_offset: usize,
    stco_entries_offset: usize,
}

#[allow(clippy::too_many_arguments)]
fn mp4_trak(
    track_id: u32,
    movie_duration: u32,
    media_timescale: u32,
    media_duration: u32,
    handler: &[u8; 4],
    handler_name: &str,
    is_video: bool,
    tkhd_width_height: Option<(u32, u32)>,
    sample_entry: &[u8],
    sample_sizes: &[u32],
    stts_delta: u32,
    sync_all: bool,
) -> Result<TrakBuilt, String> {
    let mut out: Vec<u8> = Vec::with_capacity(1024);
    let trak_pos = mp4_begin_box(&mut out, b"trak");
    // tkhd (flags 3 = track_enabled | track_in_movie).
    let mut tkhd = Vec::with_capacity(80);
    put_u32be(&mut tkhd, 0); // creation_time
    put_u32be(&mut tkhd, 0); // modification_time
    put_u32be(&mut tkhd, track_id);
    put_u32be(&mut tkhd, 0); // reserved
    put_u32be(&mut tkhd, movie_duration); // in mvhd timescale
    tkhd.extend_from_slice(&[0u8; 8]); // reserved
    put_u16be(&mut tkhd, 0); // layer
    put_u16be(&mut tkhd, 0); // alternate_group
    put_u16be(&mut tkhd, if is_video { 0 } else { 0x0100 }); // volume
    put_u16be(&mut tkhd, 0); // reserved
    // Unity matrix.
    put_u32be(&mut tkhd, 0x0001_0000);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0x0001_0000);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0);
    put_u32be(&mut tkhd, 0x4000_0000);
    let (width, height) = tkhd_width_height.unwrap_or((0, 0));
    put_u32be(&mut tkhd, width); // 16.16 fixed
    put_u32be(&mut tkhd, height);
    mp4_box(&mut out, b"tkhd", &mp4_fullbox_body(0, 3, &tkhd));
    // mdia
    let mdia_pos = mp4_begin_box(&mut out, b"mdia");
    let mut mdhd = Vec::with_capacity(16);
    put_u32be(&mut mdhd, 0); // creation_time (no wall clock — ever)
    put_u32be(&mut mdhd, 0); // modification_time
    put_u32be(&mut mdhd, media_timescale);
    put_u32be(&mut mdhd, media_duration);
    put_u16be(&mut mdhd, 0x55c4); // language 'und'
    put_u16be(&mut mdhd, 0); // pre_defined
    mp4_box(&mut out, b"mdhd", &mp4_fullbox_body(0, 0, &mdhd));
    let mut hdlr = Vec::with_capacity(25);
    put_u32be(&mut hdlr, 0); // pre_defined
    hdlr.extend_from_slice(handler);
    hdlr.extend_from_slice(&[0u8; 12]); // reserved
    hdlr.extend_from_slice(handler_name.as_bytes());
    hdlr.push(0); // null-terminated
    mp4_box(&mut out, b"hdlr", &mp4_fullbox_body(0, 0, &hdlr));
    // minf
    let minf_pos = mp4_begin_box(&mut out, b"minf");
    if is_video {
        let mut vmhd = Vec::with_capacity(6);
        put_u16be(&mut vmhd, 0); // graphicsmode: copy
        put_u16be(&mut vmhd, 0); // opcolor r
        put_u16be(&mut vmhd, 0); // opcolor g
        put_u16be(&mut vmhd, 0); // opcolor b
        mp4_box(&mut out, b"vmhd", &mp4_fullbox_body(0, 1, &vmhd));
    } else {
        let mut smhd = Vec::with_capacity(4);
        put_u16be(&mut smhd, 0); // balance
        put_u16be(&mut smhd, 0); // reserved
        mp4_box(&mut out, b"smhd", &mp4_fullbox_body(0, 0, &smhd));
    }
    mp4_dinf(&mut out);
    let sample_count = sample_sizes.len();
    let (stsz_count_offset, stco_entries_offset) = mp4_stbl(
        &mut out,
        sample_entry,
        sample_count,
        sample_sizes,
        stts_delta,
        sync_all,
    )?;
    mp4_end_box(&mut out, minf_pos);
    mp4_end_box(&mut out, mdia_pos);
    mp4_end_box(&mut out, trak_pos);
    Ok(TrakBuilt {
        bytes: out,
        stsz_count_offset,
        stco_entries_offset,
    })
}

/// Build the MP4+MJPEG+PCM parent (doc 03 §7, decision-independent
/// variant B): `ftyp(isom)` + `moov` (movie timescale 1000; video trak
/// `jpeg` 30 Hz, audio trak `sowt` 44100 Hz) + `mdat` (one video sample
/// and one audio sample per frame tick, interleaved). `stco` offsets are
/// zero-patched in a size-neutral second pass — no float, no wall clock.
fn build_mp4(frames: &[Vec<u8>], pcm: &[u8]) -> Result<Mp4Built, String> {
    if frames.len() != FRAME_COUNT {
        return Err(format!(
            "mp4 writer needs exactly {FRAME_COUNT} frames, got {}",
            frames.len()
        ));
    }
    let pcm_need = TOTAL_SAMPLES * AUDIO_CHANNELS * 2;
    if pcm.len() != pcm_need {
        return Err(format!(
            "mp4 writer needs {pcm_need} PCM bytes, got {}",
            pcm.len()
        ));
    }
    let tick_bytes = SAMPLES_PER_TICK * AUDIO_CHANNELS * 2;
    let movie_timescale: u32 = 1000;
    let movie_duration: u32 =
        ((FRAME_COUNT as u64 * movie_timescale as u64) / u64::from(FPS)) as u32; // 2000 ms

    // ftyp: isom / 0 / isom+iso2+mp41.
    let mut ftyp = Vec::with_capacity(20);
    ftyp.extend_from_slice(b"isom");
    put_u32be(&mut ftyp, 0); // minor_version
    ftyp.extend_from_slice(b"isom");
    ftyp.extend_from_slice(b"iso2");
    ftyp.extend_from_slice(b"mp41");

    // Video track: one sample per frame, timescale = fps.
    let video_sizes: Vec<u32> = frames.iter().map(|f| f.len() as u32).collect();
    let video_trak = mp4_trak(
        1,
        movie_duration,
        FPS,
        FRAME_COUNT as u32,
        b"vide",
        "VideoHandler",
        true,
        Some(((FRAME_WIDTH as u32) << 16, (FRAME_HEIGHT as u32) << 16)),
        &mp4_jpeg_sample_entry(),
        &video_sizes,
        1,
        true,
    )?;
    // Audio track: one sample per tick (SAMPLES_PER_TICK sample-frames of
    // s16le stereo), timescale = sample rate — the standard PCM-in-MP4
    // convention (stts delta counts sample-frames).
    let audio_sizes: Vec<u32> = vec![tick_bytes as u32; FRAME_COUNT];
    let audio_trak = mp4_trak(
        2,
        movie_duration,
        AUDIO_RATE,
        TOTAL_SAMPLES as u32,
        b"soun",
        "SoundHandler",
        false,
        None,
        &mp4_sowt_sample_entry(),
        &audio_sizes,
        SAMPLES_PER_TICK as u32,
        false,
    )?;

    // Assembly: ftyp + moov + mdat.
    let mut out: Vec<u8> = Vec::with_capacity(
        8 + ftyp.len()
            + 8
            + video_trak.bytes.len()
            + audio_trak.bytes.len()
            + 8
            + frames.iter().map(Vec::len).sum::<usize>()
            + TOTAL_SAMPLES * 4,
    );
    mp4_box(&mut out, b"ftyp", &ftyp);
    let moov_pos = mp4_begin_box(&mut out, b"moov");
    // mvhd
    let mut mvhd = Vec::with_capacity(96);
    put_u32be(&mut mvhd, 0); // creation_time
    put_u32be(&mut mvhd, 0); // modification_time
    put_u32be(&mut mvhd, movie_timescale);
    put_u32be(&mut mvhd, movie_duration);
    put_u32be(&mut mvhd, 0x0001_0000); // rate 1.0
    put_u16be(&mut mvhd, 0x0100); // volume 1.0
    mvhd.extend_from_slice(&[0u8; 10]); // reserved
    put_u32be(&mut mvhd, 0x0001_0000); // matrix[0][0]
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0x0001_0000); // matrix[1][1]
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0);
    put_u32be(&mut mvhd, 0x4000_0000); // matrix[2][2]
    mvhd.extend_from_slice(&[0u8; 24]); // pre_defined[6]
    put_u32be(&mut mvhd, 3); // next_track_ID
    let mvhd_offset = out.len();
    mp4_box(&mut out, b"mvhd", &mp4_fullbox_body(0, 0, &mvhd));
    let trak_offsets = [out.len(), out.len() + video_trak.bytes.len()];
    let video_sites = (
        trak_offsets[0] + video_trak.stsz_count_offset,
        trak_offsets[0] + video_trak.stco_entries_offset,
    );
    let audio_sites = (
        trak_offsets[1] + audio_trak.stsz_count_offset,
        trak_offsets[1] + audio_trak.stco_entries_offset,
    );
    out.extend_from_slice(&video_trak.bytes);
    out.extend_from_slice(&audio_trak.bytes);
    mp4_end_box(&mut out, moov_pos);

    // mdat: one video sample + one audio sample per tick, interleaved.
    let mdat_offset = out.len();
    let mdat_pos = mp4_begin_box(&mut out, b"mdat");
    let mdat_data_offset = out.len();
    let mut video_offsets: Vec<u32> = Vec::with_capacity(FRAME_COUNT);
    let mut audio_offsets: Vec<u32> = Vec::with_capacity(FRAME_COUNT);
    for (index, frame) in frames.iter().enumerate() {
        let offset =
            u32::try_from(out.len()).map_err(|_| "mp4 sample offset overflows u32".to_string())?;
        video_offsets.push(offset);
        out.extend_from_slice(frame);
        let offset =
            u32::try_from(out.len()).map_err(|_| "mp4 sample offset overflows u32".to_string())?;
        audio_offsets.push(offset);
        let from = index * tick_bytes;
        out.extend_from_slice(&pcm[from..from + tick_bytes]);
    }
    mp4_end_box(&mut out, mdat_pos);

    // Size-neutral stco patch pass.
    for (index, &offset) in video_offsets.iter().enumerate() {
        let at = video_sites.1 + 4 * index;
        out[at..at + 4].copy_from_slice(&offset.to_be_bytes());
    }
    for (index, &offset) in audio_offsets.iter().enumerate() {
        let at = audio_sites.1 + 4 * index;
        out[at..at + 4].copy_from_slice(&offset.to_be_bytes());
    }

    Ok(Mp4Built {
        bytes: out,
        layout: Mp4Layout {
            trak_offsets,
            mvhd_offset,
            mdat_offset,
            mdat_data_offset,
            stsz_video_count_offset: video_sites.0,
            video_offsets,
            audio_offsets,
            video_sizes,
        },
    })
}

/// Serialize the MP4 structure map (compact single-line JSON).
fn mp4_structure_json(mp4: &Mp4Built) -> Vec<u8> {
    let layout = &mp4.layout;
    let len = mp4.bytes.len();
    let mut s = String::with_capacity(4096 + 40 * 2 * FRAME_COUNT);
    s.push_str("{\"schema\":\"structure-map/1\",\"fixture\":");
    json_put_string(&mut s, "generated/valid/mp4_mjpeg_pcm.mp4");
    s.push_str(",\"size\":");
    json_put_uint(&mut s, len as u64);
    s.push_str(",\"note\":\"offsets of top-level boxes, headers and samples; class-B derivatives derive their cut/patch positions from here (corpus-plan s1 B1/B3)\"");
    s.push_str(",\"top_level\":[");
    s.push_str("{\"box\":\"ftyp\",\"offset\":0,\"size\":");
    json_put_uint(&mut s, (layout.mvhd_offset - 8) as u64);
    s.push_str("},{\"box\":\"moov\",\"offset\":");
    json_put_uint(&mut s, (layout.mvhd_offset - 8) as u64);
    s.push_str(",\"size\":");
    json_put_uint(&mut s, (layout.mdat_offset - layout.mvhd_offset + 8) as u64);
    s.push_str("},{\"box\":\"mdat\",\"offset\":");
    json_put_uint(&mut s, layout.mdat_offset as u64);
    s.push_str(",\"size\":");
    json_put_uint(&mut s, (len - layout.mdat_offset) as u64);
    s.push_str("}]");
    s.push_str(",\"mp4\":{\"mvhd_offset\":");
    json_put_uint(&mut s, layout.mvhd_offset as u64);
    s.push_str(",\"trak_offsets\":[");
    json_put_uint(&mut s, layout.trak_offsets[0] as u64);
    s.push(',');
    json_put_uint(&mut s, layout.trak_offsets[1] as u64);
    s.push_str("],\"mdat_offset\":");
    json_put_uint(&mut s, layout.mdat_offset as u64);
    s.push_str(",\"mdat_data_offset\":");
    json_put_uint(&mut s, layout.mdat_data_offset as u64);
    s.push_str(",\"stsz_video_count_offset\":");
    json_put_uint(&mut s, layout.stsz_video_count_offset as u64);
    s.push_str(",\"video_samples\":[");
    for (index, (&offset, &size)) in layout
        .video_offsets
        .iter()
        .zip(layout.video_sizes.iter())
        .enumerate()
    {
        if index > 0 {
            s.push(',');
        }
        s.push_str("{\"offset\":");
        json_put_uint(&mut s, u64::from(offset));
        s.push_str(",\"size\":");
        json_put_uint(&mut s, u64::from(size));
        s.push('}');
    }
    s.push_str("],\"audio_samples\":[");
    for (index, &offset) in layout.audio_offsets.iter().enumerate() {
        if index > 0 {
            s.push(',');
        }
        s.push_str("{\"offset\":");
        json_put_uint(&mut s, u64::from(offset));
        s.push_str(",\"size\":");
        json_put_uint(&mut s, (SAMPLES_PER_TICK * AUDIO_CHANNELS * 2) as u64);
        s.push('}');
    }
    s.push_str("]}}");
    s.into_bytes()
}

// ---------------------------------------------------------------------------
// Class-B derivatives (corpus-plan §1 B1/B2/B3 + doc 03 §7's malformed
// variants). Every derivative is a pure function of its parent's bytes and
// the recorded structure map: the same parent always yields the same
// derivative, byte for byte.
// ---------------------------------------------------------------------------

/// Seeded bit-flip header extent for the RIFF family (corpus-plan §1 B2
/// names 64 for RIFF: size+form magic plus the `LIST hdrl` start).
const AVI_FLIP_EXTENT: usize = 64;

/// Seeded bit-flip header extent for MP4 [PROPOSAL: 512 — `ftyp` plus the
/// `moov` header region, mirroring corpus-plan's MKV "EBML head + segment
/// info" precedent of 512].
const MP4_FLIP_EXTENT: usize = 512;

/// Appended garbage length [PROPOSAL: one 4 KiB page — long enough to run
/// past every declared structure end without being interesting in itself].
const GARBAGE_LEN: usize = 4096;

/// The oversize sample-count lie (corpus-plan §1 B3 prescribes 2³¹−1 for
/// absurd count/size claims).
const OVERSIZE_COUNT: u32 = 0x7fff_ffff;

/// Per-derivative PRNG, seeded from the parent's sha256 (corpus-plan §1
/// B2: `sha256(parent)[:8]`). Every derivative builds a fresh instance, so
/// each file is independently reproducible from its parent alone.
fn parent_prng(parent: &[u8]) -> Xorshift64Star {
    let mut hasher = Sha256::new();
    hasher.update(parent);
    let digest = hasher.finish();
    Xorshift64Star::from_digest(&digest)
}

/// Seeded garbage block (corpus-plan §1 B2 — deterministic bytes).
fn garbage_block(rng: &mut Xorshift64Star, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push((rng.next_u64() & 0xff) as u8);
    }
    out
}

/// Flip exactly `k` distinct bits within the first `extent` bytes. Bit
/// positions are a Fisher–Yates prefix of `[0, extent·8)` drawn from a
/// fresh parent-seeded PRNG, so the flip-k positions are a k-prefix:
/// flip1's bit set is a subset of flip4's, which is a subset of flip16's
/// (a documented property — probe/demuxer tests can rely on it).
fn flip_header_bits(
    bytes: &mut [u8],
    rng: &mut Xorshift64Star,
    extent: usize,
    k: usize,
) -> Result<(), String> {
    if extent > bytes.len() {
        return Err(format!(
            "flip extent {extent} exceeds the file size {}",
            bytes.len()
        ));
    }
    let total = extent * 8;
    if k == 0 || k > total {
        return Err(format!("bit-flip count {k} outside 1..={total}"));
    }
    let mut indices: Vec<u32> = (0..total as u32).collect();
    for i in (1..total).rev() {
        let j = rng.below((i + 1) as u64) as usize;
        indices.swap(i, j);
    }
    for &bit in indices.iter().take(k) {
        let index = bit as usize;
        bytes[index / 8] ^= 1u8 << (index % 8);
    }
    Ok(())
}

/// One class-B derivative plus the `[derivation]` TOML lines its
/// expectation file records — the parameters that make the mutation
/// inspectable without reading this source.
struct Derivative {
    file: GeneratedFile,
    derivation: Vec<String>,
}

/// Class-B derivative path: `generated/valid/<stem>.<ext>` becomes
/// `generated/malformed/<stem>__<op>.<ext>` (corpus-plan §1 naming).
fn derivative_path(parent_path: &str, op: &str) -> String {
    let name = parent_path.trim_start_matches("generated/valid/");
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) => (stem, ext),
        None => (name, "bin"),
    };
    format!("generated/malformed/{stem}__{op}.{ext}")
}

/// B1 truncation: the parent's first `cut` bytes.
fn truncate_derivative(
    parent: &GeneratedFile,
    op: &'static str,
    cut: usize,
    note: &str,
) -> Result<Derivative, String> {
    if cut == 0 || cut >= parent.bytes.len() {
        return Err(format!(
            "{op}: cut offset {cut} outside 1..{}",
            parent.bytes.len()
        ));
    }
    Ok(Derivative {
        file: GeneratedFile {
            path: derivative_path(&parent.path, op),
            bytes: parent.bytes[..cut].to_vec(),
            class: "B",
            op,
            parent: Some(parent.path.clone()),
        },
        derivation: vec![
            "op = \"truncate\"".to_string(),
            format!("cut_offset = {}", toml_uint(cut as u64)),
            format!("note = {}", toml_string(note)),
        ],
    })
}

/// B2 seeded header bit-flips (k bits within the family's header extent).
fn flip_derivative(
    parent: &GeneratedFile,
    op: &'static str,
    k: usize,
    extent: usize,
) -> Result<Derivative, String> {
    let mut bytes = parent.bytes.clone();
    let mut rng = parent_prng(&parent.bytes);
    flip_header_bits(&mut bytes, &mut rng, extent, k)?;
    Ok(Derivative {
        file: GeneratedFile {
            path: derivative_path(&parent.path, op),
            bytes,
            class: "B",
            op,
            parent: Some(parent.path.clone()),
        },
        derivation: vec![
            "op = \"bit-flip\"".to_string(),
            format!("flip_bits = {k}"),
            format!("flip_extent = {extent}"),
            "note = \"seeded distinct bit positions within the header extent; flip-k positions are a k-prefix of one shuffle (corpus-plan s1 B2)\""
                .to_string(),
        ],
    })
}

/// Garbage append: the parent plus a seeded garbage block.
fn garbage_derivative(parent: &GeneratedFile) -> Result<Derivative, String> {
    let mut rng = parent_prng(&parent.bytes);
    let mut bytes = parent.bytes.clone();
    bytes.extend(garbage_block(&mut rng, GARBAGE_LEN));
    Ok(Derivative {
        file: GeneratedFile {
            path: derivative_path(&parent.path, "garbage0"),
            bytes,
            class: "B",
            op: "garbage0",
            parent: Some(parent.path.clone()),
        },
        derivation: vec![
            "op = \"garbage-append\"".to_string(),
            format!("appended_bytes = {}", toml_uint(GARBAGE_LEN as u64)),
            "note = \"seeded garbage block appended past every declared structure end\""
                .to_string(),
        ],
    })
}

/// Spec violation (doc 03 §7 malformed variant): the AVI with its `idx1`
/// dropped and the RIFF size patched to cover the surviving bytes — while
/// `avih` still declares `AVIF_HASINDEX`.
fn avi_no_idx1_derivative(
    parent: &GeneratedFile,
    layout: &AviLayout,
) -> Result<Derivative, String> {
    if layout.idx1_offset >= parent.bytes.len() || layout.idx1_offset < 8 {
        return Err(format!(
            "idx1 offset {} outside 8..{}",
            layout.idx1_offset,
            parent.bytes.len()
        ));
    }
    let mut bytes = parent.bytes[..layout.idx1_offset].to_vec();
    let size = layout.idx1_offset - 8;
    bytes[layout.riff_size_pos..layout.riff_size_pos + 4]
        .copy_from_slice(&(size as u32).to_le_bytes());
    Ok(Derivative {
        file: GeneratedFile {
            path: derivative_path(&parent.path, "noidx0"),
            bytes,
            class: "B",
            op: "noidx0",
            parent: Some(parent.path.clone()),
        },
        derivation: vec![
            "op = \"drop-idx1\"".to_string(),
            format!("cut_offset = {}", toml_uint(layout.idx1_offset as u64)),
            format!(
                "riff_size_field_offset = {}",
                toml_uint(layout.riff_size_pos as u64)
            ),
            "note = \"idx1 dropped, RIFF size patched; avih still claims AVIF_HASINDEX (doc 03 s7)\""
                .to_string(),
        ],
    })
}

/// Spec violation (corpus-plan §1 B3 oversize-field): the MP4 with the
/// video `stsz` `sample_count` patched to 2³¹−1.
fn mp4_stsz_lie_derivative(
    parent: &GeneratedFile,
    layout: &Mp4Layout,
) -> Result<Derivative, String> {
    let at = layout.stsz_video_count_offset;
    if at + 4 > parent.bytes.len() {
        return Err(format!(
            "stsz count offset {at} + 4 exceeds the file size {}",
            parent.bytes.len()
        ));
    }
    let mut bytes = parent.bytes.clone();
    bytes[at..at + 4].copy_from_slice(&OVERSIZE_COUNT.to_be_bytes());
    Ok(Derivative {
        file: GeneratedFile {
            path: derivative_path(&parent.path, "stszlie0"),
            bytes,
            class: "B",
            op: "stszlie0",
            parent: Some(parent.path.clone()),
        },
        derivation: vec![
            "op = \"stsz-count-lie\"".to_string(),
            format!("patched_offset = {}", toml_uint(at as u64)),
            format!("patched_value = {}", toml_uint(u64::from(OVERSIZE_COUNT))),
            "note = \"video stsz sample_count patched to 2^31-1 (corpus-plan s1 B3 oversize lie)\""
                .to_string(),
        ],
    })
}

/// The AVI class-B set: five structure-boundary truncations (B1), three
/// seeded header bit-flips (B2), one garbage append, and the missing-`idx1`
/// spec violation — ten files, every offset taken from `layout`.
fn avi_derivatives(parent: &GeneratedFile, layout: &AviLayout) -> Result<Vec<Derivative>, String> {
    let mid_avih = layout.avih_offset + 8 + 20;
    let &(fourcc, chunk_offset, chunk_size) = layout
        .chunks
        .get(FRAME_COUNT)
        .ok_or_else(|| "avi chunk table too short for the mid-chunk cut".to_string())?;
    if &fourcc != b"00dc" {
        return Err(format!(
            "movi chunk {FRAME_COUNT} is {}, expected 00dc",
            fourcc_str(&fourcc)
        ));
    }
    let mid_chunk = chunk_offset + 8 + chunk_size / 2;
    Ok(vec![
        truncate_derivative(parent, "trunc0", 12, "cut after the RIFF size+form magic")?,
        truncate_derivative(parent, "trunc1", mid_avih, "cut mid-avih (main AVI header)")?,
        truncate_derivative(
            parent,
            "trunc2",
            layout.movi_data_offset,
            "cut at the first movi chunk header (structure-map movi_data_offset)",
        )?,
        truncate_derivative(
            parent,
            "trunc3",
            mid_chunk,
            "cut mid-payload of the frame-30 00dc chunk",
        )?,
        truncate_derivative(
            parent,
            "trunc4",
            parent.bytes.len() - 1,
            "cut the final byte",
        )?,
        garbage_derivative(parent)?,
        flip_derivative(parent, "flip1", 1, AVI_FLIP_EXTENT)?,
        flip_derivative(parent, "flip4", 4, AVI_FLIP_EXTENT)?,
        flip_derivative(parent, "flip16", 16, AVI_FLIP_EXTENT)?,
        avi_no_idx1_derivative(parent, layout)?,
    ])
}

/// The MP4 class-B set: the same ten-derivative shape, with boundaries
/// taken from the MP4 structure map and the `stsz` count lie as the spec
/// violation.
fn mp4_derivatives(parent: &GeneratedFile, layout: &Mp4Layout) -> Result<Vec<Derivative>, String> {
    let moov_offset = layout.mvhd_offset - 8; // ftyp ends where moov begins
    if moov_offset < 8 || moov_offset >= parent.bytes.len() {
        return Err(format!(
            "moov offset {moov_offset} outside 8..{}",
            parent.bytes.len()
        ));
    }
    let mid_mvhd = layout.mvhd_offset + 8 + 20;
    let sample_offset = layout
        .video_offsets
        .get(FRAME_COUNT / 2)
        .copied()
        .ok_or_else(|| "mp4 sample table too short for the mid-sample cut".to_string())?;
    let sample_size = layout
        .video_sizes
        .get(FRAME_COUNT / 2)
        .copied()
        .ok_or_else(|| "mp4 size table too short for the mid-sample cut".to_string())?;
    let mid_sample = sample_offset as usize + sample_size as usize / 2;
    Ok(vec![
        truncate_derivative(
            parent,
            "trunc0",
            moov_offset,
            "cut after ftyp (the ISO brand box)",
        )?,
        truncate_derivative(parent, "trunc1", mid_mvhd, "cut mid-mvhd (movie header)")?,
        truncate_derivative(
            parent,
            "trunc2",
            layout.mdat_data_offset,
            "cut at the first mdat sample (mdat header present, no samples)",
        )?,
        truncate_derivative(
            parent,
            "trunc3",
            mid_sample,
            "cut mid-payload of video sample 30",
        )?,
        truncate_derivative(
            parent,
            "trunc4",
            parent.bytes.len() - 1,
            "cut the final byte",
        )?,
        garbage_derivative(parent)?,
        flip_derivative(parent, "flip1", 1, MP4_FLIP_EXTENT)?,
        flip_derivative(parent, "flip4", 4, MP4_FLIP_EXTENT)?,
        flip_derivative(parent, "flip16", 16, MP4_FLIP_EXTENT)?,
        mp4_stsz_lie_derivative(parent, layout)?,
    ])
}

// ---------------------------------------------------------------------------
// Ledger files: expectation/1 TOMLs (doc 10 §5.4), MANIFEST.toml and
// CORPUS.md (corpus-plan §4–§5). All of them are `GeneratedFile`s in the
// tree, so `--check` byte-compares them like everything else — the hashes
// they record are re-derived from the regenerated bytes on every run.
// ---------------------------------------------------------------------------

/// TOML basic-string emitter (the corpus strings are authored ASCII; the
/// escapes are defensive).
fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Integer with 3-digit underscore grouping (doc 10 §5.4's style:
/// `size_bytes = 58_112_004`).
fn toml_uint(value: u64) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    let len = raw.len();
    for (i, ch) in raw.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

/// Expectation-file path: `generated/<class>/<name>` →
/// `expectations/<class>/<name>.toml`. Structure maps carry no
/// expectation (companion metadata, manifest rows only).
fn expectation_path(fixture_path: &str) -> String {
    let rest = fixture_path.trim_start_matches("generated/");
    format!("expectations/{rest}.toml")
}

/// The common header of every expectation file.
fn expectation_header(file: &GeneratedFile, class: &str, parent: Option<&str>) -> String {
    let mut s = String::with_capacity(256);
    s.push_str("# Generated by `cargo xtask fixtures` (card 2.A3) — DO NOT EDIT.\n");
    s.push_str("# expectation/1 (doc 10 §5.4). Class-A parents carry [first_frame]\n");
    s.push_str("# (recorded from Lumen's first compliant decode, card 2.B4) and\n");
    s.push_str("# [audio_probe] (first PCM decode, card 2.B5).\n");
    s.push_str("schema = \"expectation/1\"\n");
    s.push_str(&format!("fixture = {}\n", toml_string(&file.path)));
    s.push_str(&format!("class = \"{class}\"\n"));
    if let Some(parent) = parent {
        s.push_str(&format!("parent = {}\n", toml_string(parent)));
    }
    s.push_str("generator = \"cargo xtask fixtures\"\n");
    s.push_str(&format!("seed_root = {}\n", toml_string(SEED_ROOT)));
    s.push_str(&format!(
        "sha256 = {}\n",
        toml_string(&sha256_hex(&file.bytes))
    ));
    s.push_str(&format!(
        "size_bytes = {}\n",
        toml_uint(file.bytes.len() as u64)
    ));
    s.push_str("license = \"CC0\"\n");
    s
}

/// Duration of the corpus media, in microseconds (60 frames at 30 fps).
fn duration_us() -> u64 {
    (FRAME_COUNT as u64 * 1_000_000) / u64::from(FPS)
}

/// One frame tick in microseconds — the seek tolerance (±1 frame).
fn frame_period_us() -> u64 {
    (1_000_000 + u64::from(FPS) / 2) / u64::from(FPS)
}

/// The class-A parent expectation: full track plan, duration, seek plan
/// and generator-truth essence digests (per-sample video hashes + the
/// audio stream hash — the frame-index-in-content rule's ground truth).
fn parent_expectation(
    file: &GeneratedFile,
    video_id: u32,
    audio_id: u32,
    frames: &[Vec<u8>],
    pcm: &[u8],
) -> Vec<u8> {
    let mut s = String::with_capacity(2048 + 72 * frames.len());
    s.push_str(&expectation_header(file, "A", None));
    s.push_str("\n[tracks]\ncount = 2\n");
    s.push_str(&format!(
        "video = [{{ id = {video_id}, codec = \"mjpeg\", width = {FRAME_WIDTH}, \
         height = {FRAME_HEIGHT}, fps = {{ num = {FPS}, den = 1 }} }}]\n"
    ));
    s.push_str(&format!(
        "audio = [{{ id = {audio_id}, codec = \"pcm_s16le\", rate = {AUDIO_RATE}, \
         channels = {AUDIO_CHANNELS} }}]\n\n"
    ));
    s.push_str(&format!("duration_us = {}\n\n", toml_uint(duration_us())));
    s.push_str("[seek]\n");
    s.push_str(&format!(
        "targets_us = [0, 500_000, 1_000_000, {}]\n",
        toml_uint(duration_us() - 1_000)
    ));
    s.push_str(&format!(
        "tolerance_us = {}\n\n",
        toml_uint(frame_period_us())
    ));
    s.push_str("[first_frame]\n");
    s.push_str("# Recorded from Lumen's first compliant decode (card 2.B4; see the\n");
    s.push_str("# FIRST_FRAME_HASH_Y provenance note in this file). hash_y = SHA-256\n");
    s.push_str("# over the luma 8x8-block mean descriptor (doc 10 sec. 5.4, doc 03\n");
    s.push_str("# sec. 8 FrameRecorder shape), verified by the codecs crate's AC suite.\n");
    s.push_str(&format!("hash_y = {}\n", toml_string(FIRST_FRAME_HASH_Y)));
    s.push_str(&format!(
        "stamp_us = {}\n\n",
        toml_uint(FIRST_FRAME_STAMP_US)
    ));
    s.push_str("[audio_probe]\n");
    s.push_str("# First PCM decode (card 2.B5). first_samples_md5 = MD5 over the\n");
    s.push_str("# pinned f32 conversion of the first 4800 sample-instants (all\n");
    s.push_str("# channels, interleave order): each s16 scalar v -> v/32768.0 as\n");
    s.push_str("# IEEE-754 f32, little-endian bytes; see AUDIO_PROBE_INSTANTS and\n");
    s.push_str("# audio_probe_md5 in this writer for the provenance — derived from\n");
    s.push_str("# the generator's own samples and verified decoder-side by the\n");
    s.push_str("# codecs crate's AC suite (decode -> probe -> compare).\n");
    s.push_str(&format!(
        "first_samples_md5 = {}\n\n",
        toml_string(&audio_probe_md5(pcm))
    ));
    s.push_str("[essence_v0]\n");
    s.push_str("# Supplemental generator-truth digests (beyond doc 10 §5.4's base).\n");
    s.push_str(&format!("video_sample_count = {}\n", frames.len()));
    s.push_str("video_sample_sha256 = [\n");
    for frame in frames {
        s.push_str(&format!("  {},\n", toml_string(&sha256_hex(frame))));
    }
    s.push_str("]\n");
    s.push_str(&format!(
        "audio_stream_sha256 = {}\n",
        toml_string(&sha256_hex(pcm))
    ));
    s.push_str(&format!(
        "audio_stream_bytes = {}\n",
        toml_uint(pcm.len() as u64)
    ));
    s.into_bytes()
}

/// The class-B derivative expectation: derivation parameters plus the
/// error contract (corpus-plan §4's `[error]` extension).
fn derivative_expectation(derivative: &Derivative) -> Vec<u8> {
    let mut s = String::with_capacity(512);
    s.push_str(&expectation_header(
        &derivative.file,
        "B",
        derivative.file.parent.as_deref(),
    ));
    s.push_str("\n[derivation]\n");
    for line in &derivative.derivation {
        s.push_str(line);
        s.push('\n');
    }
    s.push_str("\n[error]\n");
    s.push_str("# v0: the union contract — the specific flavor (typed-error |\n");
    s.push_str("# bounded-recovery | rejected) is pinned by demuxer conformance\n");
    s.push_str("# (cards 2.B2/2.B3).\n");
    s.push_str("expected = \"typed-error-or-bounded-recovery\"\n");
    s.push_str("never = \"panic, hang, unbounded allocation\"\n");
    s.push_str("rss_cap_mb = 64\n");
    s.push_str("wall_cap_ms = 5_000\n");
    s.into_bytes()
}

/// `generated/MANIFEST.toml` (corpus-plan §1: generator + seed + hash per
/// fixture). Rows cover the class-A/B fixture files; the ledger files are
/// drift-gated by `--check` instead of self-referential rows.
fn manifest_toml(files: &[GeneratedFile]) -> Vec<u8> {
    let fixtures: Vec<&GeneratedFile> = files
        .iter()
        .filter(|f| f.class == "A" || f.class == "B")
        .collect();
    let mut s = String::with_capacity(256 + 192 * fixtures.len());
    s.push_str("# Generated by `cargo xtask fixtures` (card 2.A3) — DO NOT EDIT.\n");
    s.push_str("# Regenerate: cargo xtask fixtures. Drift gate: cargo xtask fixtures --check.\n");
    s.push_str("schema = \"manifest/1\"\n");
    s.push_str("generator = \"cargo xtask fixtures (tools/xtask/src/cmd/fixtures.rs)\"\n");
    s.push_str(&format!("seed_root = {}\n", toml_string(SEED_ROOT)));
    s.push_str(&format!("file_count = {}\n", fixtures.len()));
    for f in &fixtures {
        s.push_str("\n[[files]]\n");
        s.push_str(&format!("path = {}\n", toml_string(&f.path)));
        s.push_str(&format!("class = \"{}\"\n", f.class));
        s.push_str(&format!("op = \"{}\"\n", f.op));
        if let Some(parent) = &f.parent {
            s.push_str(&format!("parent = {}\n", toml_string(parent)));
        }
        s.push_str(&format!(
            "sha256 = {}\n",
            toml_string(&sha256_hex(&f.bytes))
        ));
        s.push_str(&format!(
            "size_bytes = {}\n",
            toml_uint(f.bytes.len() as u64)
        ));
    }
    s.into_bytes()
}

/// The one-line purpose recorded for a fixture in CORPUS.md.
fn corpus_purpose(file: &GeneratedFile) -> String {
    let is_avi = file.path.contains("avi_mjpeg_pcm");
    match file.op {
        "parent" if is_avi => "decision-independent first-vertical parent: AVI 1.0, MJPEG video \
             + PCM s16le audio (doc 03 §7 candidate 2)"
            .to_string(),
        "parent" => "decision-independent first-vertical parent: ISO-BMFF \
             progressive, MJPEG video + PCM s16le audio (doc 04 §8.1 \
             candidate B)"
            .to_string(),
        "structure-map" => "structure map: the recorded chunk/sample offsets the class-B \
             cuts and patches derive from (corpus-plan §2)"
            .to_string(),
        "trunc0" | "trunc1" | "trunc2" | "trunc3" | "trunc4" => {
            "class-B truncation at a structure boundary (corpus-plan §1 B1); \
             cut offset recorded in the expectation file"
                .to_string()
        }
        op if op.starts_with("flip") => {
            let k = op.trim_start_matches("flip");
            format!(
                "class-B seeded header bit-flips, k = {k} within the family \
                 header extent (corpus-plan §1 B2)"
            )
        }
        "garbage0" => "class-B garbage append past the declared structure \
             (EOF-boundary / probe-race case)"
            .to_string(),
        "noidx0" => "spec violation: idx1 dropped, RIFF size patched, avih still \
             claims AVIF_HASINDEX (doc 03 §7 malformed variant)"
            .to_string(),
        "stszlie0" => "spec violation: video stsz sample_count patched to 2^31-1 \
             (corpus-plan §1 B3 oversize lie)"
            .to_string(),
        other => format!("generated ledger companion ({other})"),
    }
}

/// `tests/fixtures/CORPUS.md` — the licensing ledger (corpus-plan §5; the
/// card requires CORPUS.md entries for the generated corpus). Rows cover
/// the class-A/B fixtures; ledger files are byte-compared by `--check`
/// rather than carrying self-referential hashes.
fn corpus_markdown(files: &[GeneratedFile]) -> Vec<u8> {
    let fixtures: Vec<&GeneratedFile> = files
        .iter()
        .filter(|f| f.class == "A" || f.class == "B")
        .collect();
    let total: usize = fixtures.iter().map(|f| f.bytes.len()).sum();
    let mut s = String::with_capacity(4096 + 200 * fixtures.len());
    s.push_str("# Test-corpus licensing ledger (tests/fixtures)\n\n");
    s.push_str("Generated by `cargo xtask fixtures` (card 2.A3; doc 03 §7/§8,\n");
    s.push_str("docs/inventory/corpus-plan.md §4–§5). Do not edit by hand —\n");
    s.push_str("regenerate with `cargo xtask fixtures` and commit; drift (including\n");
    s.push_str("this file) is detected by `cargo xtask fixtures --check`.\n\n");
    s.push_str("## License — CC0\n\n");
    s.push_str("Every file under `generated/` and `expectations/` is authored in-tree\n");
    s.push_str("by the deterministic generator in `tools/xtask/src/cmd/fixtures.rs`,\n");
    s.push_str("from public specifications only (ISO/IEC 10918-1 baseline JPEG, the\n");
    s.push_str("RIFF/AVI 1.0 file-format reference, ISO/IEC 14496-12/-14). No\n");
    s.push_str("third-party media, no VLC-derived bytes (clean-room, AGENTS.md §3.1),\n");
    s.push_str("no external encoder output — ffmpeg was used only as a local\n");
    s.push_str("cross-check oracle during development, never in the build.\n\n");
    s.push_str("To the extent possible under law, the Lumen project contributors have\n");
    s.push_str("dedicated all copyright and related and neighboring rights in the\n");
    s.push_str("files listed below to the public domain worldwide, under the\n");
    s.push_str("CC0 1.0 Universal Public Domain Dedication\n");
    s.push_str("(https://creativecommons.org/publicdomain/zero/1.0/).\n\n");
    s.push_str("## Layout\n\n");
    s.push_str("- `generated/valid/` — class A: the two decision-independent parents\n");
    s.push_str("  and their structure maps (`*.structure.json`, corpus-plan §2 — the\n");
    s.push_str("  recorded offsets the class-B cuts and patches derive from).\n");
    s.push_str("- `generated/malformed/` — class B: ten derivatives per parent\n");
    s.push_str("  (`<parent>__<op><n>.<ext>`), every mutation derived from the\n");
    s.push_str("  parent's structure map and a PRNG seeded by the parent's sha256\n");
    s.push_str("  (corpus-plan §1 B1/B2/B3; doc 03 §7's malformed variants).\n");
    s.push_str("- `generated/MANIFEST.toml` — machine-readable form of the table\n");
    s.push_str("  below (generator, seed root, per-file sha256).\n");
    s.push_str("- `expectations/valid/`, `expectations/malformed/` — one\n");
    s.push_str("  `expectation/1` TOML (doc 10 §5.4) per media fixture: parents\n");
    s.push_str("  carry the track plan, duration, seek plan and essence digests;\n");
    s.push_str("  derivatives carry their derivation and the class-B error\n");
    s.push_str("  contract. Structure maps carry no expectation (manifest rows\n");
    s.push_str("  only).\n");
    s.push_str("- Curated third-party files (class D): none in v0 — any future row\n");
    s.push_str("  needs source URL + license + hash per corpus-plan §5.\n\n");
    s.push_str("## Fixture rows\n\n");
    s.push_str("| path | class | op | bytes | sha256 | purpose |\n");
    s.push_str("|---|---|---|---|---|---|\n");
    for f in &fixtures {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            f.path,
            f.class,
            f.op,
            f.bytes.len(),
            sha256_hex(&f.bytes),
            corpus_purpose(f)
        ));
    }
    s.push_str(&format!(
        "\nTotal: {} fixture file(s), {} bytes. Ledger files (this file,\n",
        fixtures.len(),
        total
    ));
    s.push_str("MANIFEST.toml, expectations/) are verified by `--check` byte-comparison.\n");
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
    let avi_parent = GeneratedFile {
        path: "generated/valid/avi_mjpeg_pcm.avi".to_string(),
        bytes: avi.bytes.clone(),
        class: "A",
        op: "parent",
        parent: None,
    };
    let mp4 = build_mp4(&frames, &pcm)?;
    let mp4_parent = GeneratedFile {
        path: "generated/valid/mp4_mjpeg_pcm.mp4".to_string(),
        bytes: mp4.bytes.clone(),
        class: "A",
        op: "parent",
        parent: None,
    };

    // Class-B derivative sets (corpus-plan §1 B1/B2/B3): ten per parent,
    // every cut/patch/bit position derived from the recorded structure
    // map and the parent-seeded PRNG.
    let mut derivatives: Vec<Derivative> = Vec::new();
    derivatives.extend(avi_derivatives(&avi_parent, &avi.layout)?);
    derivatives.extend(mp4_derivatives(&mp4_parent, &mp4.layout)?);

    // Expectation/1 files (doc 10 §5.4) — one per media fixture. Parents
    // carry the full track plan + essence digests (AVI stream ids are
    // 0/1; MP4 track ids are 1/2 — each container's own numbering);
    // derivatives carry their derivation and the class-B error contract.
    let mut expectations: Vec<GeneratedFile> = Vec::with_capacity(2 + derivatives.len());
    expectations.push(GeneratedFile {
        path: expectation_path(&avi_parent.path),
        bytes: parent_expectation(&avi_parent, 0, 1, &frames, &pcm),
        class: "L",
        op: "expectation",
        parent: Some(avi_parent.path.clone()),
    });
    expectations.push(GeneratedFile {
        path: expectation_path(&mp4_parent.path),
        bytes: parent_expectation(&mp4_parent, 1, 2, &frames, &pcm),
        class: "L",
        op: "expectation",
        parent: Some(mp4_parent.path.clone()),
    });
    for derivative in &derivatives {
        expectations.push(GeneratedFile {
            path: expectation_path(&derivative.file.path),
            bytes: derivative_expectation(derivative),
            class: "L",
            op: "expectation",
            parent: derivative.file.parent.clone(),
        });
    }

    // Assembly. The manifest and the ledger cover the class-A/B fixtures
    // (they are computed before the ledger files themselves exist, so no
    // self-referential hash is possible); the expectations follow.
    let mut files: Vec<GeneratedFile> =
        Vec::with_capacity(4 + derivatives.len() + 2 + expectations.len());
    files.push(avi_parent);
    files.push(GeneratedFile {
        path: "generated/valid/avi_mjpeg_pcm.avi.structure.json".to_string(),
        bytes: avi_structure_json(&avi),
        class: "A",
        op: "structure-map",
        parent: Some("generated/valid/avi_mjpeg_pcm.avi".to_string()),
    });
    files.push(mp4_parent);
    files.push(GeneratedFile {
        path: "generated/valid/mp4_mjpeg_pcm.mp4.structure.json".to_string(),
        bytes: mp4_structure_json(&mp4),
        class: "A",
        op: "structure-map",
        parent: Some("generated/valid/mp4_mjpeg_pcm.mp4".to_string()),
    });
    for derivative in derivatives {
        files.push(derivative.file);
    }
    files.push(GeneratedFile {
        path: "generated/MANIFEST.toml".to_string(),
        bytes: manifest_toml(&files),
        class: "L",
        op: "manifest",
        parent: None,
    });
    files.push(GeneratedFile {
        path: "CORPUS.md".to_string(),
        bytes: corpus_markdown(&files),
        class: "L",
        op: "corpus-ledger",
        parent: None,
    });
    files.extend(expectations);
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

    // -- MD5 (RFC 1321 §A.5 test suite, public) -----------------------------

    #[test]
    fn md5_matches_rfc1321_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex(b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            md5_hex(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            md5_hex(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            ),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
        // Block-straddling inputs (55/56/64 bytes cross the padding
        // edge where the length field spills to a second block) —
        // digests pinned against the system md5sum.
        let fifty_five = vec![b'x'; 55];
        let fifty_six = vec![b'x'; 56];
        let sixty_four = vec![b'x'; 64];
        assert_eq!(md5_hex(&fifty_five), "04364420e25c512fd958a70738aa8f72");
        assert_eq!(md5_hex(&fifty_six), "668a72d5ba17f08e62dabcafad6db14b");
        assert_eq!(md5_hex(&sixty_four), "c1bb4f81d892b2d57947682aeb252456");
    }

    // -- [audio_probe] (doc 10 §5.4, card 2.B5) ------------------------------

    #[test]
    fn audio_probe_conversion_pins_the_f32_serialization() {
        // A two-instant stream: +16384 (→ exactly +0.5) and −16384
        // (→ exactly −0.5). The probe bytes must be the little-endian
        // IEEE-754 encodings of those two f32 values — pinned byte
        // for byte against the pinned conversion's own definition.
        let mut pcm = Vec::new();
        pcm.extend_from_slice(&16384i16.to_le_bytes());
        pcm.extend_from_slice(&(-16384i16).to_le_bytes());
        let mut expected = Vec::new();
        expected.extend_from_slice(&0.5f32.to_le_bytes());
        expected.extend_from_slice(&(-0.5f32).to_le_bytes());
        assert_eq!(md5_hex(&expected), audio_probe_md5(&pcm));
        // The values themselves, so the test fails on a conversion
        // drift even if both sides drift together.
        assert_eq!(
            expected,
            vec![0x00, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0xbf]
        );
    }

    #[test]
    fn audio_probe_spans_multiple_chunks_and_is_stable() {
        let pcm = generate_pcm();
        // The corpus carries 88 200 instants; the probe takes 4800 —
        // deterministic and independent of everything past the cut.
        assert!(TOTAL_SAMPLES > AUDIO_PROBE_INSTANTS);
        let digest = audio_probe_md5(&pcm);
        assert_eq!(digest, audio_probe_md5(&pcm));
        // Truncating AFTER the probe window changes nothing; cutting
        // INTO it changes the digest only if the prefix changed.
        let mut longer = pcm.clone();
        longer.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(digest, audio_probe_md5(&longer));
        let cut = AUDIO_PROBE_INSTANTS * AUDIO_CHANNELS * 2 - 2;
        assert_ne!(digest, audio_probe_md5(&pcm[..cut]));
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
        for pair in pcm.as_chunks::<4>().0 {
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

    // -- MP4 writer -----------------------------------------------------------

    /// Walk one level of boxes in `bytes[at..end]`; returns
    /// (kind, offset, size) triples.
    fn walk_boxes(b: &[u8], mut at: usize, end: usize) -> Vec<([u8; 4], usize, usize)> {
        let mut boxes = Vec::new();
        while at + 8 <= end {
            let size = be32(b, at) as usize;
            let kind = [b[at + 4], b[at + 5], b[at + 6], b[at + 7]];
            boxes.push((kind, at, size));
            if size < 8 {
                break; // hostile-size guard for the walker itself
            }
            at += size;
        }
        boxes
    }

    fn find_box<'a>(
        boxes: &'a [([u8; 4], usize, usize)],
        kind: &[u8; 4],
    ) -> &'a ([u8; 4], usize, usize) {
        match boxes.iter().find(|(k, _, _)| k == kind) {
            Some(triple) => triple,
            None => unreachable!("box {} missing", fourcc_str(kind)),
        }
    }

    #[test]
    fn mp4_parent_boxes_walk_cleanly() {
        let b = &find("generated/valid/mp4_mjpeg_pcm.mp4").bytes;
        let top = walk_boxes(b, 0, b.len());
        assert_eq!(top.len(), 3, "top-level boxes: {top:?}");
        assert_eq!(&find_box(&top, b"ftyp").0, b"ftyp");
        let (kind, moov_at, moov_size) = *find_box(&top, b"moov");
        assert_eq!(&kind, b"moov");
        let (kind, mdat_at, mdat_size) = *find_box(&top, b"mdat");
        assert_eq!(&kind, b"mdat");
        assert_eq!(moov_at + moov_size, mdat_at, "moov/mdat adjacency");
        assert_eq!(mdat_at + mdat_size, b.len(), "mdat size covers the rest");
        // moov before mdat (headers first — demuxer-friendly layout).
        assert!(moov_at < mdat_at);

        // ftyp payload: major brand, minor version 0.
        assert_eq!(&b[8..12], b"isom");
        assert_eq!(be32(b, 12), 0, "minor version");

        // moov children: mvhd, trak, trak.
        let moov = walk_boxes(b, moov_at + 8, moov_at + moov_size);
        assert_eq!(moov.len(), 3);
        assert_eq!(&moov[0].0, b"mvhd");
        assert_eq!(&moov[1].0, b"trak");
        assert_eq!(&moov[2].0, b"trak");
        // mvhd: version 0, timescale 1000, duration 2000, next_track 3.
        let (_, mvhd_at, mvhd_size) = moov[0];
        assert_eq!(mvhd_size, 108);
        assert_eq!(b[mvhd_at + 8], 0); // version
        assert_eq!(be32(b, mvhd_at + 20), 1000, "timescale");
        assert_eq!(be32(b, mvhd_at + 24), 2000, "duration (ms)");
        assert_eq!(be32(b, mvhd_at + 104), 3, "next_track_ID");

        // trak children: tkhd, mdia.
        for (trak_index, &(_, trak_at, trak_size)) in moov.iter().skip(1).enumerate() {
            let trak = walk_boxes(b, trak_at + 8, trak_at + trak_size);
            assert_eq!(trak.len(), 2, "trak {trak_index} children");
            assert_eq!(&trak[0].0, b"tkhd");
            assert_eq!(&trak[1].0, b"mdia");
            // tkhd: version 0, flags 3, track id, duration in movie units.
            let (_, tkhd_at, tkhd_size) = trak[0];
            assert_eq!(tkhd_size, 92);
            assert_eq!(be32(b, tkhd_at + 8) & 0xff_ffff, 3, "tkhd flags");
            assert_eq!(be32(b, tkhd_at + 20), trak_index as u32 + 1, "track_ID");
            assert_eq!(be32(b, tkhd_at + 28), 2000, "tkhd duration");
            // mdia children: mdhd, hdlr, minf.
            let (_, mdia_at, mdia_size) = trak[1];
            let mdia = walk_boxes(b, mdia_at + 8, mdia_at + mdia_size);
            assert_eq!(mdia.len(), 3);
            assert_eq!(&mdia[0].0, b"mdhd");
            assert_eq!(&mdia[1].0, b"hdlr");
            assert_eq!(&mdia[2].0, b"minf");
            // (exact media timescales are asserted in the sample-table test)
        }

        // Video tkhd width/height 16.16.
        let (_, trak_at, trak_size) = moov[1];
        let trak = walk_boxes(b, trak_at + 8, trak_at + trak_size);
        let (_, tkhd_at, _) = trak[0];
        assert_eq!(
            be32(b, tkhd_at + 84),
            (FRAME_WIDTH as u32) << 16,
            "tkhd width"
        );
        assert_eq!(
            be32(b, tkhd_at + 88),
            (FRAME_HEIGHT as u32) << 16,
            "tkhd height"
        );
        // Audio tkhd volume 0x0100, zero extent.
        let (_, trak_at, trak_size) = moov[2];
        let trak = walk_boxes(b, trak_at + 8, trak_at + trak_size);
        let (_, tkhd_at, _) = trak[0];
        assert_eq!(be16(b, tkhd_at + 44), 0x0100, "audio volume");
        assert_eq!(be32(b, tkhd_at + 84), 0, "audio width");
    }

    #[test]
    fn mp4_sample_tables_point_at_the_essence() {
        let b = &find("generated/valid/mp4_mjpeg_pcm.mp4").bytes;
        let pcm = generate_pcm();
        let tick_bytes = SAMPLES_PER_TICK * AUDIO_CHANNELS * 2;
        let top = walk_boxes(b, 0, b.len());
        let (_, mdat_at, _) = *find_box(&top, b"mdat");
        let mdat_data = mdat_at + 8;

        // Locate both stbls: moov > trak > mdia > minf > stbl.
        let (_, moov_at, moov_size) = *find_box(&top, b"moov");
        let moov = walk_boxes(b, moov_at + 8, moov_at + moov_size);
        let mut tracks = Vec::new(); // (minf boxes, stbl boxes, mdia boxes)
        for &(_, trak_at, trak_size) in moov.iter().skip(1) {
            let trak = walk_boxes(b, trak_at + 8, trak_at + trak_size);
            let (_, mdia_at, mdia_size) = trak[1];
            let mdia = walk_boxes(b, mdia_at + 8, mdia_at + mdia_size);
            let (_, minf_at, minf_size) = mdia[2];
            let minf = walk_boxes(b, minf_at + 8, minf_at + minf_size);
            let (_, stbl_at, stbl_size) = *find_box(&minf, b"stbl");
            let stbl = walk_boxes(b, stbl_at + 8, stbl_at + stbl_size);
            tracks.push((minf, stbl, mdia));
        }
        // Video minf: vmhd + dinf + stbl; audio: smhd + dinf + stbl.
        assert_eq!(&find_box(&tracks[0].0, b"vmhd").0, b"vmhd");
        assert_eq!(&find_box(&tracks[1].0, b"smhd").0, b"smhd");
        for (index, (_minf, stbl, _mdia)) in tracks.iter().enumerate() {
            let expected: Vec<&[u8; 4]> = if index == 0 {
                vec![b"stsd", b"stts", b"stss", b"stsc", b"stsz", b"stco"]
            } else {
                vec![b"stsd", b"stts", b"stsc", b"stsz", b"stco"]
            };
            let names: Vec<[u8; 4]> = stbl.iter().map(|(k, _, _)| *k).collect();
            let expected_names: Vec<[u8; 4]> = expected.iter().map(|k| **k).collect();
            assert_eq!(names, expected_names, "trak {index} stbl children");
        }

        // Video mdhd/stsd details.
        let (_video_minf, video_stbl, video_mdia) = &tracks[0];
        let (_, mdhd_at, _) = *find_box(video_mdia, b"mdhd");
        assert_eq!(be32(b, mdhd_at + 20), FPS, "video timescale");
        assert_eq!(be32(b, mdhd_at + 24), FRAME_COUNT as u32, "video duration");
        let (_, hdlr_at, _) = *find_box(video_mdia, b"hdlr");
        assert_eq!(&b[hdlr_at + 16..hdlr_at + 20], b"vide");
        let (_, stsd_at, _) = *find_box(video_stbl, b"stsd");
        assert_eq!(&b[stsd_at + 20..stsd_at + 24], b"jpeg");
        assert_eq!(be16(b, stsd_at + 48), FRAME_WIDTH as u16, "stsd width");
        assert_eq!(be16(b, stsd_at + 50), FRAME_HEIGHT as u16, "stsd height");
        // Audio mdhd/stsd details.
        let (_audio_minf, audio_stbl, audio_mdia) = &tracks[1];
        let (_, mdhd_at, _) = *find_box(audio_mdia, b"mdhd");
        assert_eq!(be32(b, mdhd_at + 20), AUDIO_RATE, "audio timescale");
        assert_eq!(
            be32(b, mdhd_at + 24),
            TOTAL_SAMPLES as u32,
            "audio duration"
        );
        let (_, hdlr_at, _) = *find_box(audio_mdia, b"hdlr");
        assert_eq!(&b[hdlr_at + 16..hdlr_at + 20], b"soun");
        let (_, stsd_at, _) = *find_box(audio_stbl, b"stsd");
        assert_eq!(&b[stsd_at + 20..stsd_at + 24], b"sowt");
        assert_eq!(be16(b, stsd_at + 40), AUDIO_CHANNELS as u16, "channels");
        assert_eq!(be32(b, stsd_at + 48), AUDIO_RATE << 16, "samplerate 16.16");

        // Video tables: stts (60,1), stss 1..=60, stsc (1,1,1), stsz
        // per-sample, stco -> mdat bytes.
        let (_, stts_at, _) = *find_box(video_stbl, b"stts");
        assert_eq!(be32(b, stts_at + 12), 1, "stts runs");
        assert_eq!(be32(b, stts_at + 16), FRAME_COUNT as u32, "stts count");
        assert_eq!(be32(b, stts_at + 20), 1, "stts delta");
        let (_, stss_at, _) = *find_box(video_stbl, b"stss");
        assert_eq!(be32(b, stss_at + 12), FRAME_COUNT as u32, "stss count");
        for i in 0..FRAME_COUNT {
            assert_eq!(be32(b, stss_at + 16 + 4 * i), i as u32 + 1, "stss {i}");
        }
        let (_, stsc_at, _) = *find_box(video_stbl, b"stsc");
        assert_eq!(be32(b, stsc_at + 16), 1, "stsc first_chunk");
        assert_eq!(be32(b, stsc_at + 20), 1, "stsc samples_per_chunk");
        let (_, stsz_at, _) = *find_box(video_stbl, b"stsz");
        assert_eq!(
            be32(b, stsz_at + 16),
            FRAME_COUNT as u32,
            "video stsz count"
        );
        let (_, stco_at, _) = *find_box(video_stbl, b"stco");
        assert_eq!(
            be32(b, stco_at + 12),
            FRAME_COUNT as u32,
            "video stco count"
        );
        // Every sample: stco offset + stsz size must land the exact JPEG
        // bytes in mdat (first/last re-derived; middles are SOI/EOI).
        let frame0 = match draw_frame(0).and_then(|img| encode_baseline_jpeg(&img)) {
            Ok(jpeg) => jpeg,
            Err(message) => unreachable!("re-encode failed: {message}"),
        };
        let frame_last =
            match draw_frame(FRAME_COUNT - 1).and_then(|img| encode_baseline_jpeg(&img)) {
                Ok(jpeg) => jpeg,
                Err(message) => unreachable!("re-encode failed: {message}"),
            };
        for i in 0..FRAME_COUNT {
            let offset = be32(b, stco_at + 16 + 4 * i) as usize;
            let size = be32(b, stsz_at + 20 + 4 * i) as usize;
            assert!(
                offset >= mdat_data && offset + size <= b.len(),
                "sample {i} range"
            );
            assert_eq!(&b[offset..offset + 2], &[0xff, 0xd8], "sample {i} SOI");
            assert_eq!(
                &b[offset + size - 2..offset + size],
                &[0xff, 0xd9],
                "sample {i} EOI"
            );
            if i == 0 {
                assert_eq!(&b[offset..offset + size], &frame0[..], "video sample 0");
            }
            if i == FRAME_COUNT - 1 {
                assert_eq!(
                    &b[offset..offset + size],
                    &frame_last[..],
                    "video sample last"
                );
            }
        }
        // Audio tables: stts (60, 1470), stsz 5880 uniform, stco -> PCM.
        let (_, stts_at, _) = *find_box(audio_stbl, b"stts");
        assert_eq!(
            be32(b, stts_at + 16),
            FRAME_COUNT as u32,
            "audio stts count"
        );
        assert_eq!(
            be32(b, stts_at + 20),
            SAMPLES_PER_TICK as u32,
            "audio delta"
        );
        let (_, stsz_at, _) = *find_box(audio_stbl, b"stsz");
        let (_, stco_at, _) = *find_box(audio_stbl, b"stco");
        for i in 0..FRAME_COUNT {
            assert_eq!(
                be32(b, stsz_at + 20 + 4 * i),
                tick_bytes as u32,
                "audio size {i}"
            );
            let offset = be32(b, stco_at + 16 + 4 * i) as usize;
            let from = i * tick_bytes;
            assert_eq!(
                &b[offset..offset + tick_bytes],
                &pcm[from..from + tick_bytes]
            );
        }
        // mdat size == 8 + the sum of every sample size (video + audio).
        let (_, video_stsz_at, _) = *find_box(video_stbl, b"stsz");
        let video_bytes: usize = (0..FRAME_COUNT)
            .map(|i| be32(b, video_stsz_at + 20 + 4 * i) as usize)
            .sum();
        let (_, mdat_box_at, mdat_size) = *find_box(&top, b"mdat");
        assert_eq!(
            mdat_size,
            8 + video_bytes + FRAME_COUNT * tick_bytes,
            "mdat covers exactly the samples"
        );
        assert_eq!(mdat_box_at + mdat_size, b.len());
    }

    #[test]
    fn mp4_structure_map_records_the_walked_layout() {
        let map = find("generated/valid/mp4_mjpeg_pcm.mp4.structure.json");
        let text = match String::from_utf8(map.bytes.clone()) {
            Ok(text) => text,
            Err(_) => unreachable!("structure map is not valid UTF-8"),
        };
        let b = &find("generated/valid/mp4_mjpeg_pcm.mp4").bytes;
        assert!(text.contains("\"schema\":\"structure-map/1\""));
        assert_eq!(text.matches("\"box\":\"ftyp\"").count(), 1);
        assert_eq!(text.matches("\"box\":\"moov\"").count(), 1);
        assert_eq!(text.matches("\"box\":\"mdat\"").count(), 1);
        // Recorded stsz count offset points at 60 in the real file.
        let needle = "\"stsz_video_count_offset\":";
        let at = match text.find(needle) {
            Some(at) => at + needle.len(),
            None => unreachable!("stsz offset missing"),
        };
        let end = text[at..].find(',').unwrap_or(text.len() - at) + at;
        let recorded: usize = match text[at..end].parse() {
            Ok(value) => value,
            Err(_) => unreachable!("stsz offset not an integer"),
        };
        assert_eq!(be32(b, recorded), FRAME_COUNT as u32);
        // Recorded sample arrays: 2*FRAME_COUNT entries plus the three
        // top-level box offsets.
        assert_eq!(text.matches("\"offset\":").count(), 2 * FRAME_COUNT + 3);
        assert!(text.contains("\"mdat_data_offset\":"));
    }

    // -- Class-B derivatives -----------------------------------------------

    fn utf8(file: &'static GeneratedFile) -> String {
        match String::from_utf8(file.bytes.clone()) {
            Ok(text) => text,
            Err(_) => unreachable!("{} is not valid UTF-8", file.path),
        }
    }

    /// Parse the integer that follows `"key":` in a structure map or
    /// expectation file (machine-generated JSON/TOML: the next run of
    /// ASCII digits after the key).
    fn number_after(text: &str, key: &str) -> usize {
        let marker = format!("\"{key}\":");
        let at = match text.find(&marker) {
            Some(at) => at + marker.len(),
            None => unreachable!("key {key} missing"),
        };
        let digits: String = text[at..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        match digits.parse() {
            Ok(value) => value,
            Err(_) => unreachable!("key {key} not followed by an integer"),
        }
    }

    /// The n-th object (0-based) of a structure map's JSON array — entries
    /// are `{"fourcc":"00dc","offset":N,"size":M}` or `{"offset":N,…}`.
    fn nth_object(text: &str, array_key: &str, n: usize) -> String {
        let marker = format!("\"{array_key}\":[");
        let at = match text.find(&marker) {
            Some(at) => at + marker.len(),
            None => unreachable!("array {array_key} missing"),
        };
        let rest = &text[at..];
        let end = rest.find(']').unwrap_or(rest.len());
        match rest[..end].split("},{").nth(n) {
            Some(piece) => piece.to_string(),
            None => unreachable!("array {array_key} has no entry {n}"),
        }
    }

    fn structure_map(fixture: &str) -> String {
        utf8(find(&format!("{fixture}.structure.json")))
    }

    /// The cut offset a truncation derivative recorded in its expectation.
    fn recorded_cut(parent_stem: &str, op: &str, ext: &str) -> usize {
        let text = utf8(find(&format!(
            "expectations/malformed/{parent_stem}__{op}.{ext}.toml"
        )));
        number_after_toml(&text, "cut_offset")
    }

    /// TOML flavor of [`number_after`] (`key = 123`).
    fn number_after_toml(text: &str, key: &str) -> usize {
        let marker = format!("{key} = ");
        let at = match text.find(&marker) {
            Some(at) => at + marker.len(),
            None => unreachable!("TOML key {key} missing"),
        };
        let digits: String = text[at..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .collect();
        match digits.replace('_', "").parse() {
            Ok(value) => value,
            Err(_) => unreachable!("TOML key {key} not followed by an integer"),
        }
    }

    #[test]
    fn full_tree_is_byte_stable_across_two_runs() {
        // The card's AC, proven at the unit level: two independent
        // generate_all() calls produce identical (path, bytes) vectors.
        // (The CLI-level proof is the double `--check` gate.)
        let first = match generate_all() {
            Ok(files) => files,
            Err(message) => unreachable!("first run failed: {message}"),
        };
        let second = match generate_all() {
            Ok(files) => files,
            Err(message) => unreachable!("second run failed: {message}"),
        };
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.path, b.path, "file order drifted");
            assert_eq!(a.bytes, b.bytes, "file {} drifted between runs", a.path);
        }
    }

    #[test]
    fn tree_shape_is_the_planned_48_files() {
        let files = files();
        assert_eq!(files.len(), 48, "total tree size");
        assert_eq!(
            files
                .iter()
                .filter(|f| f.path.starts_with("generated/valid/"))
                .count(),
            4,
            "class-A files"
        );
        assert_eq!(
            files
                .iter()
                .filter(|f| f.path.starts_with("generated/malformed/"))
                .count(),
            20,
            "class-B derivatives"
        );
        assert_eq!(
            files
                .iter()
                .filter(|f| f.path.starts_with("expectations/"))
                .count(),
            22,
            "expectation files"
        );
        assert!(files.iter().any(|f| f.path == "generated/MANIFEST.toml"));
        assert!(files.iter().any(|f| f.path == "CORPUS.md"));
        // Every media fixture has exactly one expectation file; structure
        // maps have none (companion metadata, manifest rows only).
        for file in files.iter().filter(|f| f.class == "A" || f.class == "B") {
            let is_media = file.path.ends_with(".avi") || file.path.ends_with(".mp4");
            let has_expectation = files.iter().any(|f| f.path == expectation_path(&file.path));
            assert_eq!(
                has_expectation, is_media,
                "expectation presence wrong for {}",
                file.path
            );
        }
        // Ledger files are marked class L and never counted as fixtures.
        assert!(
            files
                .iter()
                .filter(|f| f.class == "L")
                .all(|f| f.op == "expectation" || f.op == "manifest" || f.op == "corpus-ledger")
        );
    }

    #[test]
    fn avi_derivatives_cut_at_recorded_structure_boundaries() {
        let parent = find("generated/valid/avi_mjpeg_pcm.avi");
        let map = structure_map("generated/valid/avi_mjpeg_pcm.avi");
        let chunk60 = nth_object(&map, "chunks", FRAME_COUNT);
        let expected_cuts = [
            (12usize, "trunc0"),
            (number_after(&map, "avih_offset") + 8 + 20, "trunc1"),
            (number_after(&map, "movi_data_offset"), "trunc2"),
            (
                number_after(&chunk60, "offset") + 8 + number_after(&chunk60, "size") / 2,
                "trunc3",
            ),
            (parent.bytes.len() - 1, "trunc4"),
        ];
        for (expected_cut, op) in expected_cuts {
            let cut = recorded_cut("avi_mjpeg_pcm", op, "avi");
            assert_eq!(cut, expected_cut, "{op} cut offset");
            let derivative = find(&format!("generated/malformed/avi_mjpeg_pcm__{op}.avi"));
            assert_eq!(
                derivative.bytes,
                parent.bytes[..cut],
                "{op} bytes must be the parent prefix"
            );
            assert!(derivative.bytes.len() < parent.bytes.len());
            assert_eq!(derivative.class, "B");
            assert_eq!(
                derivative.parent.as_deref(),
                Some("generated/valid/avi_mjpeg_pcm.avi")
            );
        }
    }

    #[test]
    fn mp4_derivatives_cut_at_recorded_structure_boundaries() {
        let parent = find("generated/valid/mp4_mjpeg_pcm.mp4");
        let map = structure_map("generated/valid/mp4_mjpeg_pcm.mp4");
        let sample30 = nth_object(&map, "video_samples", FRAME_COUNT / 2);
        let moov_offset = number_after(&map, "mvhd_offset") - 8;
        let expected_cuts = [
            (moov_offset, "trunc0"),
            (number_after(&map, "mvhd_offset") + 8 + 20, "trunc1"),
            (number_after(&map, "mdat_data_offset"), "trunc2"),
            (
                number_after(&sample30, "offset") + number_after(&sample30, "size") / 2,
                "trunc3",
            ),
            (parent.bytes.len() - 1, "trunc4"),
        ];
        for (expected_cut, op) in expected_cuts {
            let cut = recorded_cut("mp4_mjpeg_pcm", op, "mp4");
            assert_eq!(cut, expected_cut, "{op} cut offset");
            let derivative = find(&format!("generated/malformed/mp4_mjpeg_pcm__{op}.mp4"));
            assert_eq!(
                derivative.bytes,
                parent.bytes[..cut],
                "{op} bytes must be the parent prefix"
            );
            assert!(derivative.bytes.len() < parent.bytes.len());
        }
    }

    #[test]
    fn bitflip_derivatives_flip_exactly_k_header_bits() {
        for (parent_path, extent) in [
            ("generated/valid/avi_mjpeg_pcm.avi", AVI_FLIP_EXTENT),
            ("generated/valid/mp4_mjpeg_pcm.mp4", MP4_FLIP_EXTENT),
        ] {
            let parent = find(parent_path);
            let name = &parent_path["generated/valid/".len()..];
            let (stem, ext) = match name.rsplit_once('.') {
                Some((stem, ext)) => (stem, ext),
                None => unreachable!("parent path without extension"),
            };
            // diff masks per k, for the nesting check below.
            let mut masks: Vec<(usize, Vec<u8>)> = Vec::new();
            for k in [1usize, 4, 16] {
                let derivative = find(&format!("generated/malformed/{stem}__flip{k}.{ext}"));
                assert_eq!(
                    derivative.bytes.len(),
                    parent.bytes.len(),
                    "{stem} flip{k} size"
                );
                let mask: Vec<u8> = derivative
                    .bytes
                    .iter()
                    .zip(parent.bytes.iter())
                    .take(extent)
                    .map(|(&d, &p)| d ^ p)
                    .collect();
                assert_eq!(mask.len(), extent, "{stem} flip{k} mask size");
                let bits: usize = mask.iter().map(|b| b.count_ones() as usize).sum();
                assert_eq!(bits, k, "{stem} flip{k} flipped bit count");
                assert_eq!(
                    &derivative.bytes[extent..],
                    &parent.bytes[extent..],
                    "{stem} flip{k} touched bytes past the header extent"
                );
                masks.push((k, mask));
            }
            // k-prefix property: flip1's bits ⊆ flip4's ⊆ flip16's.
            for ((small_k, small), (big_k, big)) in masks.iter().zip(masks.iter().skip(1)) {
                assert!(
                    small.iter().zip(big.iter()).all(|(&a, &b)| (a & b) == a),
                    "flip{small_k} bits are not a subset of flip{big_k} ({stem})"
                );
            }
        }
    }

    #[test]
    fn garbage_derivatives_append_a_seeded_block() {
        let mut suffixes: Vec<Vec<u8>> = Vec::new();
        for parent_path in [
            "generated/valid/avi_mjpeg_pcm.avi",
            "generated/valid/mp4_mjpeg_pcm.mp4",
        ] {
            let parent = find(parent_path);
            let derivative = find(&derivative_path(parent_path, "garbage0"));
            assert_eq!(
                derivative.bytes.len(),
                parent.bytes.len() + GARBAGE_LEN,
                "{} garbage size",
                parent_path
            );
            assert_eq!(
                &derivative.bytes[..parent.bytes.len()],
                &parent.bytes[..],
                "garbage derivative must keep the parent as prefix"
            );
            let suffix = derivative.bytes[parent.bytes.len()..].to_vec();
            // The block is seeded garbage, not zeros or any simple pattern.
            let first = suffix[0];
            assert!(
                suffix.iter().any(|&b| b != first),
                "garbage block is a constant fill"
            );
            suffixes.push(suffix);
        }
        // Different parents hash to different seeds → different blocks.
        assert_ne!(suffixes[0], suffixes[1], "garbage blocks are identical");
    }

    #[test]
    fn avi_no_idx1_derivative_drops_the_index_and_patches_riff_size() {
        let parent = find("generated/valid/avi_mjpeg_pcm.avi");
        let map = structure_map("generated/valid/avi_mjpeg_pcm.avi");
        let idx1_offset = number_after(&map, "idx1_offset");
        let derivative = find("generated/malformed/avi_mjpeg_pcm__noidx0.avi");
        assert_eq!(derivative.bytes.len(), idx1_offset, "idx1 dropped");
        // The RIFF size covers exactly the surviving bytes.
        assert_eq!(le32(&derivative.bytes, 4) as usize, idx1_offset - 8);
        // Everything except the patched size field is parent-identical.
        assert_eq!(&derivative.bytes[..4], &parent.bytes[..4]);
        assert_eq!(&derivative.bytes[8..], &parent.bytes[8..idx1_offset]);
        // The expectation recorded the patch site.
        let text = utf8(find(
            "expectations/malformed/avi_mjpeg_pcm__noidx0.avi.toml",
        ));
        assert!(text.contains("op = \"drop-idx1\""));
        assert_eq!(number_after_toml(&text, "riff_size_field_offset"), 4);
    }

    #[test]
    fn mp4_stsz_lie_derivative_patches_only_the_count() {
        let parent = find("generated/valid/mp4_mjpeg_pcm.mp4");
        let map = structure_map("generated/valid/mp4_mjpeg_pcm.mp4");
        let at = number_after(&map, "stsz_video_count_offset");
        let derivative = find("generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4");
        assert_eq!(derivative.bytes.len(), parent.bytes.len());
        let mut differing = 0usize;
        for i in 0..parent.bytes.len() {
            if derivative.bytes[i] != parent.bytes[i] {
                differing += 1;
            }
        }
        assert_eq!(differing, 4, "exactly the 4 count bytes differ");
        assert_eq!(be32(&derivative.bytes, at), 0x7fff_ffff);
        assert_eq!(be32(&parent.bytes, at), FRAME_COUNT as u32);
    }

    #[test]
    fn derivatives_never_equal_their_parent() {
        // A no-op derivative would be a hole in the malformed corpus:
        // every class-B file must differ from its parent's bytes.
        for file in files().iter().filter(|f| f.class == "B") {
            let parent_path = match &file.parent {
                Some(path) => path,
                None => unreachable!("class-B file without a parent"),
            };
            let parent = find(parent_path);
            assert_ne!(
                file.bytes, parent.bytes,
                "{} is byte-identical to its parent",
                file.path
            );
        }
    }

    // -- Ledger files: MANIFEST, CORPUS, expectations -----------------------

    #[test]
    fn manifest_records_every_fixture_with_recomputed_hashes() {
        let text = utf8(find("generated/MANIFEST.toml"));
        assert!(text.contains("schema = \"manifest/1\""));
        assert!(text.contains(&format!("seed_root = \"{}\"", SEED_ROOT)));
        let fixtures: Vec<&GeneratedFile> = files()
            .iter()
            .filter(|f| f.class == "A" || f.class == "B")
            .collect();
        assert_eq!(fixtures.len(), 24);
        assert_eq!(text.matches("[[files]]").count(), 24);
        for file in &fixtures {
            let row = format!("path = \"{}\"", file.path);
            assert!(text.contains(&row), "manifest row for {}", file.path);
            assert!(
                text.contains(&format!("sha256 = \"{}\"", sha256_hex(&file.bytes))),
                "manifest hash for {}",
                file.path
            );
            assert!(
                text.contains(&format!(
                    "size_bytes = {}",
                    toml_uint(file.bytes.len() as u64)
                )),
                "manifest size for {}",
                file.path
            );
        }
        // Ledger files are not manifest rows (no self-reference).
        assert!(!text.contains("path = \"generated/MANIFEST.toml\""));
        assert!(!text.contains("path = \"CORPUS.md\""));
        assert!(!text.contains("expectations/"));
    }

    #[test]
    fn corpus_ledger_declares_cc0_and_lists_every_fixture() {
        let text = utf8(find("CORPUS.md"));
        assert!(text.contains("CC0 1.0 Universal Public Domain Dedication"));
        assert!(text.contains("https://creativecommons.org/publicdomain/zero/1.0/"));
        assert!(text.contains("clean-room"));
        assert!(text.contains("ffmpeg was used only as a local"));
        let fixtures: Vec<&GeneratedFile> = files()
            .iter()
            .filter(|f| f.class == "A" || f.class == "B")
            .collect();
        for file in &fixtures {
            assert!(
                text.contains(&format!("| {} |", file.path)),
                "CORPUS row for {}",
                file.path
            );
            assert!(
                text.contains(&sha256_hex(&file.bytes)),
                "CORPUS hash for {}",
                file.path
            );
        }
        let total: usize = fixtures.iter().map(|f| f.bytes.len()).sum();
        assert!(text.contains(&format!(
            "Total: {} fixture file(s), {} bytes.",
            fixtures.len(),
            total
        )));
    }

    #[test]
    fn parent_expectations_match_the_doc10_v0_shape() {
        let pcm = generate_pcm();
        for (path, video_id, audio_id) in [
            ("generated/valid/avi_mjpeg_pcm.avi", 0u32, 1u32),
            ("generated/valid/mp4_mjpeg_pcm.mp4", 1u32, 2u32),
        ] {
            let file = find(path);
            let text = utf8(find(&expectation_path(path)));
            assert!(text.contains("schema = \"expectation/1\""), "{path}");
            assert!(text.contains(&format!("fixture = \"{path}\"")));
            assert!(text.contains("class = \"A\""));
            assert!(text.contains(&format!("sha256 = \"{}\"", sha256_hex(&file.bytes))));
            assert!(text.contains(&format!(
                "size_bytes = {}",
                toml_uint(file.bytes.len() as u64)
            )));
            assert!(text.contains("[tracks]"));
            assert!(text.contains(&format!(
                "video = [{{ id = {video_id}, codec = \"mjpeg\", width = {FRAME_WIDTH}, height = {FRAME_HEIGHT}, fps = {{ num = {FPS}, den = 1 }} }}]"
            )));
            assert!(text.contains(&format!(
                "audio = [{{ id = {audio_id}, codec = \"pcm_s16le\", rate = {AUDIO_RATE}, channels = {AUDIO_CHANNELS} }}]"
            )));
            assert!(text.contains("duration_us = 2_000_000"));
            assert!(text.contains("[seek]"));
            assert!(text.contains("tolerance_us = 33_333"));
            assert!(text.contains("[first_frame]"));
            assert!(text.contains(&format!("hash_y = \"{}\"", FIRST_FRAME_HASH_Y)));
            assert!(text.contains("stamp_us = 0"));
            // Card 2.B5: the audio probe, derived from the generator's
            // own samples (see audio_probe_md5's provenance note).
            assert!(text.contains("[audio_probe]"));
            assert!(text.contains(&format!(
                "first_samples_md5 = \"{}\"",
                audio_probe_md5(&pcm)
            )));
            assert!(text.contains("[essence_v0]"));
            // Sixty per-sample video digests; the first is frame 0's JPEG.
            let digests: Vec<&str> = text.lines().filter(|l| l.starts_with("  \"")).collect();
            assert_eq!(digests.len(), FRAME_COUNT, "{path} digest count");
            let frame0 = match draw_frame(0).and_then(|img| encode_baseline_jpeg(&img)) {
                Ok(jpeg) => jpeg,
                Err(message) => unreachable!("re-encode failed: {message}"),
            };
            assert!(
                digests[0].contains(&sha256_hex(&frame0)),
                "{path} first frame digest"
            );
            // The audio digest is the generator's own PCM stream.
            assert!(text.contains(&format!("audio_stream_sha256 = \"{}\"", sha256_hex(&pcm))));
            assert!(text.contains("audio_stream_bytes = 352_800"));
        }
    }

    #[test]
    fn derivative_expectations_carry_the_error_contract() {
        for file in files().iter().filter(|f| f.class == "B") {
            let text = utf8(find(&expectation_path(&file.path)));
            assert!(text.contains("class = \"B\""), "{}", file.path);
            let parent = match &file.parent {
                Some(parent) => parent,
                None => unreachable!("class-B file without a parent"),
            };
            assert!(text.contains(&format!("parent = \"{parent}\"")));
            assert!(text.contains(&format!("sha256 = \"{}\"", sha256_hex(&file.bytes))));
            assert!(text.contains("[derivation]"));
            assert!(text.contains("[error]"));
            assert!(text.contains("expected = \"typed-error-or-bounded-recovery\""));
            assert!(text.contains("never = \"panic, hang, unbounded allocation\""));
            assert!(text.contains("rss_cap_mb = 64"));
            assert!(text.contains("wall_cap_ms = 5_000"));
            // The recorded derivation matches the file's own op family.
            let expected_op = match file.op {
                "trunc0" | "trunc1" | "trunc2" | "trunc3" | "trunc4" => "op = \"truncate\"",
                "flip1" | "flip4" | "flip16" => "op = \"bit-flip\"",
                "garbage0" => "op = \"garbage-append\"",
                "noidx0" => "op = \"drop-idx1\"",
                "stszlie0" => "op = \"stsz-count-lie\"",
                other => unreachable!("unknown derivative op {other}"),
            };
            assert!(text.contains(expected_op), "{}: {expected_op}", file.path);
        }
    }
}
