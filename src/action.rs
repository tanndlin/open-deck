use serde::{Deserialize, Serialize};

/// Something that can be bound to a key and run when it's pressed.
/// Tagged on `type` so the frontend can switch on it to render the right editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    RunCommand { command: String },
    OpenUrl { url: String },
    OpenFolder { path: String },
}

impl Action {
    pub fn execute(&self) {
        match self {
            Action::RunCommand { command } => {
                if let Err(e) = shell_command(command).spawn() {
                    eprintln!("Failed to run command '{command}': {e}");
                }
            }
            Action::OpenUrl { url } => {
                if let Err(e) = open_url_command(url).spawn() {
                    eprintln!("Failed to open URL '{url}': {e}");
                }
            }
            Action::OpenFolder { path } => {
                if let Err(e) = open_folder_command(path).spawn() {
                    eprintln!("Failed to open folder '{path}': {e}");
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn shell_command(command: &str) -> std::process::Command {
    let mut c = std::process::Command::new("cmd");
    c.args(["/C", command]);
    c
}

#[cfg(not(target_os = "windows"))]
fn shell_command(command: &str) -> std::process::Command {
    let mut c = std::process::Command::new("sh");
    c.args(["-c", command]);
    c
}

#[cfg(target_os = "windows")]
fn open_url_command(url: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    // Quoted manually so cmd doesn't parse `|`/`&` in the URL as operators; `""` is `start`'s window-title arg.
    let escaped_url = url.replace('"', "%22");
    let mut c = std::process::Command::new("cmd");
    c.raw_arg(format!("/C start \"\" \"{escaped_url}\""));
    c
}

#[cfg(target_os = "macos")]
fn open_url_command(url: &str) -> std::process::Command {
    let mut c = std::process::Command::new("open");
    c.arg(url);
    c
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url_command(url: &str) -> std::process::Command {
    let mut c = std::process::Command::new("xdg-open");
    c.arg(url);
    c
}

#[cfg(target_os = "windows")]
fn open_folder_command(path: &str) -> std::process::Command {
    let mut c = std::process::Command::new("explorer");
    c.arg(path);
    c
}

#[cfg(target_os = "macos")]
fn open_folder_command(path: &str) -> std::process::Command {
    let mut c = std::process::Command::new("open");
    c.arg(path);
    c
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_folder_command(path: &str) -> std::process::Command {
    let mut c = std::process::Command::new("xdg-open");
    c.arg(path);
    c
}
