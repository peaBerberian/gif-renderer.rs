mod decoder;
mod gif_reader;
mod render;

use gif_reader::GifReader;
use decoder::LzwDecoder;

/// Minimum size a GIF buffer should have to be valid.
const HEADER_SIZE : usize = 13;

/// GIF block ID for the "Image Descriptor".
const IMAGE_DESCRIPTOR_BLOCK_ID : u8 = 0x2C;

/// GIF block ID for the "Trailer".
const TRAILER_BLOCK_ID : u8 = 0x3B;

/// GIF block ID for the "Extension Introducer".
const EXTENSION_INTRODUCER_ID : u8 = 0x21;

/// GIF block ID for the "Graphic Control Extension".
const GRAPHIC_CONTROL_EXTENSION_LABEL : u8 = 0xF9;

/// GIF block ID for an "Application Extension".
const APPLICATION_EXTENSION_LABEL : u8 = 0xFF;

/// GIF block ID for a "Comment Extension".
const COMMENT_EXTENSION_LABEL : u8 = 0xFE;

/// GIF block ID for a "Plain Text Extension".
const PLAIN_TEXT_EXTENSION_LABEL : u8 = 0x01;

/// Background color used when none is defined.
const DEFAULT_BACKGROUND_COLOR : u32 = 0xFFFFFF;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        panic!("Missing file argument");
    }
    let file_data = std::fs::read(&args[1]).unwrap();
    if file_data.len() < HEADER_SIZE {
        panic!("Invalid GIF file: too short");
    }

    let mut rdr = GifReader::new(file_data);

    let header = parse_header(&mut rdr);

    let (background_color, global_color_table) =
        if let Some(gct) = &header.global_color_table {
            let index = header.background_color_index as usize;
            if gct.len() <= index {
                panic!("Invalid GIF File: Invalid background color index: {}", index);
            }
            (Some(&gct[index]), Some(gct.as_slice()))
        } else {
            (None, None)
        };

    // Last graphic extension encountered. Will be needed when an Image Descriptor
    // is encountered.
    let mut last_graphic_ext : Option<GraphicControlExtension> = None;

    // Background for the next frame encountered. Its content depends on the
    // "disposal method" of the next frame encountered.
    let mut next_frame_base_buffer : Option<Vec<u32>> = None;

    // Number of time the GIF should be looped on according to the NETSCAPE2.0
    // Application extension. A value of `Some(0)` indicates that it should be looped
    // forever.
    let mut nb_loop : Option<u16> = None;

    // Store every frames and the corresponding delays to the next frame, if one.
    let mut frames : Vec<(Vec<u32>, Option<u16>)> = vec![];

    while rdr.bytes_left() > 0 {
        match rdr.read_u8() {
            IMAGE_DESCRIPTOR_BLOCK_ID => {
                let (delay, transparent_color_index) = match &last_graphic_ext {
                    Some(e) => (Some(e.delay), e.transparent_color_index),
                    None => (None, None)
                };

                // The "RestoreToPrevious" disposal method force us to keep the current base
                // buffer for the frame coming after that one.
                use DisposalMethod::{*};
                let cloned_image_background = match last_graphic_ext {
                    Some(GraphicControlExtension { disposal_method: RestoreToPrevious, .. }) =>
                        next_frame_base_buffer.clone(),
                    _ => None
                };

                let block = construct_next_frame(&mut rdr,
                                                 next_frame_base_buffer,
                                                 header.height,
                                                 header.width,
                                                 &background_color,
                                                 &global_color_table,
                                                 &transparent_color_index);

                // Obtain the base buffer for the next frame according to the current disposal
                // method
                next_frame_base_buffer = match last_graphic_ext {
                    Some(GraphicControlExtension { disposal_method: DoNotDispose, ..  }) => {
                        Some(block.clone())
                    },
                    Some(GraphicControlExtension { disposal_method: RestoreToBackgroundColor, ..  }) =>
                        None,
                    _ => cloned_image_background,
                };
                frames.push((block, delay));
            }
            TRAILER_BLOCK_ID => {
                render::render_image(&frames, nb_loop, header.width as usize, header.height as usize);
            }
            EXTENSION_INTRODUCER_ID => {
                match rdr.read_u8() {
                    GRAPHIC_CONTROL_EXTENSION_LABEL => {
                        last_graphic_ext = Some(parse_graphic_control_extension(&mut rdr));
                    }
                    APPLICATION_EXTENSION_LABEL => {
                        // Only NETSCAPE2.0 is parsed for now as looping is an essential feature
                        // (And I just don't want to set it to infinite by default)
                        nb_loop = match parse_application_extension(&mut rdr).extension {
                            ApplicationExtensionValue::NetscapeLooping(x) => Some(x),
                            ApplicationExtensionValue::NotKnown => nb_loop,
                        };
                    }
                    COMMENT_EXTENSION_LABEL => {
                        // We don't care about comments
                        skip_sub_blocks(&mut rdr);
                        if rdr.read_u8() != 0x00 /* block terminator */ {
                            panic!("Invalid GIF File: A cooment extension does not \
                               terminate with a block terminator");
                        }
                    }
                    PLAIN_TEXT_EXTENSION_LABEL => {
                        skip_plain_text_extension(&mut rdr);
                    }
                    _ => {
                        panic!("Invalid GIF File: unknown extension");
                    }
                }
            }
            x => { panic!("Unrecognized code {} at line {}", x, rdr.get_pos()); }
        }
    }
}

enum ApplicationExtensionValue {
    /// Looping value from the NETSCAPE2.0 extension.
    /// 0 means infinite looping, any other value would be the number of time
    /// the GIF image needs to be looped (played back from the beginning).
    NetscapeLooping(u16),
    NotKnown,
}

struct ApplicationExtension {
    _app_name : String,
    _app_auth_code : (u8, u8, u8),
    extension : ApplicationExtensionValue,
}

/// Allows to skip sub-blocks when reached. You might want to do that when
/// reaching a part of the GIF buffer containing sub-blocks you don't care for
/// (e.g. comments).
fn skip_sub_blocks(rdr : &mut GifReader) {
    if rdr.bytes_left() == 0 {
        panic!("Invalid GIF File: Invalid sub-block data");
    }
    loop {
        let size_of_block = rdr.read_u8() as usize;
        if size_of_block == 0 {
            return;
        }
        if rdr.bytes_left() <= size_of_block {
            panic!("Invalid GIF File: Invalid sub-block data");
        }
        rdr.skip_bytes(size_of_block);
    }
}

/// The plain text extention is a 89a GIF extension allowing to render text in a
/// GIF image. This feature seems to be very rarely used, we can safely ignore
/// it for now.
/// TODO?
fn skip_plain_text_extension(rdr : &mut GifReader) {
    if rdr.read_u8() != 12 || rdr.bytes_left() <= 12 {
        panic!("Invalid GIF File: Plain text extension should have a length of 12.");
    }
    rdr.skip_bytes(12); // Skip whole plain text header
    skip_sub_blocks(rdr);
}

fn parse_application_extension(rdr : &mut GifReader) -> ApplicationExtension {
    let size_until_app_data = rdr.read_u8() as usize;

    if size_until_app_data != 11 || rdr.bytes_left() <= 11 {
        panic!("Invalid GIF File: Application Extension has an invalid length");
    }
    let _app_name = match rdr.read_str(8) {
        Err(e) => panic!("Invalid GIF file:
            Impossible to read the application name: {}", e),
        Ok(x) => x
    };
    let _app_auth_code = (rdr.read_u8(), rdr.read_u8(), rdr.read_u8());

    let mut data_len = rdr.read_u8() as usize;
    if data_len == 0 {
        return ApplicationExtension {
            _app_name,
            _app_auth_code,
            extension: ApplicationExtensionValue::NotKnown,
        };
    }
    if rdr.bytes_left() <= data_len {
        panic!("Invalid GIF File: Application Extension truncated");
    }

    let mut ext : ApplicationExtensionValue = ApplicationExtensionValue::NotKnown;

    if _app_name == "NETSCAPE" &&
       _app_auth_code == (50, 46, 48)
    {
        let mut cur_offset = 0;
        let sub_block_id = rdr.read_u8();
        if data_len != 0x03 || sub_block_id != 0x01 {
            cur_offset += 1;
        } else {
            let loop_count = rdr.read_u16();
            ext = ApplicationExtensionValue::NetscapeLooping(loop_count);
            cur_offset += 3;
        }
        if data_len < cur_offset {
            panic!("Invalid GIF File: Application Extension truncated");
        }
        data_len -= cur_offset;
    }

    // Skip all remaining data blocks
    loop {
        let bytes_left = rdr.bytes_left();
        if bytes_left < data_len {
            panic!("Invalid GIF File: Application Extension truncated 1");
        }
        if bytes_left == 0 || data_len == 0 {
            break;
        }
        rdr.skip_bytes(data_len - 1);
        data_len = rdr.read_u8() as usize;
    }
    if rdr.bytes_left() == 0 || rdr.read_u8() != 0x00 /* block terminator */ {
        panic!("Invalid GIF File: Application Extension truncated");
    }

    ApplicationExtension {
        _app_name,
        _app_auth_code,
        extension: ext,
    }
}

/// The available value for the `disposal_method` parsed from a graphic control
/// extension.
#[derive(Debug)]
enum DisposalMethod {
    /// The decoder is not required to take any action.
    NoDisposalSpecified,

    /// The graphic is to be left in place.
    DoNotDispose,

    /// The area used by the graphic must be restored to the background color.
    RestoreToBackgroundColor,

    /// The decoder is required to restore the area overwritten by the graphic
    /// with what was there prior to rendering the graphic.
    RestoreToPrevious,
}

/// Value of a parsed Graphic Control Extension from a GIF buffer
#[derive(Debug)]
struct GraphicControlExtension {
    /// Indicates the way in which the graphic is to be treated after being
    /// displayed.
    disposal_method: DisposalMethod,

    /// If set to `true`, processing will continue when user input is entered.
    /// The nature of the User input is determined by the application (Carriage
    /// Return, Mouse Button Click, etc.).
    /// When a Delay Time is used and the User Input Flag is set, processing
    /// will continue when user input is received or when the delay time
    /// expires, whichever occurs first.
    user_input : bool,

    /// The Transparency Index is such that when encountered, the corresponding
    /// pixel of the display device is not modified and processing goes on to
    /// the next pixel. The index is present if and only if the Transparency
    /// Flag is set to 1.
    transparent_color_index : Option<u8>,

    /// If not 0, this field specifies the number of hundredths (1/100) of a
    /// second to wait before continuing with the processing of the Data Stream.
    /// The clock starts ticking immediately after the graphic is rendered. This
    /// field may be used in conjunction with the User Input Flag field.
    delay : u16,
}

fn parse_graphic_control_extension(rdr : &mut GifReader) -> GraphicControlExtension {
    let block_size = rdr.read_u8() as usize;

    if rdr.bytes_left() <= block_size || block_size != 4 {
        panic!("Invalid GIF File: Invalid Graphic Control Extension Block");
    }
    let packed_fields = rdr.read_u8();
    let disposal_method = match (packed_fields & 0b00011100) >> 2 {
        1 => DisposalMethod::DoNotDispose,
        2 => DisposalMethod::RestoreToBackgroundColor,
        3 => DisposalMethod::RestoreToPrevious,
        _ => DisposalMethod::NoDisposalSpecified,
    };
    let user_input : bool = packed_fields & 0x02 != 0;
    let transparent_color_flag : bool = packed_fields & 0x01 != 0;
    let delay = rdr.read_u16();
    let transparent_color_index = match transparent_color_flag {
        true => Some(rdr.read_u8()),
        false => {
            rdr.skip_bytes(1);
            None
        }
    };
    if rdr.bytes_left() == 0 || rdr.read_u8() != 0 {
        panic!("Invalid GIF File: Graphic Control Extension truncated");
    }
    GraphicControlExtension {
        disposal_method,
        user_input,
        transparent_color_index,
        delay,
    }
}

fn construct_next_frame(
    rdr : &mut GifReader,
    base_buffer : Option<Vec<u32>>,
    img_height : u16,
    img_width : u16,
    background_color : &Option<&RGB>,
    global_color_table : &Option<&[RGB]>,
    transparent_color_index : &Option<u8>
) -> Vec<u32> {
    let curr_block_left = rdr.read_u16();
    let curr_block_top = rdr.read_u16();
    let curr_block_width = rdr.read_u16();
    let curr_block_height = rdr.read_u16();
    let field = rdr.read_u8();

    let has_local_color_table = field & 0x80 != 0;

    let has_interlacing = field & 0x40 != 0;
    let _is_sorted = field & 0x20 != 0;
    let _reserved_1 = field & 0x10;
    let _reserved_2 = field & 0x08;
    let nb_color_entries : usize = 1 << ((field & 0x07) + 1);

    // Current interlacing cycle - from 0 to 3 - and factor used to obtain the next line
    // we should draw. Both are only needed when interlacing is enabled.
    let (mut interlacing_cycle, mut line_factor) = if has_interlacing {
        (0, 8)
    } else {
        (0, 1)
    };

    let lct = if has_local_color_table {
        Some(parse_color_table(rdr, nb_color_entries))
    } else { None };

    let current_color_table : &[RGB] = if let Some(c) = &lct {
        c
    } else {
        match global_color_table {
            &None => { panic!("Invalid GIF File: no color table found."); }
            &Some(val) => val
        }
    };

    let initial_code_size = rdr.read_u8();
    let mut decoder = LzwDecoder::new(initial_code_size);

    if curr_block_width == 0 || curr_block_height == 0 {
        let bg_color : u32 = match background_color {
            Some(color) => (*color).into(),
            None => DEFAULT_BACKGROUND_COLOR,
        };
        return vec![bg_color; img_height as usize * img_width as usize];
    }

    let (has_background_frame, mut global_buffer) = match base_buffer {
        Some(frame) => (true, frame),
        None => (false, vec![0; img_height as usize * img_width as usize]),
    };

    let mut x_pos : usize = curr_block_left as usize;
    let mut y_pos : usize = curr_block_top as usize;
    let max_pos_width = curr_block_width as usize + curr_block_left as usize - 1;
    let max_pos_height = curr_block_height as usize + curr_block_top as usize - 1;
    loop {
        if rdr.bytes_left() <= 0 {
            panic!("Invalid GIF File: Image Descriptor Truncated");
        }

        let sub_block_size = rdr.read_u8() as usize;
        if sub_block_size == 0x00 /* block terminator */ {
            return global_buffer;
        } else {
            if rdr.bytes_left() <= sub_block_size {
                panic!("Invalid GIF File: Image Descriptor Truncated");
            }
            let sub_block_data = rdr.read_slice(sub_block_size);
            let decoded_data = decoder.decode_next(&sub_block_data);
            for elt in decoded_data {
                if elt as usize >= current_color_table.len() {
                    panic!("Invalid GIF File: ");
                }
                let pos = (y_pos * img_width as usize) + x_pos;
                if pos >= global_buffer.len() {
                    panic!("Invalid GIF File: too much data");
                }
                let pix_val : u32 = match transparent_color_index {
                    Some(t_idx) if *t_idx == elt => {
                        if has_background_frame {
                            global_buffer[pos] // do not change anything
                        } else {
                            match background_color {
                                Some(c) => (*c).into(),
                                None => DEFAULT_BACKGROUND_COLOR,
                            }
                        }
                    },
                    _ => (&current_color_table[elt as usize]).into(),
                };
                global_buffer[pos] = pix_val;
                x_pos += 1;
                if x_pos > max_pos_width {
                    y_pos += 1 * line_factor;
                    if y_pos > max_pos_height {
                        if !has_interlacing || interlacing_cycle >= 3 {
                            if rdr.read_u8() == 0 {
                                return global_buffer;
                            }
                            panic!("Invalid GIF File: Wrong amount of image data");
                        }
                        interlacing_cycle += 1;
                        let (new_y_pos, new_line_factor) = match interlacing_cycle {
                            1 => (4, 8),
                            2 => (2, 4),
                            _ => (1, 2)
                        };
                        y_pos = new_y_pos;
                        line_factor = new_line_factor;
                    }
                    x_pos = curr_block_left as usize;
                }
            }
        }
    }
}

#[derive(Debug)]
struct GifHeader {
    width : u16,
    height : u16,
    nb_color_resolution_bits : u8,
    is_table_sorted : bool,
    background_color_index : u8,
    pixel_aspect_ratio : u8,
    global_color_table : Option<Vec<RGB>>,
}

#[derive(Debug, Clone)]
struct RGB {
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
        ((self.r as u32) << 16) + ((self.g as u32) << 8) + ((self.b as u32) << 0)
    }

}

impl Into<u32> for RGB {
    fn into(self) -> u32 {
        ((self.r as u32) << 16) + ((self.g as u32) << 8) + ((self.b as u32) << 0)
    }

}

// TODO use C repr to parse it more rapidly?
fn parse_color_table(rdr : &mut GifReader, nb_entries : usize) -> Vec<RGB> {
    let ct_size : usize = nb_entries * 3;
    if rdr.bytes_left() < ct_size  {
        panic!("Invalid GIF file: truncated color table");
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

fn parse_header(rdr : &mut GifReader) -> GifHeader {
    match rdr.read_str(3) {
        Err(e) => panic!("Invalid GIF file:
            Impossible to read the header, obtained: {}.", e),
        Ok(x) if x != "GIF" => panic!("Invalid GIF file: Missing GIF header."),
        _ => {}
    }

    match rdr.read_str(3) {
        Err(x) => panic!("Impossible to parse the version: {}.", x),
        Ok(v) if v != "89a" && v != "87a" => panic!("Unmanaged version: {}", v),
        _ => {}
    }

    let width = rdr.read_u16();
    let height = rdr.read_u16();

    let field = rdr.read_u8();
    let has_global_color_table = field & 0x80 != 0;
    let nb_color_resolution_bits = ((field & 0x70) >> 4) + 1;
    let is_table_sorted = field & 0x08 != 0;
    let nb_entries : usize = 1 << ((field & 0x07) + 1);

    let background_color_index = rdr.read_u8();
    let pixel_aspect_ratio = rdr.read_u8();

    let global_color_table = if has_global_color_table {
        Some(parse_color_table(rdr, nb_entries))
    } else {
        None
    };

    GifHeader {
        width,
        height,
        nb_color_resolution_bits,
        is_table_sorted,
        background_color_index,
        pixel_aspect_ratio,
        global_color_table,
    }
}
