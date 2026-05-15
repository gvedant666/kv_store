use std::convert::TryInto;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    Unknown = 1,
    ArgError = 2,
    TypeError = 3,
}


pub struct Command<'a> {
    pub args: Vec<&'a [u8]>,
}


pub fn parse_request(buf: &[u8]) -> Result<Option<(Command, usize)>, &'static str> {
    // We need at least 4 bytes to read the total packet length
    if buf.len() < 4 {
        return Ok(None);
    }

    let total_len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
    
    // Check if we have received the full packet yet
    if buf.len() < 4 + total_len {
        return Ok(None); 
    }

    let mut offset = 4;
    if total_len < 4 {
        return Err("Invalid total packet length");
    }

    let num_args = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let mut args = Vec::with_capacity(num_args);
    
    for _ in 0..num_args {
        if offset + 4 > 4 + total_len {
            return Err("Buffer overflow parsing argument length");
        }
        let arg_len = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if offset + arg_len > 4 + total_len {
            return Err("Buffer overflow parsing argument data");
        }
        
        // ZERO-COPY MAGIC: Slice directly into the buffer. No allocations!
        args.push(&buf[offset..offset + arg_len]);
        offset += arg_len;
    }

    Ok(Some((Command { args }, 4 + total_len)))
}

/// Builds binary responses according to the custom protocol format.
pub struct ResponseBuilder {
    buf: Vec<u8>,
}

impl ResponseBuilder {
    #[must_use]
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(64);
        // Reserve 4 bytes at the front for the total length prefix
        buf.extend_from_slice(&[0, 0, 0, 0]);
        Self { buf }
    }

    pub fn nil(&mut self) {
        self.buf.push(0); // Type 0: NIL
    }

    pub fn error(&mut self, code: ErrorCode, msg: &str) {
        self.buf.push(1); // Type 1: ERR
        self.buf.extend_from_slice(&(code as u32).to_le_bytes());
        let msg_bytes = msg.as_bytes();
        self.buf.extend_from_slice(&(msg_bytes.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(msg_bytes);
    }

    pub fn str(&mut self, val: &[u8]) {
        self.buf.push(2); // Type 2: STR
        self.buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(val);
    }

    pub fn int(&mut self, val: i64) {
        self.buf.push(3); // Type 3: INT
        self.buf.extend_from_slice(&val.to_le_bytes());
    }

    pub fn double(&mut self, val: f64) {
        self.buf.push(4); // Type 4: DBL
        self.buf.extend_from_slice(&val.to_le_bytes());
    }

    /// Starts an array response and returns a builder to add items to it.
    pub fn array_start(&mut self) -> ArrayBuilder {
        self.buf.push(5); // Type 5: ARR
        let count_pos = self.buf.len();
        self.buf.extend_from_slice(&[0, 0, 0, 0]); // Placeholder for array element count
        ArrayBuilder { builder: self, count_pos, count: 0 }
    }

    /// Finalizes the packet, calculates the total length, and returns the byte vector.
    pub fn finish(mut self) -> Vec<u8> {
        let total_len = (self.buf.len() - 4) as u32;
        let len_bytes = total_len.to_le_bytes();
        self.buf[0..4].copy_from_slice(&len_bytes);
        self.buf
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct to build array responses safely
pub struct ArrayBuilder<'a> {
    builder: &'a mut ResponseBuilder,
    count_pos: usize,
    count: u32,
}

impl<'a> ArrayBuilder<'a> {
    pub fn item_str(&mut self, val: &[u8]) {
        self.builder.str(val);
        self.count += 1;
    }

    pub fn item_double(&mut self, val: f64) {
        self.builder.double(val);
        self.count += 1;
    }

    pub fn finish(self) {
        let count_bytes = self.count.to_le_bytes();
        self.builder.buf[self.count_pos..self.count_pos + 4].copy_from_slice(&count_bytes);
    }
}

/// Fast hashing utility for the concurrent map and storage engine
pub fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}