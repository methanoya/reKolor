/// Everything that can go wrong with caller-supplied input. Input never causes a panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("image has no pixels ({width}×{height})")]
    EmptyImage { width: u32, height: u32 },

    #[error("image dimensions {width}×{height} are too large to address")]
    TooLarge { width: u32, height: u32 },

    #[error("pixel buffer has {actual} bytes, expected {expected} (width × height × 4)")]
    BufferLength { expected: usize, actual: usize },

    #[error("output buffer has {actual} bytes, expected {expected}")]
    OutputLength { expected: usize, actual: usize },

    #[error("pixel ({x}, {y}) is outside the {width}×{height} image")]
    OutOfBounds {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },

    #[error("palette has no entries")]
    EmptyPalette,
}
