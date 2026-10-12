use fang::{Action, Button, DpiStages, Layer, LiftOff, Mouse, Result};

const USAGE: &str = "usage:
  fang devices | info | status
  fang bind <button> <action> [hypershift]
      button: left right wheel back forward up down dpi
      action: mouse:<1-5|9|10> | key:<A-Z|F1-F24|0xNN usage> | dpi-cycle | hypershift | none
      hypershift = the layer used while the hypershift button is held
  fang dpi <stage>,<stage>,... [active N]   stage = 800 or 1000x1100 (100..30000, step 50)
  fang polling <125|500|1000>
  fang liftoff <1-3>                        preset: 1 low, 2 medium, 3 high
  fang liftoff <lift 2-26> <land 1-25>      asymmetric, land < lift";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let result = match args.first().map(String::as_str) {
        Some("devices") => devices(),
        Some("info") => info(),
        Some("status") => status(),
        Some("bind") => bind(&rest),
        Some("dpi") => dpi(&rest),
        Some("polling") => polling(&rest),
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

fn devices() -> Result<()> {
    for d in fang::devices()? {
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
    let info = Mouse::open()?.info()?;
    println!("serial   {}", info.serial);
    println!("firmware {}.{}", info.firmware.0, info.firmware.1);
    Ok(())
}

fn status() -> Result<()> {
    let s = Mouse::open()?.status()?;
    let mode = match s.device_mode {
        0x00 => "normal",
        0x03 => "driver (host software in control)",
        _ => "unknown",
    };
    println!("device mode   {:02x} {mode}", s.device_mode);
    match s.polling_hz {
        Some(hz) => println!("polling       {hz} Hz"),
        None => println!("polling       unknown code"),
    }
    println!("dpi now       {} x {}", s.dpi_now.0, s.dpi_now.1);
    for (i, (x, y)) in s.dpi.stages.iter().enumerate() {
        let mark = if i + 1 == s.dpi.active as usize { "*" } else { " " };
        println!("dpi stage {}{mark}  {x} x {y}", i + 1);
    }
    match s.liftoff {
        Some(LiftOff::Preset(l)) => println!("lift-off      preset {l} (1 low, 2 medium, 3 high)"),
        Some(LiftOff::Asymmetric { lift, land }) => println!("lift-off      asymmetric lift {lift} / land {land}"),
        None => println!("lift-off      unknown mode"),
    }
    for (button, layer, action) in &s.binds {
        let l = if *layer == Layer::Normal { "  " } else { "HS" };
        println!("bind {l} {:<8} {action}", button.name());
    }
    println!("battery       {}%{}", s.battery_pct, if s.charging { " charging" } else { "" });
    println!("idle sleep    {} s", s.idle_secs);
    Ok(())
}

fn bind(args: &[&str]) -> Result<()> {
    let (button, action, layer) = match args {
        [b, a] => (*b, *a, Layer::Normal),
        [b, a, "hypershift"] => (*b, *a, Layer::Hypershift),
        _ => return Err(USAGE.into()),
    };
    let button = Button::ALL.into_iter().find(|b| b.name() == button).ok_or_else(|| format!("unknown button {button}"))?;
    let action = match action.split_once(':') {
        Some(("mouse", n)) => Action::Mouse(n.parse()?),
        Some(("key", k)) => Action::Key { modifiers: 0, usage: fang::key_usage(k).ok_or_else(|| format!("unknown key {k}"))? },
        None if action == "dpi-cycle" => Action::DpiCycle,
        None if action == "hypershift" => Action::Hypershift,
        None if action == "none" => Action::None,
        _ => return Err(format!("unknown action {action}").into()),
    };
    Mouse::open()?.set_bind(button, layer, &action)?;
    println!("ok: {} = {action}{}", button.name(), if layer == Layer::Hypershift { " (hypershift)" } else { "" });
    Ok(())
}

fn dpi(args: &[&str]) -> Result<()> {
    let (list, active) = match args {
        [l] => (*l, 1),
        [l, "active", n] => (*l, n.parse()?),
        _ => return Err(USAGE.into()),
    };
    let stages = list
        .split(',')
        .map(|s| {
            let (x, y) = s.split_once('x').unwrap_or((s, s));
            Ok((x.parse()?, y.parse()?))
        })
        .collect::<Result<Vec<_>>>()?;
    let dpi = DpiStages { active, stages };
    Mouse::open()?.set_dpi(&dpi)?;
    println!("ok: dpi {:?}, active {}", dpi.stages, dpi.active);
    Ok(())
}

fn polling(args: &[&str]) -> Result<()> {
    let [hz] = args else { return Err(USAGE.into()) };
    let hz = hz.parse()?;
    Mouse::open()?.set_polling(hz)?;
    println!("ok: polling {hz} Hz");
    Ok(())
}

fn liftoff(args: &[&str]) -> Result<()> {
    let lo = match args {
        [level] => LiftOff::Preset(level.parse()?),
        [lift, land] => LiftOff::Asymmetric { lift: lift.parse()?, land: land.parse()? },
        _ => return Err(USAGE.into()),
    };
    Mouse::open()?.set_liftoff(lo)?;
    println!("ok: lift-off {lo:?}");
    Ok(())
}
