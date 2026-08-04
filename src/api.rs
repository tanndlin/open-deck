use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::config::save_key_config;
use crate::push_image::{clear_key_image, set_key_icon};
use crate::{AppState, KEY_COUNT};

type ApiError = (StatusCode, String);

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/key-count", get(key_count))
        .route("/api/keys", get(list_keys))
        .route("/api/keys/{id}", get(get_key).put(set_key).delete(clear_key))
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

async fn list_keys(State(state): State<Arc<AppState>>) -> Json<HashMap<u8, String>> {
    let keys = state.keys.lock().unwrap();
    Json(keys.clone())
}

async fn get_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<Json<String>, ApiError> {
    let keys = state.keys.lock().unwrap();
    match keys.get(&id) {
        Some(path) => Ok(Json(path.clone())),
        None => Err((StatusCode::NOT_FOUND, format!("no icon set for key {id}"))),
    }
}

async fn get_key_image(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<Response, ApiError> {
    let path = {
        let keys = state.keys.lock().unwrap();
        keys.get(&id).cloned()
    };
    let path = path.ok_or_else(|| (StatusCode::NOT_FOUND, format!("no icon set for key {id}")))?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("failed to read icon: {e}")))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    Ok(([(header::CONTENT_TYPE, mime.as_ref())], bytes).into_response())
}

#[derive(Deserialize)]
struct SetKeyRequest {
    path: String,
}

async fn set_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
    Json(req): Json<SetKeyRequest>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    {
        let device = state.device.lock().unwrap();
        set_key_icon(&device, id, &req.path)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("failed to set key icon: {e}")))?;
    }

    let mut keys = state.keys.lock().unwrap();
    keys.insert(id, req.path);
    save_key_config(&state.config_path, &keys)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to save config: {e}")))?;

    Ok(StatusCode::OK)
}

async fn clear_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u8>,
) -> Result<StatusCode, ApiError> {
    check_key_range(id)?;

    {
        let device = state.device.lock().unwrap();
        clear_key_image(&device, id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to clear key: {e}")))?;
    }

    let mut keys = state.keys.lock().unwrap();
    keys.remove(&id);
    save_key_config(&state.config_path, &keys)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to save config: {e}")))?;

    Ok(StatusCode::OK)
}
