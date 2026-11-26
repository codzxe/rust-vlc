// Gate-proof fixture (FX8-A09): a DELIBERATELY UNREGISTERED fuzz target.
//
// Seeded into fuzz/fuzz_targets/ of the proof copy by run_proofs.sh to prove
// the campaign gate's W17-A06 F-003 branch goes red: a .rs file here WITHOUT
// a matching [[bin]] name entry in fuzz/Cargo.toml used to die at run time
// and be labeled "clean" by the old script. This file is never compiled (the
// gate exits before any run) — it only has to be a *.rs file whose
// stem is absent from the [[bin]] registry. NEVER register it in
// fuzz/Cargo.toml.
#![allow(dead_code)]

pub fn fixture_not_a_registered_target() {}
