use std::io::{Read, Seek};
use std::string::String;

/// The GifRead trait provides function to easily read GIF data from a Read
/// type.
pub trait GifRead {
    /// Read the next N bytes as an utf8 string.
    /// TODO GIF strings always seem to be in ASCII.
    /// Here I'm left with a dilemma:
    ///   - should I return an error if the most significant bit is set to `1`
    ///     (considering ASCII codes are 7 bits only)
    ///   - should I ignore it and just consider the other bits
    ///
    /// For now, we parse it as if it was UTF-8 which may be compatible, but seems
    /// overkill. Maybe a better solution can be found.
    fn read_str(&mut self, nb_bytes: usize) -> Result<String, GifReaderStringError>;

    /// Get the next two bytes as an u16.
    fn read_u16(&mut self) -> Result<u16, std::io::Error>;

    /// Get the next byte.
    fn read_u8(&mut self) -> Result<u8, std::io::Error>;

    /// Return the next N bytes as a slice of u8.
    fn read_bytes(&mut self, nb_bytes: usize) -> Result<Vec<u8>, std::io::Error>;

    /// Skip `nb_bytes` number of bytes.
    fn skip_bytes(&mut self, nb_bytes: usize) -> Result<(), std::io::Error>;

    /// Get the GifReader's current cursor position
    fn get_pos(&self) -> usize;
}

/// Struct helping with the parsing of the different values encountered in a GIF
/// image file.
/// This struct provides methods to consume and parse the N next bytes into the
/// wanted "format" (e.g. ASCII, u16, u8...).
/// Please not that this struct does no image decoding, you will also need a
/// decoder to make sense of GIF image data.
pub struct GifReader<T: Read + Seek> {
    /// Reader returning the GIF buffer
    reader: T,
    /// Current position in the GIF file.
    pos: usize,
}

/// Errors triggered when reading a string from a GIF buffer
pub enum GifReaderStringError {
    /// The string is an invalid UTF8 character
    FromUtf8Error,
    /// We could not read the specified amount of bytes from the GIF buffer.
    IOError(std::io::Error),
}

impl<T: Read + Seek> GifReader<T> {
    /// Create a new GifReader from the given GIF buffer.
    pub fn new(reader: T) -> GifReader<T> {
        GifReader { reader, pos: 0 }
    }
}

impl<T: Read + Seek> GifRead for GifReader<T> {
    /// Read the next N bytes as an utf8 string.
    /// TODO GIF strings always seem to be in ASCII.
    /// Here I'm left with a dilemma:
    ///   - should I return an error if the most significant bit is set to `1`
    ///     (considering ASCII codes are 7 bits only)
    ///   - should I ignore it and just consider the other bits
    ///
    /// For now, we parse it as if it was UTF-8 which may be compatible, but seems
    /// overkill. Maybe a better solution can be found.
    fn read_str(&mut self, nb_bytes: usize) -> Result<String, GifReaderStringError> {
        self.pos += nb_bytes;
        let mut buffer = vec![0; nb_bytes];
        if let Err(e) = self.reader.read_exact(&mut buffer) {
            return Err(GifReaderStringError::IOError(e));
        }
        match String::from_utf8(buffer) {
            Err(_) => Err(GifReaderStringError::FromUtf8Error),
            Ok(x) => Ok(x),
        }
    }

    /// Get the next two bytes as an u16.
    fn read_u16(&mut self) -> Result<u16, std::io::Error> {
        self.pos += 2;
        let mut buffer = [0; 2];
        self.reader.read_exact(&mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    /// Get the next byte.
    fn read_u8(&mut self) -> Result<u8, std::io::Error> {
        self.pos += 1;
        let mut buffer = [0; 1];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    /// Return the next N bytes as a slice of u8.
    fn read_bytes(&mut self, nb_bytes: usize) -> Result<Vec<u8>, std::io::Error> {
        self.pos += nb_bytes;
        let mut buffer = vec![0; nb_bytes];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Skip `nb_bytes` number of bytes.
    fn skip_bytes(&mut self, nb_bytes: usize) -> Result<(), std::io::Error> {
        self.pos += nb_bytes;
        self.reader
            .seek(std::io::SeekFrom::Start(self.pos as u64))?;
        Ok(())
    }

    /// Get the GifReader's current cursor position
    fn get_pos(&self) -> usize {
        self.pos
    }
}
