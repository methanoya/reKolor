// An `enum` is a type with a fixed set of variants; each variant can carry its own fields.
// `#[derive(...)]` asks the compiler (or a library's macro) to write standard code for the type:
// `Debug` makes it printable with `{:?}`, `Clone` copyable with `.clone()`, `PartialEq`/`Eq`
// comparable with `==`, and `thiserror::Error` turns it into a proper error type whose message is
// the `#[error("...")]` text of the variant (`{width}` inserts that field). Functions that can fail
// return `Result<T, Error>`: either `Ok(value)` or `Err(Error::...)`.
// `#[non_exhaustive]`: code outside this crate must handle unknown variants too (a `_ =>` arm), so
// new variants can be added later without breaking callers.
/// Everything that can go wrong with caller-supplied input. Input never causes a panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    // `u32` is an unsigned 32-bit integer; `usize` (below) is the unsigned integer used for sizes
    // and indexes in memory (32 bits in WebAssembly, 64 bits on most computers).
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
