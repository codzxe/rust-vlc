struct Xs { state: u64 }
impl Xs {
    fn from_digest(seed: &[u8]) -> Xs {
        let mut value: u64 = 0;
        for &byte in seed.iter().take(8) {
            value = (value << 8) | byte as u64;
        }
        Xs { state: if value == 0 { 1 } else { value } }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, bound: u64) -> u64 { self.next_u64() % bound }
}
fn shuffle_first(total: usize, seed: u64) -> Vec<u32> {
    let mut rng = Xs { state: seed };
    let mut indices: Vec<u32> = (0..total as u32).collect();
    for i in (1..total).rev() {
        let j = rng.below((i + 1) as u64) as usize;
        indices.swap(i, j);
    }
    indices
}
fn main() {
    // AVI parent sha256 = 1f56202d7b113d38f9e1564fd09091803a264ea2ac88ec9780232ca10a3d271a
    let avi_seed = u64::from_be_bytes([0x1f,0x56,0x20,0x2d,0x7b,0x11,0x3d,0x38]);
    // MP4 parent sha256 = d0459524e3be5e90174c2a861724c3bb06fc12fbcbb52c7feea5f117aaa26da6
    let mp4_seed = u64::from_be_bytes([0xd0,0x45,0x95,0x24,0xe3,0xbe,0x5e,0x90]);
    for (name, seed) in [("AVI", avi_seed), ("MP4", mp4_seed)] {
        for total in [64usize, 512, 4096] {
            let idx = shuffle_first(total, seed);
            println!("{name} total={total} first16={:?}", &idx[..16.min(idx.len())]);
        }
    }
}
