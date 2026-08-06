use windows_icons::{get_icon_base64_by_process_id, get_icon_by_process_id};

fn main() {
    let _ = std::fs::create_dir("output");

    // Substitute the process id to test
    let process_id = 2188;

    let icon = get_icon_by_process_id(process_id).unwrap();
    icon.save("output/process.png").unwrap();

    let base64 = get_icon_base64_by_process_id(process_id).unwrap();
    println!("Process {}: {}", process_id, base64);
}
