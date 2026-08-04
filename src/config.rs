use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::action::Action;

/// Everything configured for a single key: the icon it shows and what
/// happens when it's pressed. Either field may be absent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
}

pub type KeyConfigMap = HashMap<u8, KeyConfig>;

/// Reads the JSON config mapping key index to its icon/action, e.g.
/// `{"0": {"icon": "icons/play.png", "action": {"type": "run_command", "command": "..."}}}`.
/// Returns `Ok(None)` if the file doesn't exist.
pub fn load_key_config(path: &str) -> anyhow::Result<Option<KeyConfigMap>> {
    let config_str = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let keys: KeyConfigMap = serde_json::from_str(&config_str)?;
    Ok(Some(keys))
}

/// Writes the key config map back to the config file.
pub fn save_key_config(path: &str, keys: &KeyConfigMap) -> anyhow::Result<()> {
    let config_str = serde_json::to_string_pretty(keys)?;
    std::fs::write(path, config_str)?;
    Ok(())
}
