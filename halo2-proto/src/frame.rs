//! On-air frame layout.
//!
//! ```text
//! | preamble | address | PCF    | payload  | CRC     |
//! | 8 bits   | 32 bits | 9 bits | 80 bits  | 16 bits |   = 145 bits
//! ```
//!
//! Everything is MSB-first. The CRC covers address + PCF + payload; the preamble
//! is excluded. The payload observed on the Halo 2 link is always 10 bytes even
//! though the PCF carries a length field, which is why [`PAYLOAD_LEN`] is fixed.

use crate::bits::{BitReader, BitWriter};
use crate::crc::Crc16;

/// Preamble byte that opens every frame.
pub const PREAMBLE: u8 = 0b1010_1010;

/// Address width in bytes.
pub const ADDRESS_LEN: usize = 4;

/// Payload width in bytes, constant on this link.
pub const PAYLOAD_LEN: usize = 10;

/// Width of the packet control field, in bits.
pub const PCF_BITS: u8 = 9;

/// Total frame width in bits.
pub const FRAME_BITS: usize = 8 + ADDRESS_LEN * 8 + PCF_BITS as usize + PAYLOAD_LEN * 8 + 16;

/// Bytes needed to hold [`FRAME_BITS`], with the last byte zero-padded.
pub const FRAME_BYTES: usize = FRAME_BITS.div_ceil(8);

/// A four-byte link address, held in **on-air** byte order.
///
/// The BC5602 address registers are written in the reverse order: a lamp whose
/// registers read `9C EA BB 86` transmits `86 BB EA 9C`. Use
/// [`from_register_order`](Address::from_register_order) to convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address(pub [u8; ADDRESS_LEN]);

impl Address {
    /// Address the remote and the lamp use while pairing, before the lamp's own
    /// unique address is known.
    pub const PAIRING: Self = Self([0xE2, 0x08, 0x00, 0xB0]);

    /// Builds an address from bytes in on-air order.
    #[must_use]
    pub const fn from_air_order(bytes: [u8; ADDRESS_LEN]) -> Self {
        Self(bytes)
    }

    /// Builds an address from bytes as they appear in the BC5602 registers.
    #[must_use]
    pub const fn from_register_order(mut bytes: [u8; ADDRESS_LEN]) -> Self {
        bytes.reverse();
        Self(bytes)
    }

    /// Returns the bytes in on-air order.
    #[must_use]
    pub const fn air_order(self) -> [u8; ADDRESS_LEN] {
        self.0
    }

    /// Returns the bytes in BC5602 register order.
    #[must_use]
    pub const fn register_order(self) -> [u8; ADDRESS_LEN] {
        let mut bytes = self.0;
        bytes.reverse();
        bytes
    }
}

/// Packet control field.
///
/// The 6/2/1 split matches `nRF24` Enhanced `ShockBurst` and reproduces every
/// captured frame so far, but it is still a hypothesis — see `docs/PROTOCOL.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcf {
    /// Payload length in bytes, 6 bits.
    pub length: u8,
    /// Packet identity, 2 bits. Lets the receiver drop retransmissions.
    pub pid: u8,
    /// `true` when the sender does not expect an acknowledgement.
    pub no_ack: bool,
}

impl Pcf {
    /// PCF for a standard 10-byte unacknowledged command with the given `pid`.
    #[must_use]
    pub const fn command(pid: u8) -> Self {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "PAYLOAD_LEN is 10, far below u8::MAX"
        )]
        Self {
            length: PAYLOAD_LEN as u8,
            pid: pid & 0b11,
            no_ack: true,
        }
    }

    /// Packs the field into its low 9 bits.
    #[must_use]
    pub const fn to_bits(self) -> u16 {
        ((self.length as u16 & 0b11_1111) << 3)
            | ((self.pid as u16 & 0b11) << 1)
            | self.no_ack as u16
    }

    /// Unpacks the field from the low 9 bits of `raw`.
    #[must_use]
    pub const fn from_bits(raw: u16) -> Self {
        Self {
            length: ((raw >> 3) & 0b11_1111) as u8,
            pid: ((raw >> 1) & 0b11) as u8,
            no_ack: raw & 1 != 0,
        }
    }
}

/// A complete command frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Destination address.
    pub address: Address,
    /// Packet control field.
    pub pcf: Pcf,
    /// Command payload. See [`crate::command`] for what the bytes mean.
    pub payload: [u8; PAYLOAD_LEN],
}

/// Why a frame could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The bit stream ended before a full frame was read.
    Truncated,
    /// The leading byte was not [`PREAMBLE`].
    BadPreamble,
    /// The trailing checksum did not match.
    BadCrc {
        /// Checksum carried by the frame.
        found: u16,
        /// Checksum computed over the received bits.
        expected: u16,
    },
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated => write!(f, "bit stream ended mid-frame"),
            Self::BadPreamble => write!(f, "frame does not start with the preamble"),
            Self::BadCrc { found, expected } => {
                write!(
                    f,
                    "crc mismatch: frame carries {found:#06x}, computed {expected:#06x}"
                )
            }
        }
    }
}

impl core::error::Error for FrameError {}

impl Frame {
    /// Builds an unacknowledged command frame.
    #[must_use]
    pub const fn command(address: Address, pid: u8, payload: [u8; PAYLOAD_LEN]) -> Self {
        Self {
            address,
            pcf: Pcf::command(pid),
            payload,
        }
    }

    /// Computes the frame checksum with the given seed.
    #[must_use]
    pub const fn checksum(&self, crc_init: u16) -> u16 {
        let mut crc = Crc16::new(crc_init);
        crc.push_bytes(&self.address.air_order());
        let pcf = self.pcf.to_bits();
        let mut i = 0;
        while i < PCF_BITS {
            crc.push_bit((pcf >> (PCF_BITS - 1 - i)) & 1 != 0);
            i += 1;
        }
        crc.push_bytes(&self.payload);
        crc.finish()
    }

    /// Serialises the frame, preamble included, ready to be clocked into the radio.
    #[must_use]
    pub fn encode(&self, crc_init: u16) -> BitWriter<FRAME_BYTES> {
        let mut w = BitWriter::new();
        w.push_byte(PREAMBLE);
        w.push_bytes(&self.address.air_order());
        w.push_u16(self.pcf.to_bits(), PCF_BITS);
        w.push_bytes(&self.payload);
        w.push_u16(self.checksum(crc_init), 16);
        debug_assert_eq!(w.len(), FRAME_BITS);
        w
    }

    /// Parses a frame that starts with the preamble and verifies its checksum.
    ///
    /// # Errors
    /// Returns [`FrameError`] if the stream is short, the preamble is wrong, or
    /// the checksum does not match `crc_init`.
    pub fn decode(bits: &[u8], crc_init: u16) -> Result<Self, FrameError> {
        let mut r = BitReader::new(bits);
        match r.read_byte() {
            Some(PREAMBLE) => {}
            Some(_) => return Err(FrameError::BadPreamble),
            None => return Err(FrameError::Truncated),
        }

        let mut address = [0u8; ADDRESS_LEN];
        if !r.read_bytes(&mut address) {
            return Err(FrameError::Truncated);
        }
        let pcf = r.read_u16(PCF_BITS).ok_or(FrameError::Truncated)?;
        let mut payload = [0u8; PAYLOAD_LEN];
        if !r.read_bytes(&mut payload) {
            return Err(FrameError::Truncated);
        }
        let found = r.read_u16(16).ok_or(FrameError::Truncated)?;

        let frame = Self {
            address: Address::from_air_order(address),
            pcf: Pcf::from_bits(pcf),
            payload,
        };
        let expected = frame.checksum(crc_init);
        if found == expected {
            Ok(frame)
        } else {
            Err(FrameError::BadCrc { found, expected })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crc::{INIT_HALO2, INIT_NRF24};

    const ADDR: Address = Address::from_register_order([0x9C, 0xEA, 0xBB, 0x86]);

    #[test]
    fn register_order_is_reversed_on_air() {
        assert_eq!(ADDR.air_order(), [0x86, 0xBB, 0xEA, 0x9C]);
        assert_eq!(ADDR.register_order(), [0x9C, 0xEA, 0xBB, 0x86]);
    }

    #[test]
    fn frame_is_145_bits() {
        assert_eq!(FRAME_BITS, 145);
        assert_eq!(FRAME_BYTES, 19);
        let frame = Frame::command(ADDR, 0, [0; PAYLOAD_LEN]);
        assert_eq!(frame.encode(INIT_HALO2).len(), FRAME_BITS);
    }

    #[test]
    fn encode_decode_round_trip() {
        let payload = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32];
        let frame = Frame::command(ADDR, 2, payload);
        let encoded = frame.encode(INIT_HALO2);
        let decoded = Frame::decode(encoded.filled(), INIT_HALO2).expect("round trip");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn pcf_round_trips() {
        let pcf = Pcf::command(3);
        assert_eq!(Pcf::from_bits(pcf.to_bits()), pcf);
        assert_eq!(pcf.length, 10);
        assert!(pcf.no_ack);
    }

    #[test]
    fn preamble_starts_the_frame() {
        let frame = Frame::command(ADDR, 0, [0; PAYLOAD_LEN]);
        assert_eq!(frame.encode(INIT_HALO2).filled()[0], PREAMBLE);
        assert_eq!(
            Frame::decode(&[0x55; FRAME_BYTES], INIT_HALO2),
            Err(FrameError::BadPreamble)
        );
    }

    #[test]
    fn wrong_seed_fails_the_checksum() {
        let frame = Frame::command(ADDR, 1, [0xAA; PAYLOAD_LEN]);
        let encoded = frame.encode(INIT_HALO2);
        assert!(matches!(
            Frame::decode(encoded.filled(), INIT_NRF24),
            Err(FrameError::BadCrc { .. })
        ));
    }

    #[test]
    fn truncated_stream_is_rejected() {
        let frame = Frame::command(ADDR, 0, [0; PAYLOAD_LEN]);
        let encoded = frame.encode(INIT_HALO2);
        let short = &encoded.filled()[..10];
        assert_eq!(Frame::decode(short, INIT_HALO2), Err(FrameError::Truncated));
    }

    #[test]
    fn a_flipped_payload_bit_is_caught() {
        let frame = Frame::command(ADDR, 0, [0; PAYLOAD_LEN]);
        let mut bytes = *frame.encode(INIT_HALO2).bytes();
        bytes[8] ^= 0b0000_0100;
        assert!(matches!(
            Frame::decode(&bytes, INIT_HALO2),
            Err(FrameError::BadCrc { .. })
        ));
    }
}
