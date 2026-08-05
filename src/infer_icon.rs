use std::path::Path;

use crate::action::Action;

/// Best-effort guess at an icon for a key whose action was just set but that
/// has no icon of its own yet:
/// - `OpenUrl` uses the target site's favicon.
/// - `RunCommand` uses the target program's own icon (Windows only).
///
/// Returns a path or URL the existing icon pipeline already knows how to
/// load (see `push_image::set_key_icon` / `api::get_key_image`), or `None`
/// if nothing could be inferred — callers should treat that as "leave the
/// key without an icon" rather than an error.
pub fn infer_icon(action: &Action, cache_dir: &Path) -> Option<String> {
    match action {
        Action::OpenUrl { url } => favicon_url(url),
        Action::RunCommand { command } => command_icon(command, cache_dir),
        Action::OpenFolder { .. } => None,
    }
}

/// Derives the conventional favicon location for `url`: `/favicon.ico` at
/// its origin. Returns `None` if `url` isn't an absolute `http(s)` URL.
fn favicon_url(url: &str) -> Option<String> {
    let uri: axum::http::Uri = url.parse().ok()?;
    let scheme = uri.scheme_str()?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = uri.authority()?;
    Some(format!("{scheme}://{authority}/favicon.ico"))
}

/// Extracts the program name/path from a shell command line: the first
/// token, respecting a leading quoted path (e.g.
/// `"C:\Program Files\App\app.exe" --flag`).
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

    /// Resolves `program` to a file Windows can extract an icon from: as
    /// given, if it already names a path, or by searching `PATH` (trying a
    /// `.exe` suffix if the bare name has none).
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
    fn favicon_url_derives_from_origin() {
        let action = Action::OpenUrl {
            url: "https://example.com/some/page".to_string(),
        };
        assert_eq!(
            infer_icon(&action, Path::new(".")),
            Some("https://example.com/favicon.ico".to_string())
        );
    }
}
