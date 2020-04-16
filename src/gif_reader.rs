use std::string::String;

/// Struct helping with the parsing of the different values encountered in a GIF
/// image file.
/// This struct provides methods to consume and parse the N next bytes into the
/// wanted "format" (e.g. ASCII, u16, u8...).
/// Please not that this struct does no image decoding, you will also need a
/// decoder to make sense of GIF image data.
pub struct GifReader {
    /// Whole GIF buffer.
    buf : Vec<u8>,
    /// Current position in the GIF file.
    pos : usize,
}

impl GifReader {
    /// Create a new GifReader from the given GIF buffer.
    pub fn new(buf : Vec<u8>) -> GifReader {
        GifReader {
            buf,
            pos: 0,
        }
    }

    /// Read the next N bytes as an utf8 string.
    /// /!\ Perform no bound checking. Will panic if there's less than the
    /// indicated number of bytes in the buffer.
    /// TODO GIF strings always seem to be in ASCII.
    /// Here I'm left with a dilemma:
    ///   - should I return an error if the most significant bit is set to `1`
    ///     (considering ASCII codes are 7 bits only)
    ///   - should I ignore it and just consider the other bits
    /// For now, we parse it as if it was UTF-8 which may be compatible, but seems
    /// overkill. Maybe a better solution can be found.
    pub fn read_str(&mut self, nb_bytes : usize) -> Result<String, std::string::FromUtf8Error> {
        let end = self.pos + nb_bytes;
        let data = &self.buf[self.pos..end];
        self.pos += nb_bytes;
        String::from_utf8(data.to_vec())
    }

    /// Get the next two bytes as an u16.
    /// /!\ Perform no bound checking. Will panic if there's less than two bytes
    /// left in the buffer.
    pub fn read_u16(&mut self) -> u16 {
        let val = self.buf[self.pos] as u16 |
            ((self.buf[self.pos + 1] as u16) << 8);
        self.pos += 2;
        val
    }

    /// Get the next byte.
    /// /!\ Perform no bound checking. Will panic if there's no byte left in
    /// the buffer.
    pub fn read_u8(&mut self) -> u8 {
        let val = self.buf[self.pos];
        self.pos += 1;
        val
    }

    /// Return the next N bytes as a slice of u8.
    /// /!\ Perform no bound checking. Will panic if there's less than the
    /// indicated number of bytes in the buffer.
    pub fn read_slice(&mut self, nb_bytes : usize) -> &[u8] {
        let end = self.pos + nb_bytes;
        let val = &self.buf[self.pos..end];
        self.pos += nb_bytes;
        val
    }

    /// Skip `nb_bytes` number of bytes.
    pub fn skip_bytes(&mut self, nb_bytes : usize) {
        self.pos += nb_bytes;
    }

    /// Get the GifReader's current cursor position
    pub fn get_pos(&self) -> usize {
        self.pos
    }

    /// Get the remaining amount of bytes to read in the GifReader's buffer.
    pub fn bytes_left(&self) -> usize {
        self.buf.len() - self.pos
    }
}
