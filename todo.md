# TODO

## Discord mute and deafen actions
- [x] Toggle mute via `SET_VOICE_SETTINGS` (`{ "mute": bool }`)
- [x] Toggle deafen via `SET_VOICE_SETTINGS` (`{ "deaf": bool }`)
- [x] Request the `rpc.voice.write` scope (plus `rpc.voice.read`)
- [x] Frontend action fields for both

## Media keys
- [x] Add media/volume names to `parse_key` (`Key::MediaPlayPause`, `MediaNextTrack`, `MediaPrevTrack`, `VolumeUp`, `VolumeDown`, `VolumeMute`)

## HTTP request action
- [x] `HttpRequest { method, url, headers, body }` action using `ureq`
- [x] Frontend action field

## Live key state
- [ ] Background task that owns the device handle and can push key image/title updates at runtime
- [ ] "Dynamic" key type whose image is driven by an integration
- [ ] Discord mute/deafen keys reflect state via `VOICE_SETTINGS_UPDATE`
- [ ] Clock / system stats rendered with `stream_deck/src/title.rs`

