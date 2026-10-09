//! ΔE fingerprint: this build's ΔE must reproduce `testdata/baseline/delta-e-fingerprint.tsv`
//! bit for bit. The WASM suite runs the same check, so both targets match the same file and hence
//! each other. Update the file with
//! `cargo run --release -p rekolor-core --example update_baseline`.

// Files in a crate's `tests/` folder are integration tests: each is compiled as a separate
// program that uses the crate like any outside user would (only its `pub` items). Run them with
// `cargo test -p rekolor-core`.
//
// `#[path = ...]` includes a source file from outside this folder as the module `generator`; the
// same file is shared by several test programs and by the example that updates the baseline.
#[path = "../../../testdata/generator.rs"]
mod generator;

use rekolor_core::delta_e_2000;

// `#[test]` marks a function as a test; it fails if it panics, e.g. when an `assert_eq!` sees
// two different values.
#[test]
fn delta_e_matches_the_recorded_fingerprint() {
    // `include_str!` embeds the file's text into the test program at compile time.
    let recorded = include_str!("../../../testdata/baseline/delta-e-fingerprint.tsv");
    let now = generator::delta_e_fingerprint(|x, y| delta_e_2000(x.into(), y.into()));
    assert_eq!(now, recorded, "ΔE differs from the recorded fingerprint");
}
