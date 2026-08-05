use std::path::Path;

use crate::action::Action;

/// Best-effort: `OpenUrl` uses the site's favicon, `RunCommand` uses the
/// program's own icon (Windows only). `None` means leave the key without one.
pub fn infer_icon(action: &Action, cache_dir: &Path) -> Option<String> {
    match action {
        Action::OpenUrl { url } => favicon_url(url),
        Action::RunCommand { command } => command_icon(command, cache_dir),
        Action::OpenFolder { .. } | Action::TypeText { .. } | Action::Hotkey { .. } => None,
    }
}

/// Re-runs favicon inference against `url`'s origin when `url` looks like a
/// guessed `/favicon.ico` path. `None` means the original failure stands.
pub fn recover_favicon(url: &str) -> Option<String> {
    if !looks_like_favicon_guess(url) {
        return None;
    }
    let uri: axum::http::Uri = url.parse().ok()?;
    let scheme = uri.scheme_str()?;
    let authority = uri.authority()?;
    let recovered = favicon_url(&format!("{scheme}://{authority}/"))?;
    (recovered != url).then_some(recovered)
}

/// Scopes [`recover_favicon`] to the conventional `/favicon.ico` guess, not any broken URL.
fn looks_like_favicon_guess(url: &str) -> bool {
    url::Url::parse(url)
        .is_ok_and(|u| u.query().is_none() && u.path().eq_ignore_ascii_case("/favicon.ico"))
}

/// Only the `<head>` (where favicon `<link>`s live) needs to be scanned, so
/// a page's often much larger body never has to be downloaded.
const MAX_HTML_SCAN_BYTES: usize = 256 * 1024;

/// Tries, in order: the site's declared favicon, `DuckDuckGo`'s favicon lookup
/// (works even behind e.g. Cloudflare), then the conventional `/favicon.ico`.
fn favicon_url(url: &str) -> Option<String> {
    let fallback = origin_favicon(url)?;
    if let Some(icon) = declared_favicon(url) {
        return Some(icon);
    }
    Some(favicon_service_url(url).unwrap_or(fallback))
}

/// Doesn't verify the icon exists — the service returns *some* icon for
/// almost any reachable host, so this is only a fallback.
fn favicon_service_url(url: &str) -> Option<String> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_string();
    Some(format!("https://icons.duckduckgo.com/ip3/{host}.ico"))
}

/// Returns `None` if `url` isn't an absolute `http(s)` URL.
fn origin_favicon(url: &str) -> Option<String> {
    let uri: axum::http::Uri = url.parse().ok()?;
    let scheme = uri.scheme_str()?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = uri.authority()?;
    Some(format!("{scheme}://{authority}/favicon.ico"))
}

/// Returns `None` on any failure; callers should fall back to [`origin_favicon`].
fn declared_favicon(url: &str) -> Option<String> {
    let mut response = ureq::get(url).call().ok()?;
    let mut bytes = Vec::new();
    let reader = response.body_mut().as_reader();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(reader, MAX_HTML_SCAN_BYTES as u64),
        &mut bytes,
    )
    .ok()?;
    let html = String::from_utf8_lossy(&bytes);

    let href = find_icon_href(&html)?;
    let base = url::Url::parse(url).ok()?;
    base.join(&href).ok().map(String::from)
}

/// Prefers an exact `rel="icon"`/`"shortcut icon"`, else the first `rel` containing `"icon"`.
fn find_icon_href(html: &str) -> Option<String> {
    let mut fallback: Option<String> = None;
    for tag in find_tags(html, "link") {
        let attrs = parse_attrs(tag);
        let Some(rel) = attrs.get("rel").map(|r| r.to_lowercase()) else {
            continue;
        };
        let Some(href) = attrs.get("href").filter(|h| !h.is_empty()) else {
            continue;
        };
        if rel == "icon" || rel == "shortcut icon" {
            return Some(href.clone());
        }
        if fallback.is_none() && rel.contains("icon") {
            fallback = Some(href.clone());
        }
    }
    fallback
}

/// Returns each `<name ...>` tag's inner text (without the surrounding `<`/`>`), case-insensitively.
fn find_tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let lower = html.to_lowercase();
    let open = format!("<{name}");
    let mut tags = Vec::new();
    let mut pos = 0;
    while let Some(rel_start) = lower[pos..].find(open.as_str()) {
        let start = pos + rel_start;
        let after = start + open.len();
        let boundary_ok = lower
            .as_bytes()
            .get(after)
            .is_none_or(|&c| c.is_ascii_whitespace() || c == b'>' || c == b'/');
        let Some(rel_end) = lower[after..].find('>') else {
            break;
        };
        let end = after + rel_end;
        if boundary_ok {
            tags.push(&html[start + 1..end]);
        }
        pos = end + 1;
    }
    tags
}

/// Parses `name="value"`, `name='value'`, or bare `name` attributes from a tag's inner text.
fn parse_attrs(tag_inner: &str) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    let bytes = tag_inner.as_bytes();
    let mut i = 0;

    // Skip the tag name itself.
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name = tag_inner[name_start..i].trim_end_matches('/');
        if name.is_empty() {
            i += 1;
            continue;
        }

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            attrs.insert(name.to_lowercase(), String::new());
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let value = if let Some(&quote @ (b'"' | b'\'')) = bytes.get(i) {
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let value = &tag_inner[value_start..i];
            i += 1;
            value
        } else {
            let value_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            &tag_inner[value_start..i]
        };
        attrs.insert(name.to_lowercase(), value.to_string());
    }

    attrs
}

/// The first token of `command`, respecting a leading quoted path.
#[cfg(windows)]
fn command_program(command: &str) -> Option<&str> {
    let trimmed = command.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        return rest.split('"').next().filter(|s| !s.is_empty());
    }
    trimmed.split_whitespace().next()
}

#[cfg(windows)]
fn command_icon(command: &str, cache_dir: &Path) -> Option<String> {
    use std::hash::{Hash, Hasher};

    /// As given if it already names a path, else searches `PATH` (trying `.exe`).
    fn resolve_executable(program: &str) -> Option<std::path::PathBuf> {
        let path = Path::new(program);
        if path.components().count() > 1 {
            return path.exists().then(|| path.to_path_buf());
        }

        let names: Vec<String> = if path.extension().is_some() {
            vec![program.to_string()]
        } else {
            vec![program.to_string(), format!("{program}.exe")]
        };

        let path_var = std::env::var_os("PATH")?;
        std::env::split_paths(&path_var).find_map(|dir| {
            names
                .iter()
                .map(|name| dir.join(name))
                .find(|candidate| candidate.exists())
        })
    }

    let program = command_program(command)?;
    let exe = resolve_executable(program)?;

    let image = windows_icons::get_icon_by_path(&exe).ok()?;

    std::fs::create_dir_all(cache_dir).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    exe.hash(&mut hasher);
    let out_path = cache_dir.join(format!("{:x}.png", hasher.finish()));
    image.save(&out_path).ok()?;

    Some(out_path.to_string_lossy().into_owned())
}

#[cfg(not(windows))]
fn command_icon(_command: &str, _cache_dir: &Path) -> Option<String> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn command_program_handles_quoted_and_bare_commands() {
        assert_eq!(
            command_program(r#""C:\Program Files\App\app.exe" --flag"#),
            Some(r"C:\Program Files\App\app.exe")
        );
        assert_eq!(command_program("notepad.exe"), Some("notepad.exe"));
        assert_eq!(command_program("python script.py"), Some("python"));
    }

    #[test]
    fn infers_icon_for_a_command_on_path() {
        let cache_dir = std::env::temp_dir().join("open-deck-test-icon-cache");
        let action = Action::RunCommand {
            command: "notepad.exe".to_string(),
        };

        let icon = infer_icon(&action, &cache_dir).expect("notepad.exe should have an icon");
        assert!(std::path::Path::new(&icon).exists());

        std::fs::remove_dir_all(&cache_dir).ok();
    }

    #[test]
    fn origin_favicon_derives_from_scheme_and_authority() {
        assert_eq!(
            origin_favicon("https://example.com/some/page"),
            Some("https://example.com/favicon.ico".to_string())
        );
        assert_eq!(origin_favicon("not a url"), None);
        assert_eq!(origin_favicon("ftp://example.com/x"), None);
    }

    #[test]
    fn favicon_service_url_derives_from_host() {
        assert_eq!(
            favicon_service_url("https://example.com/some/page"),
            Some("https://icons.duckduckgo.com/ip3/example.com.ico".to_string())
        );
        assert_eq!(favicon_service_url("not a url"), None);
    }

    #[test]
    fn looks_like_favicon_guess_matches_only_a_bare_favicon_ico_path() {
        assert!(looks_like_favicon_guess("https://example.com/favicon.ico"));
        assert!(looks_like_favicon_guess("https://example.com/FAVICON.ICO"));
        assert!(!looks_like_favicon_guess(
            "https://example.com/path/favicon.ico"
        ));
        assert!(!looks_like_favicon_guess(
            "https://example.com/favicon.ico?v=1"
        ));
        assert!(!looks_like_favicon_guess("https://example.com/logo.png"));
        assert!(!looks_like_favicon_guess("not a url"));
    }

    #[test]
    fn recover_favicon_ignores_urls_that_dont_look_like_a_guess() {
        assert_eq!(recover_favicon("https://example.com/logo.png"), None);
    }

    #[test]
    fn find_icon_href_prefers_exact_icon_rel() {
        let html = r#"
            <html><head>
            <link rel="apple-touch-icon" href="/apple.png">
            <link rel="stylesheet" href="/site.css">
            <link rel="icon" href="/favicon.png" type="image/png">
            </head></html>
        "#;
        assert_eq!(find_icon_href(html), Some("/favicon.png".to_string()));
    }

    #[test]
    fn find_icon_href_matches_shortcut_icon_and_is_case_insensitive() {
        let html = r#"<LINK REL="Shortcut Icon" HREF="/shortcut.ico">"#;
        assert_eq!(find_icon_href(html), Some("/shortcut.ico".to_string()));
    }

    #[test]
    fn find_icon_href_falls_back_to_apple_touch_icon() {
        let html = r"<link rel='apple-touch-icon' href='/apple-touch.png'>";
        assert_eq!(find_icon_href(html), Some("/apple-touch.png".to_string()));
    }

    #[test]
    fn find_icon_href_returns_none_without_a_matching_link() {
        let html = r#"<link rel="stylesheet" href="/site.css">"#;
        assert_eq!(find_icon_href(html), None);
    }

    #[test]
    fn find_icon_href_ignores_tags_with_icon_as_a_prefix() {
        let html = r#"<linkage rel="icon" href="/nope.png"><link rel="icon" href="/yes.png">"#;
        assert_eq!(find_icon_href(html), Some("/yes.png".to_string()));
    }

    #[test]
    fn declared_favicon_resolves_relative_href_against_page_url() {
        let href = find_icon_href(r#"<link rel="icon" href="../static/icon.png">"#).unwrap();
        let base = url::Url::parse("https://example.com/a/b/page").unwrap();
        assert_eq!(
            base.join(&href).unwrap().to_string(),
            "https://example.com/a/static/icon.png"
        );
    }
}
