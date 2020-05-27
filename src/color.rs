use crate::error::Result;
use crate::gif_reader::GifRead;

#[derive(Debug, Clone, Copy)]
#[repr(packed)] // Should not be needed but still make sure, as this is needed
                // for a parsing optimization.
pub struct RGB {
    pub r : u8,
    pub g : u8,
    pub b : u8,
}

// impl From<u32> for RGB {
//     fn from(val : u32) -> RGB {
//         RGB {
//             r : (val >> 16) as u8,
//             g : (val >> 8) as u8,
//             b : val as u8,
//         }
//     }
// }

// impl Into<u32> for &RGB {
//     fn into(self) -> u32 {
//         ((self.r as u32) << 16) + ((self.g as u32) << 8) + (self.b as u32)
//     }

// }

// impl Into<u32> for RGB {
//     fn into(self) -> u32 {
//         ((self.r as u32) << 16) + ((self.g as u32) << 8) + (self.b as u32)
//     }

// }

// impl Into<[u8; 3]> for RGB {
//     fn into(self) -> [u8; 3] {
//         [self.r, self.g, self.b]
//     }
// }

pub fn parse_color_table(rdr : &mut impl GifRead, nb_entries : usize) -> Result<Vec<RGB>> {
    // Old implementation:

    // let mut ct : Vec<RGB> = vec![RGB { r: 0, g: 0, b: 0}; nb_entries];
    // for curr_elt_idx in 0..(nb_entries) {
    //     let colors = rdr.read_bytes(3)?;
    //     ct[curr_elt_idx as usize] = RGB {
    //         r: colors[0],
    //         g: colors[1],
    //         b: colors[2],
    //     };
    // }
    // Ok(ct)

    // New implementation: Directly transmute the read GIF color table into
    // ours, as they should be in the same format.
    let raw_color_table = rdr.read_bytes(nb_entries * 3)?;
    let (ptr, len, cap) = raw_color_table.into_raw_parts();

    let ct = unsafe {
        let ptr = ptr as *mut RGB;
        Vec::from_raw_parts(ptr, len, cap)
    };
    Ok(ct)
}
