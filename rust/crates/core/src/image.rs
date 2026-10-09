// `crate::` refers to this crate's own top level (the items re-exported in `lib.rs`).
use crate::{Error, Rgba8};

// `Copy`: the struct is small (two numbers and a reference), so assigning or passing it copies it
// instead of moving it; the caller can keep using its own copy.
// `<'a>` is a lifetime: a name for "as long as the borrowed bytes exist". The compiler then
// rejects any use of an `ImageRef` after its buffer is gone, so it can never point at freed memory.
// The fields have no `pub`, so other modules can only build one through `new`, which validates it.
/// A validated, borrowed RGBA8 image: row-major, 4 bytes per pixel, straight alpha.
///
/// "Borrowed" means it doesn't own the bytes: it only points at a buffer someone else owns, so no
/// pixels are copied. "Straight alpha" means the color channels are stored as they are, not
/// pre-multiplied by the alpha (opacity) channel.
#[derive(Debug, Clone, Copy)]
pub struct ImageRef<'a> {
    width: u32,
    height: u32,
    // `&'a [u8]`: a borrowed slice (a view into a list) of bytes, valid for the lifetime `'a`.
    rgba: &'a [u8],
}

// `impl` adds methods to a type. `&self` methods only read the value; `Self` means `ImageRef`.
impl<'a> ImageRef<'a> {
    /// Checks that both dimensions are non-zero and that `rgba.len() == width × height × 4`,
    /// with checked arithmetic (no overflow on 32-bit WASM either).
    pub fn new(rgba: &'a [u8], width: u32, height: u32) -> Result<Self, Error> {
        // `return Err(...)` ends the function with an error; the `?` operator below does the same
        // for an error produced by the expression in front of it.
        if width == 0 || height == 0 {
            return Err(Error::EmptyImage { width, height });
        }
        // width × height × 4 without risking a silent overflow: each step yields `None` if it
        // wouldn't fit (`try_from`, `checked_mul`), and `ok_or(...)?` turns that `None` into the
        // `TooLarge` error.
        let expected = usize::try_from(width)
            .ok()
            .zip(usize::try_from(height).ok())
            .and_then(|(w, h)| w.checked_mul(h))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(Error::TooLarge { width, height })?;
        if rgba.len() != expected {
            return Err(Error::BufferLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        self.rgba
    }

    // `impl Iterator<Item = Rgba8>`: returns "some iterator of pixels" without naming its exact
    // type; callers loop over it with `for` or chain methods like `.map(...)`. Iterators are lazy:
    // nothing is computed until someone asks for the next item.
    /// The pixels in row-major order.
    pub fn pixels(&self) -> impl Iterator<Item = Rgba8> + 'a {
        // `new` guarantees the length is a multiple of 4, so nothing is left over.
        self.rgba.as_chunks::<4>().0.iter().map(|&p| Rgba8::from(p))
    }

    /// The pixel at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> Result<Rgba8, Error> {
        if x >= self.width || y >= self.height {
            return Err(Error::OutOfBounds {
                x,
                y,
                width: self.width,
                height: self.height,
            });
        }
        // In bounds, so the index fits (the buffer of this size exists).
        let index = (y as usize * self.width as usize + x as usize) * 4;
        // `[index..index + 4]` takes a sub-slice of 4 bytes. Indexing outside a slice would panic
        // (crash), but the bounds check above makes that impossible here.
        let p = &self.rgba[index..index + 4];
        Ok(Rgba8::new(p[0], p[1], p[2], p[3]))
    }
}

// Unit tests, compiled only for `cargo test` (`#[cfg(test)]`). `use super::*` imports everything
// from the module above. Each `#[test]` function is one test; `assert_eq!` fails the test if its
// two values differ, and `.unwrap()` fails it if a `Result` is an error.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_length() {
        let buf = [0u8; 2 * 3 * 4];
        let img = ImageRef::new(&buf, 2, 3).unwrap();
        assert_eq!((img.width(), img.height(), img.pixels().count()), (2, 3, 6));
    }

    #[test]
    fn rejects_zero_dimensions() {
        assert_eq!(
            ImageRef::new(&[], 0, 5).unwrap_err(),
            Error::EmptyImage {
                width: 0,
                height: 5
            }
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            ImageRef::new(&[0; 7], 1, 2).unwrap_err(),
            Error::BufferLength {
                expected: 8,
                actual: 7
            }
        );
    }

    #[test]
    fn rejects_huge_dimensions_without_overflow() {
        // Overflows usize on 32-bit targets; on 64-bit it can't match an empty buffer.
        assert!(ImageRef::new(&[], u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn reads_pixels_and_checks_bounds() {
        let buf = [1, 2, 3, 4, 5, 6, 7, 8];
        let img = ImageRef::new(&buf, 2, 1).unwrap();
        assert_eq!(img.pixel(1, 0).unwrap(), Rgba8::new(5, 6, 7, 8));
        assert_eq!(
            img.pixel(2, 0).unwrap_err(),
            Error::OutOfBounds {
                x: 2,
                y: 0,
                width: 2,
                height: 1
            }
        );
        assert!(img.pixel(0, 1).is_err());
    }
}
