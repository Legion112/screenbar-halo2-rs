//! Bit-oriented CRC-16 used by the Halo 2 link.
//!
//! The link uses CRC-CCITT with polynomial `0x1021`, MSB-first, **not** reflected.
//! It is computed over the on-air bit stream *excluding* the preamble, i.e. over
//! address + PCF + payload. Because the PCF is 9 bits wide the checksum is not
//! byte-aligned, so the implementation has to consume individual bits.
//!
//! The initial state is the one piece that still needs confirming against a real
//! capture: both community projects report `0xEFDF` "before the four-byte on-air
//! address", while a stock `nRF24`-style link would use `0xFFFF`. [`Crc16`] is
//! generic over the seed so a capture can decide. See `docs/PROTOCOL.md`.

/// CRC-CCITT polynomial, normal (non-reflected) representation.
pub const POLY: u16 = 0x1021;

/// Initial state reported by the community reverse-engineering projects.
pub const INIT_HALO2: u16 = 0xEFDF;

/// Initial state of a stock `nRF24 Enhanced ShockBurst` link, kept for A/B testing
/// a capture against [`INIT_HALO2`].
pub const INIT_NRF24: u16 = 0xFFFF;

/// Running CRC-16 state, fed one bit at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crc16 {
    state: u16,
}

impl Crc16 {
    /// Starts a new checksum from `init`.
    #[must_use]
    pub const fn new(init: u16) -> Self {
        Self { state: init }
    }

    /// Feeds a single bit.
    pub const fn push_bit(&mut self, bit: bool) {
        let feedback = ((self.state >> 15) & 1) != 0;
        self.state <<= 1;
        if feedback != bit {
            self.state ^= POLY;
        }
    }

    /// Feeds the `count` most significant bits of `byte`, MSB first.
    ///
    /// # Panics
    /// Panics if `count > 8`.
    pub const fn push_bits(&mut self, byte: u8, count: u8) {
        assert!(count <= 8, "a byte holds at most 8 bits");
        let mut i = 0;
        while i < count {
            self.push_bit((byte >> (7 - i)) & 1 != 0);
            i += 1;
        }
    }

    /// Feeds a whole byte, MSB first.
    pub const fn push_byte(&mut self, byte: u8) {
        self.push_bits(byte, 8);
    }

    /// Feeds every byte of `bytes`, MSB first.
    pub const fn push_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            self.push_byte(bytes[i]);
            i += 1;
        }
    }

    /// Returns the current checksum.
    #[must_use]
    pub const fn finish(self) -> u16 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer test for CRC-16/CCITT-FALSE: `check` of the standard
    /// `"123456789"` vector is `0x29B1`. This pins down the bit-level algorithm
    /// itself, independently of which seed the Halo 2 turns out to use.
    #[test]
    fn matches_ccitt_false_check_vector() {
        let mut crc = Crc16::new(INIT_NRF24);
        crc.push_bytes(b"123456789");
        assert_eq!(crc.finish(), 0x29B1);
    }

    #[test]
    fn partial_bytes_are_msb_first() {
        // Pushing 0b1010_0000's top 3 bits must equal pushing those bits one by one.
        let mut a = Crc16::new(INIT_HALO2);
        a.push_bits(0b1010_0000, 3);

        let mut b = Crc16::new(INIT_HALO2);
        for bit in [true, false, true] {
            b.push_bit(bit);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn seed_changes_the_result() {
        let run = |init| {
            let mut crc = Crc16::new(init);
            crc.push_bytes(&[0x86, 0xBB, 0xEA, 0x9C]);
            crc.finish()
        };
        assert_ne!(run(INIT_HALO2), run(INIT_NRF24));
    }
}
