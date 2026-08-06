use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};

/// Something that can be bound to a key and run when it's pressed.
/// Tagged on `type` so the frontend can switch on it to render the right editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    RunCommand {
        command: String,
    },
    OpenUrl {
        url: String,
    },
    OpenFolder {
        path: String,
    },
    TypeText {
        text: String,
    },
    /// Key names pressed together, e.g. `["ctrl", "c"]`. See `parse_key` for accepted names.
    Hotkey {
        keys: Vec<String>,
    },
    /// Joins the local Discord client to a voice channel by snowflake ID.
    DiscordJoinVoice {
        channel_id: String,
    },
    /// Runs each sub-action in order.
    Multi {
        actions: Vec<Action>,
    },
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
            Action::TypeText { text } => {
                if let Err(e) = type_text(text) {
                    eprintln!("Failed to type text: {e}");
                }
            }
            Action::Hotkey { keys } => {
                if let Err(e) = press_hotkey(keys) {
                    eprintln!("Failed to send hotkey '{}': {e}", keys.join("+"));
                }
            }
            Action::DiscordJoinVoice { channel_id } => {
                if let Err(e) = crate::discord::join_voice_channel(channel_id) {
                    eprintln!("Failed to join Discord voice channel '{channel_id}': {e}");
                }
            }
            Action::Multi { actions } => {
                for action in actions {
                    action.execute();
                }
            }
        }
    }
}

fn type_text(text: &str) -> anyhow::Result<()> {
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.text(text)?;
    Ok(())
}

/// Presses `keys` down in order, then releases them in reverse order.
fn press_hotkey(keys: &[String]) -> anyhow::Result<()> {
    let parsed = keys
        .iter()
        .map(|name| parse_key(name).ok_or_else(|| anyhow::anyhow!("unknown key '{name}'")))
        .collect::<anyhow::Result<Vec<Key>>>()?;

    let mut enigo = Enigo::new(&Settings::default())?;
    for &key in &parsed {
        enigo.key(key, Direction::Press)?;
    }
    for &key in parsed.iter().rev() {
        enigo.key(key, Direction::Release)?;
    }
    Ok(())
}

/// Maps a human-typed key name (case-insensitive) to an enigo [`Key`]. Single
/// characters fall through to [`Key::Unicode`] so letters/digits/symbols work
/// without needing an entry here.
fn parse_key(name: &str) -> Option<Key> {
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "cmd" | "command" | "win" | "windows" | "super" => Key::Meta,
        "enter" | "return" => Key::Return,
        "esc" | "escape" => Key::Escape,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "del" | "delete" => Key::Delete,
        "ins" | "insert" => Key::Insert,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "capslock" => Key::CapsLock,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        // Lowercased: on Windows, VkKeyScanExW packs shift-state into the same
        // byte range enigo reads back as the virtual-key code, so an uppercase
        // letter (which needs Shift) maps to a bogus key and silently no-ops.
        // A hotkey combo doesn't care about case anyway — Ctrl+S is Ctrl+s.
        _ if lower.chars().count() == 1 => Key::Unicode(lower.chars().next()?),
        _ => return None,
    })
}

#[cfg(target_os = "windows")]
fn shell_command(command: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    // raw_arg avoids Rust's automatic Windows quoting, which backslash-escapes
    // embedded quotes (e.g. a quoted "program with spaces" path) in a way
    // cmd.exe doesn't understand and mangles into "not recognized" errors.
    let mut c = std::process::Command::new("cmd");
    c.raw_arg(format!("/C {command}"));
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
