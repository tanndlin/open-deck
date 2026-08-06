mod utils {
    pub mod image_utils;
    pub mod process_utils;
}
mod dll_icons;
mod uwp_apps;

pub use dll_icons::DllIcon;
use dll_icons::get_dll_hicon_to_image;
use utils::image_utils::{get_hicon_to_image, image_to_base64};
use utils::process_utils::get_process_path;
use uwp_apps::{get_uwp_icon, get_uwp_icon_base64};

use std::{error::Error, path::Path};

use image::RgbaImage;

fn is_uwp_app(path: &Path) -> bool {
    let is_uwp = path.to_string_lossy().contains("Programme/WindowsApps");

    let is_wsa = path
        .to_string_lossy()
        .contains("WindowsSubsystemForAndroid");

    is_uwp && !is_wsa
}

pub fn get_icon_by_path<P: AsRef<Path>>(path: P) -> Result<RgbaImage, Box<dyn Error>> {
    let path = path.as_ref();
    if is_uwp_app(path) {
        get_uwp_icon(path)
    } else {
        get_hicon_to_image(path)
    }
}

pub fn get_icon_base64_by_path<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn Error>> {
    let path = path.as_ref();
    if is_uwp_app(path) {
        get_uwp_icon_base64(path)
    } else {
        let icon_image = get_icon_by_path(path)?;
        image_to_base64(icon_image)
    }
}

pub fn get_icon_by_process_id(process_id: u32) -> Result<RgbaImage, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_by_path(&process_path)
}

pub fn get_icon_base64_by_process_id(process_id: u32) -> Result<String, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_base64_by_path(&process_path)
}

pub fn get_icon_by_dll(dll_icon: DllIcon) -> Result<RgbaImage, Box<dyn Error>> {
    get_dll_hicon_to_image(dll_icon)
}

pub fn get_icon_base64_by_dll(dll_icon: DllIcon) -> Result<String, Box<dyn Error>> {
    let dll_image = get_icon_by_dll(dll_icon)?;
    image_to_base64(dll_image)
}
