# screenbar-halo2-rs

Rust tooling for talking to a **BenQ ScreenBar Halo 2** over its 2.4 GHz remote
link.

The lamp has no USB data interface, no vendor SDK and no desktop software — its
USB-C carries power only, and the wireless remote is the only control path. This
repo aims at the other end of that link: decode it, then drive it from a host.

## Status

Early. Honest summary of what exists:

| Piece | State |
| --- | --- |
| Frame encode/decode, CRC, addressing | **working**, 26 tests green |
| `halo2 encode` / `halo2 decode` CLI | **working**, no hardware needed |
| Payload byte meanings | **not mapped** — not publicly documented, and not guessed here |
| RP2040 + BM5602 firmware | skeleton; PIO program written, not compiled |
| `halo2 send` | writes to the bridge; the bridge does not exist yet |

The payload gap is deliberate. The framing is reverse-engineered and verifiable;
the command semantics are not published anywhere I could find, so every typed
command returns `EncodeError::NotYetMapped` until a capture proves a layout.
`Command::Raw` carries captured bytes in the meantime, which is enough to replay.

## Try it without hardware

```bash
cargo run --bin halo2 -- encode --address 9CEABB86 --payload 00010203040506070809
```

```
AA86BBEA9C2880008101820283038404C00400
# 145 bits, crc 0x8008
```

```bash
cargo run --bin halo2 -- decode --frame AA86BBEA9C2880008101820283038404C00400
```

```
address (on air)  86BBEA9C
address (register) 9CEABB86
pcf length=10 pid=0 no_ack=true
payload           00010203040506070809
```

Point `decode` at bits lifted out of Universal Radio Hacker and it will tell you
whether the framing assumptions in this repo survive contact with your lamp.

## Layout

```
halo2-proto/   no_std codec: bits, CRC, frame, commands. Runs on host and MCU.
halo2-cli/     host tool: encode, decode, send. No dependencies outside std.
firmware/      RP2040 bridge skeleton. Own workspace, own target.
docs/          protocol, HackRF capture procedure, roadmap, wiring.
```

`halo2-proto` is `no_std`, allocation-free and dependency-free, so the same
encoder that the host tests exercise is the one that ends up in the firmware.

## Where this is going

[docs/ROADMAP.md](docs/ROADMAP.md). Short version: confirm the protocol with a
HackRF, map the payload, build an RP2040 + BM5602 bridge, expose it to the host
over USB CDC — giving the lamp the USB control it never shipped with, through a
dongle of my own.

[docs/PROTOCOL.md](docs/PROTOCOL.md) is the reference, with every unconfirmed
claim tagged **unverified**.

## Security note, since people ask

The link has **no encryption, no authentication and no rolling code**. A
four-byte address is the only thing separating a lamp from any transmitter in
range, and it is recoverable passively from a single button press. For a desk
lamp that is not a crisis, but do not expect your commands to be private — or
exclusive.

## Credits

Standing on published work by others; see [NOTICE](NOTICE). This repo
reimplements the protocol in Rust rather than porting any single project.

## License

MIT — see [LICENSE](LICENSE).
