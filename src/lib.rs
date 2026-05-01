mod color;
mod decoder;
mod error;
pub mod frames_store;
mod gif_reader;
mod parser;

pub use error::{GifParsingError, Result};
pub use gif_reader::{GifRead, GifReader, GifReaderStringError};
pub use parser::{parse_header, GifHeader};

#[derive(Clone, Debug)]
pub struct DecodedFrame {
    pub delay_cs: Option<u16>,
    pub pixels_rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct DecodedGif {
    pub width: u16,
    pub height: u16,
    pub loop_count: Option<u16>,
    pub frames: Vec<DecodedFrame>,
}

pub fn decode_gif(rdr: &mut impl GifRead) -> Result<DecodedGif> {
    let header = parser::parse_header(rdr)?;
    parser::decode(rdr, header)
}

pub fn decode_gif_with_header(rdr: &mut impl GifRead, header: GifHeader) -> Result<DecodedGif> {
    parser::decode(rdr, header)
}

pub fn decode_gif_bytes(bytes: &[u8]) -> Result<DecodedGif> {
    let mut reader = GifReader::new(std::io::Cursor::new(bytes));
    decode_gif(&mut reader)
}
