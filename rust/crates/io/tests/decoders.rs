//! The cross-decoder fixtures in `testdata/decoders/` must match a fresh `rekolor-io` decode:
//! pixels, size and warnings. The browser side compares its own decoding with the same references
//! (`typescript/tests/browser/decoders.test.ts`). Regenerate with
//! `cargo run --release -p rekolor-io --example update_decoder_fixtures`.

use std::path::Path;

#[test]
fn decoder_fixtures_match_their_references() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/decoders");
    let tsv = std::fs::read_to_string(dir.join("references.tsv")).unwrap();
    let mut count = 0;
    for line in tsv.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<&str> = line.split('\t').collect();
        let [name, file, width, height, warnings] = cols[..] else {
            panic!("bad line {line:?}");
        };
        let decoded = rekolor_io::decode_file(&dir.join(file)).unwrap();
        assert_eq!(
            (decoded.width().to_string(), decoded.height().to_string()),
            (width.to_string(), height.to_string()),
            "{name}: size"
        );
        assert!(
            decoded.rgba() == std::fs::read(dir.join(format!("{name}.rgba"))).unwrap(),
            "{name}: pixels differ from {name}.rgba"
        );
        let now: Vec<String> = decoded
            .warnings()
            .iter()
            .map(|w| format!("{w:?}"))
            .collect();
        assert_eq!(now.join("; "), warnings, "{name}: warnings");
        count += 1;
    }
    assert_eq!(count, 5);
}
