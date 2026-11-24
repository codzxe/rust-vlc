// W13-A08 replica probe: mint_adts_aus walk (apps/cli/src/containers/ts.rs:334-389)
// against a VERBATIM copy of demux::adts::AdtsHeader::parse (crates/demux/src/adts/mod.rs:193-214).
// Question: does a first ADTS frame declaring frame_length == 0 make the walk loop forever?
// Replica only — the real code was sed-verified separately; this proves parse semantics + walk non-progress.

// ---- VERBATIM copy of AdtsHeader::parse + the two accessors used (adts/mod.rs:193-236) ----
struct AdtsHeader {
    frame_length: u16,
    sampling_index: u8,
    blocks_minus_one: u8,
}

const SAMPLE_RATES: [u32; 13] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
];

impl AdtsHeader {
    pub fn parse(head: &[u8]) -> Option<AdtsHeader> {
        if head.len() < 7 || head[0] != 0xFF || head[1] & 0xF0 != 0xF0 {
            return None;
        }
        Some(AdtsHeader {
            frame_length: u16::from(head[3] & 0x03) << 11
                | u16::from(head[4]) << 3
                | u16::from(head[5] >> 5),
            sampling_index: head[2] >> 2 & 0x0F,
            blocks_minus_one: head[6] & 0x03,
        })
    }
    pub fn samples(&self) -> u64 {
        1024 * u64::from(self.blocks_minus_one + 1)
    }
    pub fn sample_rate(&self) -> Option<u32> {
        if self.sampling_index < SAMPLE_RATES.len() as u8 {
            Some(SAMPLE_RATES[usize::from(self.sampling_index)])
        } else {
            None
        }
    }
}

fn main() {
    // Crafted first frame: syncword FF F1 (MPEG-4, no CRC), profile LC, sf_idx 4 (44100),
    // frame_length bits all zero  =>  frame_length == 0. Then filler to a 64 KiB payload.
    let mut payload = vec![0x00u8; 65536];
    payload[0] = 0xFF;
    payload[1] = 0xF1;
    payload[2] = (1 << 6) | (4 << 2); // profile LC, sf_idx 4 — same shape as the repo's adts_frame_85
    payload[3] = 0x00; // channel hi bits 0 + frame_length bits 10..11 = 0
    payload[4] = 0x00; // frame_length bits 9..3 = 0
    payload[5] = 0x00; // frame_length bits 2..0 = 0 (top 3 bits)
    payload[6] = 0x00; // fullness + blocks_minus_one = 0 (1 raw block)

    // Sanity: the parse accepts it and reports frame_length 0.
    let head = AdtsHeader::parse(&payload).expect("parse must accept the crafted header");
    assert_eq!(head.frame_length, 0);
    assert_eq!(head.sample_rate(), Some(44100));
    println!("parse accepted: frame_length={} rate={:?}", head.frame_length, head.sample_rate());

    // The walk, verbatim in structure from ts.rs:351-377 (queue pushes replaced by a counter).
    let rate = AdtsHeader::parse(&payload).and_then(|h| h.sample_rate());
    let mut at = 0usize;
    let mut samples_before = 0u64;
    let mut aus = 0u64;
    let mut queue_len = 0usize;
    let mut zero_len_packets = 0usize;
    let cap = 1_000_000u64; // a real loop would be unbounded; cap proves non-progress
    while at + 7 <= payload.len() {
        let Some(header) = AdtsHeader::parse(&payload[at..]) else { break };
        let end = at + usize::from(header.frame_length);
        if end > payload.len() {
            break;
        }
        // mint packet payload[at..end]
        let data_len = end - at;
        if data_len == 0 { zero_len_packets += 1; }
        queue_len += 1;
        samples_before = samples_before.saturating_add(header.samples());
        aus += 1;
        at = end;
        if aus >= cap {
            break;
        }
    }
    println!(
        "after {aus} iterations: at={at} (payload len {}), queue events minted={queue_len}, zero-byte packets={zero_len_packets}, samples_before={samples_before}",
        payload.len(),
    );
    if at == 0 && aus == cap {
        println!("RESULT: NON-PROGRESS — `at` never advanced; the real mint loop pushes {} Packet events and never returns.", queue_len);
    } else {
        println!("RESULT: walk terminated on its own (at={at}, aus={aus}).");
    }
}
