use hidapi::HidDevice;
use image::{ColorType, DynamicImage, codecs::jpeg::JpegEncoder};

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
    let rgb = image.into_rgb8();

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

/// Loads `icons/<key>.png` (or .jpg) for each key that has one and pushes it
/// to the device. Keys without a matching file are left as-is.
pub fn load_key_icons(device: &HidDevice) -> anyhow::Result<()> {
    for key in 0..KEY_COUNT {
        for ext in ["png", "jpg", "jpeg"] {
            let path = format!("icons/{key}.{ext}");
            if let Ok(img) = image::open(&path) {
                let jpeg = encode_key_image(img)?;
                set_key_image(device, key, &jpeg)?;
                println!("Set key {key} image from {path}");
                break;
            }
        }
    }
    Ok(())
}
