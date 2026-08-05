use hidapi::HidDevice;
use image::{ColorType, DynamicImage, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

use crate::KEY_COUNT;
use crate::config::KeyConfigMap;
use crate::icon_cache::IconCache;

// MK.2 key images are 72x72 JPEGs, mirrored on both axes to match how the
// panel is physically mounted behind each button.
const ICON_SIZE: u32 = 72;

/// Matches the icon the web UI overlays on the back key (see KeyTile.tsx).
const BACK_ARROW_BYTES: &[u8] = include_bytes!("../assets/back_arrow.png");

/// The default icon for a folder key that has no icon of its own configured
/// — matches the fallback the web UI serves (see `get_key_image` in api.rs
/// and KeyTile.tsx).
pub const FOLDER_ICON_BYTES: &[u8] = include_bytes!("../assets/folder.png");

// Image reports are fixed 1024-byte HID output reports:
// [0x02, 0x07, key, is_last, len_lo, len_hi, page_lo, page_hi] + up to 1016
// bytes of JPEG payload, zero-padded to fill the report.
const IMAGE_REPORT_LEN: usize = 1024;
const IMAGE_REPORT_HEADER_LEN: usize = 8;
const IMAGE_REPORT_PAYLOAD_LEN: usize = IMAGE_REPORT_LEN - IMAGE_REPORT_HEADER_LEN;

fn encode_key_image(image: &DynamicImage) -> anyhow::Result<Vec<u8>> {
    let image = image.resize_exact(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    let image = image.fliph().flipv();

    let rgba = image.into_rgba8();
    let mut rgb = RgbImage::new(ICON_SIZE, ICON_SIZE);
    for (dst, src) in rgb.pixels_mut().zip(rgba.pixels()) {
        let [r, g, b, a] = src.0;
        let alpha = f32::from(a) / 255.0;
        // r, g, b, alpha are all bounded such that the blended result always
        // fits in 0..=255.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let blended = [
            (f32::from(r) * alpha).round() as u8,
            (f32::from(g) * alpha).round() as u8,
            (f32::from(b) * alpha).round() as u8,
        ];
        *dst = Rgb(blended);
    }

    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90).encode(
        &rgb,
        ICON_SIZE,
        ICON_SIZE,
        ColorType::Rgb8.into(),
    )?;
    Ok(jpeg)
}

fn set_key_image(device: &HidDevice, key: u8, jpeg: &[u8]) -> anyhow::Result<()> {
    let mut page = 0usize;
    let mut remaining = jpeg.len();

    while remaining > 0 {
        let chunk_len = remaining.min(IMAGE_REPORT_PAYLOAD_LEN);
        let sent = page * IMAGE_REPORT_PAYLOAD_LEN;
        let is_last = chunk_len == remaining;

        // chunk_len and page are split into wire-protocol low/high bytes;
        // both stay well within u16 range for any realistic icon size.
        #[allow(clippy::cast_possible_truncation)]
        let mut packet = vec![
            0x02,
            0x07,
            key,
            u8::from(is_last),
            (chunk_len & 0xff) as u8,
            (chunk_len >> 8) as u8,
            (page & 0xff) as u8,
            (page >> 8) as u8,
        ];
        packet.extend_from_slice(&jpeg[sent..sent + chunk_len]);
        packet.resize(IMAGE_REPORT_LEN, 0);

        device.write(&packet)?;

        remaining -= chunk_len;
        page += 1;
    }

    Ok(())
}

/// Clears a key's image (sets it to solid black).
pub fn clear_key_image(device: &HidDevice, key: u8) -> anyhow::Result<()> {
    let blank = DynamicImage::new_rgb8(ICON_SIZE, ICON_SIZE);
    let jpeg = encode_key_image(&blank)?;
    set_key_image(device, key, &jpeg)
}

/// Clears every key's image.
pub fn clear_all_keys(device: &HidDevice) -> anyhow::Result<()> {
    for key in 0..KEY_COUNT {
        clear_key_image(device, key)?;
    }
    Ok(())
}

/// Pushes the "go up a level" arrow onto `key`, overriding whatever icon
/// (if any) is configured there.
pub fn set_back_arrow_icon(device: &HidDevice, key: u8) -> anyhow::Result<()> {
    let img = image::load_from_memory(BACK_ARROW_BYTES)?;
    let jpeg = encode_key_image(&img)?;
    set_key_image(device, key, &jpeg)
}

/// Pushes the default folder icon onto `key`.
pub fn set_folder_icon(device: &HidDevice, key: u8) -> anyhow::Result<()> {
    let img = image::load_from_memory(FOLDER_ICON_BYTES)?;
    let jpeg = encode_key_image(&img)?;
    set_key_image(device, key, &jpeg)
}

/// Loads an image from a local file path or, if `source` is an `http(s)`
/// URL, downloads it (via `cache`) — so icons can point at a link instead of
/// requiring the image to be saved to disk first.
pub fn load_image(source: &str, cache: &IconCache) -> anyhow::Result<DynamicImage> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let icon = cache.get_or_fetch(source).map_err(|e| anyhow::anyhow!(e))?;
        Ok(image::load_from_memory(&icon.bytes)?)
    } else {
        Ok(image::open(source)?)
    }
}

/// Loads the image at `path` (a local path or `http(s)` URL) and pushes it
/// to `key`.
pub fn set_key_icon(
    device: &HidDevice,
    key: u8,
    path: &str,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let img = load_image(path, cache)?;
    let jpeg = encode_key_image(&img)?;
    set_key_image(device, key, &jpeg)
}

/// Loads the icon for each configured key and pushes it to the device.
/// Folder keys with no icon of their own fall back to the default folder
/// icon; other keys with no icon (or no entry at all) are left as-is.
pub fn load_key_icons(
    device: &HidDevice,
    keys: &KeyConfigMap,
    cache: &IconCache,
) -> anyhow::Result<()> {
    for (&key, config) in keys {
        if key >= KEY_COUNT {
            eprintln!("Skipping key {key}: out of range (device has {KEY_COUNT} keys)");
            continue;
        }

        match &config.icon {
            Some(path) => {
                set_key_icon(device, key, path, cache)?;

                #[cfg(debug_assertions)]
                println!("Set key {key} image from {path}");
            }
            None if config.folder.is_some() => set_folder_icon(device, key)?,
            None => {}
        }
    }
    Ok(())
}
