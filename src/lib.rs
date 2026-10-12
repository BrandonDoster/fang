//! Razer DeathAdder V3 Pro onboard-memory protocol. Every setter writes, reads back and
//! fails if the mouse disagrees. Byte layouts: docs/PROTOCOL.md, docs/SYNAPSE.md.

use std::{cell::Cell, fmt, thread, time::Duration};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const RAZER_VID: u16 = 0x1532;
const REPORT_LEN: usize = 90;
const PROFILE: u8 = 0x01; // the one onboard slot; Synapse always sends 1
const SENSOR: [u8; 2] = [0x00, 0x04]; // lift-off: calibration class 0, sensor 4

pub const DPI_MIN: u16 = 100;
pub const DPI_MAX: u16 = 30000;
pub const DPI_STEP: u16 = 50;
pub const POLLING_HZ: [u16; 3] = [125, 500, 1000];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Wheel,
    Back,
    Forward,
    WheelUp,
    WheelDown,
    Dpi,
}

impl Button {
    pub const ALL: [Button; 8] = [
        Button::Left,
        Button::Right,
        Button::Wheel,
        Button::Back,
        Button::Forward,
        Button::WheelUp,
        Button::WheelDown,
        Button::Dpi,
    ];

    fn id(self) -> u8 {
        match self {
            Button::Left => 0x01,
            Button::Right => 0x02,
            Button::Wheel => 0x03,
            Button::Back => 0x04,
            Button::Forward => 0x05,
            Button::WheelUp => 0x09,
            Button::WheelDown => 0x0a,
            Button::Dpi => 0x60,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Button::Left => "left",
            Button::Right => "right",
            Button::Wheel => "wheel",
            Button::Back => "back",
            Button::Forward => "forward",
            Button::WheelUp => "up",
            Button::WheelDown => "down",
            Button::Dpi => "dpi",
        }
    }
}

/// Normal layer, or the one active while a Hypershift button is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Normal,
    Hypershift,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Mouse button id: 1 left, 2 right, 3 middle, 4 back, 5 forward, 9/10 wheel up/down.
    Mouse(u8),
    /// HID keyboard usage plus modifier mask (01 LCtrl, 02 LShift, 04 LAlt, 08 LGUI, 10-80 right).
    Key { modifiers: u8, usage: u8 },
    DpiCycle,
    Hypershift,
    /// Anything else the mouse reports (macros, media, ...), kept raw.
    Other { func: u8, data: Vec<u8> },
}

impl Action {
    fn encode(&self) -> (u8, Vec<u8>) {
        match self {
            Action::None => (0x00, vec![]),
            Action::Mouse(b) => (0x01, vec![*b]),
            Action::Key { modifiers, usage } => (0x02, vec![*modifiers, *usage]),
            Action::DpiCycle => (0x06, vec![0x06]),
            Action::Hypershift => (0x0c, vec![0x01]),
            Action::Other { func, data } => (*func, data.clone()),
        }
    }

    /// From `<fn> <len> <data...>`.
    fn decode(b: &[u8]) -> Action {
        let (func, len) = (b[0], (b[1] as usize).min(b.len() - 2));
        match (func, &b[2..2 + len]) {
            (0x00, _) => Action::None,
            (0x01, [v]) => Action::Mouse(*v),
            (0x02, [m, k]) => Action::Key { modifiers: *m, usage: *k },
            (0x06, _) => Action::DpiCycle,
            (0x0c, _) => Action::Hypershift,
            (func, data) => Action::Other { func, data: data.to_vec() },
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::None => write!(f, "none"),
            Action::Mouse(b) => write!(f, "mouse button {b}"),
            Action::Key { modifiers: 0, usage } => write!(f, "key {}", key_name(*usage)),
            Action::Key { modifiers, usage } => write!(f, "key {} (mod {modifiers:02x})", key_name(*usage)),
            Action::DpiCycle => write!(f, "dpi cycle"),
            Action::Hypershift => write!(f, "hypershift (hold)"),
            Action::Other { func, data } => write!(f, "fn {func:02x} {}", hex(data)),
        }
    }
}

/// 1-5 stages of (x, y) DPI; `active` is 1-based.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DpiStages {
    pub active: u8,
    pub stages: Vec<(u16, u16)>,
}

impl DpiStages {
    pub fn validate(&self) -> Result<()> {
        if self.stages.is_empty() || self.stages.len() > 5 {
            return Err("1-5 DPI stages".into());
        }
        if self.active == 0 || self.active as usize > self.stages.len() {
            return Err("active stage must be one of the stages".into());
        }
        for &v in self.stages.iter().flat_map(|(x, y)| [x, y]) {
            if !(DPI_MIN..=DPI_MAX).contains(&v) || !v.is_multiple_of(DPI_STEP) {
                return Err(format!("dpi {v}: must be {DPI_MIN}..{DPI_MAX} in steps of {DPI_STEP}").into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiftOff {
    /// Synapse "tracking distance" 1 low, 2 medium, 3 high.
    Preset(u8),
    /// Synapse asymmetric sliders: lift 2..26, land 1..25, land < lift.
    Asymmetric { lift: u8, land: u8 },
}

impl LiftOff {
    pub fn validate(&self) -> Result<()> {
        match *self {
            LiftOff::Preset(l) if !(1..=3).contains(&l) => Err("lift-off preset is 1, 2 or 3".into()),
            LiftOff::Asymmetric { lift, land }
                if !(2..=26).contains(&lift) || !(1..=25).contains(&land) || land >= lift =>
            {
                Err("lift 2..26, land 1..25, land < lift".into())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Info {
    pub serial: String,
    pub firmware: (u8, u8),
}

#[derive(Clone, Debug)]
pub struct Status {
    /// 00 normal, 03 driver (host software in control).
    pub device_mode: u8,
    /// None if the mouse reports a code outside the known table.
    pub polling_hz: Option<u16>,
    pub dpi_now: (u16, u16),
    pub dpi: DpiStages,
    /// None if the lift mode is one Synapse doesn't use on this mouse.
    pub liftoff: Option<LiftOff>,
    pub binds: Vec<(Button, Layer, Action)>,
    pub battery_pct: u8,
    pub charging: bool,
    pub idle_secs: u16,
}

impl Status {
    pub fn bind(&self, button: Button, layer: Layer) -> Option<&Action> {
        self.binds.iter().find(|(b, l, _)| *b == button && *l == layer).map(|(_, _, a)| a)
    }
}

pub struct Mouse {
    dev: hidapi::HidDevice,
    tid: Cell<u8>,
}

impl Mouse {
    /// Interface 0 of the first Razer device: Synapse sends its reports there.
    pub fn open() -> Result<Self> {
        let api = hidapi::HidApi::new()?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == RAZER_VID && d.interface_number() == 0)
            .ok_or("no Razer mouse found")?;
        Ok(Mouse { dev: info.open_device(&api)?, tid: Cell::new(0) })
    }

    pub fn info(&self) -> Result<Info> {
        let serial = self.query(0x00, 0x82, &[0; 0x16])?;
        let fw = self.query(0x00, 0x81, &[0; 2])?;
        let end = serial.iter().position(|&b| b == 0).unwrap_or(serial.len());
        Ok(Info { serial: String::from_utf8_lossy(&serial[..end]).into_owned(), firmware: (fw[0], fw[1]) })
    }

    /// Read everything stored on the mouse. Read-only.
    pub fn status(&self) -> Result<Status> {
        let device_mode = self.query(0x00, 0x84, &[0; 2])?[0];
        let polling_hz = polling_hz(self.query(0x00, 0x8e, &[PROFILE, 0])?[1]);
        let now = self.query(0x04, 0x85, &pad(&[PROFILE], 7))?;
        let mut binds = Vec::new();
        for layer in [Layer::Normal, Layer::Hypershift] {
            for b in Button::ALL {
                binds.push((b, layer, self.read_bind(b, layer)?));
            }
        }
        let battery = self.query(0x07, 0x80, &[0; 2])?[1];
        Ok(Status {
            device_mode,
            polling_hz,
            dpi_now: (be16(&now[1..]), be16(&now[3..])),
            dpi: self.read_dpi()?,
            liftoff: self.read_liftoff()?,
            binds,
            battery_pct: (battery as u32 * 100 / 255) as u8,
            charging: self.query(0x07, 0x84, &[0; 2])?[1] == 1,
            idle_secs: be16(&self.query(0x07, 0x83, &[0; 2])?),
        })
    }

    pub fn set_dpi(&self, dpi: &DpiStages) -> Result<()> {
        dpi.validate()?;
        let mut w = vec![PROFILE, dpi.active, dpi.stages.len() as u8];
        for (i, (x, y)) in dpi.stages.iter().enumerate() {
            w.push(i as u8);
            w.extend(x.to_be_bytes());
            w.extend(y.to_be_bytes());
            w.extend([0, 0]); // Z-DPI
        }
        self.query(0x04, 0x06, &w)?;
        verify("dpi", &self.read_dpi()?, dpi)
    }

    pub fn set_polling(&self, hz: u16) -> Result<()> {
        let code = match hz {
            1000 => 0x01,
            500 => 0x02,
            125 => 0x08,
            _ => return Err(format!("polling {hz}: must be one of {POLLING_HZ:?}").into()),
        };
        self.query(0x00, 0x0e, &[PROFILE, code])?;
        let got = polling_hz(self.query(0x00, 0x8e, &[PROFILE, 0])?[1]);
        verify("polling", &got, &Some(hz))
    }

    pub fn set_liftoff(&self, lo: LiftOff) -> Result<()> {
        lo.validate()?;
        let [c, s] = SENSOR;
        match lo {
            LiftOff::Preset(level) => {
                self.query(0x0b, 0x03, &[c, s, 0x00])?;
                self.query(0x0b, 0x0b, &[c, s, 0x01, level - 1])?;
            }
            LiftOff::Asymmetric { lift, land } => {
                self.query(0x0b, 0x03, &[c, s, 0x01])?;
                self.query(0x0b, 0x0b, &[c, s, 0x04, 0x00])?;
                self.query(0x0b, 0x05, &pad(&[c, s, lift - 1, land - 1], 0x0a))?;
            }
        }
        verify("lift-off", &self.read_liftoff()?, &Some(lo))
    }

    /// A Hypershift key is written on both layers, as Synapse does, so it still works
    /// while the layer is active.
    pub fn set_bind(&self, button: Button, layer: Layer, action: &Action) -> Result<()> {
        let layers: &[Layer] = if *action == Action::Hypershift { &[Layer::Normal, Layer::Hypershift] } else { &[layer] };
        let (func, data) = action.encode();
        for &l in layers {
            let mut w = vec![PROFILE, button.id(), l as u8, func, data.len() as u8];
            w.extend(&data);
            w.resize(0x0a, 0);
            self.query(0x02, 0x0c, &w)?;
            verify("bind", &self.read_bind(button, l)?, action)?;
        }
        Ok(())
    }

    fn read_dpi(&self) -> Result<DpiStages> {
        let r = self.query(0x04, 0x86, &pad(&[PROFILE], 0x50))?;
        let stages = r[3..].chunks(7).take(r[2] as usize).map(|s| (be16(&s[1..]), be16(&s[3..]))).collect();
        Ok(DpiStages { active: r[1], stages })
    }

    /// 0b/83 doesn't show the mode (reads 00 or 01 either way); the lift mode in 0b/8b does.
    fn read_liftoff(&self) -> Result<Option<LiftOff>> {
        let [c, s] = SENSOR;
        let setting = self.query(0x0b, 0x8b, &[c, s, 0x01, 0])?;
        Ok(match setting[2] {
            0x01 => Some(LiftOff::Preset(setting[3] + 1)),
            0x04 => {
                let r = self.query(0x0b, 0x85, &pad(&[c, s], 0x0a))?;
                Some(LiftOff::Asymmetric { lift: r[4] + 1, land: r[5] + 1 })
            }
            _ => None,
        })
    }

    /// Reply byte 2 is the layer of the last write to that button, not the requested one;
    /// the data is right for the requested layer, so skip that byte.
    fn read_bind(&self, button: Button, layer: Layer) -> Result<Action> {
        let r = self.query(0x02, 0x8c, &pad(&[PROFILE, button.id(), layer as u8], 0x0a))?;
        Ok(Action::decode(&r[3..]))
    }

    /// Send a command and return the reply args (size = args.len()). Mirrors Synapse:
    /// rotating transaction id 00..1E, re-read while busy, re-send on "not processed yet"
    /// or a reply that doesn't match the request.
    pub fn query(&self, class: u8, id: u8, args: &[u8]) -> Result<Vec<u8>> {
        let tid = self.tid.get();
        self.tid.set(if tid >= 0x1e { 0 } else { tid + 1 });
        let mut out = [0u8; REPORT_LEN + 1]; // byte 0 = HID report id 0
        out[1..].copy_from_slice(&report(tid, class, id, args));
        for _ in 0..20 {
            self.dev.send_feature_report(&out)?;
            // Synapse waits ~30 ms before reading; the mouse says busy if asked too soon.
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(30));
                let mut buf = [0u8; REPORT_LEN + 1];
                self.dev.get_feature_report(&mut buf)?;
                let r = &buf[1..];
                let ours = r[1] == tid && r[6] == class && r[7] == id;
                match r[0] {
                    0x01 if ours => continue,
                    0x02 if ours => return Ok(r[8..8 + r[5] as usize].to_vec()),
                    0x03 if ours => return Err(format!("{class:02x}/{id:02x}: command failed").into()),
                    0x04 if ours => return Err(format!("{class:02x}/{id:02x}: timeout").into()),
                    0x05 if ours => return Err(format!("{class:02x}/{id:02x}: not supported").into()),
                    _ => break, // 00 not processed yet, or someone else's reply: re-send
                }
            }
        }
        Err(format!("{class:02x}/{id:02x}: no valid reply").into())
    }
}

/// Every Razer HID interface.
pub fn devices() -> Result<Vec<hidapi::DeviceInfo>> {
    let api = hidapi::HidApi::new()?;
    Ok(api.device_list().filter(|d| d.vendor_id() == RAZER_VID).cloned().collect())
}

/// HID usage for `A`-`Z`, `F1`-`F24`, or raw `0xNN`.
pub fn key_usage(k: &str) -> Option<u8> {
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

pub fn key_name(usage: u8) -> String {
    match usage {
        0x04..=0x1d => ((b'A' + usage - 0x04) as char).to_string(),
        0x3a..=0x45 => format!("F{}", usage - 0x39),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        0x53 => "NumLock".to_string(),
        0x59..=0x61 => format!("Num{}", usage - 0x58),
        0x62 => "Num0".to_string(),
        _ => format!("0x{usage:02x}"),
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

fn polling_hz(code: u8) -> Option<u16> {
    match code {
        0x01 => Some(1000),
        0x02 => Some(500),
        0x03 => Some(333),
        0x04 => Some(250),
        0x05 => Some(200),
        0x08 => Some(125),
        0x0a => Some(100),
        _ => None,
    }
}

fn verify<T: PartialEq + fmt::Debug>(what: &str, got: &T, want: &T) -> Result<()> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what} read-back mismatch: got {got:?}, wrote {want:?}").into())
    }
}

fn pad(head: &[u8], size: usize) -> Vec<u8> {
    let mut v = head.to_vec();
    v.resize(size, 0);
    v
}

fn be16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
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
    r[88] = r[2..88].iter().fold(0, |c, b| c ^ b);
    r
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
    fn keys() {
        assert_eq!(key_usage("f13"), Some(0x68)); // as Synapse sent for F13
        assert_eq!(key_usage("F14"), Some(0x69));
        assert_eq!(key_usage("F1"), Some(0x3a));
        assert_eq!(key_usage("a"), Some(0x04));
        assert_eq!(key_usage("0x59"), Some(0x59));
        assert_eq!(key_usage("F25"), None);
        assert_eq!(key_name(0x59), "Num1");
    }

    #[test]
    fn binds_round_trip_captured_bytes() {
        // fn/len/data as seen in captures.
        assert_eq!(Action::decode(&[0x02, 0x02, 0x00, 0x68]), Action::Key { modifiers: 0, usage: 0x68 });
        assert_eq!(Action::decode(&[0x0c, 0x01, 0x01]), Action::Hypershift);
        assert_eq!(Action::decode(&[0x06, 0x01, 0x06]), Action::DpiCycle);
        for a in [Action::Mouse(4), Action::Key { modifiers: 1, usage: 0x06 }, Action::DpiCycle, Action::None] {
            let (func, data) = a.encode();
            let mut b = vec![func, data.len() as u8];
            b.extend(data);
            assert_eq!(Action::decode(&b), a);
        }
    }

    #[test]
    fn limits() {
        let dpi = |active, stages: &[(u16, u16)]| DpiStages { active, stages: stages.to_vec() }.validate();
        assert!(dpi(1, &[(1000, 1100), (800, 800)]).is_ok());
        assert!(dpi(1, &[(1234, 1234)]).is_err());
        assert!(dpi(1, &[(30050, 800)]).is_err());
        assert!(dpi(3, &[(800, 800), (1600, 1600)]).is_err());
        assert!(LiftOff::Asymmetric { lift: 26, land: 25 }.validate().is_ok());
        assert!(LiftOff::Asymmetric { lift: 1, land: 1 }.validate().is_err());
        assert!(LiftOff::Asymmetric { lift: 10, land: 10 }.validate().is_err());
        assert!(LiftOff::Preset(4).validate().is_err());
    }
}
