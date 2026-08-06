# Open Deck

A driver and web UI for the Elgato Stream Deck MK2, written in Rust. The
React frontend is compiled and embedded directly into the binary, so the
result is a single executable.

## Download

Prebuilt Linux and Windows binaries are published on every successful build
to the rolling [`latest` release](https://github.com/tanndlin/open-deck/releases/tag/latest).

## Building from source

### Prerequisites

- Rust (stable) with Cargo — https://rustup.rs
- Node.js 22+ and npm
- Linux only: `libudev-dev` and `pkg-config` for the HID backend
  ```
  sudo apt install libudev-dev pkg-config
  ```

### Steps

1. Build the frontend first. The Rust binary embeds `frontend/dist` at
   compile time via `rust-embed`, so it has to exist before `cargo build`.
   ```
   cd frontend
   npm ci
   npm run build
   cd ..
   ```
2. Build the release binary.
   ```
   cargo build --release
   ```
   The result is `target/release/open-deck` (`open-deck.exe` on Windows).

3. Run it.
   ```
   ./target/release/open-deck
   ```
   It reads `config.json` from the working directory and serves the web UI
   at http://127.0.0.1:3000.

### Cross-compiling for Windows from Linux

CI builds the Windows binary this way; to do the same locally:

```
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Discord integration

The `discord_join_voice` action drives your own local Discord desktop client
over its RPC IPC socket (the same mechanism Discord's SDK and rich-presence
tools use) to join a voice channel by ID — it moves your real client, not a
bot, and only works while the Discord desktop app is running on the same
machine.

Setup:

1. Create an application at the
   [Discord Developer Portal](https://discord.com/developers/applications).
2. Under **OAuth2 → General**, add `http://127.0.0.1:3000/api/discord/callback`
   as a redirect. It's never actually visited (this flow authorizes in-client,
   not through a browser), but Discord requires the redirect used in the
   token exchange to be registered.
3. Copy `discord_config.example.json` to `discord_config.json` next to
   `config.json`, and fill in the application's Client ID and Client Secret
   from **OAuth2 → General**.
4. Set a key's action to "Join Discord voice channel" and enter the target
   channel's ID (right-click the channel in Discord with Developer Mode
   enabled → Copy Channel ID). The first press pops an in-client "Authorize"
   dialog; approve it once and the resulting token is cached in
   `discord_config.json` for later presses.

## Development

- `cargo run` rebuilds and runs the backend directly (still needs
  `frontend/dist` built at least once beforehand).
- `cd frontend && npm run dev` runs the frontend with hot reload against
  `vite`, separate from the embedded build.
