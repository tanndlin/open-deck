use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a downloaded remote icon is reused before being re-fetched.
/// Every page navigation re-pushes every key's icon to the device *and* the
/// web UI re-requests every key's image, so an uncached URL icon costs two
/// network round trips per navigation. Icons rarely change, so a modest TTL
/// avoids nearly all of that cost.
const TTL: Duration = Duration::from_mins(5);

pub struct CachedIcon {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Caches downloaded `http(s)` icons, shared between the physical-device
/// push path and the web UI's own image-preview endpoint.
#[derive(Default)]
pub struct IconCache {
    entries: Mutex<HashMap<String, (Instant, Arc<CachedIcon>)>>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the icon at `url`, downloading it only if there's no fresh
    /// cached copy.
    pub fn get_or_fetch(&self, url: &str) -> Result<Arc<CachedIcon>, String> {
        if let Some((fetched_at, icon)) = self.entries.lock().unwrap().get(url)
            && fetched_at.elapsed() < TTL
        {
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
        self.entries
            .lock()
            .unwrap()
            .insert(url.to_string(), (Instant::now(), Arc::clone(&icon)));
        Ok(icon)
    }
}
