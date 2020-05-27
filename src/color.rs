use crate::error::Result;
use crate::gif_reader::GifRead;

/// Simple structure containing "RGB" (Red Green Blue) colors as defined in a
/// GIF's color table.
#[derive(Debug, Clone, Copy)]
#[repr(packed)] // Should not be needed but still make sure, as this is needed
                // for a parsing optimization.
pub struct RGB {
    r : u8,
    g : u8,
    b : u8,
}

impl Into<u32> for &RGB {
    fn into(self) -> u32 {
        (*self).into()
    }

}

impl Into<u32> for RGB {
    fn into(self) -> u32 {
        // Into a reverse ABGR order as it seems that's what the current openGL
        // logic directly understands.
        // TODO investigate why as it is not immediately straightforward
        0xFF000000
            + ((self.b as u32) << 16)
            + ((self.g as u32) << 8)
            + (self.r as u32)
    }
}

/// Parse color table from a GIF file (the `rdr` should currently be just at the
/// start of the first element in that color table) into the corresponding
/// vector of RGB values.
pub fn parse_color_table(rdr : &mut impl GifRead, nb_entries : usize) -> Result<Vec<RGB>> {
    // Directly transmute the read GIF color table into ours, as they should be
    // in the same format.
    let raw_color_table = rdr.read_bytes(nb_entries * 3)?;
    let (ptr, len, cap) = raw_color_table.into_raw_parts();

    let ct = unsafe {
        let ptr = ptr as *mut RGB;
        Vec::from_raw_parts(ptr, len, cap)
    };
    Ok(ct)

    // Old - safer - implementation:

    // let mut ct : Vec<RGB> = vec![RGB { r: 0, g: 0, b: 0}; nb_entries as usize];
    // for curr_elt_idx in 0..(nb_entries) {
    //     let colors = rdr.read_bytes(3)?;
    //     ct[curr_elt_idx as usize] = RGB {
    //         r: colors[0],
    //         g: colors[1],
    //         b: colors[2],
    //     };
    // }
    // Ok(ct)
}
