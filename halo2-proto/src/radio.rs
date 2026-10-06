//! Link parameters of the Halo 2 radio.
//!
//! The remote is built around a Holtek BC5602; the usual bridge hardware is the
//! BM5602-60-1 module. Values here describe the link, not the register map —
//! register programming lives in the radio driver.

/// Carrier frequency in MHz.
///
/// Both community projects agree on 2405 MHz but disagree on the channel index
/// (one says channel 1, the other channel 5), so trust the frequency and derive
/// the channel from whichever register map the radio you use actually has.
pub const FREQUENCY_MHZ: u16 = 2405;

/// Over-the-air bit rate.
pub const BITRATE_BPS: u32 = 125_000;

/// Nominal time of one bit, in nanoseconds. At 125 kbps this is 8 µs, which is
/// the budget the transmitter has to present the next bit after `TBCLK` falls.
pub const BIT_PERIOD_NS: u32 = 1_000_000_000 / BITRATE_BPS;

/// Modulation used on the link.
pub const MODULATION: &str = "GFSK";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_period_is_eight_microseconds() {
        assert_eq!(BIT_PERIOD_NS, 8_000);
    }
}
