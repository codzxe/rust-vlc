use subtitles::cc::{CcDecoder, CcCueBuilder};
use subtitles::cc::cc_data::CcDataTriplet;
use subtitles::cc::eia608::with_odd_parity;

fn t(d1: u8, d2: u8) -> CcDataTriplet { CcDataTriplet::cc(0, [with_odd_parity(d1), with_odd_parity(d2)]) }

fn show(dec: &mut CcDecoder, pairs: &[(u8,u8)], pts: i64) {
    let trips: Vec<CcDataTriplet> = pairs.iter().map(|&(a,b)| t(a,b)).collect();
    let u = dec.decode(&trips, Some(pts));
    for e in &u.events {
        let rows: Vec<String> = (0..15).map(|r| e.grid.row_text(r)).filter(|s| !s.is_empty()).collect();
        println!("  pts={pts} ch={:?} rows={:?}", e.channel, rows);
    }
    if !u.damage.is_empty() {
        println!("  damage: {:?}", u.damage.iter().map(|d| d.clone()).take(4).collect::<Vec<_>>());
    }
}

fn main() {
    // REAL-WORLD pop-on caption: RCL, PAC row 5 (real spec: 0x15 0x40),
    // "HELLO WORLD" as raw text pairs, EOC.
    let mut dec = CcDecoder::new();
    show(&mut dec, &[(0x14,0x20)], 1);                       // RCL pop-on
    show(&mut dec, &[(0x15,0x40)], 2);                       // real-spec PAC row 5
    show(&mut dec, &[(0x48,0x45),(0x4C,0x4C),(0x4F,0x20),(0x57,0x4F),(0x52,0x4C),(0x44,0x21)], 3); // HELLO WORLD!
    show(&mut dec, &[(0x14,0x2F)], 4);                       // EOC swap
    println!("-- expect (real spec): row 5 'HELLO WORLD!'");

    // Repo-model PAC (0x45 0x40 = its row 5) with lowercase text.
    let mut dec = CcDecoder::new();
    show(&mut dec, &[(0x14,0x20),(0x45,0x40),(0x68,0x69),(0x14,0x2F)], 10); // "hi" after PAC
    println!("-- expect (repo model): row 5 'hi'");

    // Text pair 'AB' (0x41,0x42) vs 'ab':
    let mut dec = CcDecoder::new();
    show(&mut dec, &[(0x14,0x29),(0x41,0x42),(0x61,0x62)], 20); // paint-on "AB" "ab"
    println!("-- expect: 'ABab'; observe what survives");

    // Ledger cap: 300 parity-broken pairs.
    let mut dec = CcDecoder::new();
    let trips: Vec<CcDataTriplet> = (0..300).map(|_| CcDataTriplet::cc(0, [0x14, 0x29])).collect();
    let u = dec.decode(&trips, Some(1));
    println!("-- 300 bad-parity pairs: damage.len()={} (cap 256, no overflow counter field)", u.damage.len());

    // End-at-next-event minus 1: half-open seam check.
    let mut dec = CcDecoder::new();
    let u = dec.decode(&[t(0x14,0x29), t(0x61,0x62)], Some(1_000_000_000));
    let cues = CcCueBuilder::new().cues(&u, Some(2_000_000_000), 0).unwrap();
    println!("-- end={:?} active_at(next-1)={} active_at(next)={}",
        cues[0].end, cues[0].active_at(1_999_999_999), cues[0].active_at(2_000_000_000));
}
