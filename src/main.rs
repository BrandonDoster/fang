use std::{thread, time::Duration};

const RAZER_VID: u16 = 0x1532;
const REPORT_LEN: usize = 90;

const STATUS_BUSY: u8 = 0x01;
const STATUS_OK: u8 = 0x02;

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("devices") => devices(),
        Some("info") => info(),
        _ => {
            eprintln!("usage: fang devices | info");
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
