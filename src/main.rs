use std::{thread, time::Duration};

const RAZER_VID: u16 = 0x1532;
const REPORT_LEN: usize = 90;

const STATUS_BUSY: u8 = 0x01;
const STATUS_OK: u8 = 0x02;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let result = match args.first().map(String::as_str) {
        Some("devices") => devices(),
        Some("info") => info(),
        Some("status") => status(),
        Some("bind") => bind(&rest),
        Some("dpi") => dpi(&rest),
        Some("liftoff") => liftoff(&rest),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("fang: {e}");
        std::process::exit(1);
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const USAGE: &str = "usage:
  fang devices | info | status
  fang bind <button> <action> [hypershift]
      button: left right wheel back forward up down dpi
      action: mouse:<1-5|9|10> | key:<A-Z|F1-F24|0xNN usage> | dpi-cycle | hypershift | none
      hypershift = the layer used while the hypershift button is held
  fang dpi <stage>,<stage>,... [active N]   stage = 800 or 1000x1100 (100..30000, step 50)
  fang liftoff <1-3>                        preset: 1 low, 2 medium, 3 high
  fang liftoff <lift 2-26> <land 1-25>      asymmetric, land < lift";

const PROFILE: u8 = 0x01; // the one onboard slot; Synapse always sends 1

/// Write a button bind, then read it back.
fn bind(args: &[&str]) -> Result<()> {
    let (button, action) = match args {
        [b, a] | [b, a, "hypershift"] => (*b, *a),
        _ => return Err(USAGE.into()),
    };
    let layer = u8::from(args.len() == 3);
    let btn = BUTTONS
        .iter()
        .find(|(_, n)| n.replace(' ', "") == button || (button == "up" && *n == "wheel up") || (button == "down" && *n == "wheel dn"))
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("unknown button {button}"))?;
    let (func, data): (u8, Vec<u8>) = match action.split_once(':') {
        Some(("mouse", n)) => (0x01, vec![n.parse()?]),
        Some(("key", k)) => (0x02, vec![0x00, key_usage(k).ok_or_else(|| format!("unknown key {k}"))?]),
        None if action == "dpi-cycle" => (0x06, vec![0x06]),
        None if action == "hypershift" => (0x0c, vec![0x01]),
        None if action == "none" => (0x00, vec![]),
        _ => return Err(format!("unknown action {action}").into()),
    };
    let mut w = vec![PROFILE, btn, layer, func, data.len() as u8];
    w.extend(&data);
    w.resize(0x0a, 0);

    let mouse = Mouse::open()?;
    mouse.query(0x02, 0x0c, &w)?;
    let r = mouse.query(0x02, 0x8c, &pad(&[PROFILE, btn, layer], 0x0a))?;
    println!("wrote {}", hex(&w));
    println!("read  {}  -> {}", hex(&r), describe_bind(&r[2..]));
    check(&r[3..5 + data.len()], &w[3..5 + data.len()])
}

/// Write DPI stages, then read them back.
fn dpi(args: &[&str]) -> Result<()> {
    let (list, active) = match args {
        [l] => (*l, 1u8),
        [l, "active", n] => (*l, n.parse()?),
        _ => return Err(USAGE.into()),
    };
    let stages = list
        .split(',')
        .map(|s| {
            let (x, y) = s.split_once('x').unwrap_or((s, s));
            Ok((dpi_value(x)?, dpi_value(y)?))
        })
        .collect::<Result<Vec<_>>>()?;
    if stages.is_empty() || stages.len() > 5 || active == 0 || active as usize > stages.len() {
        return Err("1-5 stages, active must be one of them".into());
    }
    let mut w = vec![PROFILE, active, stages.len() as u8];
    for (i, (x, y)) in stages.iter().enumerate() {
        w.push(i as u8);
        w.extend(x.to_be_bytes());
        w.extend(y.to_be_bytes());
        w.extend([0, 0]); // Z-DPI
    }

    let mouse = Mouse::open()?;
    mouse.query(0x04, 0x06, &w)?;
    let r = mouse.query(0x04, 0x86, &pad(&[PROFILE], 0x50))?;
    println!("wrote {}", hex(&w));
    println!("read  {}", hex(&r[..3 + 7 * r[2] as usize]));
    // Read-back ids are 1-based, written ids 0-based; compare everything else.
    let strip = |b: &[u8]| -> Vec<u8> { b[3..].chunks(7).flat_map(|s| s[1..].to_vec()).collect() };
    check(&r[1..3], &w[1..3])?;
    check(&strip(&r[..3 + 7 * r[2] as usize]), &strip(&w))
}

fn dpi_value(s: &str) -> Result<u16> {
    let v: u16 = s.parse()?;
    if !(100..=30000).contains(&v) || !v.is_multiple_of(50) {
        return Err(format!("dpi {v}: must be 100..30000 in steps of 50").into());
    }
    Ok(v)
}

/// Set lift-off (preset or asymmetric), then read it back.
fn liftoff(args: &[&str]) -> Result<()> {
    const SENSOR: [u8; 2] = [0x00, 0x04]; // calibration class 0, sensor 4
    let mouse = Mouse::open()?;
    match args {
        [level] => {
            let level: u8 = level.parse()?;
            if !(1..=3).contains(&level) {
                return Err("preset is 1, 2 or 3".into());
            }
            mouse.query(0x0b, 0x03, &[SENSOR[0], SENSOR[1], 0x00])?;
            mouse.query(0x0b, 0x0b, &[SENSOR[0], SENSOR[1], 0x01, level - 1])?;
            // 0b/83 reads 01 in both modes; the lift mode in 0b/8b is what tells them apart.
            let lvl = mouse.query(0x0b, 0x8b, &[SENSOR[0], SENSOR[1], 0x01, 0])?;
            println!("read  lift setting {}", hex(&lvl));
            check(&lvl[2..4], &[0x01, level - 1])
        }
        [lift, land] => {
            let (lift, land): (u8, u8) = (lift.parse()?, land.parse()?);
            if !(2..=26).contains(&lift) || !(1..=25).contains(&land) || land >= lift {
                return Err("lift 2..26, land 1..25, land < lift".into());
            }
            mouse.query(0x0b, 0x03, &[SENSOR[0], SENSOR[1], 0x01])?;
            mouse.query(0x0b, 0x0b, &[SENSOR[0], SENSOR[1], 0x04, 0x00])?;
            mouse.query(0x0b, 0x05, &pad(&[SENSOR[0], SENSOR[1], lift - 1, land - 1], 0x0a))?;
            let mode = mouse.query(0x0b, 0x8b, &[SENSOR[0], SENSOR[1], 0x01, 0])?;
            let r = mouse.query(0x0b, 0x85, &pad(&[SENSOR[0], SENSOR[1]], 0x0a))?;
            println!("read  lift setting {}  distances {}", hex(&mode), hex(&r));
            check(&[mode[2], r[4], r[5]], &[0x04, lift - 1, land - 1])
        }
        _ => Err(USAGE.into()),
    }
}

fn check(got: &[u8], want: &[u8]) -> Result<()> {
    if got == want {
        println!("ok: read-back matches");
        Ok(())
    } else {
        Err(format!("read-back mismatch: got {} want {}", hex(got), hex(want)).into())
    }
}

fn pad(head: &[u8], size: usize) -> Vec<u8> {
    let mut v = head.to_vec();
    v.resize(size, 0);
    v
}

fn key_usage(k: &str) -> Option<u8> {
    if let Some(h) = k.strip_prefix("0x") {
        return u8::from_str_radix(h, 16).ok();
    }
    let k = k.to_ascii_uppercase();
    match k.as_bytes() {
        [c @ b'A'..=b'Z'] => Some(c - b'A' + 0x04),
        [b'F', n @ ..] => match std::str::from_utf8(n).ok()?.parse::<u8>().ok()? {
            n @ 1..=12 => Some(0x3a + n - 1),
            n @ 13..=24 => Some(0x68 + n - 13),
            _ => None,
        },
        _ => None,
    }
}

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
        (0x0c, _) => "hypershift (hold)".to_string(),
        _ => "unknown".to_string(),
    };
    format!("{what:<34} raw {}", hex(b))
}

fn key_name(usage: u8) -> String {
    match usage {
        0x04..=0x1d => ((b'A' + usage - 0x04) as char).to_string(),
        0x3a..=0x45 => format!("F{}", usage - 0x39),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        0x53 => "NumLock".to_string(),
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

    #[test]
    fn keys_and_dpi_limits() {
        assert_eq!(key_usage("f13"), Some(0x68)); // as Synapse sent for F13
        assert_eq!(key_usage("F14"), Some(0x69));
        assert_eq!(key_usage("F1"), Some(0x3a));
        assert_eq!(key_usage("a"), Some(0x04));
        assert_eq!(key_usage("F25"), None);
        assert!(dpi_value("1100").is_ok());
        assert!(dpi_value("1234").is_err());
        assert!(dpi_value("30050").is_err());
    }
}
