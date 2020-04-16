use crate::gif_reader::GifReader;

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
pub fn parse_color_table(rdr : &mut GifReader, nb_entries : usize) -> Vec<RGB> {
    let ct_size : usize = nb_entries * 3;
    if rdr.bytes_left() < ct_size  {
        eprintln!("Error: Imcomplete color table found.");
        std::process::exit(1);
    }
    let mut ct : Vec<RGB> = vec![RGB { r: 0, g: 0, b: 0}; nb_entries as usize];
    for curr_elt_idx in 0..(nb_entries) {
        ct[curr_elt_idx as usize] = RGB {
            r: rdr.read_u8(),
            g: rdr.read_u8(),
            b: rdr.read_u8(),
        };
    }
    ct
}
