use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// The built React frontend (`frontend/dist`), baked into the binary at
/// compile time. Run `npm run build` in `frontend/` before `cargo build`.
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

/// Serves an embedded asset by request path, falling back to `index.html`
/// for both the SPA root and unknown paths (client-side routing).
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    serve(path).unwrap_or_else(|| {
        serve("index.html").unwrap_or_else(|| (StatusCode::NOT_FOUND, "404").into_response())
    })
}

fn serve(path: &str) -> Option<Response> {
    let asset = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(([(header::CONTENT_TYPE, mime.as_ref())], asset.data).into_response())
}
