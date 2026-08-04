use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::action::Action;
use crate::config::{KeyConfig, KeyConfigMap, save_key_config};
use crate::push_image::{clear_key_image, set_key_icon};
use crate::{AppState, KEY_COUNT};

type ApiError = (StatusCode, String);

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/key-count", get(key_count))
        .route("/api/keys", get(list_keys))
        .route("/api/keys/{id}", get(get_key))
        .route(
            "/api/keys/{id}/icon",
            get(get_key_icon)
                .put(set_key_icon_route)
                .delete(clear_key_icon),
        )
        .route(
            "/api/keys/{id}/action",
            get(get_key_action)
                .put(set_key_action)
                .delete(clear_key_action),
        )
        .route("/api/keys/{id}/image", get(get_key_image))
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

/// Persists `keys`, dropping any entries that are now empty (no icon, no
/// action) so the config file doesn't accumulate dead keys.
fn persist(state: &AppState, keys: &mut KeyConfigMap) -> Result<(), ApiError> {
    keys.retain(|_, c| c.icon.is_some() || c.action.is_some());
    save_key_config(&state.config_path, keys).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to save config: {e}"),
        )
    })
}

async fn list_keys(State(state): State<Arc<AppState>>) -> Json<KeyConfigMap> {
    let keys = state.keys.lock().unwrap();
    Json(keys.clone())
}

async fn get_key(State(state): State<Arc<AppState>>, Path(id): Path<u8>) -> Json<KeyConfig> {
    let keys = state.keys.lock().unwrap();
    Json(keys.get(&id).cloned().unwrap_or_default())
}

async fn get_key_icon(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<Json<String>, ApiError> {
    let keys = state.keys.lock().unwrap();
    match keys.get(&id).and_then(|c| c.icon.clone()) {
        Some(path) => Ok(Json(path)),
        None => Err((StatusCode::NOT_FOUND, format!("no icon set for key {id}"))),
    }
}

async fn get_key_image(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<Response, ApiError> {
    let path = {
        let keys = state.keys.lock().unwrap();
        keys.get(&id).and_then(|c| c.icon.clone())
    };
    let path = path.ok_or_else(|| (StatusCode::NOT_FOUND, format!("no icon set for key {id}")))?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("failed to read icon: {e}")))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    Ok(([(header::CONTENT_TYPE, mime.as_ref())], bytes).into_response())
}

#[derive(Deserialize)]
struct SetIconRequest {
    path: String,
}

async fn set_key_icon_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
    Json(req): Json<SetIconRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    {
        let device = state.device.lock().unwrap();
        set_key_icon(&device, id, &req.path).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("failed to set key icon: {e}"),
            )
        })?;
    }

    let mut keys = state.keys.lock().unwrap();
    keys.entry(id).or_default().icon = Some(req.path);
    persist(&state, &mut keys)?;

    Ok(StatusCode::OK)
}

async fn clear_key_icon(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    {
        let device = state.device.lock().unwrap();
        clear_key_image(&device, id).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to clear key: {e}"),
            )
        })?;
    }

    let mut keys = state.keys.lock().unwrap();
    if let Some(config) = keys.get_mut(&id) {
        config.icon = None;
    }
    persist(&state, &mut keys)?;

    Ok(StatusCode::OK)
}

async fn get_key_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<Json<Action>, ApiError> {
    let keys = state.keys.lock().unwrap();
    match keys.get(&id).and_then(|c| c.action.clone()) {
        Some(action) => Ok(Json(action)),
        None => Err((StatusCode::NOT_FOUND, format!("no action set for key {id}"))),
    }
}

async fn set_key_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
    Json(action): Json<Action>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    let mut keys = state.keys.lock().unwrap();
    keys.entry(id).or_default().action = Some(action);
    persist(&state, &mut keys)?;

    Ok(StatusCode::OK)
}

async fn clear_key_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    let mut keys = state.keys.lock().unwrap();
    if let Some(config) = keys.get_mut(&id) {
        config.action = None;
    }
    persist(&state, &mut keys)?;

    Ok(StatusCode::OK)
}
