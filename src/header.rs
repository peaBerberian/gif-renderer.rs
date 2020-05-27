use crate::color::{self, RGB};
use crate::error::{GifParsingError, Result};
use crate::gif_reader::{GifRead, GifReaderStringError};

#[derive(Debug)]
pub struct GifHeader {
    pub width : u16,
    pub height : u16,
    pub nb_color_resolution_bits : u8,
    pub is_table_sorted : bool,
    pub background_color_index : u8,
    pub pixel_aspect_ratio : u8,
    pub global_color_table : Option<Vec<RGB>>,
}

/// Parse Header part of a GIF buffer and the Global Color Table, if one.
pub fn parse_header(rdr : &mut impl GifRead) -> Result<GifHeader> {
    match rdr.read_str(3) {
        Err(GifReaderStringError::FromUtf8Error(_)) => {
            return Err(GifParsingError::NoGIFHeader);
        },
        Ok(x) if x != "GIF" => {
            return Err(GifParsingError::NoGIFHeader);
        },
        Err(GifReaderStringError::IOError(x)) => {
            return Err(GifParsingError::IOError(x));
        }
        Ok(_) => {}
    };

    match rdr.read_str(3) {
        Err(GifReaderStringError::FromUtf8Error(_)) => {
            return Err(GifParsingError::UnsupportedVersion(None));
        },
        Ok(v) if v != "89a" && v != "87a" => {
            return Err(GifParsingError::UnsupportedVersion(Some(v)));
        },
        Err(GifReaderStringError::IOError(x)) => {
            return Err(GifParsingError::IOError(x));
        }
        Ok(_) => {}
    };

    let width = rdr.read_u16()?;
    let height = rdr.read_u16()?;

    let field = rdr.read_u8()?;
    let has_global_color_table = field & 0x80 != 0;
    let nb_color_resolution_bits = ((field & 0x70) >> 4) + 1;
    let is_table_sorted = field & 0x08 != 0;
    let nb_entries : usize = 1 << ((field & 0x07) + 1);

    let background_color_index = rdr.read_u8()?;
    let pixel_aspect_ratio = rdr.read_u8()?;

    let global_color_table = if has_global_color_table {
        Some(color::parse_color_table(rdr, nb_entries)?)
    } else {
        None
    };

    Ok(GifHeader {
        width,
        height,
        nb_color_resolution_bits,
        is_table_sorted,
        background_color_index,
        pixel_aspect_ratio,
        global_color_table,
    })
}
