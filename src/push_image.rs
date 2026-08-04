use std::collections::HashMap;

use hidapi::HidDevice;
use image::{ColorType, DynamicImage, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

use crate::KEY_COUNT;

// MK.2 key images are 72x72 JPEGs, mirrored on both axes to match how the
// panel is physically mounted behind each button.
const ICON_SIZE: u32 = 72;

// Image reports are fixed 1024-byte HID output reports:
// [0x02, 0x07, key, is_last, len_lo, len_hi, page_lo, page_hi] + up to 1016
// bytes of JPEG payload, zero-padded to fill the report.
const IMAGE_REPORT_LEN: usize = 1024;
const IMAGE_REPORT_HEADER_LEN: usize = 8;
const IMAGE_REPORT_PAYLOAD_LEN: usize = IMAGE_REPORT_LEN - IMAGE_REPORT_HEADER_LEN;

fn encode_key_image(image: DynamicImage) -> anyhow::Result<Vec<u8>> {
    let image = image.resize_exact(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    let image = image.fliph().flipv();

    let rgba = image.into_rgba8();
    let mut rgb = RgbImage::new(ICON_SIZE, ICON_SIZE);
    for (dst, src) in rgb.pixels_mut().zip(rgba.pixels()) {
        let [r, g, b, a] = src.0;
        let a = a as f32 / 255.0;
        *dst = Rgb([
            (r as f32 * a) as u8,
            (g as f32 * a) as u8,
            (b as f32 * a) as u8,
        ]);
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

        let mut packet = vec![
            0x02,
            0x07,
            key,
            if is_last { 1 } else { 0 },
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
    let jpeg = encode_key_image(blank)?;
    set_key_image(device, key, &jpeg)
}

/// Clears every key's image.
pub fn clear_all_keys(device: &HidDevice) -> anyhow::Result<()> {
    for key in 0..KEY_COUNT {
        clear_key_image(device, key)?;
    }
    Ok(())
}

/// Loads the image at `path` and pushes it to `key`.
pub fn set_key_icon(device: &HidDevice, key: u8, path: &str) -> anyhow::Result<()> {
    let img = image::open(path)?;
    let jpeg = encode_key_image(img)?;
    set_key_image(device, key, &jpeg)
}

/// Loads the image at each configured path and pushes it to the matching
/// key. Keys without an entry in `key_paths` are left as-is.
pub fn load_key_icons(device: &HidDevice, key_paths: &HashMap<u8, String>) -> anyhow::Result<()> {
    for (&key, path) in key_paths {
        if key >= KEY_COUNT {
            eprintln!("Skipping key {key}: out of range (device has {KEY_COUNT} keys)");
            continue;
        }

        set_key_icon(device, key, path)?;

        #[cfg(debug_assertions)]
        println!("Set key {key} image from {path}");
    }
    Ok(())
}
