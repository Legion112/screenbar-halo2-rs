//! MSB-first bit buffers.
//!
//! The on-air frame is not byte-aligned (the PCF is 9 bits), so both encoding and
//! decoding work on a bit cursor over a fixed-size buffer. No allocation, so the
//! same code runs on the host in tests and on the MCU.

/// Writes bits MSB-first into a fixed `N`-byte buffer.
#[derive(Debug, Clone, Copy)]
pub struct BitWriter<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Default for BitWriter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> BitWriter<N> {
    /// Creates an empty writer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    /// Number of bits written so far.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if nothing has been written yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends one bit.
    ///
    /// # Panics
    /// Panics if the buffer is full.
    pub const fn push_bit(&mut self, bit: bool) {
        assert!(self.len < N * 8, "bit buffer overflow");
        if bit {
            self.buf[self.len / 8] |= 1 << (7 - (self.len % 8));
        }
        self.len += 1;
    }

    /// Appends the `count` most significant bits of `byte`.
    ///
    /// # Panics
    /// Panics if `count > 8` or the buffer overflows.
    pub const fn push_bits(&mut self, byte: u8, count: u8) {
        assert!(count <= 8, "a byte holds at most 8 bits");
        let mut i = 0;
        while i < count {
            self.push_bit((byte >> (7 - i)) & 1 != 0);
            i += 1;
        }
    }

    /// Appends a whole byte.
    pub const fn push_byte(&mut self, byte: u8) {
        self.push_bits(byte, 8);
    }

    /// Appends every byte of `bytes`.
    pub const fn push_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            self.push_byte(bytes[i]);
            i += 1;
        }
    }

    /// Appends the low `count` bits of `value`, MSB-first.
    ///
    /// # Panics
    /// Panics if `count > 16` or the buffer overflows.
    pub const fn push_u16(&mut self, value: u16, count: u8) {
        assert!(count <= 16, "a u16 holds at most 16 bits");
        let mut i = 0;
        while i < count {
            self.push_bit((value >> (count - 1 - i)) & 1 != 0);
            i += 1;
        }
    }

    /// Returns the packed bytes. The final byte is zero-padded when
    /// [`len`](Self::len) is not a multiple of eight.
    #[must_use]
    pub const fn bytes(&self) -> &[u8; N] {
        &self.buf
    }

    /// Returns only the bytes that hold written bits.
    #[must_use]
    pub fn filled(&self) -> &[u8] {
        &self.buf[..self.len.div_ceil(8)]
    }
}

/// Reads bits MSB-first out of a byte slice.
#[derive(Debug, Clone, Copy)]
pub struct BitReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    /// Creates a reader positioned at the first bit of `buf`.
    #[must_use]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Bits still available.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.buf.len() * 8 - self.pos
    }

    /// Reads one bit, or `None` at the end of the buffer.
    pub const fn read_bit(&mut self) -> Option<bool> {
        if self.pos >= self.buf.len() * 8 {
            return None;
        }
        let bit = (self.buf[self.pos / 8] >> (7 - (self.pos % 8))) & 1 != 0;
        self.pos += 1;
        Some(bit)
    }

    /// Reads `count` bits into the low bits of a `u16`, or `None` if fewer remain.
    ///
    /// # Panics
    /// Panics if `count > 16`.
    pub const fn read_u16(&mut self, count: u8) -> Option<u16> {
        assert!(count <= 16, "a u16 holds at most 16 bits");
        if self.remaining() < count as usize {
            return None;
        }
        let mut value = 0u16;
        let mut i = 0;
        while i < count {
            value <<= 1;
            if let Some(true) = self.read_bit() {
                value |= 1;
            }
            i += 1;
        }
        Some(value)
    }

    /// Reads one byte, or `None` if fewer than eight bits remain.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "read_u16(8) yields at most 8 bits"
    )]
    pub const fn read_byte(&mut self) -> Option<u8> {
        match self.read_u16(8) {
            Some(value) => Some(value as u8),
            None => None,
        }
    }

    /// Fills `out` byte by byte, returning `false` if the buffer runs out.
    pub const fn read_bytes(&mut self, out: &mut [u8]) -> bool {
        let mut i = 0;
        while i < out.len() {
            match self.read_byte() {
                Some(byte) => out[i] = byte,
                None => return false,
            }
            i += 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_msb_first() {
        let mut w = BitWriter::<2>::new();
        w.push_bits(0b1011_0000, 4);
        w.push_bits(0b0110_0000, 4);
        assert_eq!(w.len(), 8);
        assert_eq!(w.filled(), &[0b1011_0110]);
    }

    #[test]
    fn unaligned_fields_pack_tightly() {
        // 9-bit PCF straddling a byte boundary, exactly like the real frame.
        let mut w = BitWriter::<4>::new();
        w.push_byte(0xFF);
        w.push_u16(0b1_0101_0101, 9);
        assert_eq!(w.len(), 17);
        assert_eq!(w.filled(), &[0xFF, 0b1010_1010, 0b1000_0000]);
    }

    #[test]
    fn reader_round_trips_the_writer() {
        let mut w = BitWriter::<8>::new();
        w.push_bytes(&[0x86, 0xBB, 0xEA, 0x9C]);
        w.push_u16(0b1_0011_0010, 9);

        let mut r = BitReader::new(w.filled());
        let mut addr = [0u8; 4];
        assert!(r.read_bytes(&mut addr));
        assert_eq!(addr, [0x86, 0xBB, 0xEA, 0x9C]);
        assert_eq!(r.read_u16(9), Some(0b1_0011_0010));
    }

    #[test]
    fn reader_reports_exhaustion() {
        let mut r = BitReader::new(&[0xAA]);
        assert_eq!(r.read_u16(9), None);
        assert_eq!(r.remaining(), 8);
    }
}
