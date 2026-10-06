//! RP2040 bridge between the host and the Halo 2 radio link.
//!
//! Planned shape:
//!
//! 1. Bring up SPI and program the BC5602 registers (frequency, 125 kbps GFSK,
//!    synchronous serial mode, the lamp's address).
//! 2. Accept `TX <address> <payload> <repeat>` lines over USB CDC from
//!    [`halo2-cli`](../../halo2-cli).
//! 3. Build the frame with `halo2_proto`, push it to the PIO state machine in
//!    [`pio_tx`], strobe `CE`.
//! 4. Later: receive mode, so frames from the original remote are forwarded back
//!    to the host and the lamp's state can be tracked.
//!
//! None of this is written yet — see `firmware/README.md`.

#![no_std]
#![no_main]

pub mod pio_tx;
