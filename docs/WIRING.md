# Wiring

**Proposal, not yet verified on hardware.** Pin numbers mirror
`firmware/src/pio_tx.rs::pins`.

| BM5602 | RP2040 GPIO | Notes |
| --- | --- | --- |
| VCC | 3V3 | the BC5602 runs 1.9–3.6 V — **not** 5 V |
| GND | GND | |
| SCK | 2 | SPI, register access |
| MOSI | 3 | SPI |
| MISO | 4 | SPI |
| CSN | 5 | SPI chip select |
| CE | 6 | transmit strobe |
| GIO3 / TBCLK | 7 | **the one that matters** — PIO `in_base` |
| DATA | 8 | PIO `out_base` |

That is seven signal wires plus power, matching what the existing bridge
projects report.

`TBCLK` is the wire people forget. Without it the radio has no way to tell the
MCU when to present the next bit, and the link silently never works.

Keep the SPI leads short. 125 kbps is slow, but the 2.4 GHz front end is not.
