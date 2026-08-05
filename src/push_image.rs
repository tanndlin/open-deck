use hidapi::HidDevice;
use image::{ColorType, DynamicImage, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

use crate::KEY_COUNT;
use crate::config::KeyConfigMap;
use crate::icon_cache::IconCache;
use crate::title::draw_title;

// MK.2 key images are 72x72 JPEGs, mirrored on both axes to match how the
// panel is physically mounted behind each button.
pub const ICON_SIZE: u32 = 72;

/// Matches the icon the web UI overlays on the back key (see KeyTile.tsx).
const BACK_ARROW_BYTES: &[u8] = include_bytes!("../assets/back_arrow.png");

/// Matches the fallback the web UI serves (see `get_key_image` in api.rs and KeyTile.tsx).
pub const FOLDER_ICON_BYTES: &[u8] = include_bytes!("../assets/folder.png");

// NUL-prefixed cache keys so they never collide with a real icon path or URL.
const BLANK_CACHE_KEY: &str = "\0blank";
const BACK_ARROW_CACHE_KEY: &str = "\0back_arrow";
const FOLDER_ICON_CACHE_KEY: &str = "\0folder";

// Image reports are fixed 1024-byte HID output reports:
// [0x02, 0x07, key, is_last, len_lo, len_hi, page_lo, page_hi] + up to 1016
// bytes of JPEG payload, zero-padded to fill the report.
const IMAGE_REPORT_LEN: usize = 1024;
const IMAGE_REPORT_HEADER_LEN: usize = 8;
const IMAGE_REPORT_PAYLOAD_LEN: usize = IMAGE_REPORT_LEN - IMAGE_REPORT_HEADER_LEN;

fn encode_key_image(image: &DynamicImage, title: Option<&str>) -> anyhow::Result<Vec<u8>> {
    // Fit (not stretch) into the key's bounds, then center on a padded square canvas.
    let fitted = image.resize(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    let mut canvas = image::RgbaImage::new(ICON_SIZE, ICON_SIZE);
    let x_offset = i64::from((ICON_SIZE - fitted.width()) / 2);
    let y_offset = i64::from((ICON_SIZE - fitted.height()) / 2);
    image::imageops::overlay(&mut canvas, &fitted.to_rgba8(), x_offset, y_offset);

    if let Some(title) = title.map(str::trim).filter(|t| !t.is_empty()) {
        draw_title(&mut canvas, title);
    }

    let image = DynamicImage::ImageRgba8(canvas).fliph().flipv();

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

/// Composes a cache key that also varies with `title`, so a title change
/// busts the cache even when the underlying icon (identified by `base`)
/// doesn't. NUL-separated so it can't collide with a real `base`.
fn with_title_suffix<'a>(base: &'a str, title: Option<&str>) -> std::borrow::Cow<'a, str> {
    match title.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => std::borrow::Cow::Owned(format!("{base}\0title\0{t}")),
        None => std::borrow::Cow::Borrowed(base),
    }
}

/// Clears a key's image (sets it to solid black), still showing `title` if set.
pub fn clear_key_image(
    device: &HidDevice,
    key: u8,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let cache_key = with_title_suffix(BLANK_CACHE_KEY, title);
    let jpeg = cache.get_image(&cache_key, || {
        encode_key_image(&DynamicImage::new_rgb8(ICON_SIZE, ICON_SIZE), title)
    })?;
    set_key_image(device, key, &jpeg)
}

pub fn clear_all_keys(device: &HidDevice, cache: &IconCache) -> anyhow::Result<()> {
    for key in 0..KEY_COUNT {
        clear_key_image(device, key, None, cache)?;
    }
    Ok(())
}

/// Pushes the "go up a level" arrow onto `key`, overriding whatever icon
/// (if any) is configured there.
pub fn set_back_arrow_icon(device: &HidDevice, key: u8, cache: &IconCache) -> anyhow::Result<()> {
    let jpeg = cache.get_image(BACK_ARROW_CACHE_KEY, || {
        encode_key_image(&image::load_from_memory(BACK_ARROW_BYTES)?, None)
    })?;
    set_key_image(device, key, &jpeg)
}

/// Pushes the default folder icon onto `key`, still showing `title` if set.
pub fn set_folder_icon(
    device: &HidDevice,
    key: u8,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let cache_key = with_title_suffix(FOLDER_ICON_CACHE_KEY, title);
    let jpeg = cache.get_image(&cache_key, || {
        encode_key_image(&image::load_from_memory(FOLDER_ICON_BYTES)?, title)
    })?;
    set_key_image(device, key, &jpeg)
}

/// Loads the image at `path` (a local path or `http(s)` URL) and pushes it to
/// `key`, overlaying `title` if set. The encoded result is cached under
/// `path` (and `title`), so this only happens once per combination.
pub fn set_key_icon(
    device: &HidDevice,
    key: u8,
    path: &str,
    title: Option<&str>,
    cache: &IconCache,
) -> anyhow::Result<()> {
    let cache_key = with_title_suffix(path, title);
    let jpeg = cache.get_image(&cache_key, || {
        let image = if path.starts_with("http://") || path.starts_with("https://") {
            let icon = cache.get_or_fetch(path).map_err(|e| anyhow::anyhow!(e))?;
            image::load_from_memory(&icon.bytes)?
        } else {
            image::open(path)?
        };
        encode_key_image(&image, title)
    })?;
    set_key_image(device, key, &jpeg)
}

/// Folder keys with no icon of their own fall back to the default folder icon.
/// A single key's icon failing to load is logged and skipped rather than
/// aborting the page, so the screen doesn't end up stuck mid-render.
pub fn load_key_icons(device: &HidDevice, keys: &KeyConfigMap, cache: &IconCache) {
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

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn encode_key_image_fits_wide_images_instead_of_stretching() {
        // A wide, fully opaque red rectangle: fitting it into a 72x72 key
        // should shrink it down to 72x18 and pad above/below, not stretch it
        // to fill the whole square.
        let wide = DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 50, Rgba([255, 0, 0, 255])));
        let jpeg = encode_key_image(&wide, None).unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().into_rgb8();

        // Corners fall in the padded area, so they should stay black rather
        // than the stretched-to-fill red a plain resize_exact would produce.
        assert_eq!(*decoded.get_pixel(0, 0), Rgb([0, 0, 0]));
        assert_eq!(*decoded.get_pixel(ICON_SIZE - 1, 0), Rgb([0, 0, 0]));
        assert_eq!(*decoded.get_pixel(0, ICON_SIZE - 1), Rgb([0, 0, 0]));

        // The center falls inside the fitted band, so it should still be red.
        let center = decoded.get_pixel(ICON_SIZE / 2, ICON_SIZE / 2);
        assert!(center[0] > 200 && center[1] < 50 && center[2] < 50);
    }
}
