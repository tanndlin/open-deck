use std::borrow::Cow;
use std::sync::Arc;

use image::DynamicImage;
use stream_deck::{ICON_SIZE, StreamDeck, encode_key_image};

use crate::KEY_COUNT;
use crate::config::KeyConfigMap;
use crate::icon_cache::IconCache;

/// Matches the icon the web UI overlays on the back key (see KeyTile.tsx).
const BACK_ARROW_BYTES: &[u8] = include_bytes!("../assets/back_arrow.png");

/// Matches the fallback the web UI serves (see `get_key_image` in api.rs and KeyTile.tsx).
pub const FOLDER_ICON_BYTES: &[u8] = include_bytes!("../assets/folder.png");

// NUL-prefixed cache keys so they never collide with a real icon path or URL.
const BLANK_CACHE_KEY: &str = "\0blank";
const BACK_ARROW_CACHE_KEY: &str = "\0back_arrow";
const FOLDER_ICON_CACHE_KEY: &str = "\0folder";

/// Composes a cache key that also varies with `title`, so a title change
/// busts the cache even when the underlying icon (identified by `base`)
/// doesn't. NUL-separated so it can't collide with a real `base`.
fn with_title_suffix<'a>(base: &'a str, title: Option<&str>) -> Cow<'a, str> {
    title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map_or(Cow::Borrowed(base), |t| {
            Cow::Owned(format!("{base}\0title\0{t}"))
        })
}

/// Clears a key's image (sets it to solid black), still showing `title` if set.
pub fn clear_key_image(
    device: &StreamDeck,
    key: u8,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let cache_key = with_title_suffix(BLANK_CACHE_KEY, title);
    let jpeg = cache.get_image(&cache_key, || {
        encode_key_image(&DynamicImage::new_rgb8(ICON_SIZE, ICON_SIZE), title)
    })?;
    device.push_key_image(key, &jpeg)
}

/// Pushes the "go up a level" arrow onto `key`, overriding whatever icon
/// (if any) is configured there.
pub fn set_back_arrow_icon(device: &StreamDeck, key: u8, cache: &IconCache) -> anyhow::Result<()> {
    let jpeg = cache.get_image(BACK_ARROW_CACHE_KEY, || {
        encode_key_image(&image::load_from_memory(BACK_ARROW_BYTES)?, None)
    })?;
    device.push_key_image(key, &jpeg)
}

fn encoded_folder_icon(title: Option<&str>, cache: &IconCache) -> anyhow::Result<Arc<Vec<u8>>> {
    let cache_key = with_title_suffix(FOLDER_ICON_CACHE_KEY, title);
    cache.get_image(&cache_key, || {
        encode_key_image(&image::load_from_memory(FOLDER_ICON_BYTES)?, title)
    })
}

/// Pushes the default folder icon onto `key`, still showing `title` if set.
pub fn set_folder_icon(
    device: &StreamDeck,
    key: u8,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let jpeg = encoded_folder_icon(title, cache)?;
    device.push_key_image(key, &jpeg)
}

/// Loads the image at `path` (a local path or `http(s)` URL) and encodes it
/// (or fetches the cached encoding), without pushing it to any key. The
/// encoded result is cached under `path` (and `title`), so this only
/// happens once per combination.
fn encoded_icon(
    path: &str,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<Arc<Vec<u8>>> {
    let cache_key = with_title_suffix(path, title);
    cache.get_image(&cache_key, || {
        let image = if path.starts_with("http://") || path.starts_with("https://") {
            let icon = cache.get_or_fetch(path).map_err(|e| anyhow::anyhow!(e))?;
            image::load_from_memory(&icon.bytes)?
        } else {
            image::open(path)?
        };
        encode_key_image(&image, title)
    })
}

/// Loads the image at `path` (a local path or `http(s)` URL) and pushes it to
/// `key`, overlaying `title` if set.
pub fn set_key_icon(
    device: &StreamDeck,
    key: u8,
    path: &str,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let jpeg = encoded_icon(path, title, cache)?;
    device.push_key_image(key, &jpeg)
}

/// Warms the cache for every icon in `keys` and its nested folders, without
/// pushing anything to a device. Meant to run in the background so startup
/// isn't blocked on fetching/encoding icons that aren't on screen yet.
/// A single icon failing is logged and skipped rather than aborting the walk.
pub fn precache_all_icons(keys: &KeyConfigMap, cache: &IconCache) {
    for config in keys.values() {
        let title = config.title.as_deref();
        let result = match &config.icon {
            Some(path) => encoded_icon(path, title, cache).map(|_| ()),
            None if config.folder.is_some() => encoded_folder_icon(title, cache).map(|_| ()),
            None => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("Failed to precache icon: {e}");
        }

        if let Some(folder) = &config.folder {
            precache_all_icons(folder, cache);
        }
    }
}

/// Folder keys with no icon of their own fall back to the default folder icon.
/// A single key's icon failing to load is logged and skipped rather than
/// aborting the page, so the screen doesn't end up stuck mid-render.
pub fn load_key_icons(device: &StreamDeck, keys: &KeyConfigMap, cache: &IconCache) {
    for (&key, config) in keys {
        if key >= KEY_COUNT {
            eprintln!("Skipping key {key}: out of range (device has {KEY_COUNT} keys)");
            continue;
        }

        let title = config.title.as_deref();
        let result = match &config.icon {
            Some(path) => {
                #[cfg(debug_assertions)]
                println!("Set key {key} image from {path}");

                set_key_icon(device, key, path, title, cache)
            }
            None if config.folder.is_some() => set_folder_icon(device, key, title, cache),
            None if title.is_some() => clear_key_image(device, key, title, cache),
            None => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("Failed to set icon for key {key}: {e}");
        }
    }
}
