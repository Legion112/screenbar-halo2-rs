# Capturing the link with a HackRF One

2405 MHz sits comfortably inside the HackRF's 1 MHz – 6 GHz range. The goal of a
capture session is two things: **your lamp's four-byte address**, and a **diff
table** mapping each remote action to its ten payload bytes.

## Settings that matter

| Setting | Value | Why |
| --- | --- | --- |
| Centre frequency | 2405 MHz | see [PROTOCOL.md](PROTOCOL.md) |
| Sample rate | 2–4 Msps | HackRF's floor is 2 Msps; plenty for 125 kbps |
| Modulation | FSK | |
| Samples per symbol | rate ÷ 125000 | at 2 Msps that is 16 |
| Bit length | 8 µs | the one parameter URH cannot guess |
| LNA / VGA | start low, the remote is centimetres away | avoid clipping |

## Procedure

```bash
# 1. Raw capture while pressing exactly one button.
hackrf_transfer -r power-on.cu8 -f 2405000000 -s 2000000 -a 0 -l 16 -g 20

# 2. Open in Universal Radio Hacker, set FSK + 8 µs, and find 10101010.
urh power-on.cu8
```

In URH's *Interpretation* tab the demodulated bits should start with the
preamble `10101010`, followed by 32 address bits. Everything after that is PCF,
payload and CRC.

## Checking a capture against this repo

Paste the bits URH shows (as hex, preamble included) into the decoder:

```bash
cargo run --bin halo2 -- decode --frame AA86BBEA9C2880008101820283038404C00400
```

A clean parse with a matching CRC confirms the framing *and* the `0xEFDF` seed
at once. If the CRC fails, try the other seed before assuming the capture is
bad:

```bash
cargo run --bin halo2 -- decode --frame <hex> --crc-init nrf24
```

If neither matches, whitening is the next suspect — see
[PROTOCOL.md](PROTOCOL.md).

## Building the diff table

One capture per action, changing exactly one thing each time:

1. Power on, power off.
2. Front brightness 1 %, 2 %, 50 %, 99 %, 100 % — find the byte that counts.
3. Rear brightness, same ladder — find which byte or bit selects the channel.
4. Colour temperature 2700 K, 3925 K, 6500 K.
5. Ultrasonic presence on, off.
6. The pairing exchange, on address `E2 08 00 B0`.

Record the payload hex next to each action. That table is what turns
`EncodeError::NotYetMapped` into real code in
[`halo2-proto/src/command.rs`](../halo2-proto/src/command.rs).

## Transmitting

A recorded capture can be replayed straight back with `hackrf_transfer -t`,
which is the quickest proof that the lamp will listen to something other than
its remote. For anything beyond a replay, use the BM5602 bridge: the HackRF is
half-duplex and has no packet engine, so it cannot transmit a command and watch
for the response in the same breath.

Keep transmit power low. This is the 2.4 GHz ISM band shared with Wi-Fi and
Bluetooth, and the lamp is on the same desk as the radio.
