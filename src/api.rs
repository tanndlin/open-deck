use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::action::Action;
use crate::config::{KeyConfig, KeyConfigMap, page_at, page_at_mut, save_key_config};
use crate::icon_cache::IconCache;
use crate::infer_icon::infer_icon;
use crate::push_image::{FOLDER_ICON_BYTES, clear_key_image, set_folder_icon, set_key_icon};
use crate::{AppState, KEY_COUNT, switch_to_path};

type ApiError = (StatusCode, String);

/// Generous enough for any reasonable key icon image, well past axum's 2MB
/// default request body limit.
const UPLOAD_ICON_MAX_BYTES: usize = 25 * 1024 * 1024;

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
            "/api/pages/{path}/keys/{id}/icon/upload",
            put(upload_key_icon).layer(DefaultBodyLimit::max(UPLOAD_ICON_MAX_BYTES)),
        )
        .route(
            "/api/pages/{path}/keys/{id}/title",
            put(set_key_title).delete(clear_key_title),
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
        .route("/api/ws", get(ws_handler))
        .with_state(state)
}

/// Pushed to `/api/ws` clients so the GUI's device state never drifts from
/// what's actually happening on the physical Stream Deck.
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// The page currently pushed onto the device changed (device-triggered or
    /// GUI-triggered). Also sent once, immediately, when a client connects,
    /// so it never has to separately fetch `/api/current-page` to sync up.
    PageChanged { path: String },
    /// A physical key was pressed, on the page shown on the device at that
    /// moment. Purely informational — a `PageChanged` may follow if the press
    /// navigated somewhere.
    KeyPressed { path: String, id: u8 },
}

async fn ws_handler(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut events = state.events.subscribe();

    let snapshot = ServerEvent::PageChanged {
        path: format_page_path(&state.current_path.lock().unwrap()),
    };
    if !send_event(&mut socket, &snapshot).await {
        return;
    }

    loop {
        tokio::select! {
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        if !send_event(&mut socket, &event).await {
                            break;
                        }
                    }
                    // A slow client missed some events — its next message is
                    // still current, so just keep going.
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // The GUI never sends anything meaningful; this is only here to
            // notice the socket closing so the task can exit.
            msg = socket.recv() => {
                if !matches!(msg, Some(Ok(_))) {
                    break;
                }
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: &ServerEvent) -> bool {
    let Ok(text) = serde_json::to_string(event) else {
        return true;
    };
    socket.send(Message::Text(text.into())).await.is_ok()
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

/// Parses a `.`-joined sequence of key indices (e.g. `"3.1"`), or `"home"` for the empty path.
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

pub(crate) fn format_page_path(path: &[u8]) -> String {
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

/// Persists the page tree, dropping now-empty key entries so the config file doesn't accumulate dead keys.
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
    page.retain(|_, c| {
        c.icon.is_some() || c.title.is_some() || c.action.is_some() || c.folder.is_some()
    });
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

/// Pushes the page at `path` onto the device immediately, so the web UI can preview it.
async fn activate_page(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> Result<StatusCode, ApiError> {
    let path = parse_page_path(&raw_path)?;
    // Icons can be http(s) URLs, so keep this off the async runtime.
    let result = tokio::task::spawn_blocking(move || switch_to_path(&state, &path))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    result.map_err(|_| page_not_found(&raw_path))?;
    Ok(StatusCode::OK)
}

/// A key's config as sent to the frontend: `folder` is collapsed to a flag rather than the nested page.
#[derive(Serialize)]
struct KeyConfigView {
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<Action>,
    is_folder: bool,
}

impl From<&KeyConfig> for KeyConfigView {
    fn from(config: &KeyConfig) -> Self {
        Self {
            icon: config.icon.clone(),
            title: config.title.clone(),
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

/// Downloads (or reuses a cached copy of) a remote icon, with a best-effort content type.
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

/// Pushes `icon` to the device and persists it as key `id`'s icon. Shared by the
/// icon-setting route and the favicon auto-fill in [`set_key_action`].
async fn apply_icon(
    state: &Arc<AppState>,
    path: &[u8],
    id: u8,
    icon: String,
) -> Result<(), ApiError> {
    // Only push to the device if the edited page is the one currently shown
    // — otherwise it's picked up next time it's activated.
    if path == state.current_path.lock().unwrap().as_slice() {
        let title = {
            let root = state.root.lock().unwrap();
            page_at(&root, path)
                .and_then(|p| p.get(&id))
                .and_then(|c| c.title.clone())
        };

        // May block on network I/O (see activate_page).
        let icon_state = Arc::clone(state);
        let icon_path = icon.clone();
        tokio::task::spawn_blocking(move || {
            let device = icon_state.device.lock().unwrap();
            set_key_icon(
                &device,
                id,
                &icon_path,
                title.as_deref(),
                &icon_state.icon_cache,
            )
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
    let page =
        page_at_mut(&mut root, path).ok_or_else(|| page_not_found(&format_page_path(path)))?;
    page.entry(id).or_default().icon = Some(icon);
    drop(root);

    persist(state)
}

async fn set_key_icon_route(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
    Json(req): Json<SetIconRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;
    apply_icon(&state, &path, id, req.path).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct UploadIconQuery {
    /// Used only to recover the extension for a correct content type.
    filename: String,
}

/// Saves an uploaded icon under `<config dir>/icons/`, named by a content hash
/// (so re-uploads reuse the same file), and sets it as key `id`'s icon.
async fn upload_key_icon(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
    Query(query): Query<UploadIconQuery>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    let icons_dir = std::path::Path::new(&state.config_path).with_file_name("icons");
    tokio::fs::create_dir_all(&icons_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to create icons directory: {e}"),
        )
    })?;

    let ext = std::path::Path::new(&query.filename)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("png");

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&body[..], &mut hasher);
    let out_path = icons_dir.join(format!("{:x}.{ext}", std::hash::Hasher::finish(&hasher)));

    tokio::fs::write(&out_path, &body).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to save icon: {e}"),
        )
    })?;

    apply_icon(&state, &path, id, out_path.to_string_lossy().into_owned()).await?;
    Ok(StatusCode::OK)
}

async fn clear_key_icon(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;

    if path == *state.current_path.lock().unwrap() {
        let title = {
            let root = state.root.lock().unwrap();
            page_at(&root, &path)
                .and_then(|p| p.get(&id))
                .and_then(|c| c.title.clone())
        };
        let device = state.device.lock().unwrap();
        clear_key_image(&device, id, title.as_deref(), &state.icon_cache).map_err(|e| {
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

#[derive(Deserialize)]
struct SetTitleRequest {
    title: String,
}

/// Re-renders and pushes the key's image with an updated title (if the
/// edited page is currently shown) and persists it. Mirrors `apply_icon`.
async fn apply_title(
    state: &Arc<AppState>,
    path: &[u8],
    id: u8,
    title: Option<String>,
) -> Result<(), ApiError> {
    if path == state.current_path.lock().unwrap().as_slice() {
        let (icon, is_folder) = {
            let root = state.root.lock().unwrap();
            let config = page_at(&root, path).and_then(|p| p.get(&id));
            (
                config.and_then(|c| c.icon.clone()),
                config.is_some_and(|c| c.folder.is_some()),
            )
        };

        // May block on network I/O (see activate_page).
        let push_state = Arc::clone(state);
        let push_title = title.clone();
        tokio::task::spawn_blocking(move || {
            let device = push_state.device.lock().unwrap();
            match &icon {
                Some(icon_path) => set_key_icon(
                    &device,
                    id,
                    icon_path,
                    push_title.as_deref(),
                    &push_state.icon_cache,
                ),
                None if is_folder => {
                    set_folder_icon(&device, id, push_title.as_deref(), &push_state.icon_cache)
                }
                None => clear_key_image(&device, id, push_title.as_deref(), &push_state.icon_cache),
            }
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to set key title: {e}"),
            )
        })?;
    }

    let mut root = state.root.lock().unwrap();
    let page =
        page_at_mut(&mut root, path).ok_or_else(|| page_not_found(&format_page_path(path)))?;
    page.entry(id).or_default().title = title;
    drop(root);

    persist(state)
}

async fn set_key_title(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
    Json(req): Json<SetTitleRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;
    apply_title(&state, &path, id, Some(req.title)).await?;
    Ok(StatusCode::OK)
}

async fn clear_key_title(
    State(state): State<Arc<AppState>>,
    Path((raw_path, id)): Path<(String, u8)>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;
    let path = parse_page_path(&raw_path)?;
    apply_title(&state, &path, id, None).await?;
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

    let has_icon = {
        let mut root = state.root.lock().unwrap();
        let page = page_at_mut(&mut root, &path).ok_or_else(|| page_not_found(&raw_path))?;
        let config = page.entry(id).or_default();
        if config.folder.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                "key is a folder; remove the folder before setting an action".into(),
            ));
        }
        config.action = Some(action.clone());
        config.icon.is_some()
    };

    persist(&state)?;

    // Best-effort: infer an icon for a key that doesn't have one yet.
    // Failure here shouldn't fail the request — the action is already saved.
    if !has_icon {
        let cache_dir = std::path::Path::new(&state.config_path).with_file_name("icon-cache");
        let inferred = tokio::task::spawn_blocking(move || infer_icon(&action, &cache_dir))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
        if let Some(icon) = inferred
            && let Err((_, e)) = apply_icon(&state, &path, id, icon).await
        {
            eprintln!("Failed to set inferred icon for key {id}: {e}");
        }
    }

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

/// Idempotent — does nothing if `id` is already a folder, so an existing nested page survives.
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

/// Deletes everything nested inside key `id`'s folder along with it.
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
        // May block on network I/O (see activate_page).
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

/// Moves a key's config to another slot (possibly on a different page); swaps if occupied.
async fn move_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MoveKeyRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(req.from_id)?;
    check_key_range(req.to_id)?;
    let from_path = parse_page_path(&req.from_path)?;
    let to_path = parse_page_path(&req.to_path)?;

    // A folder can't be moved into its own nested page — that would
    // disconnect it from the tree reachable from the root.
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
        // May block on network I/O (see activate_page).
        let result = tokio::task::spawn_blocking(move || switch_to_path(&state, &current))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
        result.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    }

    Ok(StatusCode::OK)
}
