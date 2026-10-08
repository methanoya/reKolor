//! ΔE fingerprint (R11): this build's ΔE must reproduce `testdata/baseline/delta-e-fingerprint.tsv`
//! bit for bit. The WASM suite runs the same check, so both targets match the same file and hence
//! each other. Update the file with
//! `cargo run --release -p rekolor-core --example update_baseline`.

#[path = "../../../testdata/generator.rs"]
mod generator;

use rekolor_core::delta_e_2000;

#[test]
fn delta_e_matches_the_recorded_fingerprint() {
    let recorded = include_str!("../../../testdata/baseline/delta-e-fingerprint.tsv");
    let now = generator::delta_e_fingerprint(|x, y| delta_e_2000(x.into(), y.into()));
    assert_eq!(now, recorded, "ΔE differs from the recorded fingerprint");
}
