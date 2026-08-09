// Suppresses the console window in release builds; debug builds keep it so `cargo run` still shows output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use hidapi::HidApi;
use tokio::sync::broadcast;

use crate::api::{ServerEvent, format_page_path};
use crate::config::{KeyConfigMap, load_key_config, page_at};
use crate::icon_cache::IconCache;
use crate::push_image::{clear_all_keys, load_key_icons, precache_all_icons, set_back_arrow_icon};
use crate::stream_deck::StreamDeck;

mod action;
mod api;
mod assets;
mod config;
mod discord;
mod icon_cache;
mod infer_icon;
mod push_image;
mod stream_deck;
mod title;

const KEY_COUNT: u8 = StreamDeck::KEY_COUNT;
const CONFIG_FILE_NAME: &str = "config.json";
const API_ADDR: &str = "127.0.0.1:3000";

/// On every non-home page, pressing this goes back up a level instead of running its configured action.
const BACK_KEY: u8 = 0;

/// State shared between the HID polling loop and the REST API.
struct AppState {
    device: Mutex<StreamDeck>,
    /// The home page; subpages nest inside their keys' `folder` fields.
    root: Mutex<KeyConfigMap>,
    /// Key indices from home to the page currently pushed onto the device.
    current_path: Mutex<Vec<u8>>,
    config_path: String,
    icon_cache: IconCache,
    /// Broadcasts page changes and key presses to connected `/api/ws` clients,
    /// so the GUI's notion of device state never drifts from the real thing.
    events: broadcast::Sender<ServerEvent>,
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

    let _ = state.events.send(ServerEvent::PageChanged {
        path: format_page_path(path),
    });

    let device = state.device.lock().unwrap();
    clear_all_keys(&device, &state.icon_cache)?;
    load_key_icons(&device, page, &state.icon_cache);
    // Matches KeyTile.tsx's isBackKey.
    if !path.is_empty() {
        set_back_arrow_icon(&device, BACK_KEY, &state.icon_cache)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hid = HidApi::new()?;

    #[cfg(debug_assertions)]
    for dev in hid.device_list() {
        if dev.vendor_id() == stream_deck::VENDOR_ID {
            println!(
                "Found: {:?} PID={:#06x}",
                dev.product_string(),
                dev.product_id()
            );
        }
    }

    let device = StreamDeck::open_with_retry(&hid);
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

    let (events_tx, _events_rx) = broadcast::channel(32);

    let state = Arc::new(AppState {
        device: Mutex::new(device),
        root: Mutex::new(root),
        current_path: Mutex::new(Vec::new()),
        config_path,
        icon_cache,
        events: events_tx,
    });

    let poll_state = state.clone();
    std::thread::spawn(move || {
        StreamDeck::poll_keys(
            &poll_state.device,
            hid,
            |key_id| {
                println!("Key {key_id} pressed");
                // Path as shown at press time — run_key_action may itself
                // change it (folder/back navigation), which fires its own
                // PageChanged broadcast via switch_to_path.
                let _ = poll_state.events.send(ServerEvent::KeyPressed {
                    path: format_page_path(&poll_state.current_path.lock().unwrap()),
                    id: key_id,
                });
                run_key_action(&poll_state, key_id);
            },
            || {
                // The newly (re)connected device can start blank, so redraw
                // whatever page was on screen before the disconnect.
                let path = poll_state.current_path.lock().unwrap().clone();
                if let Err(e) = switch_to_path(&poll_state, &path) {
                    eprintln!("Failed to refresh icons after reconnect: {e}");
                }
            },
        );
    });

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
        // Spawn thread to not block
        std::thread::spawn(move || action.execute());
    }
}
