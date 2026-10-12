// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Thin Tauri shell over the fang library. The web page talks in plain JSON:
//! a bind is `{t: "mouse"|"key"|"dpi"|"hs"|"off"|"other", v?, mods?}`, layers are "std"/"hs".

use fang::{Action, Button, DpiStages, Layer, LiftOff, Mouse};
use serde::Deserialize;
use serde_json::{Value, json};

const MODS: [(&str, u8); 4] = [("Ctrl", 0x01), ("Shift", 0x02), ("Alt", 0x04), ("Win", 0x08)];

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![status, apply])
        .run(tauri::generate_context!())
        .expect("error while running fang");
}

/// Everything on the mouse, shaped for the page.
#[tauri::command]
async fn status() -> Result<Value, String> {
    let mouse = Mouse::open().map_err(|e| e.to_string())?;
    status_json(&mouse).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct Changes {
    dpi: Option<DpiIn>,
    polling: Option<u16>,
    liftoff: Option<LiftIn>,
    #[serde(default)]
    binds: Vec<BindIn>,
}

#[derive(Deserialize)]
struct DpiIn {
    active: u8,
    stages: Vec<(u16, u16)>,
}

#[derive(Deserialize)]
struct LiftIn {
    mode: String,
    preset: u8,
    lift: u8,
    land: u8,
}

#[derive(Deserialize)]
struct BindIn {
    button: String,
    layer: String,
    action: Value,
}

/// Write each changed section (each write is read back by the library), then return
/// per-section results plus a fresh status. One failing section doesn't stop the others.
#[tauri::command]
async fn apply(changes: Changes) -> Result<Value, String> {
    let mouse = Mouse::open().map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut record = |section: &str, r: fang::Result<()>| {
        results.push(match r {
            Ok(()) => json!({"section": section, "ok": true}),
            Err(e) => json!({"section": section, "ok": false, "error": e.to_string()}),
        });
    };

    if let Some(d) = changes.dpi {
        record("DPI", mouse.set_dpi(&DpiStages { active: d.active, stages: d.stages }));
    }
    if let Some(hz) = changes.polling {
        record("Polling", mouse.set_polling(hz));
    }
    if let Some(l) = changes.liftoff {
        let lo = if l.mode == "preset" { LiftOff::Preset(l.preset) } else { LiftOff::Asymmetric { lift: l.lift, land: l.land } };
        record("Lift-off", mouse.set_liftoff(lo));
    }
    for b in &changes.binds {
        let r = (|| {
            let button = Button::ALL.into_iter().find(|x| x.name() == b.button).ok_or("unknown button")?;
            let layer = if b.layer == "hs" { Layer::Hypershift } else { Layer::Normal };
            mouse.set_bind(button, layer, &action_from(&b.action)?)
        })();
        record(&format!("Button {}", b.button), r);
    }

    let status = status_json(&mouse).map_err(|e| e.to_string())?;
    Ok(json!({"results": results, "status": status}))
}

fn status_json(mouse: &Mouse) -> fang::Result<Value> {
    let info = mouse.info()?;
    let s = mouse.status()?;
    let liftoff = match s.liftoff {
        Some(LiftOff::Preset(p)) => json!({"mode": "preset", "preset": p, "lift": 13, "land": 12}),
        Some(LiftOff::Asymmetric { lift, land }) => json!({"mode": "asym", "preset": 2, "lift": lift, "land": land}),
        None => json!({"mode": "asym", "preset": 2, "lift": 13, "land": 12, "unknown": true}),
    };
    let mut binds = json!({"std": {}, "hs": {}});
    for (button, layer, action) in &s.binds {
        let l = if *layer == Layer::Normal { "std" } else { "hs" };
        binds[l][button.name()] = action_json(action);
    }
    Ok(json!({
        "name": "DeathAdder V3 Pro",
        "connection": if mouse.pid == 0x00b6 { "Wired" } else { "Wireless dongle" },
        "firmware": format!("{}.{}", info.firmware.0, info.firmware.1),
        "battery": s.battery_pct,
        "charging": s.charging,
        "polling": s.polling_hz,
        "dpi": {"active": s.dpi.active, "stages": s.dpi.stages},
        "liftoff": liftoff,
        "binds": binds,
    }))
}

fn action_json(a: &Action) -> Value {
    match a {
        Action::None => json!({"t": "off"}),
        Action::Mouse(b) => json!({"t": "mouse", "v": b.to_string()}),
        Action::Key { modifiers, usage } => {
            let mods: Vec<&str> = MODS.iter().filter(|(_, bit)| modifiers & bit != 0).map(|(n, _)| *n).collect();
            json!({"t": "key", "v": fang::key_name(*usage), "mods": mods})
        }
        Action::DpiCycle => json!({"t": "dpi"}),
        Action::Hypershift => json!({"t": "hs"}),
        Action::Other { func, data } => json!({"t": "other", "func": func, "data": data}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the mouse: `cargo test -- --ignored`. Reads the mouse, re-applies exactly what
    /// it read (no net change), and checks every section and bind comes back ok and equal.
    #[test]
    #[ignore]
    fn apply_round_trips_on_hardware() {
        let before = tauri::async_runtime::block_on(status()).unwrap();
        let mut binds = Vec::new();
        for layer in ["std", "hs"] {
            for (button, action) in before["binds"][layer].as_object().unwrap() {
                binds.push(json!({"button": button, "layer": layer, "action": action}));
            }
        }
        let changes = json!({
            "dpi": before["dpi"],
            "polling": before["polling"],
            "liftoff": before["liftoff"],
            "binds": binds,
        });
        let r = tauri::async_runtime::block_on(apply(serde_json::from_value(changes).unwrap())).unwrap();
        for res in r["results"].as_array().unwrap() {
            assert_eq!(res["ok"], true, "{res}");
        }
        assert_eq!(r["status"], before);
    }

    #[test]
    fn actions_round_trip() {
        for a in [
            Action::None,
            Action::Mouse(4),
            Action::Key { modifiers: 0x01 | 0x08, usage: 0x68 },
            Action::DpiCycle,
            Action::Hypershift,
        ] {
            assert_eq!(action_from(&action_json(&a)).unwrap(), a);
        }
    }
}

fn action_from(v: &Value) -> fang::Result<Action> {
    let s = |k: &str| v[k].as_str().unwrap_or_default();
    Ok(match s("t") {
        "off" => Action::None,
        "mouse" => Action::Mouse(s("v").parse()?),
        "key" => {
            let usage = fang::key_usage(s("v")).ok_or_else(|| format!("unknown key {}", s("v")))?;
            let names: Vec<&str> = v["mods"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            let modifiers = MODS.iter().filter(|(n, _)| names.contains(n)).fold(0, |m, (_, bit)| m | bit);
            Action::Key { modifiers, usage }
        }
        "dpi" => Action::DpiCycle,
        "hs" => Action::Hypershift,
        t => return Err(format!("can't write bind type {t:?}").into()),
    })
}
