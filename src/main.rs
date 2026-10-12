use std::{thread, time::Duration};

const RAZER_VID: u16 = 0x1532;
const REPORT_LEN: usize = 90;

const STATUS_BUSY: u8 = 0x01;
const STATUS_OK: u8 = 0x02;

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("devices") => devices(),
        Some("info") => info(),
        Some("status") => status(),
        _ => {
            eprintln!("usage: fang devices | info | status");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("fang: {e}");
        std::process::exit(1);
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn devices() -> Result<()> {
    let api = hidapi::HidApi::new()?;
    for d in api.device_list().filter(|d| d.vendor_id() == RAZER_VID) {
        println!(
            "{:04x}:{:04x} if={} usage={:04x}/{:04x} {}",
            d.vendor_id(),
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            d.product_string().unwrap_or("?"),
        );
    }
    Ok(())
}

fn info() -> Result<()> {
    let mouse = Mouse::open()?;
    let serial = mouse.query(0x00, 0x82, &[0; 0x16])?;
    let fw = mouse.query(0x00, 0x81, &[0; 2])?;
    let end = serial.iter().position(|&b| b == 0).unwrap_or(serial.len());
    println!("serial   {}", String::from_utf8_lossy(&serial[..end]));
    println!("firmware {}.{}", fw[0], fw[1]);
    Ok(())
}

/// Read-only dump of everything stored on the mouse. Args mirror what Synapse sent.
fn status() -> Result<()> {
    let mouse = Mouse::open()?;
    let padded =|class, id, head: &[u8], size| {
        let mut args = vec![0u8; size];
        args[..head.len()].copy_from_slice(head);
        mouse.query(class, id, &args)
    };

    let mode = padded(0x00, 0x84, &[], 2)?;
    let mode_name = match mode[0] {
        0x00 => "normal",
        0x03 => "driver (host software in control)",
        _ => "unknown",
    };
    println!("device mode   {:02x} {mode_name}", mode[0]);

    let poll = padded(0x00, 0x8e, &[0x01], 2)?;
    let hz = match poll[1] {
        0x01 => "1000",
        0x02 => "500",
        0x04 => "250",
        0x08 => "125",
        _ => "?",
    };
    println!("polling       {hz} Hz (raw {:02x})", poll[1]);

    let dpi = padded(0x04, 0x85, &[0x01], 7)?;
    println!("dpi now       {} x {}", be16(&dpi[1..]), be16(&dpi[3..]));

    let stages = padded(0x04, 0x86, &[0x01], 0x50)?;
    let (active, count) = (stages[1], stages[2]);
    for s in stages[3..].chunks(7).take(count as usize) {
        let mark = if s[0] == active { "*" } else { " " };
        println!("dpi stage {}{mark}  {} x {}", s[0], be16(&s[1..]), be16(&s[3..]));
    }

    // ponytail: lift-off reads are guesses (write id | 0x80, same args); print raw.
    for (id, head) in [(0x83, &[0x00, 0x04][..]), (0x8b, &[0x00, 0x04, 0x01]), (0x85, &[0x00, 0x04])] {
        let size = match id { 0x83 => 3, 0x8b => 4, _ => 0x0a };
        match padded(0x0b, id, head, size) {
            Ok(r) => println!("lift-off 0b/{id:02x} {}", hex(&r)),
            Err(e) => println!("lift-off 0b/{id:02x} {e}"),
        }
    }

    for layer in [0x00, 0x01] {
        for (btn, name) in BUTTONS {
            let b = padded(0x02, 0x8c, &[0x01, btn, layer], 0x0a)?;
            println!("bind L{layer} {name:<8} {}", describe_bind(&b[2..]));
        }
    }

    let battery = padded(0x07, 0x80, &[], 2)?;
    let charging = padded(0x07, 0x84, &[], 2)?;
    let idle = padded(0x07, 0x83, &[], 2)?;
    println!("battery       {}%{}", battery[1] as u32 * 100 / 255, if charging[1] == 1 { " charging" } else { "" });
    println!("idle sleep    {} s", be16(&idle));
    Ok(())
}

const BUTTONS: [(u8, &str); 8] = [
    (0x01, "left"),
    (0x02, "right"),
    (0x03, "wheel"),
    (0x04, "back"),
    (0x05, "forward"),
    (0x09, "wheel up"),
    (0x0a, "wheel dn"),
    (0x60, "dpi"),
];

/// `<b2> <category> <len> <value...>` from a 02/8c reply.
fn describe_bind(b: &[u8]) -> String {
    let (cat, len) = (b[1], b[2] as usize);
    let val = &b[3..3 + len.min(b.len() - 3)];
    let what = match (cat, val) {
        (0x00, _) => "none".to_string(),
        (0x01, [v]) => format!("mouse button {v:02x}"),
        (0x02, [m, k]) => format!("key {} (usage {k:02x}, mod {m:02x})", key_name(*k)),
        (0x06, _) => "dpi cycle".to_string(),
        _ => "unknown".to_string(),
    };
    format!("{what:<34} raw {}", hex(b))
}

fn key_name(usage: u8) -> String {
    match usage {
        0x04..=0x1d => ((b'A' + usage - 0x04) as char).to_string(),
        0x3a..=0x45 => format!("F{}", usage - 0x39),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        _ => "?".to_string(),
    }
}

fn be16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

/// Razer feature report: status, transaction id, 3 reserved, args size, class, id,
/// 80 arg bytes, CRC (XOR of bytes 2..88), reserved.
fn report(tid: u8, class: u8, id: u8, args: &[u8]) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[1] = tid;
    r[5] = args.len() as u8;
    r[6] = class;
    r[7] = id;
    r[8..8 + args.len()].copy_from_slice(args);
    r[88] = crc(&r);
    r
}

fn crc(r: &[u8; REPORT_LEN]) -> u8 {
    r[2..88].iter().fold(0, |c, b| c ^ b)
}

struct Mouse(hidapi::HidDevice);

impl Mouse {
    /// Interface 0 of the first Razer device: Synapse sends its reports there.
    fn open() -> Result<Self> {
        let api = hidapi::HidApi::new()?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == RAZER_VID && d.interface_number() == 0)
            .ok_or("no Razer mouse found")?;
        Ok(Mouse(info.open_device(&api)?))
    }

    /// Send a command, return the reply args. Size = args.len().
    fn query(&self, class: u8, id: u8, args: &[u8]) -> Result<Vec<u8>> {
        // ponytail: fixed tid; Synapse counts 00..1E but the mouse just echoes it.
        let tid = 0x1f;
        let mut out = [0u8; REPORT_LEN + 1]; // byte 0 = HID report id 0
        out[1..].copy_from_slice(&report(tid, class, id, args));
        self.0.send_feature_report(&out)?;
        // Synapse waits ~30 ms before reading the reply; the mouse says busy if asked too soon.
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(30));
            let mut buf = [0u8; REPORT_LEN + 1];
            self.0.get_feature_report(&mut buf)?;
            let r = &buf[1..];
            match r[0] {
                STATUS_BUSY => continue,
                STATUS_OK if r[1] == tid && r[6] == class && r[7] == id => {
                    return Ok(r[8..8 + r[5] as usize].to_vec());
                }
                status => {
                    return Err(format!("{class:02x}/{id:02x}: status {status:02x}").into());
                }
            }
        }
        Err(format!("{class:02x}/{id:02x}: mouse stayed busy").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_matches_synapse_capture() {
        // cap:settings01 frame 615: back button = F13.
        let r = report(0x0b, 0x02, 0x0c, &[0x01, 0x04, 0x00, 0x02, 0x02, 0x00, 0x68, 0, 0, 0]);
        assert_eq!(&r[..8], &[0x00, 0x0b, 0, 0, 0, 0x0a, 0x02, 0x0c]);
        assert_eq!(r[88], 0x69); // CRC byte as captured
    }
}
