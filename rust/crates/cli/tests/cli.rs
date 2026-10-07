//! End-to-end: runs the `rekolor` binary on a temporary samples tree.

#[path = "../../../testdata/generator.rs"]
mod generator;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rekolor_cli::config::PaletteConfig;

fn repo_palette() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../palettes/pantone.json"
    ))
}

fn rekolor(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rekolor"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run rekolor")
}

fn ok(output: Output) -> String {
    assert!(
        output.status.success(),
        "rekolor failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A samples tree with two generated images: `edges.png` and `sub/gradient.png`.
fn samples_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("samples/sub")).unwrap();
    for (name, path) in [
        ("edges", "samples/edges.png"),
        ("gradient", "samples/sub/gradient.png"),
    ] {
        let f = generator::fixture(name);
        rekolor_io::write_png(&dir.path().join(path), &f.rgba, f.width, f.height).unwrap();
    }
    dir
}

#[test]
fn generate_configs_then_update_goldens() {
    let tree = samples_tree();
    let root = tree.path();
    let palette = repo_palette();
    let palette = palette.to_str().unwrap();

    let stdout = ok(rekolor(
        &["palettes", "generate", "samples", "--palette", palette],
        root,
    ));
    assert!(stdout.contains("edges.palettes.toml"), "{stdout}");

    for image in ["samples/edges.png", "samples/sub/gradient.png"] {
        let config_path = root.join(image.replace(".png", ".palettes.toml"));
        let config = PaletteConfig::load(&config_path).unwrap();
        let sizes: Vec<u32> = config.palette.iter().map(|p| p.size).collect();
        assert_eq!(sizes, [3, 7, 16]);
        for sized in &config.palette {
            assert!(sized.picks.len() <= sized.size as usize);
            let inks: HashSet<&str> = sized.picks.iter().map(|p| p.ink.as_str()).collect();
            assert_eq!(inks.len(), sized.picks.len(), "inks must be distinct");
        }
        // Smaller palettes are prefixes of larger ones.
        let p16 = &config.size(16).unwrap().picks;
        assert_eq!(
            &p16[..config.size(3).unwrap().picks.len()],
            &config.size(3).unwrap().picks[..]
        );
    }

    // Existing configs (possibly hand-edited) are kept unless --force.
    let edges_config = root.join("samples/edges.palettes.toml");
    let edited = std::fs::read_to_string(&edges_config).unwrap() + "# hand edit\n";
    std::fs::write(&edges_config, &edited).unwrap();
    let stdout = ok(rekolor(
        &["palettes", "generate", "samples", "--palette", palette],
        root,
    ));
    assert!(stdout.contains("kept existing config"), "{stdout}");
    assert_eq!(std::fs::read_to_string(&edges_config).unwrap(), edited);

    // Goldens: three outputs per image, same size, only the config's inks.
    ok(rekolor(
        &["golden", "update", "samples", "--palette", palette],
        root,
    ));
    let palette_data = rekolor_cli::palette_file::load(Path::new(palette)).unwrap();
    for (image, size) in [
        ("samples/edges", 3),
        ("samples/edges", 7),
        ("samples/sub/gradient", 16),
    ] {
        let decoded =
            rekolor_io::decode_file(&root.join(format!("{image}-out-{size}.png"))).unwrap();
        let source = rekolor_io::decode_file(&root.join(format!("{image}.png"))).unwrap();
        assert_eq!(
            (decoded.width, decoded.height),
            (source.width, source.height)
        );

        let config = PaletteConfig::load(&root.join(format!("{image}.palettes.toml"))).unwrap();
        let inks: HashSet<[u8; 3]> = config
            .size(size)
            .unwrap()
            .mappings(&palette_data)
            .unwrap()
            .iter()
            .map(|m| [m.ink.r, m.ink.g, m.ink.b])
            .collect();
        for p in decoded.rgba.as_chunks::<4>().0 {
            assert_eq!(p[3], 255);
            assert!(
                inks.contains(&[p[0], p[1], p[2]]),
                "{image}-out-{size}: {p:?}"
            );
        }
    }
    // Outputs are not picked up as inputs on the next run.
    let stdout = ok(rekolor(
        &["palettes", "generate", "samples", "--palette", palette],
        root,
    ));
    assert!(!stdout.contains("-out-"), "{stdout}");
}

#[test]
fn recolor_with_picks_and_analyze() {
    let tree = samples_tree();
    let root = tree.path();
    let palette = repo_palette();
    let palette = palette.to_str().unwrap();

    let stdout = ok(rekolor(
        &[
            "recolor",
            "samples/edges.png",
            "-o",
            "out.png",
            "--pick",
            "255,184,0=Pantone 1235",
            "--pick",
            "44,44,57=Pantone 532",
            "--palette",
            palette,
        ],
        root,
    ));
    assert!(stdout.contains("2 picks"), "{stdout}");
    assert!(root.join("out.png").exists());

    let stdout = ok(rekolor(&["analyze", "samples/edges.png"], root));
    assert!(stdout.contains("96×96"), "{stdout}");

    let failed = rekolor(
        &[
            "recolor",
            "samples/edges.png",
            "-o",
            "x.png",
            "--pick",
            "1,2,3=No Such Ink",
            "--palette",
            palette,
        ],
        root,
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("not in the palette"));
}

#[test]
fn default_palette_is_found_in_a_parent_directory() {
    let tree = samples_tree();
    let root = tree.path();
    std::fs::create_dir_all(root.join("palettes")).unwrap();
    std::fs::copy(repo_palette(), root.join("palettes/pantone.json")).unwrap();
    // No --palette, run from a subdirectory.
    ok(rekolor(
        &[
            "recolor",
            "edges.png",
            "-o",
            "../out.png",
            "--pick",
            "255,184,0=Pantone 1235",
        ],
        &root.join("samples"),
    ));
    assert!(root.join("out.png").exists());
}

#[test]
fn decoder_warnings_go_to_stderr() {
    let tree = samples_tree();
    let output = rekolor(&["analyze", "samples/edges.png"], tree.path());
    assert!(output.status.success());
    // `edges` has an alpha-anti-aliased rim.
    assert!(String::from_utf8_lossy(&output.stderr).contains("semi-transparent pixels"));
}
