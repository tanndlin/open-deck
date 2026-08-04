use serde::{Deserialize, Serialize};

/// Something that can be bound to a key and run when it's pressed.
///
/// Tagged on `type` so new variants can be added without breaking existing
/// config files, and the frontend can switch on the same tag to render the
/// right editor for each action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Runs `command` through the platform shell, e.g. `python script.py`.
    RunCommand { command: String },
}

impl Action {
    pub fn execute(&self) {
        match self {
            Action::RunCommand { command } => {
                if let Err(e) = shell_command(command).spawn() {
                    eprintln!("Failed to run command '{command}': {e}");
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
