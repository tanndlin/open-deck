use image::{ColorType, DynamicImage, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

use crate::{KEY_COUNT, StreamDeck, draw_title};

// MK.2 key images are 72x72 JPEGs, mirrored on both axes to match how the
// panel is physically mounted behind each button.
pub const ICON_SIZE: u32 = 72;

/// Fits `image` onto a key, overlays `title` if set, and encodes it in the
/// device's native (mirrored JPEG) format, ready for [`StreamDeck::push_key_image`].
///
/// # Errors
/// Returns an error if JPEG encoding fails.
pub fn encode_key_image(image: &DynamicImage, title: Option<&str>) -> anyhow::Result<Vec<u8>> {
    // Fit (not stretch) into the key's bounds, then center on a padded square canvas.
    let fitted = image.resize(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    let mut canvas = image::RgbaImage::new(ICON_SIZE, ICON_SIZE);
    let x_offset = i64::from(ICON_SIZE.saturating_sub(fitted.width()) / 2);
    let y_offset = i64::from(ICON_SIZE.saturating_sub(fitted.height()) / 2);
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
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::as_conversions
        )]
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

/// Clears a key's image (sets it to solid black), still showing `title` if set.
///
/// # Errors
/// Returns an error if encoding or the HID write fails.
pub fn clear_key_image(device: &StreamDeck, key: u8, title: Option<&str>) -> anyhow::Result<()> {
    let jpeg = encode_key_image(&DynamicImage::new_rgb8(ICON_SIZE, ICON_SIZE), title)?;
    device.push_key_image(key, &jpeg)
}

/// # Errors
/// Returns an error if clearing any key fails.
pub fn clear_all_keys(device: &StreamDeck) -> anyhow::Result<()> {
    for key in 0..KEY_COUNT {
        clear_key_image(device, key, None)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
