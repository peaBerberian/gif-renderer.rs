use crate::color::{self, RGB};
use crate::decoder::LzwDecoder;
use crate::error::{GifParsingError, Result};
use crate::event_loop::{ EventLoopProxy, GifEvent };
use crate::gif_reader::GifRead;
use crate::header::GifHeader;

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
const DEFAULT_BACKGROUND_COLOR : RGB = RGB { r: 0xFF, g: 0xFF, b: 0xFF };

pub fn decode_and_render(
    rdr : &mut impl GifRead,
    header : &GifHeader,
    el_proxy : EventLoopProxy
) -> Result<()> {
    let (background_color, global_color_table) =
        if let Some(gct) = &header.global_color_table {
            let index = header.background_color_index as usize;
            if gct.len() <= index {
                // TODO log "Invalid background color index" warning?
                (None, Some(gct.as_slice()))
            } else {
                (Some(gct[index]), Some(gct.as_slice()))
            }
        } else {
            (None, None)
        };

    // Last graphic extension encountered. Will be needed when an Image Descriptor
    // is encountered.
    let mut last_graphic_ext : Option<GraphicControlExtension> = None;

    // Background for the next frame encountered. Its content depends on the
    // "disposal method" of the next frame encountered.
    let mut next_frame_base_buffer : Option<Vec<u8>> = None;

    let mut found_loop_attribute = false;

    loop {
        match rdr.read_u8()? {
            IMAGE_DESCRIPTOR_BLOCK_ID => {
                let (delay, transparent_color_index) = match &last_graphic_ext {
                    Some(e) => (Some(e.delay), e.transparent_color_index),
                    None => (None, None)
                };

                // The "RestoreToPrevious" disposal method forces us to keep the current base
                // buffer for the frame coming after that one.
                use DisposalMethod::*;
                let cloned_image_background = match last_graphic_ext {
                    Some(GraphicControlExtension { disposal_method: RestoreToPrevious, .. }) =>
                        next_frame_base_buffer.clone(),
                    _ => None
                };

                let block = construct_next_frame(
                    rdr,
                    &global_color_table,
                    next_frame_base_buffer,
                    header.height,
                    header.width,
                    background_color,
                    transparent_color_index)?;

                // Obtain the base buffer for the next frame according to the current disposal
                // method
                next_frame_base_buffer = match last_graphic_ext {
                    Some(GraphicControlExtension { disposal_method: DoNotDispose, ..  }) |
                    Some(GraphicControlExtension { disposal_method: NoDisposalSpecified, ..  }) => {
                        Some(block.clone())
                    },
                    Some(GraphicControlExtension { disposal_method: RestoreToPrevious, ..}) =>
                        cloned_image_background,
                    _ => None,
                };
                el_proxy.send_event(GifEvent::GifFrameData {
                    data: block,
                    delay_until_next: delay,
                }).unwrap_or_else(|err| {
                    eprintln!("Error: Impossible to communicate a new decoded frame: {}", err);
                    std::process::exit(1);
                });
            }
            TRAILER_BLOCK_ID => {
                if !found_loop_attribute {
                    el_proxy.send_event(GifEvent::LoopingInfo(None)).unwrap_or_else(|err| {
                        eprintln!("Error: Impossible to communicate absence of looping information: {}", err);
                        std::process::exit(1);
                    });
                }
                el_proxy.send_event(GifEvent::GifFrameEnd).unwrap_or_else(|err| {
                    eprintln!("Error: Impossible to communicate the end of decoded frames: {}", err);
                    std::process::exit(1);
                });
                break
            }
            EXTENSION_INTRODUCER_ID => {
                match rdr.read_u8()? {
                    GRAPHIC_CONTROL_EXTENSION_LABEL => {
                        last_graphic_ext = Some(parse_graphic_control_extension(rdr)?);
                    }
                    APPLICATION_EXTENSION_LABEL => {
                        let extension = parse_application_extension(rdr)?;

                        // Only NETSCAPE2.0 is parsed for now as looping is an essential feature
                        // (And I just don't want to set it to infinite by default)
                        if let ApplicationExtension::NetscapeLooping(x) = extension {
                            found_loop_attribute = true;
                            el_proxy.send_event(GifEvent::LoopingInfo(Some(x))).unwrap_or_else(|err| {
                                eprintln!("Error: Impossible to communicate looping information: {}", err);
                                std::process::exit(1);
                            });
                        }
                    }
                    COMMENT_EXTENSION_LABEL => {
                        // We don't care about comments
                        skip_sub_blocks(rdr)?;
                        if rdr.read_u8()? != 0x00 /* block terminator */ {
                            panic!("TOTO");
                            // error::fail_on_expected_block_terminator(Some("Comment"));
                        }
                    }
                    PLAIN_TEXT_EXTENSION_LABEL => {
                        skip_plain_text_extension(rdr)?;
                    }
                    x => {
                        return Err(GifParsingError::UnrecognizedExtension(x));
                    }
                }
            }
            x => {
                return Err(GifParsingError::UnrecognizedBlock {
                    code: x,
                    position: rdr.get_pos()
                });
            }
        }
    }
    Ok(())
}

enum ApplicationExtension {
    /// Looping value from the NETSCAPE2.0 extension.
    /// 0 means infinite looping, any other value would be the number of time
    /// the GIF image needs to be looped (played back from the beginning).
    NetscapeLooping(u16),
    NotKnown,
}

/// Allows to skip sub-blocks when reached. You might want to do that when
/// reaching a part of the GIF buffer containing sub-blocks you don't care for
/// (e.g. comments).
fn skip_sub_blocks(rdr : &mut impl GifRead) -> Result<()> {
    loop {
        let size_of_block = rdr.read_u8()? as usize;
        if size_of_block == 0 {
            return Ok(());
        }
        rdr.skip_bytes(size_of_block)?;
    }
}

/// The plain text extention is a 89a GIF extension allowing to render text in a
/// GIF image. This feature seems to be very rarely used, we can safely ignore
/// it for now.
/// TODO?
fn skip_plain_text_extension(rdr : &mut impl GifRead) -> Result<()> {
    let block_size = rdr.read_u8()?;
    if block_size != 12 {
        return Err(GifParsingError::UnexpectedLength {
            block_name : "Plain Text Extension".to_owned(),
            expected : 12,
            got : block_size,
        });
    }
    rdr.skip_bytes(12)?; // Skip whole plain text header
    skip_sub_blocks(rdr)?;
    Ok(())
}

fn parse_application_extension(rdr : &mut impl GifRead) -> Result<ApplicationExtension> {
    let block_size = rdr.read_u8()?;
    if block_size != 11 {
        return Err(GifParsingError::UnexpectedLength {
            block_name : "Application Extension".to_owned(),
            expected : 11,
            got : block_size,
        })
    }
    let app_name = match rdr.read_str(8) {
        Err(_) => None,
        Ok(x) => Some(x)
    };
    let app_auth_code = (rdr.read_u8()?, rdr.read_u8()?, rdr.read_u8()?);

    let mut data_len = rdr.read_u8()? as usize;
    if data_len == 0 {
        return Ok(ApplicationExtension::NotKnown);
    }

    let mut ext = ApplicationExtension::NotKnown;

    if app_name == Some("NETSCAPE".to_owned()) &&
       app_auth_code == (50, 46, 48) &&
       data_len >= 3
    {
        let cur_offset;
        let sub_block_id = rdr.read_u8()?;
        if data_len != 0x03 || sub_block_id != 0x01 {
            cur_offset = 1; // Not a valid NETSCAPE2.0 Looping extension, ignore
        } else {
            let loop_count = rdr.read_u16()?;
            ext = ApplicationExtension::NetscapeLooping(loop_count);
            cur_offset = 3;
        }
        data_len -= cur_offset;
    }

    // Skip all remaining data blocks
    loop {
        if data_len == 0 {
            break;
        }
        rdr.skip_bytes(data_len - 1)?;
        data_len = rdr.read_u8()? as usize;
    }
    if rdr.read_u8()? != 0x00 /* block terminator */ {
        return Err(GifParsingError::ExpectedBlockTerminator {
            block_name : Some("ApplicationExtension Extension".to_owned())
        });
    }
    Ok(ext)
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

fn parse_graphic_control_extension(
    rdr : &mut impl GifRead
) -> Result<GraphicControlExtension> {
    let block_size = rdr.read_u8()? as usize;

    if block_size != 4 {
        return Err(GifParsingError::UnexpectedLength {
            block_name : "Graphic Control Extension".to_owned(),
            expected : 4,
            got: block_size as u8,
        });
    }
    let packed_fields = rdr.read_u8()?;
    let disposal_method = match (packed_fields & 0b0001_1100) >> 2 {
        1 => DisposalMethod::DoNotDispose,
        2 => DisposalMethod::RestoreToBackgroundColor,
        3 => DisposalMethod::RestoreToPrevious,
        _ => DisposalMethod::NoDisposalSpecified,
    };
    let user_input : bool = packed_fields & 0x02 != 0;
    let transparent_color_flag : bool = packed_fields & 0x01 != 0;
    let delay = rdr.read_u16()?;
    let transparent_color_index = if transparent_color_flag {
        Some(rdr.read_u8()?)
    } else {
        rdr.skip_bytes(1)?;
        None
    };
    if rdr.read_u8()? != 0 {
        return Err(GifParsingError::ExpectedBlockTerminator {
            block_name: Some("Graphic Control Extension".to_owned())
        });
    }
    Ok(GraphicControlExtension {
        disposal_method,
        user_input,
        transparent_color_index,
        delay,
    })
}

fn construct_next_frame(
    rdr : &mut impl GifRead,
    global_color_table : &Option<&[RGB]>,
    base_buffer : Option<Vec<u8>>,
    img_height : u16,
    img_width : u16,
    background_color : Option<RGB>,
    transparent_color_index : Option<u8>
) -> Result<Vec<u8>> {
    let curr_block_left = rdr.read_u16()?;
    let curr_block_top = rdr.read_u16()?;
    let curr_block_width = rdr.read_u16()?;
    let curr_block_height = rdr.read_u16()?;
    let field = rdr.read_u8()?;

    let has_local_color_table = field & 0x80 != 0;

    let has_interlacing = field & 0x40 != 0;
    let _is_sorted = field & 0x20 != 0;
    let _reserved_1 = field & 0x10;
    let _reserved_2 = field & 0x08;
    let nb_color_entries : usize = 1 << ((field & 0x07) + 1);

    // Current interlacing cycle - from 0 to 3 - and step used to obtain the next line
    // we should draw. Both are only needed when interlacing is enabled.
    let (mut interlacing_cycle, mut line_step) = if has_interlacing {
        (0, 8)
    } else {
        (0, 1)
    };

    let lct = if has_local_color_table {
        Some(color::parse_color_table(rdr, nb_color_entries)?)
    } else {
        None
    };

    let current_color_table : &[RGB] = if let Some(c) = &lct {
        c
    } else {
        match global_color_table {
            None => {
                return Err(GifParsingError::NoColorTable);
            }
            Some(val) => val
        }
    };

    let (has_background_frame, mut global_buffer) = match base_buffer {
        Some(frame) => (true, frame),
        None => (false, vec![0; img_height as usize * img_width as usize * 3]),
    };

    let initial_code_size = rdr.read_u8()?;
    let mut decoder = LzwDecoder::new(initial_code_size);

    if curr_block_width == 0 || curr_block_height == 0 {
        let bg_color : RGB = match background_color {
            Some(color) => color,
            None => DEFAULT_BACKGROUND_COLOR,
        };
        let elts = img_height as usize * img_width as usize;
        let mut ret : Vec<u8> = Vec::with_capacity(elts * 3);
        for _ in 0..elts {
            ret.push(bg_color.r);
            ret.push(bg_color.g);
            ret.push(bg_color.b);
        }
        return Ok(ret);
    }

    let mut x_pos : usize = curr_block_left as usize;
    let mut y_pos : usize = curr_block_top as usize;
    let max_pos_width = curr_block_width as usize + curr_block_left as usize - 1;
    let max_pos_height = curr_block_height as usize + curr_block_top as usize - 1;
    loop {
        let sub_block_size = rdr.read_u8()? as usize;
        if sub_block_size == 0x00 /* block terminator */ {
            return Ok(global_buffer);
        } else {
            let sub_block_data = rdr.read_bytes(sub_block_size)?;
            let decoded_data = decoder.decode_next(&sub_block_data);
            for elt in decoded_data {
                if elt as usize >= current_color_table.len() {
                    return Err(GifParsingError::InvalidColor);
                }

                let curr_buffer_idx = ((y_pos * img_width as usize) + x_pos) * 3;
                if (curr_buffer_idx + 2) >= global_buffer.len() {
                    return Err(GifParsingError::TooMuchPixels);
                }
                match transparent_color_index {
                    Some(t_idx) if t_idx == elt => { // transparent color
                        if !has_background_frame {
                            let color : RGB = match background_color {
                                Some(c) => c,
                                None => DEFAULT_BACKGROUND_COLOR,
                            };

                            global_buffer[curr_buffer_idx] = color.r;
                            global_buffer[curr_buffer_idx + 1] = color.g;
                            global_buffer[curr_buffer_idx + 2] = color.b;
                        }
                    }
                    _ => {
                        let color : RGB = current_color_table[elt as usize];
                        global_buffer[curr_buffer_idx] = color.r;
                        global_buffer[curr_buffer_idx + 1] = color.g;
                        global_buffer[curr_buffer_idx + 2] = color.b;
                    }
                }

                x_pos += 1;
                if x_pos > max_pos_width {
                    y_pos += line_step;
                    if y_pos > max_pos_height {
                        if !has_interlacing || interlacing_cycle >= 3 {
                            skip_sub_blocks(rdr)?;
                            return Ok(global_buffer);
                        }
                        interlacing_cycle += 1;
                        let (new_y_pos, new_line_step) = match interlacing_cycle {
                            1 => (4, 8),
                            2 => (2, 4),
                            _ => (1, 2)
                        };
                        y_pos = new_y_pos;
                        line_step = new_line_step;
                    }
                    x_pos = curr_block_left as usize;
                }
            }
        }
    }
}
