# Roadmap

Ordered so that each step can be verified before the next one depends on it, and
so the parts that need no hardware come first.

## 1. Confirm the protocol on my own lamp — *HackRF only*

Capture the remote, parse the frames with `halo2 decode`, and settle the three
open questions from [PROTOCOL.md](PROTOCOL.md): the CRC seed, the PCF layout,
and whether whitening is on. Output: my lamp's address, and a protocol doc with
no **unverified** tags left.

**Done when** a frame captured off the air round-trips through `halo2 encode`
byte for byte.

## 2. Map the payload — *HackRF only*

Build the diff table from [HACKRF.md](HACKRF.md) and implement
`Command::payload` for every variant. Each arm gets a host test pinned to a
recorded frame.

**Done when** `semantic_commands_are_not_mapped_yet` is deleted and replaced by
real assertions.

## 3. Transmit from the bridge — *needs BM5602 + RP2040*

Bring up SPI register access to the BC5602, then the PIO transmitter from
`firmware/src/pio_tx.rs`. Verify with the HackRF in receive mode: frames sent by
the bridge must decode identically to frames sent by the remote.

**Done when** the lamp responds to a command from the Pico.

## 4. Host control over USB CDC

`embassy-usb` CDC on the firmware, `halo2 send` on the host. This is the point
where the lamp gains the USB control it never shipped with — through my own
dongle rather than its own port.

**Done when** `halo2 send --payload …` changes the light.

## 5. Receive path

Listen on the lamp's address and decode frames from the original remote, so
turning the physical ring keeps the host's idea of the state correct. Without
this the state is open-loop and drifts.

**Done when** turning the remote updates the host.

## Later, maybe

- Wi-Fi and MQTT (`embassy-net` + `cyw43`) for Home Assistant, as an alternative
  front end to USB rather than a replacement.
- Probe undocumented opcodes: what does the lamp do with payloads the remote
  never sends?
- Confirm whether the link is one-way or the lamp ever talks back.
