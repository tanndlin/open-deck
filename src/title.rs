use std::sync::OnceLock;

use image::Rgba;
use imageproc::rect::Rect;

use crate::push_image::ICON_SIZE;

const TITLE_BAR_HEIGHT: u32 = 18;
const TITLE_FONT_SIZE: f32 = 13.0;

#[cfg(target_os = "windows")]
const CANDIDATE_FONT_PATHS: &[&str] = &[
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
];

#[cfg(target_os = "macos")]
const CANDIDATE_FONT_PATHS: &[&str] = &["/System/Library/Fonts/Supplemental/Arial.ttf"];

#[cfg(all(unix, not(target_os = "macos")))]
const CANDIDATE_FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
];

/// Best-effort lookup of a usable system font for rendering key titles on the
/// physical device. Titles are simply skipped there (though they still show
/// in the web UI, which renders them itself) if none of this OS's well-known
/// font paths exist.
fn system_font_bytes() -> Option<&'static [u8]> {
    static BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    BYTES
        .get_or_init(|| {
            CANDIDATE_FONT_PATHS
                .iter()
                .find_map(|p| std::fs::read(p).ok())
        })
        .as_deref()
}

/// Overlays `title` in a semi-transparent bar along the bottom of `canvas` so
/// it stays legible over any icon underneath.
pub fn draw_title(canvas: &mut image::RgbaImage, title: &str) {
    let Some(font_bytes) = system_font_bytes() else {
        return;
    };
    let Ok(font) = ab_glyph::FontRef::try_from_slice(font_bytes) else {
        return;
    };
    let scale = ab_glyph::PxScale::from(TITLE_FONT_SIZE);

    let bar_height = TITLE_BAR_HEIGHT.min(ICON_SIZE);
    // bar_y fits in i32: ICON_SIZE is a small fixed constant (72).
    #[allow(clippy::cast_possible_wrap)]
    let bar_y = (ICON_SIZE - bar_height) as i32;
    imageproc::drawing::draw_filled_rect_mut(
        canvas,
        Rect::at(0, bar_y).of_size(ICON_SIZE, bar_height),
        Rgba([0, 0, 0, 170]),
    );

    let (text_width, _) = imageproc::drawing::text_size(scale, &font, title);
    let x = ((i64::from(ICON_SIZE) - i64::from(text_width)) / 2).max(0);
    // x fits in i32: it's clamped to >= 0 and bounded above by ICON_SIZE.
    #[allow(clippy::cast_possible_truncation)]
    let x = x as i32;
    imageproc::drawing::draw_text_mut(
        canvas,
        Rgba([255, 255, 255, 255]),
        x,
        bar_y + 2,
        scale,
        &font,
        title,
    );
}
