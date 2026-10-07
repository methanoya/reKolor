use crate::{Error, Rgba8};

/// A validated, borrowed RGBA8 image: row-major, 4 bytes per pixel, straight alpha.
#[derive(Debug, Clone, Copy)]
pub struct ImageRef<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

impl<'a> ImageRef<'a> {
    /// Checks that both dimensions are non-zero and that `rgba.len() == width × height × 4`,
    /// with checked arithmetic (no overflow on 32-bit WASM either).
    pub fn new(rgba: &'a [u8], width: u32, height: u32) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::EmptyImage { width, height });
        }
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
        let p = &self.rgba[index..index + 4];
        Ok(Rgba8::new(p[0], p[1], p[2], p[3]))
    }
}

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
