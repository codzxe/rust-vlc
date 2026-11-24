// Driver: run the REAL streaming::dash::plan_representation (prebuilt
// debug rlibs: streaming + playlists) on a crafted .mpd manifest.
fn main() {
    let path = std::env::args().nth(1).expect("usage: dash_driver <file.mpd>");
    let bytes = std::fs::read(&path).expect("read");
    let parsed = match playlists::parse_mpd_bytes(&bytes) {
        Ok(p) => p,
        Err(e) => { println!("parse error: {e:?}"); return; }
    };
    let mpd = &parsed.mpd;
    let period = &mpd.periods[0];
    let set = &period.adaptation_sets[0];
    let rep = &set.representations[0];
    match streaming::dash::plan_representation(mpd, period, set, rep, "http://ex.test/v.mpd") {
        Ok(plan) => println!("plan ok: segments={} first={:?}", plan.segments.len(), plan.segments.first()),
        Err(e) => println!("plan error: {e:?}"),
    }
}
