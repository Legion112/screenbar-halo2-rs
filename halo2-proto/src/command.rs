//! Typed commands and their payload encoding.
//!
//! # Status
//!
//! The framing around the payload is known (see [`crate::frame`]), but the
//! **meaning of the ten payload bytes is not publicly documented** and is not
//! guessed here. Every semantic variant therefore returns
//! [`EncodeError::NotYetMapped`] until a capture fills in [`encode`](Command::payload).
//!
//! Until then [`Command::Raw`] carries payload bytes straight from a capture,
//! which is enough to replay commands end to end and to validate the radio path.
//! Filling the table in is step 2 of `docs/ROADMAP.md`.

use crate::frame::PAYLOAD_LEN;

/// Which of the two light channels a command addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// The downward-facing lamp over the desk.
    Front,
    /// The backlight facing the wall.
    Rear,
}

/// A value that fell outside the range the lamp accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange {
    /// What was being set.
    pub what: &'static str,
    /// Lowest accepted value.
    pub min: u16,
    /// Highest accepted value.
    pub max: u16,
    /// The value that was rejected.
    pub got: u16,
}

impl core::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Self {
            what,
            min,
            max,
            got,
        } = self;
        write!(f, "{what} must be {min}..={max}, got {got}")
    }
}

impl core::error::Error for OutOfRange {}

/// Brightness percentage, 1..=100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    /// Lowest brightness the lamp accepts.
    pub const MIN: u8 = 1;
    /// Highest brightness the lamp accepts.
    pub const MAX: u8 = 100;

    /// Creates a brightness, rejecting values outside 1..=100.
    ///
    /// # Errors
    /// Returns [`OutOfRange`] when `percent` is not in 1..=100.
    pub const fn new(percent: u8) -> Result<Self, OutOfRange> {
        if percent < Self::MIN || percent > Self::MAX {
            return Err(OutOfRange {
                what: "brightness",
                min: Self::MIN as u16,
                max: Self::MAX as u16,
                got: percent as u16,
            });
        }
        Ok(Self(percent))
    }

    /// Returns the percentage.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }
}

/// Correlated colour temperature in kelvin, 2700..=6500.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColorTemp(u16);

impl ColorTemp {
    /// Warmest setting.
    pub const MIN: u16 = 2700;
    /// Coolest setting.
    pub const MAX: u16 = 6500;

    /// Creates a colour temperature, rejecting values outside 2700..=6500.
    ///
    /// # Errors
    /// Returns [`OutOfRange`] when `kelvin` is not in 2700..=6500.
    pub const fn new(kelvin: u16) -> Result<Self, OutOfRange> {
        if kelvin < Self::MIN || kelvin > Self::MAX {
            return Err(OutOfRange {
                what: "colour temperature",
                min: Self::MIN,
                max: Self::MAX,
                got: kelvin,
            });
        }
        Ok(Self(kelvin))
    }

    /// Returns the temperature in kelvin.
    #[must_use]
    pub const fn kelvin(self) -> u16 {
        self.0
    }
}

/// Something the lamp can be told to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Payload bytes copied verbatim from a capture. Always encodable.
    Raw([u8; PAYLOAD_LEN]),
    /// Turn the lamp on or off.
    Power(bool),
    /// Make `channel` the one the remote's ring adjusts.
    Select(Channel),
    /// Set the brightness of one channel.
    SetBrightness {
        /// Channel to adjust.
        channel: Channel,
        /// Target brightness.
        value: Brightness,
    },
    /// Set the colour temperature of the front light.
    SetColorTemp(ColorTemp),
    /// Enable or disable the ultrasonic presence sensor.
    UltrasonicPresence(bool),
}

/// Why a command could not be turned into payload bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The payload layout for this command has not been reverse-engineered yet.
    ///
    /// Capture the command off the original remote and extend
    /// [`Command::payload`]; see `docs/HACKRF.md`.
    NotYetMapped(&'static str),
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotYetMapped(what) => write!(
                f,
                "payload layout for `{what}` is not mapped yet; send Command::Raw from a capture"
            ),
        }
    }
}

impl core::error::Error for EncodeError {}

impl Command {
    /// Builds the ten payload bytes for this command.
    ///
    /// # Errors
    /// Returns [`EncodeError::NotYetMapped`] for every variant except
    /// [`Command::Raw`], because the payload layout is still unknown.
    pub const fn payload(&self) -> Result<[u8; PAYLOAD_LEN], EncodeError> {
        match self {
            Self::Raw(bytes) => Ok(*bytes),
            // TODO(roadmap step 2): fill these in from the diff table produced by
            // a HackRF capture. Keep each arm a pure function of its inputs so the
            // host tests can pin it to recorded frames.
            Self::Power(_) => Err(EncodeError::NotYetMapped("power")),
            Self::Select(_) => Err(EncodeError::NotYetMapped("select channel")),
            Self::SetBrightness { .. } => Err(EncodeError::NotYetMapped("brightness")),
            Self::SetColorTemp(_) => Err(EncodeError::NotYetMapped("colour temperature")),
            Self::UltrasonicPresence(_) => Err(EncodeError::NotYetMapped("ultrasonic presence")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_payload_passes_through() {
        let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(Command::Raw(bytes).payload(), Ok(bytes));
    }

    #[test]
    fn brightness_range_is_enforced() {
        assert_eq!(Brightness::new(1).map(Brightness::percent), Ok(1));
        assert_eq!(Brightness::new(100).map(Brightness::percent), Ok(100));
        assert!(Brightness::new(0).is_err());
        assert!(Brightness::new(101).is_err());
    }

    #[test]
    fn colour_temperature_range_is_enforced() {
        assert_eq!(ColorTemp::new(2700).map(ColorTemp::kelvin), Ok(2700));
        assert_eq!(ColorTemp::new(6500).map(ColorTemp::kelvin), Ok(6500));
        assert!(ColorTemp::new(2699).is_err());
        assert!(ColorTemp::new(6501).is_err());
        // 3925 K is the value the address-discovery procedure asks you to dial in.
        assert!(ColorTemp::new(3925).is_ok());
    }

    /// Guards the honest gap: these must stay unmapped until a capture proves a
    /// layout. When you implement one, replace its line with a real assertion.
    #[test]
    fn semantic_commands_are_not_mapped_yet() {
        let brightness = Brightness::new(40).unwrap();
        for command in [
            Command::Power(true),
            Command::Select(Channel::Front),
            Command::SetBrightness {
                channel: Channel::Rear,
                value: brightness,
            },
            Command::SetColorTemp(ColorTemp::new(4000).unwrap()),
            Command::UltrasonicPresence(false),
        ] {
            assert!(matches!(
                command.payload(),
                Err(EncodeError::NotYetMapped(_))
            ));
        }
    }
}
