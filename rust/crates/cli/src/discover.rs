//! Finding the input images in a samples tree.

use std::path::{Path, PathBuf};

// `anyhow::Result<T>` is `Result<T, anyhow::Error>`: any error type fits, which suits a program
// that only reports errors. `Context` adds `.with_context(...)` to attach a message.
use anyhow::{Context, Result};

// `&[&str]`: a borrowed list of string literals, fixed at compile time.
/// File extensions of the formats `rekolor-io` can read.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "tif", "tiff", "webp", "tga", "qoi", "exr", "hdr",
    "pbm", "pgm", "ppm", "pnm", "pam", "dds", "ff", "farbfeld",
];

// `Path` is a borrowed file-system path, `PathBuf` an owned one (like `&str` and `String`).
// Sorting makes the order the same on every machine (directory listings come in any order).
/// Every image file under `dir` (recursively), sorted, skipping golden outputs (`*-out-*`),
/// configs and other non-image files.
pub fn images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    walk(dir, &mut found)?;
    found.sort();
    Ok(found)
}

// An image this tool reads, but not one of its own outputs (`name-out-3.png`).
// `is_some_and(...)` is true only if there is a value and the check passes for it.
pub fn is_input_image(path: &Path) -> bool {
    let is_image = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
    let is_output = path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().contains("-out-"));
    is_image && !is_output
}

// Recursive: calls itself for each subfolder. `&mut Vec` lets it add to the caller's list.
// `Result<()>` means "nothing to return, but it can fail".
fn walk(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, found)?;
        } else if is_input_image(&path) {
            found.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_images_recursively_and_skips_outputs_and_other_files() {
        let root = tempfile::tempdir().unwrap();
        let sub = root.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        for name in [
            "a.png",
            "b.JPG",
            "c.webp",
            "a-out-3.png",
            "a.palettes.toml",
            "README.md",
        ] {
            std::fs::write(root.path().join(name), b"").unwrap();
        }
        std::fs::write(sub.join("d.tiff"), b"").unwrap();
        let names: Vec<_> = images(root.path())
            .unwrap()
            .iter()
            .map(|p| {
                p.strip_prefix(root.path())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["a.png", "b.JPG", "c.webp", "sub/d.tiff"]);
    }
}
