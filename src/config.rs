use std::collections::HashMap;

/// Reads a JSON config mapping key index to icon file path, e.g.
/// `{"0": "icons/play.png", "1": "icons/stop.png"}`.
/// Returns `Ok(None)` if the file doesn't exist.
pub fn load_key_config(path: &str) -> anyhow::Result<Option<HashMap<u8, String>>> {
    let config_str = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let key_paths: HashMap<u8, String> = serde_json::from_str(&config_str)?;
    Ok(Some(key_paths))
}
