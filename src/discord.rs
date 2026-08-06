//! Drives the local Discord desktop client over its RPC IPC socket (the same
//! mechanism Discord's own SDK and third-party rich-presence tools use), so
//! that actions here move the user's real client, not a bot.
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const DISCORD_CONFIG_PATH: &str = "discord_config.json";
const RPC_VERSION: u32 = 1;
const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;

#[cfg(windows)]
type PipeStream = std::fs::File;
#[cfg(unix)]
type PipeStream = std::os::unix::net::UnixStream;

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiscordConfig {
    client_id: String,
    client_secret: String,
    #[serde(default = "default_redirect_uri")]
    redirect_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

fn default_redirect_uri() -> String {
    "http://127.0.0.1:3000/api/discord/callback".to_string()
}

fn load_config() -> anyhow::Result<DiscordConfig> {
    let contents = std::fs::read_to_string(DISCORD_CONFIG_PATH).map_err(|e| {
        anyhow::anyhow!(
            "no {DISCORD_CONFIG_PATH} found ({e}); create one with client_id/client_secret \
             from a Discord application (see discord_config.example.json)"
        )
    })?;
    Ok(serde_json::from_str(&contents)?)
}

fn save_config(config: &DiscordConfig) -> anyhow::Result<()> {
    let contents = serde_json::to_string_pretty(config)?;
    std::fs::write(DISCORD_CONFIG_PATH, contents)?;
    Ok(())
}

/// Kept open across calls, reused until a command on it fails, at which
/// point it's dropped and the next call reconnects.
static SESSION: Mutex<Option<PipeStream>> = Mutex::new(None);

/// Opens an IPC connection to the local Discord client, authenticated and
/// ready for commands (prompting the user to authorize, if needed).
fn open_session() -> anyhow::Result<PipeStream> {
    let mut config = load_config()?;
    let mut pipe = connect_pipe()?;

    handshake(&mut pipe, &config.client_id)?;
    authenticate(&mut pipe, &mut config)?;

    Ok(pipe)
}

/// Runs `f` against the shared, authenticated Discord IPC session, opening
/// one first if there isn't one yet. On failure the session is dropped so
/// the next call reconnects instead of repeating the same error.
fn with_session<T>(f: impl FnOnce(&mut PipeStream) -> anyhow::Result<T>) -> anyhow::Result<T> {
    let mut guard = SESSION.lock().unwrap();
    if guard.is_none() {
        *guard = Some(open_session()?);
    }

    match f(guard.as_mut().unwrap()) {
        Ok(value) => Ok(value),
        Err(e) => {
            *guard = None;
            Err(e)
        }
    }
}

/// Joins the local Discord client to `channel_id` by voice channel snowflake
/// ID, or leaves it if the client is already connected to that channel.
pub fn join_voice_channel(channel_id: &str) -> anyhow::Result<()> {
    with_session(|pipe| {
        let current = send_command(pipe, "GET_SELECTED_VOICE_CHANNEL", &json!({}))?;
        if is_error(&current) {
            anyhow::bail!("Discord RPC error: {}", error_message(&current));
        }
        let already_connected = current
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(Value::as_str)
            == Some(channel_id);

        let target_channel_id = if already_connected {
            Value::Null
        } else {
            Value::String(channel_id.to_string())
        };
        let response = send_command(
            pipe,
            "SELECT_VOICE_CHANNEL",
            &json!({ "channel_id": target_channel_id, "force": true }),
        )?;
        if is_error(&response) {
            anyhow::bail!("Discord RPC error: {}", error_message(&response));
        }
        Ok(())
    })
}

/// Looks up the icon of the guild `channel_id` belongs to, for inferring a
/// key icon on a freshly set `DiscordJoinVoice` action. `Ok(None)` covers
/// both a DM channel (no guild) and a guild with no icon set.
pub fn guild_icon_for_channel(channel_id: &str) -> anyhow::Result<Option<String>> {
    with_session(|pipe| {
        let channel = send_command(pipe, "GET_CHANNEL", &json!({ "channel_id": channel_id }))?;
        if is_error(&channel) {
            anyhow::bail!("Discord RPC error: {}", error_message(&channel));
        }
        let Some(guild_id) = channel
            .get("data")
            .and_then(|d| d.get("guild_id"))
            .and_then(Value::as_str)
        else {
            return Ok(None);
        };

        let guild = send_command(pipe, "GET_GUILD", &json!({ "guild_id": guild_id }))?;
        if is_error(&guild) {
            anyhow::bail!("Discord RPC error: {}", error_message(&guild));
        }
        Ok(guild
            .get("data")
            .and_then(|d| d.get("icon_url"))
            .and_then(Value::as_str)
            .map(str::to_string))
    })
}

#[cfg(windows)]
fn connect_pipe() -> anyhow::Result<PipeStream> {
    use std::fs::OpenOptions;

    for i in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{i}");
        if let Ok(file) = OpenOptions::new().read(true).write(true).open(&path) {
            return Ok(file);
        }
    }
    anyhow::bail!("could not find a running Discord client (no discord-ipc-N pipe)")
}

#[cfg(unix)]
fn connect_pipe() -> anyhow::Result<PipeStream> {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    for i in 0..10 {
        let path = format!("{base}/discord-ipc-{i}");
        if let Ok(stream) = PipeStream::connect(&path) {
            return Ok(stream);
        }
    }
    anyhow::bail!("could not find a running Discord client (no discord-ipc-N socket)")
}

fn write_frame(pipe: &mut PipeStream, opcode: u32, payload: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_vec(payload)?;
    pipe.write_all(&opcode.to_le_bytes())?;
    #[allow(clippy::cast_possible_truncation)]
    pipe.write_all(&(body.len() as u32).to_le_bytes())?;
    pipe.write_all(&body)?;
    pipe.flush()?;
    Ok(())
}

fn read_frame(pipe: &mut PipeStream) -> anyhow::Result<Value> {
    let mut header = [0u8; 8];
    pipe.read_exact(&mut header)?;
    let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let mut body = vec![0u8; len];
    pipe.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn handshake(pipe: &mut PipeStream, client_id: &str) -> anyhow::Result<()> {
    write_frame(
        pipe,
        OP_HANDSHAKE,
        &json!({ "v": RPC_VERSION, "client_id": client_id }),
    )?;
    let response = read_frame(pipe)?;
    if response.get("evt").and_then(Value::as_str) != Some("READY") {
        anyhow::bail!("unexpected handshake response from Discord: {response}");
    }
    Ok(())
}

/// Sends a `FRAME` command and waits for the response carrying the same
/// nonce, skipping any unsolicited `DISPATCH` events in between.
fn send_command(pipe: &mut PipeStream, cmd: &str, args: &Value) -> anyhow::Result<Value> {
    let nonce = {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        format!("{}-{}", now.as_secs(), now.subsec_nanos())
    };
    write_frame(
        pipe,
        OP_FRAME,
        &json!({ "cmd": cmd, "args": args, "nonce": nonce }),
    )?;

    for _ in 0..8 {
        let response = read_frame(pipe)?;
        if response.get("nonce").and_then(Value::as_str) == Some(nonce.as_str()) {
            return Ok(response);
        }
    }
    anyhow::bail!("no response from Discord for '{cmd}' after several attempts")
}

fn is_error(response: &Value) -> bool {
    response.get("evt").and_then(Value::as_str) == Some("ERROR")
}

fn error_message(response: &Value) -> &str {
    response
        .get("data")
        .and_then(|d| d.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("unknown error")
}

/// Authenticates the IPC session, reusing a saved access token when
/// possible. Falls back to a full authorize + `OAuth2` code exchange
/// (which pops an in-client "Authorize" dialog for the user) otherwise. An
/// expired access token is silently refreshed via the saved refresh token
/// when possible, so re-authorizing only prompts the user when that fails
/// too (e.g. the grant was revoked).
fn authenticate(pipe: &mut PipeStream, config: &mut DiscordConfig) -> anyhow::Result<()> {
    if let Some(token) = config.access_token.clone()
        && try_authenticate(pipe, &token)?
    {
        return Ok(());
    }

    if let Some(refresh_token) = config.refresh_token.clone()
        && let Ok((access_token, new_refresh_token)) = refresh_access_token(config, &refresh_token)
    {
        config.access_token = Some(access_token.clone());
        config.refresh_token = Some(new_refresh_token);
        save_config(config)?;

        if try_authenticate(pipe, &access_token)? {
            return Ok(());
        }
    }

    let code = authorize(pipe, &config.client_id)?;
    let (access_token, refresh_token) = exchange_code(config, &code)?;
    config.access_token = Some(access_token.clone());
    config.refresh_token = Some(refresh_token);
    save_config(config)?;

    if !try_authenticate(pipe, &access_token)? {
        anyhow::bail!("Discord rejected the freshly authorized access token");
    }
    Ok(())
}

/// Returns `true` if `access_token` authenticated the session.
fn try_authenticate(pipe: &mut PipeStream, access_token: &str) -> anyhow::Result<bool> {
    let response = send_command(
        pipe,
        "AUTHENTICATE",
        &json!({ "access_token": access_token }),
    )?;
    Ok(!is_error(&response))
}

fn authorize(pipe: &mut PipeStream, client_id: &str) -> anyhow::Result<String> {
    let response = send_command(
        pipe,
        "AUTHORIZE",
        &json!({ "client_id": client_id, "scopes": ["rpc"] }),
    )?;
    if is_error(&response) {
        anyhow::bail!(
            "Discord rejected the authorization request: {}",
            error_message(&response)
        );
    }
    response
        .get("data")
        .and_then(|d| d.get("code"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("AUTHORIZE response had no code: {response}"))
}

/// Exchanges an RPC authorization code for an access/refresh token pair via
/// Discord's standard `OAuth2` token endpoint. `redirect_uri` must match one
/// registered on the application, even though this local flow never visits it.
fn exchange_code(config: &DiscordConfig, code: &str) -> anyhow::Result<(String, String)> {
    request_token(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("redirect_uri", config.redirect_uri.as_str()),
    ])
}

/// Silently exchanges a saved refresh token for a new access/refresh token
/// pair, avoiding the in-client "Authorize" dialog for an expired access token.
fn refresh_access_token(
    config: &DiscordConfig,
    refresh_token: &str,
) -> anyhow::Result<(String, String)> {
    request_token(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
    ])
}

fn request_token(form: &[(&str, &str)]) -> anyhow::Result<(String, String)> {
    let mut response = ureq::post("https://discord.com/api/oauth2/token")
        .send_form(form.iter().copied())
        .map_err(|e| anyhow::anyhow!("token request failed: {e}"))?;

    let body_str = response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("failed to read token response: {e}"))?;
    let body: Value = serde_json::from_str(&body_str)?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("token response missing access_token: {body}"))?
        .to_string();
    let refresh_token = body["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("token response missing refresh_token: {body}"))?
        .to_string();

    Ok((access_token, refresh_token))
}
