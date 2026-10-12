const RAZER_VID: u16 = 0x1532;

fn main() -> Result<(), hidapi::HidError> {
    match std::env::args().nth(1).as_deref() {
        Some("devices") => devices(),
        _ => {
            eprintln!("usage: fang devices");
            std::process::exit(2);
        }
    }
}

fn devices() -> Result<(), hidapi::HidError> {
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
