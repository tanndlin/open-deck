use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct CachedIcon {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Caches images for the program's lifetime so repeat pushes skip re-fetching
/// or re-encoding. Entries never expire — a changed icon is only picked up on restart.
#[derive(Default)]
pub struct IconCache {
    urls: Mutex<HashMap<String, Arc<CachedIcon>>>,
    encoded: Mutex<HashMap<String, Arc<Vec<u8>>>>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Downloads `url` only on the first request for it. If the fetch fails and
    /// `url` looks like a guessed `/favicon.ico` path, falls back to
    /// [`crate::infer_icon::recover_favicon`] before giving up.
    pub fn get_or_fetch(&self, url: &str) -> Result<Arc<CachedIcon>, String> {
        if let Some(icon) = self.urls.lock().unwrap().get(url) {
            return Ok(Arc::clone(icon));
        }

        let icon = match Self::fetch(url) {
            Ok(icon) => icon,
            Err(e) => crate::infer_icon::recover_favicon(url)
                .and_then(|recovered| Self::fetch(&recovered).ok())
                .ok_or(e)?,
        };

        let icon = Arc::new(icon);
        self.urls
            .lock()
            .unwrap()
            .insert(url.to_string(), Arc::clone(&icon));
        Ok(icon)
    }

    fn fetch(url: &str) -> Result<CachedIcon, String> {
        let mut response = ureq::get(url)
            .call()
            .map_err(|e| format!("failed to fetch icon: {e}"))?;
        let mime = response.body_mut().mime_type().map_or_else(
            || {
                mime_guess::from_path(url)
                    .first_or_octet_stream()
                    .to_string()
            },
            str::to_owned,
        );
        let bytes = response
            .body_mut()
            .read_to_vec()
            .map_err(|e| format!("failed to read icon: {e}"))?;

        Ok(CachedIcon { bytes, mime })
    }

    /// Calls `encode` only on the first request for `key`.
    pub fn get_image(
        &self,
        key: &str,
        encode: impl FnOnce() -> anyhow::Result<Vec<u8>>,
    ) -> anyhow::Result<Arc<Vec<u8>>> {
        if let Some(jpeg) = self.encoded.lock().unwrap().get(key) {
            return Ok(Arc::clone(jpeg));
        }

        let jpeg = Arc::new(encode()?);
        self.encoded
            .lock()
            .unwrap()
            .insert(key.to_string(), Arc::clone(&jpeg));
        Ok(jpeg)
    }
}
