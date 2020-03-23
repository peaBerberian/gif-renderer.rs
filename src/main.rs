// use lzw::{Decoder,LsbReader};
use minifb::{Key, Window, WindowOptions};

const HEADER_SIZE : usize = 13;

const IMAGE_DESCRIPTOR_BLOCK_ID : u8 = 0x2C;
const TRAILER_BLOCK_ID : u8 = 0x3B;

struct GifReader {
    buf : Vec<u8>,
    pos : usize,
}

impl GifReader {
    fn new(buf : Vec<u8>) -> GifReader {
        GifReader {
            buf,
            pos: 0,
        }
    }

    /// Read the next N bytes as an utf8 string.
    /// /!\ Perform no bound checking. Will panic if there's less than the
    /// indicated number of bytes in the buffer.
    fn read_str(&mut self, nb_bytes : usize) -> Result<&str, std::str::Utf8Error> {
        use std::str;
        let end = self.pos + nb_bytes;
        let data = &self.buf[self.pos..end];
        self.pos += nb_bytes;
        str::from_utf8(data)
    }

    /// Get the next two bytes as an u16.
    /// /!\ Perform no bound checking. Will panic if there's less than two bytes
    /// left in the buffer.
    fn read_u16(&mut self) -> u16 {
        let val = self.buf[self.pos] as u16 |
            ((self.buf[self.pos + 1] as u16) << 8);
        self.pos += 2;
        val
    }

    /// Get the next byte.
    /// /!\ Perform no bound checking. Will panic if there's no byte left in
    /// the buffer.
    fn read_u8(&mut self) -> u8 {
        let val = self.buf[self.pos];
        self.pos += 1;
        val
    }

    /// Return the next N bytes as a slice of u8.
    /// /!\ Perform no bound checking. Will panic if there's less than the
    /// indicated number of bytes in the buffer.
    fn read_slice(&mut self, nb_bytes : usize) -> &[u8] {
        let end = self.pos + nb_bytes;
        let val = &self.buf[self.pos..end];
        self.pos += nb_bytes;
        val
    }

    /// Get the GifReader's current cursor position
    fn get_pos(&self) -> usize {
        self.pos
    }

    /// Get the remaining amount of bytes to read in the GifReader's buffer.
    fn bytes_left(&self) -> usize {
        self.buf.len() - self.pos
    }
}

fn main() {

    let file_data = std::fs::read("./b.gif").unwrap();
    if file_data.len() < HEADER_SIZE {
        panic!("Invalid GIF file: too short");
    }
    let mut rdr = GifReader::new(file_data);
    let header = parse_header(&mut rdr);
    println!("resolution:{}x{}", header.width, header.height);
    println!("GCT: {:?}", header.global_color_table);

    while rdr.bytes_left() > 0 {
        match rdr.read_u8() {
            IMAGE_DESCRIPTOR_BLOCK_ID => {
                println!("IT'S A BLOCK {}", rdr.get_pos());
                parse_image_descriptor(&mut rdr, header.global_color_table);
                return;
            }
            TRAILER_BLOCK_ID => {
                println!("IT'S A TRAILER {}", rdr.get_pos());
                // return ();
            }
            _ => {
                println!("KEZAKO {}", rdr.get_pos());
            }
        }
    }
}

fn parse_image_descriptor(rdr : &mut GifReader, gct : Option<Vec<RGB>>) {
    let image_left_position = rdr.read_u16();
    let image_top_position = rdr.read_u16();
    let image_width = rdr.read_u16();
    let image_height = rdr.read_u16();
    let field = rdr.read_u8();
    println!("x: {}, y: {}, {}x{}",
        image_left_position, image_top_position, image_width, image_height);

    let has_local_color_table = field & 0x80 != 0;
    let _has_interlacing = field & 0x40 != 0;
    let _is_sorted = field & 0x20 != 0;
    let _reserved_1 = field & 0x10;
    let _reserved_2 = field & 0x08;
    let nb_entries : usize = 1 << ((field & 0x07) + 1);

    let lct = if has_local_color_table {
        Some(parse_color_table(rdr, nb_entries))
    } else { None };

    println!("LCT: {:?}", lct);

    let initial_code_size = rdr.read_u8();
    println!("code size: {}", initial_code_size);

    // TODO Remove
    let mut whole_data : Vec<u8> = vec![];
    loop {
        if rdr.bytes_left() <= 0 {
            panic!("Invalid GIF File: Image Descriptor Truncated");
        }

        let sub_block_size = rdr.read_u8() as usize;
        println!("Sub block size: {}", sub_block_size);
        if sub_block_size == 0 {
            println!("Whole data's len: {}", whole_data.len());
            println!("sub block: {}x{}", image_height, image_width);
            println!("{:?}", whole_data);

            let data2 = lzw_decoder(&whole_data, initial_code_size);

            let current_color_table = if let Some(c) = lct {
                c
            } else {
                gct.unwrap()
            };
            let mut buffer: Vec<u32> = Vec::with_capacity(image_width as usize *
                                                          image_height as usize);
            println!("{:?}", &data2);
            let data2_len = data2.len();
            for elt in data2 {
                let rgb = &current_color_table[elt as usize];
                let val = ((rgb.r as u32) << 16) + ((rgb.g as u32) << 8) + ((rgb.b as u32) << 0);
                buffer.push(val);
            }
            // println!(data2_len, buffer.capacity())
            for _ in data2_len..buffer.capacity() {
                buffer.push(0);
            }

            let mut window = Window::new(
                "Test - ESC to exit",
                image_width as usize,
                image_height as usize,
                WindowOptions::default(),
            ).unwrap_or_else(|e| { panic!("{}", e); });

            // Limit to max ~60 fps update rate
            window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

            while window.is_open() && !window.is_key_down(Key::Escape) {
                // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
                window
                    .update_with_buffer(&buffer, image_width as usize, image_height as usize)
                    .unwrap();
                }
            return ;
        }
        if rdr.bytes_left() <= sub_block_size {
            panic!("Invalid GIF File: Image Descriptor Truncated");
        }
        whole_data.extend(rdr.read_slice(sub_block_size));
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

    let gct = if has_global_color_table {
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
        global_color_table: gct,
    }
}

#[derive(Clone, Debug)]
enum DictionaryValue {
    None,
    Clear,
    Stop,
    Repeat,
    Value(Vec<u8>),
}

#[derive(Debug)]
struct Dictionary {
    min_code_size : u8,
    next_code : u16,
    table : Vec<DictionaryValue>,
}

impl Dictionary {
    fn new(min_code_size : u8) -> Dictionary {
        let table : Vec<DictionaryValue> = Vec::with_capacity(512);
        let mut dict = Dictionary {
            min_code_size,
            next_code: (1u16 << min_code_size) + 2,
            table,
        };
        dict.clear();
        dict
    }

    fn clear(&mut self) {
        self.table.clear();
        let initial_table_size : u16 = (1 << self.min_code_size as u16) + 2;
        for i in 0..(initial_table_size - 2) {
            self.table.push(DictionaryValue::Value(vec![i as u8]));
        }
        self.table.push(DictionaryValue::Clear);
        self.table.push(DictionaryValue::Stop);
        self.next_code = (1u16 << self.min_code_size) + 2;
    }

    fn get_code(&self, code : u16) -> &DictionaryValue {
        let code = code as usize;
        if self.table.len() > code {
            &self.table[code]
        } else if code == self.table.len() {
            &DictionaryValue::Repeat
        } else {
            &DictionaryValue::None
        }
    }

    fn push_val(&mut self, val : Vec<u8>) {
        self.table.push(DictionaryValue::Value(val));
    }
}

/// Read bits from a byte stream, least significant bits first.
/// Shamefully mostly-taken from the `gif` crate.
/// Not that I don't understand it now!
#[derive(Debug)]
struct LsbReader {
    /// Current number or bits waiting to be read
    bits: u8,

    /// Current pending value
    acc: u32,
}

impl LsbReader {
    /// Create a new LsbReader
    fn new() -> LsbReader {
        LsbReader {
            bits: 0,
            acc: 0,
        }
    }

    /// Reads and consumes `n` amount of bits from `buf`.
    /// Returns both the number or bytes read from the buffer and the read u16
    /// value.
    /// Warning: `n` cannot be superior to 16.
    fn read_bits(&mut self, mut buf: &[u8], n: u8) -> (usize, Option<u16>) {
        if n > 16 {
            // This is a logic error the program should have prevented this
            // Ideally we would used bounded a integer value instead of u8
            panic!("Cannot read more than 16 bits")
        }
        let mut consumed = 0;
        while self.bits < n {
            let byte = if buf.len() > 0 {
                let byte = buf[0];
                buf = &buf[1..];
                byte
            } else {
                return (consumed, None)
            };
            // Adds to perhaps previously-parsed bits
            self.acc |= (byte as u32) << self.bits;
            self.bits += 8;
            consumed += 1;
        }

        // Only keeps bits corresponding to `n`
        let res = self.acc & ((1 << n) - 1);

        // Remove the `n` element we just read
        self.acc >>= n;
        self.bits -= n;

        (consumed, Some(res as u16))
    }
}

fn lzw_decoder(buf : &[u8], min_code_size : u8) -> Vec<u8> {
    let mut current_vec : Vec<u8> = vec![];
    let mut bit_reader = LsbReader::new();
    let mut dict  = Dictionary::new(min_code_size);
    let mut decoded_buf : Vec<u8> = vec![];

    let mut current_code_size = min_code_size + 1;
    let mut current_offset = 0;
    loop {
        match bit_reader.read_bits(&buf[current_offset..], current_code_size) {
            (_, None) => {
                return decoded_buf;
            },
            (consumed, Some(val)) => {
                current_offset += consumed;
                println!("Code encountered: {}", val);
                match dict.get_code(val) {
                    DictionaryValue::Clear => {
                        println!("!! CLEAR");
                        dict.clear();
                        current_code_size = min_code_size + 1;
                        current_vec = vec![];
                    },
                    DictionaryValue::Stop => {
                        println!("!! Stop");
                        return decoded_buf
                    },
                    DictionaryValue::None => {
                        println!("!! None");
                        panic!("Impossible to decode. Invalid value: {} {:?}", val, dict);
                    },
                    DictionaryValue::Repeat => {
                        println!("!! REPEAT");
                        if current_vec.len() == 0 {
                            panic!("Impossible to decode. Invalid value: {} {:?}", val, dict);
                        }
                        let first_val = current_vec[0];
                        current_vec.push(first_val);
                        decoded_buf.extend(current_vec.clone());
                        println!("New code pushed: {} = {:?}", dict.table.len(), &current_vec);
                        dict.push_val(current_vec.clone());
                        if dict.table.len() == (1 << current_code_size as usize) {
                            current_code_size += 1;
                        }
                    },
                    DictionaryValue::Value(val) => {
                        println!("Corresponding value(s): {:?}", val);
                        decoded_buf.extend(val);
                        if current_vec.len() != 0 { // Only empty at the beginning or when cleared
                            current_vec.push(val[0]);
                            let val_cloned = val.clone();
                            println!("New code pushed: {} = {:?}", dict.table.len(), &current_vec);
                            dict.push_val(current_vec.clone());
                            if dict.table.len() == (1 << current_code_size as usize) {
                                current_code_size += 1;
                            }
                            current_vec = val_cloned;
                        } else {
                            current_vec.push(val[0]);
                        }
                    }
                }
            }
        }
    }
}

///// Alias for a LZW code point
//type Code = u16;
//const MAX_CODESIZE: u8 = 12;
//const MAX_ENTRIES: usize = 1 << MAX_CODESIZE as usize;

///// Containes either the consumed bytes and reconstructed bits or
///// only the consumed bytes if the supplied buffer was not bit enough
//pub enum Bits {
//    /// Consumed bytes, reconstructed bits
//    Some(usize, u16),
//    /// Consumed bytes
//    None(usize),
//}

//#[derive(Debug)]
//struct DecodingDict {
//    min_size: u8,
//    table: Vec<(Option<Code>, u8)>,
//    buffer: Vec<u8>,
//}

//use std::io;

//impl DecodingDict {
//    /// Creates a new dict
//    fn new(min_size: u8) -> DecodingDict {
//        DecodingDict {
//            min_size: min_size,
//            table: Vec::with_capacity(512),
//            buffer: Vec::with_capacity((1 << MAX_CODESIZE as usize) - 1)
//        }
//    }

//    /// Resets the dictionary
//    fn reset(&mut self) {
//        self.table.clear();
//        for i in 0..(1u16 << self.min_size as usize) {
//            self.table.push((None, i as u8));
//        }
//    }

//    /// Inserts a value into the dict
//    #[inline(always)]
//    fn push(&mut self, key: Option<Code>, value: u8) {
//        self.table.push((key, value))
//    }

//    /// Reconstructs the data for the corresponding code
//    fn reconstruct(&mut self, code: Option<Code>) -> io::Result<&[u8]> {
//        self.buffer.clear();
//        let mut code = code;
//        let mut cha;
//        // Check the first access more thoroughly since a bad code
//        // could occur if the data is malformed
//        if let Some(k) = code {
//            match self.table.get(k as usize) {
//                Some(&(code_, cha_)) => {
//                    code = code_;
//                    cha = cha_;
//                }
//                None => return Err(io::Error::new(
//                    io::ErrorKind::InvalidInput,
//                    &*format!("Invalid code {:X}, expected code <= {:X}", k, self.table.len())
//                ))
//            }
//            self.buffer.push(cha);
//        }
//        while let Some(k) = code {
//            if self.buffer.len() >= MAX_ENTRIES {
//                return Err(io::Error::new(
//                    io::ErrorKind::InvalidInput,
//                    "Invalid code sequence. Cycle in decoding table."
//                ))
//            }
//            //(code, cha) = self.table[k as usize];
//            // Note: This could possibly be replaced with an unchecked array access if
//            //  - value is asserted to be < self.next_code() in push
//            //  - min_size is asserted to be < MAX_CODESIZE
//            let entry = self.table[k as usize]; code = entry.0; cha = entry.1;
//            self.buffer.push(cha);
//        }
//        self.buffer.reverse();
//        Ok(&self.buffer)
//    }

//    /// Returns the buffer constructed by the last reconstruction
//    #[inline(always)]
//    fn buffer(&self) -> &[u8] {
//        &self.buffer
//    }

//    /// Number of entries in the dictionary
//    #[inline(always)]
//    fn next_code(&self) -> u16 {
//        self.table.len() as u16
//    }
//}

//#[derive(Debug)]
//struct Decoder {
//    r: LsbReader,
//    prev: Option<Code>,
//    table: DecodingDict,
//    buf: [u8; 1],
//    code_size: u8,
//    min_code_size: u8,
//    clear_code: Code,
//    end_code: Code,
//}

//impl Decoder {
//    /// Creates a new LZW decoder.
//    pub fn new(min_code_size: u8) -> Decoder {
//        Decoder {
//            r: LsbReader::new(),
//            prev: None,
//            table: DecodingDict::new(min_code_size),
//            buf: [0; 1],
//            code_size: min_code_size + 1,
//            min_code_size: min_code_size,
//            clear_code: 1 << min_code_size,
//            end_code: (1 << min_code_size) + 1,
//        }
//    }

//    /// Tries to obtain and decode a code word from `bytes`.
//    ///
//    /// Returns the number of bytes that have been consumed from `bytes`. An empty
//    /// slice does not indicate `EOF`.
//    pub fn decode_bytes(&mut self, bytes: &[u8]) -> io::Result<(usize, &[u8])> {
//        println!("decode_bytes cc:{} size:{}", self.clear_code, self.code_size);
//        Ok(match self.r.read_bits(bytes, self.code_size) {
//            Bits::Some(consumed, code) => {
//                (consumed, if code == self.clear_code {
//                    // println!("CCC");
//                    self.table.reset();
//                    self.table.push(None, 0); // clear code
//                    self.table.push(None, 0); // end code
//                    self.code_size = self.min_code_size + 1;
//                    self.prev = None;
//                    &[]
//                } else if code == self.end_code {
//                    // println!("DDD");
//                    &[]
//                } else {
//                    // println!("EEE");
//                    let next_code = self.table.next_code();
//                    if code > next_code {
//                        return Err(io::Error::new(
//                            io::ErrorKind::InvalidInput,
//                            &*format!("Invalid code {:X}, expected code <= {:X}",
//                                      code,
//                                      next_code
//                            )
//                        ))
//                    }
//                    let prev = self.prev;
//                    let result = if prev.is_none() {
//                        // println!("HERE {}", code);
//                        self.buf = [code as u8];
//                        &self.buf[..]
//                    } else {
//                        let data = if code == next_code {
//                            // println!("HERE2");
//                            let cha = self.table.reconstruct(prev)?[0];
//                            self.table.push(prev, cha);
//                            self.table.reconstruct(Some(code))?
//                        } else if code < next_code {
//                            // println!("HERE3 {}", code);
//                            let cha = self.table.reconstruct(Some(code))?[0];
//                            self.table.push(prev, cha);
//                            self.table.buffer()
//                        } else {
//                            // code > next_code is already tested a few lines earlier
//                            unreachable!()
//                        };
//                        data
//                    };
//                    if next_code == (1 << self.code_size as usize) - 1 - 0 // XXX TODO $offset
//                       && self.code_size < MAX_CODESIZE {
//                        self.code_size += 1;
//                    }
//                    self.prev = Some(code);
//                    result
//                })
//            },
//            Bits::None(consumed) => {
//                (consumed, &[])
//            }
//        })
//    }
//}
