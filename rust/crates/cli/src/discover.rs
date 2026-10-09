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
/// Every image file under `dir` (recursively), sorted, skipping golden outputs
/// (`<name>-out-<size>.png`), configs and other non-image files. Symbolic links to folders are not
/// followed, so a link can't make the walk loop or leave `dir`; a link to a file counts like the
/// file.
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
    is_image && !is_golden_output(path)
}

/// Whether a file is named like a golden output, `<name>-out-<size>.png`
/// ([`config::output_path`](crate::config::output_path)). Other names containing `-out-`
/// (`black-out-design.png`) are inputs.
fn is_golden_output(path: &Path) -> bool {
    let png = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    // `rsplit_once` splits at the last `-out-`; what follows it must be a whole number.
    let size = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.rsplit_once("-out-"))
        .map(|(_, size)| size);
    png && size.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

// Recursive: calls itself for each subfolder. `&mut Vec` lets it add to the caller's list.
// `Result<()>` means "nothing to return, but it can fail".
fn walk(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        // `file_type` describes the entry itself, without following a symbolic link, so a linked
        // folder is never entered. `path.is_dir()` does follow links: it skips a linked folder that
        // happens to have an image-like name.
        if entry.file_type()?.is_dir() {
            walk(&path, found)?;
        } else if !path.is_dir() && is_input_image(&path) {
            found.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The images found under `root`, as paths relative to it.
    fn found(root: &Path) -> Vec<String> {
        images(root)
            .unwrap()
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
            .collect()
    }

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
            "a-out-16.PNG",
            "a.palettes.toml",
            "README.md",
        ] {
            std::fs::write(root.path().join(name), b"").unwrap();
        }
        std::fs::write(sub.join("d.tiff"), b"").unwrap();
        assert_eq!(
            found(root.path()),
            ["a.png", "b.JPG", "c.webp", "sub/d.tiff"]
        );
    }

    #[test]
    fn only_golden_output_names_are_skipped() {
        let root = tempfile::tempdir().unwrap();
        for name in [
            "black-out-design.png",
            "fade-out-.png",
            "fade-out-3x.png",
            "photo-out-3.jpg",
            "x-out-7.png",
        ] {
            std::fs::write(root.path().join(name), b"").unwrap();
        }
        assert_eq!(
            found(root.path()),
            [
                "black-out-design.png",
                "fade-out-.png",
                "fade-out-3x.png",
                "photo-out-3.jpg"
            ]
        );
    }

    // `cfg(unix)`: creating symbolic links works differently on Windows.
    #[cfg(unix)]
    #[test]
    fn linked_folders_are_not_followed() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.png"), b"").unwrap();
        std::fs::write(outside.path().join("b.png"), b"").unwrap();
        // A loop back to the root, and a way out of the tree: neither is entered.
        symlink(root.path(), root.path().join("loop")).unwrap();
        symlink(outside.path(), root.path().join("outside")).unwrap();
        // A link to a file counts like the file.
        symlink(outside.path().join("b.png"), root.path().join("c.png")).unwrap();
        assert_eq!(found(root.path()), ["a.png", "c.png"]);
    }
}
