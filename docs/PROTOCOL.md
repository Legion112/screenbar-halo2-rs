# The Halo 2 link

Everything on this page comes from public reverse-engineering work by others
(credited in [NOTICE](../NOTICE)) plus the BC5602 datasheet. Nothing here has
been confirmed against my own lamp yet — that is step 1 of the
[roadmap](ROADMAP.md). Lines marked **unverified** are the ones to check first.

## Physical layer

| Property | Value | Source |
| --- | --- | --- |
| Radio in the remote | Holtek BC5602 | teardown |
| Usual bridge module | BM5602-60-1 | both bridge projects |
| Carrier | 2405 MHz | both projects agree |
| Channel index | 1 or 5 — **sources disagree** | see note |
| Modulation | GFSK | BC5602 datasheet |
| Bit rate | 125 kbps (8 µs/bit) | both projects |
| Bit order | MSB first | both projects |
| Data valid | updated while `TBCLK` is low | wiring notes |

The channel disagreement is cosmetic: 2405 MHz is 3 MHz above the 2402 MHz band
floor, and different register maps number channels differently. Trust the
frequency.

## Frame

```
| preamble | address | PCF    | payload  | CRC     |
| 8 bits   | 32 bits | 9 bits | 80 bits  | 16 bits |   = 145 bits
0b10101010
```

Implemented in [`halo2-proto/src/frame.rs`](../halo2-proto/src/frame.rs).

* **Address.** Four bytes. The BC5602 registers hold it in the reverse of the
  on-air order: registers `9C EA BB 86` transmit as `86 BB EA 9C`.
* **PCF.** Nine bits. The 6-bit length / 2-bit PID / 1-bit no-ack split is
  **unverified** — it is the nRF24 Enhanced ShockBurst layout and it fits, but
  nobody has published a proof. The payload is always 10 bytes in practice even
  though the length field could say otherwise.
* **CRC.** CRC-CCITT, polynomial `0x1021`, MSB-first, not reflected, computed
  over address + PCF + payload. Initial state reported as `0xEFDF`
  — **unverified**, and unusual: a stock nRF24 link seeds `0xFFFF`. `halo2-cli`
  takes `--crc-init` so a capture can decide between them.
* **Whitening.** The BC5602 supports data whitening. Whether the Halo 2 enables
  it is **unknown**. If payloads from a capture look like noise that still
  passes CRC, whitening is on and the PRBS has to be undone — that would also
  explain the odd CRC seed.

## Addressing and pairing

Each lamp answers to its own unique four-byte address, distinct from the address
used while pairing. The pairing exchange between remote and lamp uses
`E2 08 00 B0` — exposed as `Address::PAIRING`.

There is **no encryption, no authentication and no rolling counter**. The
address is the only thing standing between a lamp and any transmitter nearby.
The published address-discovery trick is itself the proof: you set the remote to
10 % brightness and 3925 K and look for those values in the payload, which only
works on plaintext.

## Payload

**Not publicly documented, and deliberately not guessed in this repo.**

`halo2_proto::Command` has typed variants for power, channel select, brightness,
colour temperature and the ultrasonic presence sensor; all of them currently
return `EncodeError::NotYetMapped`. `Command::Raw` carries bytes straight from a
capture, which is enough to replay commands and validate the radio path end to
end. Filling the table in is [roadmap](ROADMAP.md) step 2.

## Transport, not a protocol

The lamp's own USB-C is power only, and the remote's USB-C charges it. There is
no USB HID interface, no vendor SDK and no desktop software from BenQ. The radio
link is the only way in.
