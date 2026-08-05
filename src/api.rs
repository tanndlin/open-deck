use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::config::{KeyConfig, KeyConfigMap, page_at, page_at_mut, save_key_config};
use crate::icon_cache::IconCache;
use crate::push_image::{FOLDER_ICON_BYTES, clear_key_image, set_key_icon};
use crate::{AppState, KEY_COUNT, switch_to_path};

type ApiError = (StatusCode, String);

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/key-count", get(key_count))
        .route("/api/current-page", get(current_page))
        .route("/api/pages/{path}/activate", post(activate_page))
        .route("/api/pages/{path}/keys", get(list_keys))
        .route("/api/pages/{path}/keys/{id}", get(get_key))
        .route(
            "/api/pages/{path}/keys/{id}/icon",
            get(get_key_icon)
                .put(set_key_icon_route)
                .delete(clear_key_icon),
        )
        .route(
            "/api/pages/{path}/keys/{id}/action",
            get(get_key_action)
                .put(set_key_action)
                .delete(clear_key_action),
        )
        .route(
            "/api/pages/{path}/keys/{id}/folder",
            put(create_folder).delete(delete_folder),
        )
        .route("/api/move", post(move_key))
        .route("/api/pages/{path}/keys/{id}/image", get(get_key_image))
        .with_state(state)
}

async fn key_count() -> Json<u8> {
    Json(KEY_COUNT)
}

fn check_key_range(id: u8) -> Result<(), ApiError> {
    if id >= KEY_COUNT {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("key {id} out of range (device has {KEY_COUNT} keys)"),
        ));
    }
    Ok(())
}

/// Parses a `.`-joined sequence of key indices (e.g. `"3.1"`), or the
/// literal `"home"` for the empty path.
fn parse_page_path(raw: &str) -> Result<Vec<u8>, ApiError> {
    if raw == "home" {
        return Ok(Vec::new());
    }
    raw.split('.')
        .map(|segment| {
            segment.parse::<u8>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("invalid page path '{raw}'"),
                )
            })
        })
        .collect()
}

fn format_page_path(path: &[u8]) -> String {
    if path.is_empty() {
        "home".to_string()
    } else {
        path.iter().map(u8::to_string).collect::<Vec<_>>().join(".")
    }
}

fn page_not_found(raw_path: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        format!("no page at path '{raw_path}'"),
    )
}

/// Persists the whole page tree, dropping any key entries that are now
/// completely empty (no icon, action, or folder) so the config file doesn't
/// accumulate dead keys.
fn persist(state: &AppState) -> Result<(), ApiError> {
    let mut root = state.root.lock().unwrap();
    prune_empty_keys(&mut root);
    save_key_config(&state.config_path, &root).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to save config: {e}"),
        )
    })
}

fn prune_empty_keys(page: &mut KeyConfigMap) {
    for config in page.values_mut() {
        if let Some(folder) = &mut config.folder {
            prune_empty_keys(folder);
        }
    }
    page.retain(|_, c| c.icon.is_some() || c.action.is_some() || c.folder.is_some());
}

#[derive(Serialize)]
struct CurrentPageResponse {
    path: String,
}

async fn current_page(State(state): State<Arc<AppState>>) -> Json<CurrentPageResponse> {
    let path = state.current_path.lock().unwrap().clone();
    Json(CurrentPageResponse {
        path: format_page_path(&path),
    })
}

/// Pushes the page at `path` onto the physical device without waiting for a
/// key press — lets the web UI preview a page.
async fn activate_page(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> Result<StatusCode, ApiError> {
    let path = parse_page_path(&raw_path)?;
    // Icons can be http(s) URLs, so this may block on network I/O — run it
    // off the async runtime to avoid stalling other requests waiting on the
    // same state locks.
    let result = tokio::task::spawn_blocking(move || switch_to_path(&state, &path))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    result.map_err(|_| page_not_found(&raw_path))?;
    Ok(StatusCode::OK)
}

/// A key's config as sent to the frontend: `folder` is collapsed to a flag
/// rather than sending the (potentially large) nested page.
#[derive(Serialize)]
struct KeyConfigView {
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<Action>,
    is_folder: bool,
}

impl From<&KeyConfig> for KeyConfigView {
    fn from(config: &KeyConfig) -> Self {
        Self {
            icon: config.icon.clone(),
            action: config.action.clone(),
            is_folder: config.folder.is_some(),
        }
    }
}

async fn list_keys(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> Result<Json<std::collections::HashMap<u8, KeyConfigView>>, ApiError> {
    let path = parse_page_path(&raw_path)?;
    let root = state.root.lock().unwrap();
    let page = page_at(&root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    Ok(Json(page.iter().map(|(&id, c)| (id, c.into())).collect()))
}

async fn get_key(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<Json<KeyConfigView>, ApiError> {
    let path = parse_page_path(&raw_path)?;
    let root = state.root.lock().unwrap();
    let page = page_at(&root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    Ok(Json(page.get(&id).map_or_else(
        || (&KeyConfig::default()).into(),
        Into::into,
    )))
}

async fn get_key_icon(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<Json<String>, ApiError> {
    let path = parse_page_path(&raw_path)?;
    let root = state.root.lock().unwrap();
    let page = page_at(&root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    match page.get(&id).and_then(|c| c.icon.clone()) {
        Some(icon) => Ok(Json(icon)),
        None => Err((StatusCode::NOT_FOUND, format!("no icon set for key {id}"))),
    }
}

async fn get_key_image(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<Response, ApiError> {
    let path = parse_page_path(&raw_path)?;
    let (icon, is_folder) = {
        let root = state.root.lock().unwrap();
        let page = page_at(&root, &path).ok_or_else(|| page_not_found(&raw_path))?;
        let config = page.get(&id);
        (
            config.and_then(|c| c.icon.clone()),
            config.is_some_and(|c| c.folder.is_some()),
        )
    };
    let Some(icon) = icon else {
        if is_folder {
            return Ok(([(header::CONTENT_TYPE, "image/png")], FOLDER_ICON_BYTES).into_response());
        }
        return Err((StatusCode::NOT_FOUND, format!("no icon set for key {id}")));
    };

    if icon.starts_with("http://") || icon.starts_with("https://") {
        let cache_state = Arc::clone(&state);
        let (bytes, mime) =
            tokio::task::spawn_blocking(move || fetch_remote_icon(&cache_state.icon_cache, &icon))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))??;
        return Ok(([(header::CONTENT_TYPE, mime)], bytes).into_response());
    }

    let bytes = tokio::fs::read(&icon)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("failed to read icon: {e}")))?;
    let mime = mime_guess::from_path(&icon).first_or_octet_stream();

    Ok(([(header::CONTENT_TYPE, mime.as_ref())], bytes).into_response())
}

/// Downloads (or reuses a cached copy of) a remote icon and returns its
/// bytes with a best-effort content type: the server's own `Content-Type` if
/// it sent one, otherwise a guess from the URL's extension.
fn fetch_remote_icon(cache: &IconCache, url: &str) -> Result<(Vec<u8>, String), ApiError> {
    let icon = cache
        .get_or_fetch(url)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    Ok((icon.bytes.clone(), icon.mime.clone()))
}

#[derive(Deserialize)]
struct SetIconRequest {
    path: String,
}

async fn set_key_icon_route(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
    Json(req): Json<SetIconRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    // Only push to the physical device if the edited page is the one
    // currently shown — otherwise it'll be picked up next time it's
    // activated.
    if path == *state.current_path.lock().unwrap() {
        // The icon may be an http(s) URL, so this can block on network
        // I/O — keep it off the async runtime (see activate_page).
        let icon_state = Arc::clone(&state);
        let icon_path = req.path.clone();
        tokio::task::spawn_blocking(move || {
            let device = icon_state.device.lock().unwrap();
            set_key_icon(&device, id, &icon_path, &icon_state.icon_cache)
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("failed to set key icon: {e}"),
            )
        })?;
    }

    let mut root = state.root.lock().unwrap();
    let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    page.entry(id).or_default().icon = Some(req.path);
    drop(root);

    persist(&state)?;
    Ok(StatusCode::OK)
}

async fn clear_key_icon(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    if path == *state.current_path.lock().unwrap() {
        let device = state.device.lock().unwrap();
        clear_key_image(&device, id, &state.icon_cache).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to clear key: {e}"),
            )
        })?;
    }

    let mut root = state.root.lock().unwrap();
    let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    if let Some(config) = page.get_mut(&id) {
        config.icon = None;
    }
    drop(root);

    persist(&state)?;
    Ok(StatusCode::OK)
}

async fn get_key_action(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<Json<Action>, ApiError> {
    let path = parse_page_path(&raw_path)?;
    let root = state.root.lock().unwrap();
    let page = page_at(&root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    match page.get(&id).and_then(|c| c.action.clone()) {
        Some(action) => Ok(Json(action)),
        None => Err((StatusCode::NOT_FOUND, format!("no action set for key {id}"))),
    }
}

async fn set_key_action(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
    Json(action): Json<Action>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    let mut root = state.root.lock().unwrap();
    let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    let config = page.entry(id).or_default();
    if config.folder.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "key is a folder; remove the folder before setting an action".into(),
        ));
    }
    config.action = Some(action);
    drop(root);

    persist(&state)?;
    Ok(StatusCode::OK)
}

async fn clear_key_action(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    let mut root = state.root.lock().unwrap();
    let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    if let Some(config) = page.get_mut(&id) {
        config.action = None;
    }
    drop(root);

    persist(&state)?;
    Ok(StatusCode::OK)
}

/// Turns key `id` on `path` into a folder, giving it an (initially empty)
/// nested page. Idempotent — does nothing if it's already a folder, so an
/// existing nested page is never wiped out.
async fn create_folder(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    let mut root = state.root.lock().unwrap();
    let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
    let config = page.entry(id).or_default();
    if config.folder.is_none() {
        config.folder = Some(KeyConfigMap::new());
        config.action = None;
    }
    drop(root);

    persist(&state)?;
    Ok(StatusCode::CREATED)
}

/// Removes key `id`'s folder, deleting everything nested inside it — the
/// folder's page is only ever reachable through this key.
async fn delete_folder(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    let removed = {
        let mut root = state.root.lock().unwrap();
        let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
        page.get_mut(&id).is_some_and(|c| c.folder.take().is_some())
    };

    if !removed {
        return Err((StatusCode::NOT_FOUND, format!("key {id} is not a folder")));
    }

    let mut folder_path = path.clone();
    folder_path.push(id);
    let device_was_inside = state.current_path.lock().unwrap().starts_with(&folder_path);

    persist(&state)?;

    if device_was_inside {
        // See activate_page: icons can be URLs, so this may block on
        // network I/O — keep it off the async runtime.
        let result = tokio::task::spawn_blocking(move || switch_to_path(&state, &path))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
        if let Err(e) = result {
            eprintln!("Failed to switch back after deleting the active folder: {e}");
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct MoveKeyRequest {
    from_path: String,
    from_id: u8,
    to_path: String,
    to_id: u8,
}

/// Moves (or, if the destination slot is occupied, swaps) a key's whole
/// config — icon, action, and folder — to another slot, which may be on a
/// different page. Used by the web UI's drag-and-drop.
async fn move_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MoveKeyRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(req.from_id)?;
    check_key_range(req.to_id)?;
    let from_path = parse_page_path(&req.from_path)?;
    let to_path = parse_page_path(&req.to_path)?;

    // A folder can't be moved into its own (possibly deeply) nested page —
    // that would disconnect it from the tree reachable from the root.
    if to_path.len() > from_path.len()
        && to_path[..from_path.len()] == from_path[..]
        && to_path[from_path.len()] == req.from_id
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot move a folder into itself".into(),
        ));
    }

    {
        let mut root = state.root.lock().unwrap();

        let moved = {
            let from_page =
                page_at_mut(&mut root, &from_path).ok_or_else(|| page_not_found(&req.from_path))?;
            from_page.remove(&req.from_id)
        };

        let displaced = {
            let to_page =
                page_at_mut(&mut root, &to_path).ok_or_else(|| page_not_found(&req.to_path))?;
            let displaced = to_page.remove(&req.to_id);
            if let Some(cfg) = moved {
                to_page.insert(req.to_id, cfg);
            }
            displaced
        };

        if let Some(cfg) = displaced {
            // Guaranteed to exist: we just looked it up above, and the lock
            // on `root` has been held continuously since.
            let from_page = page_at_mut(&mut root, &from_path).unwrap();
            from_page.insert(req.from_id, cfg);
        }
    }

    persist(&state)?;

    let current = state.current_path.lock().unwrap().clone();
    if current == from_path || current == to_path {
        // See activate_page: icons can be URLs, so this may block on
        // network I/O — keep it off the async runtime.
        let result = tokio::task::spawn_blocking(move || switch_to_path(&state, &current))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
        result.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    }

    Ok(StatusCode::OK)
}
