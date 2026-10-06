//! Clocking frames out of the RP2040 into a BM5602.
//!
//! # Why PIO
//!
//! The BM5602 is driven in synchronous serial mode: the *radio* produces the bit
//! clock on `GIO3`/`TBCLK`, and the host must present the next data bit while that
//! clock is low. At 125 kbps that is a new bit every 8 µs, timed to an external
//! edge the CPU does not control. Bit-banging it from an interrupt handler is
//! possible but jittery; a PIO state machine does it deterministically and
//! without waking the CPU at all.
//!
//! This is the one reason the RP2040 is a better fit for this job than an ESP32.
//!
//! # Program
//!
//! Three instructions, looping forever. `wait` is relative to the state machine's
//! `in_base`, `out` to its `out_base`, so the pin mapping is set up by the caller:
//!
//! ```pio
//! .wrap_target
//!     wait 1 pin 0    ; TBCLK high
//!     wait 0 pin 0    ; falling edge
//!     out  pins, 1    ; present the next bit while TBCLK is low
//! .wrap
//! ```
//!
//! Configure the state machine with autopull on, a threshold of 32 and
//! `out_shiftdir = Left`, so words pushed into the TX FIFO are shifted out
//! MSB-first — matching [`halo2_proto::bits::BitWriter`].
//!
//! # Status
//!
//! Not built or run on hardware yet. Pin numbers below are a proposal; confirm
//! them against `docs/WIRING.md` once the BM5602 module is in hand.

/// Proposed pin assignment on an RP2040 board.
pub mod pins {
    /// SPI clock to the BM5602 (register access).
    pub const SPI_SCK: u8 = 2;
    /// SPI MOSI to the BM5602.
    pub const SPI_MOSI: u8 = 3;
    /// SPI MISO from the BM5602.
    pub const SPI_MISO: u8 = 4;
    /// SPI chip select.
    pub const SPI_CSN: u8 = 5;
    /// Radio enable / transmit strobe.
    pub const CE: u8 = 6;
    /// `GIO3` from the radio, carrying `TBCLK`. Must be the state machine's `in_base`.
    pub const TBCLK: u8 = 7;
    /// Serial data to the radio. Must be the state machine's `out_base`.
    pub const DATA: u8 = 8;
}

/// The PIO program as source, assembled at build time by `pio_asm!`.
pub const PIO_SOURCE: &str = r"
.wrap_target
    wait 1 pin 0
    wait 0 pin 0
    out  pins, 1
.wrap
";

/// Number of 32-bit words needed to hold one frame in the TX FIFO.
///
/// A frame is 145 bits, so five words are pushed and the state machine is halted
/// after the 145th bit: the trailing padding of the last word must not reach the
/// radio.
pub const FRAME_WORDS: usize = halo2_proto::frame::FRAME_BITS.div_ceil(32);

#[cfg(test)]
mod tests {
    #[test]
    fn a_frame_needs_five_words_and_a_tail() {
        assert_eq!(super::FRAME_WORDS, 5);
        assert_eq!(halo2_proto::frame::FRAME_BITS % 32, 17);
    }
}
