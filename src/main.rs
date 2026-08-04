use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use hidapi::{HidApi, HidDevice};

use crate::config::load_key_config;
use crate::push_image::{clear_all_keys, load_key_icons};

mod api;
mod assets;
mod config;
mod push_image;

const ELGATO_VID: u16 = 0x0fd9;
const STREAMDECK_MK2_PID: u16 = 0x006d;
const KEY_COUNT: u8 = 15;
const CONFIG_PATH: &str = "config.json";
const API_ADDR: &str = "127.0.0.1:3000";

/// State shared between the HID polling loop and the REST API.
struct AppState {
    device: Mutex<HidDevice>,
    keys: Mutex<HashMap<u8, String>>,
    config_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hid = HidApi::new()?;

    // Find all connected Elgato devices
    #[cfg(debug_assertions)]
    for dev in hid.device_list() {
        if dev.vendor_id() == ELGATO_VID {
            println!(
                "Found: {:?} PID={:#06x}",
                dev.product_string(),
                dev.product_id()
            );
        }
    }

    let device = hid.open(ELGATO_VID, STREAMDECK_MK2_PID)?;

    clear_all_keys(&device)?;
    let keys = match load_key_config(CONFIG_PATH)? {
        Some(key_paths) => {
            load_key_icons(&device, &key_paths)?;
            key_paths
        }
        None => {
            println!("No icon config at {CONFIG_PATH}, skipping");
            HashMap::new()
        }
    };

    device.set_blocking_mode(false)?; // non-blocking so we can poll

    let state = Arc::new(AppState {
        device: Mutex::new(device),
        keys: Mutex::new(keys),
        config_path: CONFIG_PATH.to_string(),
    });

    let poll_state = state.clone();
    std::thread::spawn(move || poll_keys(&poll_state));

    let router = api::router(state).fallback(assets::static_handler);

    let listener = tokio::net::TcpListener::bind(API_ADDR).await?;
    println!("Web UI listening on http://{API_ADDR}");
    axum::serve(listener, router).await?;

    Ok(())
}

fn poll_keys(state: &AppState) {
    let mut buf = [0u8; 512];
    let header_len = 4;

    loop {
        let read_result = {
            let device = state.device.lock().unwrap();
            device.read(&mut buf)
        };

        match read_result {
            Ok(0) => {
                // timeout — no state change, keep polling
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(n) => {
                // buf[0] = report ID (0x01)
                // buf[1..4] = header bytes to skip
                // buf[4..n] = one byte per key, 0x01 = pressed, 0x00 = released
                let key_states = &buf[header_len..n];

                for (key_index, &key_state) in key_states.iter().enumerate() {
                    match key_state {
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
