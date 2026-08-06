// Suppresses the console window in release builds; debug builds keep it so `cargo run` still shows output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use hidapi::{HidApi, HidDevice};

use crate::config::{KeyConfigMap, load_key_config, page_at};
use crate::icon_cache::IconCache;
use crate::push_image::{clear_all_keys, load_key_icons, precache_all_icons, set_back_arrow_icon};

mod action;
mod api;
mod assets;
mod config;
mod discord;
mod icon_cache;
mod infer_icon;
mod push_image;
mod title;

const ELGATO_VID: u16 = 0x0fd9;
const STREAMDECK_MK2_PID: u16 = 0x006d;
const KEY_COUNT: u8 = 15;
const CONFIG_FILE_NAME: &str = "config.json";
const API_ADDR: &str = "127.0.0.1:3000";

/// On every non-home page, pressing this goes back up a level instead of running its configured action.
const BACK_KEY: u8 = 0;

/// State shared between the HID polling loop and the REST API.
struct AppState {
    device: Mutex<HidDevice>,
    /// The home page; subpages nest inside their keys' `folder` fields.
    root: Mutex<KeyConfigMap>,
    /// Key indices from home to the page currently pushed onto the device.
    current_path: Mutex<Vec<u8>>,
    config_path: String,
    icon_cache: IconCache,
}

/// Clears the device and pushes the page at `path` onto it, then marks it as active.
pub(crate) fn switch_to_path(state: &AppState, path: &[u8]) -> anyhow::Result<()> {
    let root = state.root.lock().unwrap();
    let Some(page) = page_at(&root, path) else {
        anyhow::bail!("no page at path {path:?}");
    };

    // Key presses are routed by `current_path`, so it must switch before any
    // device I/O below — otherwise a mid-render failure leaves the screen
    // showing the new page while presses still resolve against the old one.
    *state.current_path.lock().unwrap() = path.to_vec();

    let device = state.device.lock().unwrap();
    clear_all_keys(&device, &state.icon_cache)?;
    load_key_icons(&device, page, &state.icon_cache);
    // Matches KeyTile.tsx's isBackKey.
    if !path.is_empty() {
        set_back_arrow_icon(&device, BACK_KEY, &state.icon_cache)?;
    }

    Ok(())
}

/// Retries opening the Stream Deck until it succeeds, so starting the app
/// before the device is plugged in (or while it's disconnected) doesn't
/// crash it — it just waits. Logs once per distinct failure, not on every attempt.
fn open_device_with_retry(hid: &HidApi) -> HidDevice {
    let mut last_error: Option<String> = None;
    loop {
        match hid.open(ELGATO_VID, STREAMDECK_MK2_PID) {
            Ok(device) => return device,
            Err(e) => {
                let message = e.to_string();
                if last_error.as_deref() != Some(message.as_str()) {
                    eprintln!("Stream Deck not found ({e}); waiting for it to connect...");
                    last_error = Some(message);
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hid = HidApi::new()?;

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

    let device = open_device_with_retry(&hid);
    let icon_cache = IconCache::new();

    clear_all_keys(&device, &icon_cache)?;
    let config_path = config::config_dir()
        .join(CONFIG_FILE_NAME)
        .to_string_lossy()
        .to_string();
    let root = if let Some(root) = load_key_config(&config_path)? {
        root
    } else {
        println!("No config at {config_path}, skipping");
        KeyConfigMap::new()
    };
    load_key_icons(&device, &root, &icon_cache);

    device.set_blocking_mode(false)?;

    let state = Arc::new(AppState {
        device: Mutex::new(device),
        root: Mutex::new(root),
        current_path: Mutex::new(Vec::new()),
        config_path,
        icon_cache,
    });

    let poll_state = state.clone();
    std::thread::spawn(move || poll_keys(&poll_state));

    // Warms the cache for icons outside the home page (nested folders, plus
    // anything on the home page that failed above) so opening a folder later
    // doesn't stall on a fetch. Runs off the startup path entirely.
    let precache_state = state.clone();
    std::thread::spawn(move || {
        let root = precache_state.root.lock().unwrap().clone();
        precache_all_icons(&root, &precache_state.icon_cache);
    });

    let router = api::router(state).fallback(assets::static_handler);

    let listener = tokio::net::TcpListener::bind(API_ADDR).await?;
    println!("Web UI listening on http://{API_ADDR}");
    axum::serve(listener, router).await?;

    Ok(())
}

fn poll_keys(state: &AppState) {
    let mut buf = [0u8; 512];
    let header_len = 4;
    let mut pressed = [false; KEY_COUNT as usize];
    // Tracks the last-seen read error so a disconnected device (which errors
    // on every read instead of timing out) logs once instead of spamming.
    let mut last_error: Option<String> = None;

    loop {
        let read_result = {
            let device = state.device.lock().unwrap();
            device.read(&mut buf)
        };

        match read_result {
            Ok(0) => {
                // timeout — no state change, keep polling
                last_error = None;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(n) => {
                last_error = None;
                // buf[0] = report ID (0x01)
                // buf[1..4] = header bytes to skip
                // buf[4..n] = one byte per key, 0x01 = pressed, 0x00 = released
                let key_states = &buf[header_len..n];

                for (key_index, &key_state) in key_states.iter().enumerate() {
                    if key_index >= pressed.len() {
                        break;
                    }
                    let is_pressed = key_state == 0x01;
                    // only fire on the release->press edge, not every report
                    // while the key is held down
                    if is_pressed && !pressed[key_index] {
                        println!("Key {key_index} pressed");
                        // key_index < pressed.len() == KEY_COUNT, checked above.
                        #[allow(clippy::cast_possible_truncation)]
                        run_key_action(state, key_index as u8);
                    }
                    pressed[key_index] = is_pressed;
                }
            }
            Err(e) => {
                let message = e.to_string();
                if last_error.as_deref() != Some(message.as_str()) {
                    eprintln!("HID read error: {e}");
                    last_error = Some(message);
                }
                // A disconnected device errors on every read instead of
                // timing out like an idle one does, so without this sleep
                // the loop would busy-spin as fast as the OS returns errors.
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

fn run_key_action(state: &AppState, key: u8) {
    let current_path = state.current_path.lock().unwrap().clone();

    if key == BACK_KEY && !current_path.is_empty() {
        let parent_path = &current_path[..current_path.len() - 1];
        if let Err(e) = switch_to_path(state, parent_path) {
            eprintln!("Failed to go up from {current_path:?}: {e}");
        }
        return;
    }

    let (is_folder, action) = {
        let root = state.root.lock().unwrap();
        let Some(page) = page_at(&root, &current_path) else {
            return;
        };
        let Some(config) = page.get(&key) else {
            return;
        };
        (config.folder.is_some(), config.action.clone())
    };

    if is_folder {
        let mut child_path = current_path;
        child_path.push(key);
        if let Err(e) = switch_to_path(state, &child_path) {
            eprintln!("Failed to open folder at {child_path:?}: {e}");
        }
        return;
    }

    if let Some(action) = action {
        // Off the polling thread: most actions are near-instant, but a
        // Discord join can block for seconds (or until the user dismisses
        // an "Authorize" dialog), which would otherwise stall every other
        // key press until it resolves.
        std::thread::spawn(move || action.execute());
    }
}
