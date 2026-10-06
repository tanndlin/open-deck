use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::action::Action;

/// Directory where all user config files live: `~/.open-deck`. Created if missing.
pub fn config_dir() -> anyhow::Result<PathBuf> {
    let dir = home_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
        .join(".open-deck");
    let _ = std::fs::create_dir_all(&dir);
    Ok(dir)
}

#[cfg(windows)]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

#[cfg(unix)]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// A key is a "folder" if `folder` is set: pressing it opens the nested page
/// instead of running `action`. A folder's page is reachable only through
/// this key, so deleting the folder deletes everything nested inside it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<KeyConfigMap>,
}

pub type KeyConfigMap = HashMap<u8, KeyConfig>;

/// A sequence of key indices, followed from the home page. Empty is home.
pub type PagePath = [u8];

/// Returns `None` if any segment of `path` doesn't exist or isn't a folder.
pub fn page_at<'a>(root: &'a KeyConfigMap, path: &PagePath) -> Option<&'a KeyConfigMap> {
    let mut current = root;
    for &key in path {
        current = current.get(&key)?.folder.as_ref()?;
    }
    Some(current)
}

/// Mutable counterpart of [`page_at`].
pub fn page_at_mut<'a>(
    root: &'a mut KeyConfigMap,
    path: &PagePath,
) -> Option<&'a mut KeyConfigMap> {
    let mut current = root;
    for &key in path {
        current = current.get_mut(&key)?.folder.as_mut()?;
    }
    Some(current)
}

/// Returns `Ok(None)` if `path` doesn't exist.
pub fn load_json<T: DeserializeOwned>(path: &impl AsRef<Path>) -> anyhow::Result<Option<T>> {
    let json = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    Ok(Some(serde_json::from_str(&json)?))
}

pub fn save_json<T: Serialize>(path: &impl AsRef<Path>, value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_brightness")]
    pub brightness: u8,
}

const fn default_brightness() -> u8 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness: default_brightness(),
        }
    }
}
