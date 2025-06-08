use std::{error, fmt};

/// Result type returned by the GIF parsing and decoding logic
pub type Result<T> = ::std::result::Result<T, GifParsingError>;

/// Every possible Error types
#[derive(Debug)]
pub enum GifParsingError {
    /// Error related to standard IO (e.g. file opening)
    IOError(std::io::Error),

    /// No "GIF" string found at the beginning of the GIF. This usually
    /// indicates that we are not parsing a GIF content.
    NoGIFHeader,

    /// The version in that GIF content is unknown of.
    UnsupportedVersion(Option<String>),

    /// A given "block" in the GIF content was not of the right size
    UnexpectedLength {
        block_name: String,
        expected: u8,
        got: u8,
    },

    /// The parser expected a "block terminator" but got another thing instead.
    ExpectedBlockTerminator { block_name: Option<String> },

    /// A color encountered while decoding is unknown of
    InvalidColor,

    /// There's too much color data in the GIF content
    TooMuchPixels,

    /// No color table was found at a given point.
    /// The specification actually allows that, at which point the GIF decoding
    /// software should either guess one or in the better case refer to the
    /// previously-encountered one (from a previous GIF content).
    /// In absolute, this is never encountered, so I did not bother for now.
    /// TODO?
    NoColorTable,

    /// An unknown type of "extension block" was encountered.
    /// As we don't know anything about the size of the data it brings with it,
    /// we prefer aborting there.
    UnrecognizedExtension(u8),

    /// An unknown type of block was encountered.
    /// As we don't know anything about the size of the data it brings with it,
    /// we prefer aborting there.
    UnrecognizedBlock { code: u8, position: usize },
}

impl From<std::io::Error> for GifParsingError {
    fn from(err: std::io::Error) -> GifParsingError {
        GifParsingError::IOError(err)
    }
}

impl error::Error for GifParsingError {
    fn cause(&self) -> Option<&dyn ::std::error::Error> {
        match *self {
            GifParsingError::IOError(ref e) => Some(e),
            GifParsingError::NoGIFHeader => None,
            GifParsingError::UnsupportedVersion(_) => None,
            GifParsingError::UnexpectedLength { .. } => None,
            GifParsingError::ExpectedBlockTerminator { .. } => None,
            GifParsingError::InvalidColor => None,
            GifParsingError::TooMuchPixels => None,
            GifParsingError::NoColorTable => None,
            GifParsingError::UnrecognizedExtension(_) => None,
            GifParsingError::UnrecognizedBlock { .. } => None,
        }
    }
}

impl fmt::Display for GifParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GifParsingError::IOError(x) => x.fmt(f),

            GifParsingError::NoGIFHeader => write!(
                f,
                "No \"GIF\" header found. Are you sure this is a GIF file?"
            ),

            GifParsingError::UnsupportedVersion(version) => match version {
                Some(version_number) => write!(f, "Version not recognized: {}", version_number),
                None => write!(f, "Cannot read the current version."),
            },

            GifParsingError::UnexpectedLength {
                block_name,
                expected,
                got,
            } => write!(
                f,
                "Unexpected block length for the \"{}\" block.\n\
                    Expected {}, got {}.",
                block_name, expected, got
            ),

            GifParsingError::ExpectedBlockTerminator { block_name } => match block_name {
                Some(name) => write!(
                    f,
                    "Expected a block terminator at the end of the \"{}\" \
                          block.",
                    name
                ),
                None => write!(f, "Expected a block terminator."),
            },

            GifParsingError::InvalidColor => write!(f, "Unknown color encountered."),

            GifParsingError::TooMuchPixels => write!(f, "Too much color data was found."),

            GifParsingError::NoColorTable => {
                write!(f, "No color table found for the current frame.")
            }

            GifParsingError::UnrecognizedExtension(c) => {
                write!(f, "Unrecognized Extension block with code {}", c)
            }

            GifParsingError::UnrecognizedBlock { code, position } => write!(
                f,
                "Unrecognized block with code {} at position {}.",
                code, position
            ),
        }
    }
}
