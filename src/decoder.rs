/// Decompress data compressed in the LZW compression format.
/// This struct keeps state in between `decode_next` calls so that you can call
/// it with contiguous subparts of the compressed data as you read them.
pub struct LzwDecoder {
    current_val : Vec<u8>,
    bit_reader : LsbReader,
    dict : LzwDictionary,
}

impl LzwDecoder {
    /// Create a new LzwDecoder, with the given initial code size that should
    /// have been parsed from the GIF buffer before its compressed data.
    pub fn new(min_code_size : u8) -> LzwDecoder {
        LzwDecoder {
            current_val: vec![],
            bit_reader: LsbReader::new(),
            dict: LzwDictionary::new(min_code_size),
        }
    }

    /// Decode the next block of compressed data.
    pub fn decode_next(&mut self, buf : &[u8]) -> Vec<u8> {
        let mut decoded_buf : Vec<u8> = vec![];
        let mut current_offset = 0;
        loop {
            let curr_code_size = self.dict.get_curr_code_size();
            match self.bit_reader.get_next_code(&buf[current_offset..], curr_code_size) {
                (_, None) => {
                    return decoded_buf;
                },
                (consumed, Some(code)) => {
                    current_offset += consumed;
                    match self.dict.get_value(code) {
                        DictionaryValue::Clear => {
                            self.dict.clear();
                            self.current_val = vec![];
                        },
                        DictionaryValue::Stop => {
                            return decoded_buf
                        },
                        DictionaryValue::None => {
                            panic!("Impossible to decode. Invalid value: {}", code);
                        },
                        DictionaryValue::Repeat => {
                            if self.current_val.len() == 0 {
                                panic!("Impossible to decode. Invalid value: {}", code);
                            }
                            let first_val = self.current_val[0];
                            self.current_val.push(first_val);
                            decoded_buf.extend(self.current_val.clone());
                            self.dict.push_new_value(self.current_val.clone());
                        },
                        DictionaryValue::Value(val) => {
                            self.current_val.push(val[0]);
                            if self.current_val.len() != 1 { // Only at one at the beginning or when cleared
                                let val_cloned = val.clone();
                                self.dict.push_new_value(self.current_val.clone());
                                self.current_val = val_cloned;
                            }
                            decoded_buf.extend(val);
                        }
                    }
                }
            }
        }
    }
}

/// Store codes and related values for a LZW decoder.
#[derive(Debug)]
struct LzwDictionary {
    /// The minimum code size at the instanciation of the LzwDictionary.
    min_code_size : u8,

    /// Current code size that should be read from a compressed buffer.
    curr_code_size : u8,

    /// Table of correspondance between codes and corresponding values.
    /// Here a vec of Option type, where the code will be the index and the
    /// value will be wrapped in a `Some(value)` form.
    ///
    /// The `None` form will be used for the two special codes `clear` and
    /// `stop` as those are easy to calculate and would make the table take
    /// more space than it should (an Option<Vec<T>> doesn't augment the memory
    /// imprint of a Vec<T>).
    table : Vec<Option<Vec<u8>>>,
}

/// Value returned when interrogating the dictionnary through its `get_value`
/// method.
#[derive(Clone, Debug)]
enum DictionaryValue {
    /// The code given was invalid, no related value was found.
    None,

    /// The code given corresponds to a `clear` code.
    Clear,

    /// The code given corresponds to a `stop` code.
    Stop,

    /// The code given is for the special `repeat` case, where you have to add
    /// to your current value the first value decoded.
    Repeat,

    /// The code given was linked to a found value.
    Value(Vec<u8>),
}

impl LzwDictionary {
    /// Create a new LzwDictionary with the given initial code size.
    fn new(min_code_size : u8) -> LzwDictionary {
        let table : Vec<Option<Vec<u8>>> = Vec::with_capacity(512);
        let mut dict = LzwDictionary {
            min_code_size,
            curr_code_size: min_code_size + 1,
            table,
        };
        dict.clear(); // TODO necessary?
        dict
    }

    /// Reset the LzwDictionary to its initial state.
    /// To call when a `clear` code is encountered.
    fn clear(&mut self) {
        self.table.clear();
        self.curr_code_size = self.min_code_size + 1;
        let initial_table_size : u16 = 1 << self.min_code_size as u16;
        for i in 0..initial_table_size {
            self.table.push(Some(vec![i as u8]));
        }
        self.table.push(None); // `clear` code
        self.table.push(None); // `code` size
    }

    /// Get the value corresponding to the code given.
    fn get_value(&self, code : u16) -> DictionaryValue {
        let code = code as usize;
        if self.table.len() > code {
            match &self.table[code] {
                Some(val) => DictionaryValue::Value(val.clone()),
                None => if code == 1 << self.min_code_size as u16 {
                    DictionaryValue::Clear
                } else {
                    DictionaryValue::Stop
                }
            }
        } else if code == self.table.len() {
            DictionaryValue::Repeat
        } else {
            DictionaryValue::None
        }
    }

    /// Add a new value at the next code.
    fn push_new_value(&mut self, val : Vec<u8>) {
        self.table.push(Some(val));
        if self.table.len() == (1 << self.curr_code_size as usize) &&
            self.curr_code_size < 12 {
            self.curr_code_size += 1;
        }
    }

    /// Returns the current code size you have to read from the compressed
    /// buffer.
    fn get_curr_code_size(&self) -> u8 {
        self.curr_code_size
    }
}

/// Read bits from a byte stream, least significant bit first.
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

    /// Reads and consumes `code_size` amount of bits from `buf`.
    /// Returns both the number or bytes read from the buffer and the read u16
    /// value.
    /// Warning: `code_size` cannot be superior to 16.
    fn get_next_code(&mut self, mut buf: &[u8], code_size: u8) -> (usize, Option<u16>) {
        if code_size > 16 {
            // This is a logic error the program should have prevented this
            // Ideally we would used bounded a integer value instead of u8
            panic!("Cannot read more than 16 bits")
        }
        let mut consumed = 0;
        while self.bits < code_size {
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

        // Only keeps bits corresponding to `code_size`
        let res = self.acc & ((1 << code_size) - 1);

        // Remove the `code_size` element we just read
        self.acc >>= code_size;
        self.bits -= code_size;

        (consumed, Some(res as u16))
    }
}
