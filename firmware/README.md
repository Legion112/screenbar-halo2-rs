# halo2-firmware

RP2040 bridge: takes frames from the host over USB CDC and clocks them into a
BM5602 radio module.

## Status

**Skeleton only.** Nothing here has been compiled or run on hardware yet; the
embassy dependencies in `Cargo.toml` are commented out until the board and the
radio module are actually on the desk. What *is* here is the part worth writing
down early: the PIO program and the reasoning behind it, in `src/pio_tx.rs`.

The host crates (`halo2-proto`, `halo2-cli`) build and test independently of
this directory and are excluded from each other's workspace.

## Why RP2040

The radio generates the bit clock (`TBCLK` on `GIO3`) and the host has 8 µs to
present each bit. A PIO state machine tracks that external clock exactly, with
no CPU involvement and no jitter. On an ESP32 the same job needs RMT tricks or a
GPIO interrupt per bit.

RP2350 works too — switch the target in `.cargo/config.toml`.

## Build (once dependencies are filled in)

```bash
rustup target add thumbv6m-none-eabi
cargo build --release
cargo run --release   # needs probe-rs and a debug probe
```
