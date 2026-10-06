//! `halo2` — host-side companion to the firmware bridge.
//!
//! Two of the three subcommands work with no hardware at all, which makes them
//! the first thing to reach for while reverse-engineering a capture:
//!
//! * `encode` builds a complete on-air frame and prints it as hex, so a capture
//!   recorded with a `HackRF` can be compared against it bit for bit.
//! * `decode` parses a frame lifted out of Universal Radio Hacker and verifies
//!   the checksum.
//! * `send` writes a frame to the USB CDC bridge for transmission.
//!
//! The wire format towards the bridge is deliberately plain text, one command
//! per line, so it can be driven from a terminal while bringing the board up.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::ExitCode;

use halo2_proto::crc::{INIT_HALO2, INIT_NRF24};
use halo2_proto::frame::{Address, Frame, PAYLOAD_LEN};

const USAGE: &str = "\
halo2 — ScreenBar Halo 2 link tool

USAGE:
    halo2 encode --address <HEX8> --payload <HEX20> [--pid N] [--crc-init HEX4]
    halo2 decode --frame <HEX> [--crc-init HEX4]
    halo2 send   --address <HEX8> --payload <HEX20> [--device PATH] [--repeat N]

OPTIONS:
    --address   4-byte address in BC5602 register order, e.g. 9CEABB86
    --payload   10 payload bytes as hex, e.g. 00010203040506070809
    --frame     captured frame as hex, preamble included
    --pid       2-bit packet id, 0..=3 (default 0)
    --crc-init  CRC seed: halo2 (0xEFDF, default) or nrf24 (0xFFFF) or raw hex
    --device    serial device of the bridge (default /dev/ttyACM0)
    --repeat    how many times the bridge should transmit the frame (default 1)

The address is printed by `halo2 decode` on any frame your own remote sends;
see docs/HACKRF.md for how to capture one.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(subcommand) = args.first() else {
        print!("{USAGE}");
        return Ok(());
    };
    let opts = Options::parse(&args[1..])?;

    match subcommand.as_str() {
        "encode" => encode(&opts),
        "decode" => decode(&opts),
        "send" => send(&opts),
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown subcommand `{other}`\n\n{USAGE}")),
    }
}

fn encode(opts: &Options) -> Result<(), String> {
    let frame = opts.frame_from_parts()?;
    let bits = frame.encode(opts.crc_init()?);
    println!("{}", to_hex(bits.filled()));
    println!(
        "# {} bits, crc {:#06x}",
        bits.len(),
        frame.checksum(opts.crc_init()?)
    );
    Ok(())
}

fn decode(opts: &Options) -> Result<(), String> {
    let raw = opts.require("--frame")?;
    let bytes = from_hex(raw)?;
    let frame = Frame::decode(&bytes, opts.crc_init()?).map_err(|e| e.to_string())?;
    println!("address (on air)  {}", to_hex(&frame.address.air_order()));
    println!(
        "address (register) {}",
        to_hex(&frame.address.register_order())
    );
    println!(
        "pcf length={} pid={} no_ack={}",
        frame.pcf.length, frame.pcf.pid, frame.pcf.no_ack
    );
    println!("payload           {}", to_hex(&frame.payload));
    Ok(())
}

fn send(opts: &Options) -> Result<(), String> {
    let frame = opts.frame_from_parts()?;
    let device = opts.get("--device").unwrap_or("/dev/ttyACM0");
    let repeat: u8 = opts
        .get("--repeat")
        .unwrap_or("1")
        .parse()
        .map_err(|_| "--repeat expects a number 1..=255".to_string())?;

    let line = format!(
        "TX {} {} {}\n",
        to_hex(&frame.address.air_order()),
        to_hex(&frame.payload),
        repeat
    );

    let mut port = OpenOptions::new()
        .write(true)
        .open(device)
        .map_err(|e| format!("cannot open {device}: {e}"))?;
    port.write_all(line.as_bytes())
        .map_err(|e| format!("cannot write to {device}: {e}"))?;
    print!("sent to {device}: {line}");
    Ok(())
}

/// Minimal `--flag value` parser. Deliberately dependency-free so the tool
/// builds on a machine with no crates.io access.
struct Options {
    flags: Vec<(String, String)>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut flags = Vec::new();
        let mut i = 0;
        while i < args.len() {
            let key = &args[i];
            if !key.starts_with("--") {
                return Err(format!("expected a --flag, found `{key}`"));
            }
            let value = args
                .get(i + 1)
                .ok_or_else(|| format!("`{key}` needs a value"))?;
            flags.push((key.clone(), value.clone()));
            i += 2;
        }
        Ok(Self { flags })
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn require(&self, key: &str) -> Result<&str, String> {
        self.get(key).ok_or_else(|| format!("`{key}` is required"))
    }

    fn crc_init(&self) -> Result<u16, String> {
        match self.get("--crc-init") {
            None | Some("halo2") => Ok(INIT_HALO2),
            Some("nrf24") => Ok(INIT_NRF24),
            Some(raw) => u16::from_str_radix(raw.trim_start_matches("0x"), 16)
                .map_err(|_| format!("`--crc-init` expects halo2, nrf24 or hex, got `{raw}`")),
        }
    }

    fn frame_from_parts(&self) -> Result<Frame, String> {
        let address = from_hex(self.require("--address")?)?;
        let address: [u8; 4] = address
            .try_into()
            .map_err(|_| "`--address` must be exactly 4 bytes".to_string())?;

        let payload = from_hex(self.require("--payload")?)?;
        let payload: [u8; PAYLOAD_LEN] = payload
            .try_into()
            .map_err(|_| format!("`--payload` must be exactly {PAYLOAD_LEN} bytes"))?;

        let pid: u8 = self
            .get("--pid")
            .unwrap_or("0")
            .parse()
            .map_err(|_| "`--pid` expects 0..=3".to_string())?;

        Ok(Frame::command(
            Address::from_register_order(address),
            pid,
            payload,
        ))
    }
}

fn from_hex(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-')
        .collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err(format!("`{input}` has an odd number of hex digits"));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| format!("`{}` is not a hex byte", &cleaned[i..i + 2]))
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02X}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let bytes = from_hex("9C:EA BB-86").unwrap();
        assert_eq!(bytes, vec![0x9C, 0xEA, 0xBB, 0x86]);
        assert_eq!(to_hex(&bytes), "9CEABB86");
    }

    #[test]
    fn odd_hex_is_rejected() {
        assert!(from_hex("ABC").is_err());
        assert!(from_hex("ZZ").is_err());
    }

    #[test]
    fn encode_then_decode_matches() {
        let args = [
            "--address".to_string(),
            "9CEABB86".to_string(),
            "--payload".to_string(),
            "00010203040506070809".to_string(),
        ];
        let opts = Options::parse(&args).unwrap();
        let frame = opts.frame_from_parts().unwrap();
        let encoded = frame.encode(INIT_HALO2);
        let decoded = Frame::decode(encoded.filled(), INIT_HALO2).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(decoded.address.register_order(), [0x9C, 0xEA, 0xBB, 0x86]);
    }

    #[test]
    fn crc_init_aliases_resolve() {
        let halo2 = Options::parse(&[]).unwrap();
        assert_eq!(halo2.crc_init(), Ok(INIT_HALO2));

        let raw = Options::parse(&["--crc-init".to_string(), "0x1234".to_string()]).unwrap();
        assert_eq!(raw.crc_init(), Ok(0x1234));
    }

    #[test]
    fn unpaired_flag_is_an_error() {
        assert!(Options::parse(&["--address".to_string()]).is_err());
        assert!(Options::parse(&["oops".to_string()]).is_err());
    }
}
