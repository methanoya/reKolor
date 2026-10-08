//! Native file I/O for `rekolor-core`: decodes image files to RGBA8, encodes
//! RGBA8 to PNG, and returns the decoder-discrepancy warnings as data.
//!
//! Native only: never a dependency of `rekolor-wasm`. Formats: every format `image` can read
//! with the workspace feature set (the `image` dependency in the workspace `Cargo.toml`).

use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader};
use rekolor_core::ImageRef;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unrecognized image format")]
    UnknownFormat,
    #[error("can't decode image: {0}")]
    Decode(image::ImageError),
    #[error("can't encode PNG: {0}")]
    Encode(image::ImageError),
    #[error(transparent)]
    InvalidImage(#[from] rekolor_core::Error),
}

/// A decoded image: RGBA8 pixels (straight alpha, row-major) plus the decoder warnings.
///
/// Only [`decode`] and [`decode_file`] create one, so its dimensions always match its buffer and
/// [`DecodedImage::view`] can't fail. The fields are private for that reason:
///
/// ```compile_fail
/// let image = rekolor_io::DecodedImage { width: 0, height: 1, rgba: vec![], warnings: vec![] };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    warnings: Vec<DecodeWarning>,
}

impl DecodedImage {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixels: `width × height × 4` bytes, RGBA8, row-major, straight alpha.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Takes the pixel buffer out.
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }

    /// Things this decoder may treat differently from a browser.
    pub fn warnings(&self) -> &[DecodeWarning] {
        &self.warnings
    }

    /// The pixels as a validated core image.
    pub fn view(&self) -> ImageRef<'_> {
        ImageRef::new(&self.rgba, self.width, self.height)
            .expect("only decode() creates a DecodedImage, and it validates the dimensions")
    }
}

/// Things in a file that this decoder may treat differently from a browser. The pixels are
/// still valid; these explain why they might not match what the app shows for the same file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeWarning {
    /// The file embeds an ICC color profile. Browsers convert to sRGB using it; this decoder
    /// ignores it, so colors may differ.
    IccProfile { bytes: usize },
    /// The file has an EXIF orientation, which was applied (browsers apply it too).
    OrientationApplied { exif_value: u8 },
    /// Pixels with alpha strictly between 0 and 255. A browser canvas stores premultiplied alpha,
    /// so their RGB can differ by about one level at alpha ≥ 128, more below.
    SemiTransparentPixels { count: u64 },
}

impl fmt::Display for DecodeWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IccProfile { bytes } => write!(
                f,
                "embedded ICC color profile ({bytes} bytes) ignored; colors may differ from the browser"
            ),
            Self::OrientationApplied { exif_value } => {
                write!(f, "EXIF orientation {exif_value} applied")
            }
            Self::SemiTransparentPixels { count } => write!(
                f,
                "{count} semi-transparent pixels; their RGB may differ slightly from the browser's"
            ),
        }
    }
}

/// Reads and decodes an image file. The format is detected from the content; when that fails
/// (TGA has no magic bytes), the file extension decides.
pub fn decode_file(path: &Path) -> Result<DecodedImage, Error> {
    let bytes = std::fs::read(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })?;
    decode_with_hint(&bytes, ImageFormat::from_path(path).ok())
}

/// Decodes an image in any readable format to RGBA8: 16-bit and grayscale images are converted,
/// and the EXIF orientation is applied. The format is detected from the content only; formats
/// without a signature (TGA) need [`decode_file`].
pub fn decode(bytes: &[u8]) -> Result<DecodedImage, Error> {
    decode_with_hint(bytes, None)
}

fn decode_with_hint(bytes: &[u8], hint: Option<ImageFormat>) -> Result<DecodedImage, Error> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("reading from memory can't fail");
    if reader.format().is_none() {
        match hint {
            Some(format) => reader.set_format(format),
            None => return Err(Error::UnknownFormat),
        }
    }
    let mut decoder = reader.into_decoder().map_err(Error::Decode)?;
    let icc = decoder.icc_profile().map_err(Error::Decode)?;
    let orientation = decoder.orientation().map_err(Error::Decode)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(Error::Decode)?;
    image.apply_orientation(orientation);
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba = rgba.into_raw();
    // Some decoders accept 0×N images; the core's invariants don't.
    ImageRef::new(&rgba, width, height)?;

    let mut warnings = Vec::new();
    if let Some(profile) = icc.filter(|p| !p.is_empty()) {
        warnings.push(DecodeWarning::IccProfile {
            bytes: profile.len(),
        });
    }
    if orientation != Orientation::NoTransforms {
        warnings.push(DecodeWarning::OrientationApplied {
            exif_value: exif_value(orientation),
        });
    }
    let semi_transparent = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] != 0 && p[3] != 255)
        .count() as u64;
    if semi_transparent > 0 {
        warnings.push(DecodeWarning::SemiTransparentPixels {
            count: semi_transparent,
        });
    }
    for warning in &warnings {
        log::debug!("decode: {warning}");
    }

    Ok(DecodedImage {
        width,
        height,
        rgba,
        warnings,
    })
}

/// Encodes an RGBA8 buffer (`width × height × 4` bytes) as PNG.
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Error> {
    ImageRef::new(rgba, width, height)?;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(Error::Encode)?;
    Ok(png)
}

/// Encodes an RGBA8 buffer as PNG and writes it to `path`.
pub fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), Error> {
    let png = encode_png(rgba, width, height)?;
    std::fs::write(path, png).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

/// The EXIF orientation value (1–8) of an orientation.
fn exif_value(orientation: Orientation) -> u8 {
    match orientation {
        Orientation::NoTransforms => 1,
        Orientation::FlipHorizontal => 2,
        Orientation::Rotate180 => 3,
        Orientation::FlipVertical => 4,
        Orientation::Rotate90FlipH => 5,
        Orientation::Rotate90 => 6,
        Orientation::Rotate270FlipH => 7,
        Orientation::Rotate270 => 8,
    }
}
