fn main() {
    for db in [770.6f32, 770.63, 771.0, 800.0, 1e6] {
        let v = audio_engine::volume::Volume::from_db(db);
        match v {
            Ok(v) => println!("from_db({db}) = Ok: linear={} is_finite={} apply(0.5)={} apply(0.0)={}",
                v.linear(), v.linear().is_finite(), v.apply(0.5), v.apply(0.0)),
            Err(e) => println!("from_db({db}) = Err: {}", e.context.code),
        }
    }
    // the documented invariant + the negative control
    let v = audio_engine::volume::Volume::from_db(6.0).unwrap();
    println!("control from_db(6.0): linear={} (expect ~1.99526)", v.linear());
    match audio_engine::volume::Volume::from_db(f32::INFINITY) {
        Err(_) => println!("from_db(+inf) correctly rejected"),
        Ok(_) => println!("from_db(+inf) ACCEPTED (bug)"),
    }
}
