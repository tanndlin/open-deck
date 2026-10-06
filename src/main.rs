// Suppresses the console window in release builds; debug builds keep it so `cargo run` still shows output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, PoisonError};

use anyhow::bail;
use hidapi::HidApi;
use tokio::sync::broadcast;

use crate::api::{ServerEvent, format_page_path};
use crate::config::{KeyConfigMap, Settings, load_json, page_at};
use crate::device_extensions::{load_key_icons, precache_all_icons, set_back_arrow_icon};
use crate::icon_cache::IconCache;
use stream_deck::{StreamDeck, clear_all_keys};

mod action;
mod api;
mod assets;
mod config;
mod device_extensions;
mod discord;
mod icon_cache;
mod infer_icon;

const KEY_COUNT: u8 = StreamDeck::KEY_COUNT;
const CONFIG_FILE_NAME: &str = "config.json";
const SETTINGS_FILE_NAME: &str = "settings.json";
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
    settings: Mutex<Settings>,
    settings_path: String,
    icon_cache: IconCache,
    /// Broadcasts page changes and key presses to connected `/api/ws` clients,
    /// so the GUI's notion of device state never drifts from the real thing.
    events: broadcast::Sender<ServerEvent>,
}

/// Clears the device and pushes the page at `path` onto it, then marks it as active.
fn switch_to_path(state: &AppState, path: &[u8]) -> anyhow::Result<()> {
    let root = state.root.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(page) = page_at(&root, path) else {
        bail!("no page at path {path:?}");
    };

    // Key presses are routed by `current_path`, so it must switch before any
    // device I/O below — otherwise a mid-render failure leaves the screen
    // showing the new page while presses still resolve against the old one.
    *state
        .current_path
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = path.to_vec();

    let _ = state.events.send(ServerEvent::PageChanged {
        path: format_page_path(path),
    });

    let device = state.device.lock().unwrap_or_else(PoisonError::into_inner);
    clear_all_keys(&device)?;
    load_key_icons(&device, page, &state.icon_cache);
    // Matches KeyTile.tsx's isBackKey.
    if !path.is_empty() {
        set_back_arrow_icon(&device, BACK_KEY, &state.icon_cache)?;
    }

    Ok(())
}

fn apply_brightness(state: &AppState) -> anyhow::Result<()> {
    let brightness = state
        .settings
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .brightness;
    let device = state.device.lock().unwrap_or_else(PoisonError::into_inner);
    device.set_brightness(brightness)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hid = HidApi::new()?;

    let device = StreamDeck::open_with_retry(&hid);
    let icon_cache = IconCache::new();

    clear_all_keys(&device)?;
    let config_dir = config::config_dir()?;
    let config_path = config_dir.join(CONFIG_FILE_NAME);
    let settings_path = config_dir.join(SETTINGS_FILE_NAME);
    let settings: Settings = load_json(&settings_path)?.unwrap_or_default();
    if let Err(e) = device.set_brightness(settings.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }
    let root: KeyConfigMap = load_json(&config_path)?.unwrap_or_else(|| {
        println!("No config at {}, skipping", config_path.display());
        KeyConfigMap::new()
    });
    load_key_icons(&device, &root, &icon_cache);

    device.set_blocking_mode(false)?;

    let (events_tx, _events_rx) = broadcast::channel(32);

    let state = Arc::new(AppState {
        device: Mutex::new(device),
        root: Mutex::new(root),
        current_path: Mutex::new(Vec::new()),
        config_path: config_path.to_string_lossy().to_string(),
        settings: Mutex::new(settings),
        settings_path: settings_path.to_string_lossy().to_string(),
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
                    path: format_page_path(
                        &poll_state
                            .current_path
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner),
                    ),
                    id: key_id,
                });
                run_key_action(&poll_state, key_id);
            },
            || {
                if let Err(e) = apply_brightness(&poll_state) {
                    eprintln!("Failed to restore brightness after reconnect: {e}");
                }
                // The newly (re)connected device can start blank, so redraw
                // whatever page was on screen before the disconnect.
                let path = poll_state
                    .current_path
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
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
        let root = precache_state
            .root
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        precache_all_icons(&root, &precache_state.icon_cache);
    });

    let router = api::router(state).fallback(assets::static_handler);

    let listener = tokio::net::TcpListener::bind(API_ADDR).await?;
    println!("Web UI listening on http://{API_ADDR}");
    axum::serve(listener, router).await?;

    Ok(())
}

fn run_key_action(state: &AppState, key: u8) {
    let current_path = state
        .current_path
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();

    if key == BACK_KEY && !current_path.is_empty() {
        let parent_len = current_path.len().saturating_sub(1);
        if let Some(parent_path) = current_path.get(..parent_len)
            && let Err(e) = switch_to_path(state, parent_path)
        {
            eprintln!("Failed to go up from {current_path:?}: {e}");
        }
        return;
    }

    let (is_folder, action) = {
        let root = state.root.lock().unwrap_or_else(PoisonError::into_inner);
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
