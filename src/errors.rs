#[derive(Debug)]
pub enum GifParsingError {
    IOError(std::io::Error),
    NoGIFHeader,
    UnsupportedVersion(Option<String>),
    UnexpectedLength {
        block_name: String,
        expected: u8,
        got: u8,
    },
    ExpectedBlockTerminator {
        block_name : Option<String>
    },
    InvalidColor,
    TooMuchPixels,
    NoColorTable,
    UnrecognizedExtension(u8),
    UnrecognizedBlock {
        code : u8,
        position : usize,
    }
}

impl From<std::io::Error> for GifParsingError {
    fn from(err : std::io::Error) -> GifParsingError {
        GifParsingError::IOError(err)
    }
}

use std::{error, fmt};
impl error::Error for GifParsingError {}
impl fmt::Display for GifParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {

            GifParsingError::IOError(x) => x.fmt(f),

            GifParsingError::NoGIFHeader => write!(f,
                "No \"GIF\" header found. Are you sure this is a GIF file?"),

            GifParsingError::UnsupportedVersion(version) => match version {
                Some(version_number) =>
                    write!(f, "Version not recognized: {}", version_number),
                None => write!(f, "Cannot read the current version."),
            }

            GifParsingError::UnexpectedLength { block_name, expected, got } =>
                write!(f, "Unexpected block length for the \"{}\" block.\n\
                    Expected {}, got {}.", block_name, expected, got),

            GifParsingError::ExpectedBlockTerminator { block_name } =>
                match block_name {
                    Some(name) =>
                        write!(f, "Expected a block terminator at the end of the \"{}\" \
                          block.", name),
                    None => write!(f, "Expected a block terminator.")
                },

            GifParsingError::InvalidColor => write!(f, "Unknown color encountered."),

            GifParsingError::TooMuchPixels => write!(f, "Too much color data was found."),

            GifParsingError::NoColorTable => write!(f, "No color table found for the current frame."),

            GifParsingError::UnrecognizedExtension(c) =>
                write!(f, "Unrecognized Extension block with code {}", c),

            GifParsingError::UnrecognizedBlock { code, position } =>
                write!(f, "Unrecognized block with code {} at position {}.", code, position),
        }
    }
}
