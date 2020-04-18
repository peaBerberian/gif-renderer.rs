use crate::error::Result;
use crate::gif_reader::GifRead;

#[derive(Debug, Clone, Copy)]
pub struct RGB {
    r : u8,
    g : u8,
    b : u8,
}

impl From<u32> for RGB {
    fn from(val : u32) -> RGB {
        RGB {
            r : (val >> 16) as u8,
            g : (val >> 8) as u8,
            b : val as u8,
        }
    }
}

impl Into<u32> for &RGB {
    fn into(self) -> u32 {
        ((self.r as u32) << 16) + ((self.g as u32) << 8) + (self.b as u32)
    }

}

impl Into<u32> for RGB {
    fn into(self) -> u32 {
        ((self.r as u32) << 16) + ((self.g as u32) << 8) + (self.b as u32)
    }

}

// TODO use C repr to parse it more rapidly?
pub fn parse_color_table(rdr : &mut impl GifRead, nb_entries : usize) -> Result<Vec<RGB>> {
    let mut ct : Vec<RGB> = vec![RGB { r: 0, g: 0, b: 0}; nb_entries as usize];
    for curr_elt_idx in 0..(nb_entries) {
        let colors = rdr.read_bytes(3)?;
        ct[curr_elt_idx as usize] = RGB {
            r: colors[0],
            g: colors[1],
            b: colors[2],
        };
    }
    Ok(ct)
}
