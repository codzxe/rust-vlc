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
// Placeholder for the generation pipeline (filled in by the next
// milestones of card 2.A3 — kept honest: nothing is written until the
// real writers land).
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
    Err("fixture generation is still under construction (card 2.A3 \
         milestone 2: JPEG/AVI/MP4 writers land next)"
        .to_string())
}

/// Print the `--list` table (no writes).
fn print_file_table(_files: &[GeneratedFile]) {}

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
}
