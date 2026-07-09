use hidapi::HidApi;

use crate::config::load_key_config;
use crate::push_image::{clear_all_keys, load_key_icons};

mod config;
mod push_image;

const ELGATO_VID: u16 = 0x0fd9;
const STREAMDECK_MK2_PID: u16 = 0x006d;
const KEY_COUNT: u8 = 15;
const CONFIG_PATH: &str = "config.json";

fn main() -> anyhow::Result<()> {
    let api = HidApi::new()?;

    // Find all connected Elgato devices
    #[cfg(debug_assertions)]
    for dev in api.device_list() {
        if dev.vendor_id() == ELGATO_VID {
            println!(
                "Found: {:?} PID={:#06x}",
                dev.product_string(),
                dev.product_id()
            );
        }
    }

    let device = api.open(ELGATO_VID, STREAMDECK_MK2_PID)?;

    clear_all_keys(&device)?;
    match load_key_config(CONFIG_PATH)? {
        Some(key_paths) => load_key_icons(&device, &key_paths)?,
        None => println!("No icon config at {CONFIG_PATH}, skipping"),
    }

    device.set_blocking_mode(false)?; // non-blocking so we can poll

    let mut buf = [0u8; 512];
    let header_len = 4;

    loop {
        match device.read(&mut buf) {
            Ok(0) => {
                // timeout — no state change, keep polling
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(n) => {
                // buf[0] = report ID (0x01)
                // buf[1..4] = header bytes to skip
                // buf[4..n] = one byte per key, 0x01 = pressed, 0x00 = released
                let key_states = &buf[header_len..n];

                for (key_index, &state) in key_states.iter().enumerate() {
                    match state {
                        0x01 => println!("Key {} pressed", key_index),
                        0x00 => {} // released / unchanged
                        _ => {}    // shouldn't happen
                    }
                }
            }
            Err(e) => eprintln!("HID read error: {e}"),
        }
    }
}
