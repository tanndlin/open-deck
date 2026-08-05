use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct CachedIcon {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Caches images for the lifetime of the program, so repeat pushes
/// (switching back to a page, re-showing a key) skip re-fetching or
/// re-encoding them. Shared between the physical-device push path and the
/// web UI's own image-preview endpoint.
///
/// Entries never expire or get invalidated — a changed icon (edited file, or
/// URL content behind the same link) is only picked up on restart.
#[derive(Default)]
pub struct IconCache {
    urls: Mutex<HashMap<String, Arc<CachedIcon>>>,
    encoded: Mutex<HashMap<String, Arc<Vec<u8>>>>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the raw bytes at `url`, downloading them only on the first
    /// request for that URL.
    pub fn get_or_fetch(&self, url: &str) -> Result<Arc<CachedIcon>, String> {
        if let Some(icon) = self.urls.lock().unwrap().get(url) {
            return Ok(Arc::clone(icon));
        }

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

        let icon = Arc::new(CachedIcon { bytes, mime });
        self.urls
            .lock()
            .unwrap()
            .insert(url.to_string(), Arc::clone(&icon));
        Ok(icon)
    }

    /// Returns the encoded JPEG bytes for `key`, calling `encode` only on the
    /// first request for that key.
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
