use crate::error::Result;
use crate::gif_reader::GifRead;

/// Simple structure containing "RGB" (Red Green Blue) colors as defined in a
/// GIF's color table.
#[derive(Debug, Clone, Copy)]
pub struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl From<&Rgb> for u32 {
    fn from(val: &Rgb) -> u32 {
        (*val).into()
    }
}

impl From<Rgb> for u32 {
    fn from(val: Rgb) -> Self {
        // Into a reverse ABGR order as it seems that's what the current openGL

        0xFF000000 + ((val.b as u32) << 16) + ((val.g as u32) << 8) + (val.r as u32)
    }
}

/// Parse color table from a GIF file (the `rdr` should currently be just at the
/// start of the first element in that color table) into the corresponding
/// vector of RGB values.
pub fn parse_color_table(rdr: &mut impl GifRead, nb_entries: usize) -> Result<Vec<Rgb>> {
    // NOTE: Old implem, I prefer relying on safe rust now
    //
    // // Directly transmute the read GIF color table into ours, as they should be
    // // in the same format.
    // let raw_color_table = rdr.read_bytes(nb_entries * 3)?;
    // let (ptr, len, cap) = raw_color_table.into_raw_parts();
    //
    // let ct = unsafe {
    //     let ptr = ptr as *mut Rgb;
    //     Vec::from_raw_parts(ptr, len, cap)
    // };
    // Ok(ct)
    let mut ct: Vec<Rgb> = vec![Rgb { r: 0, g: 0, b: 0 }; nb_entries];
    for item in ct.iter_mut() {
        let colors = rdr.read_bytes(3)?;
        *item = Rgb {
            r: colors[0],
            g: colors[1],
            b: colors[2],
        };
    }
    Ok(ct)
}
