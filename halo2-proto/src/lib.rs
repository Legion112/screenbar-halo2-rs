//! Codec for the 2.4 GHz link between a `BenQ ScreenBar Halo 2` and its wireless
//! remote.
//!
//! The crate is `no_std` and allocation-free, so the same code runs in the host
//! test suite and on the microcontroller that drives the radio.
//!
//! ```
//! use halo2_proto::{crc::INIT_HALO2, frame::{Address, Frame}, command::Command};
//!
//! let address = Address::from_register_order([0x9C, 0xEA, 0xBB, 0x86]);
//! let payload = Command::Raw([0; 10]).payload().unwrap();
//! let bits = Frame::command(address, 0, payload).encode(INIT_HALO2);
//! assert_eq!(bits.len(), 145);
//! ```
//!
//! # What is known and what is not
//!
//! Framing, addressing and the checksum are reverse-engineered and implemented
//! here. The meaning of the ten payload bytes is not published anywhere and is
//! deliberately **not** guessed — see [`command`] and `docs/PROTOCOL.md`.

#![no_std]

pub mod bits;
pub mod command;
pub mod crc;
pub mod frame;
pub mod radio;

pub use command::{Brightness, Channel, ColorTemp, Command};
pub use frame::{Address, Frame, FrameError};
