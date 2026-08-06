use windows_icons::{DllIcon, get_icon_base64_by_dll, get_icon_by_dll};

fn main() {
    let _ = std::fs::create_dir("output");

    let folder = DllIcon::new().with_shell32(5);
    let icon = get_icon_by_dll(folder).unwrap();
    icon.save("output/folder.png").unwrap();

    let control = DllIcon::new().with_imageres(23);
    let icon = get_icon_by_dll(control).unwrap();
    icon.save("output/control.png").unwrap();

    let share = DllIcon::new().with_resource("shell32.dll", "#16770", 128);
    let icon = get_icon_by_dll(share).unwrap();
    icon.save("output/share.png").unwrap();

    let explorer = DllIcon::new().with_explorer(1);
    let base64 = get_icon_base64_by_dll(explorer).unwrap();
    println!("Explorer: {}", base64);
}
