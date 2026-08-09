use std::sync::Mutex;

use hidapi::{HidApi, HidDevice};

pub(crate) const VENDOR_ID: u16 = 0x0fd9;
const PRODUCT_ID: u16 = 0x006d;
const KEY_COUNT: u8 = 15;

// Input reports carry a report ID byte plus 3 header bytes before the
// per-key state bytes start.
const INPUT_REPORT_HEADER_LEN: usize = 4;

// Image reports are fixed 1024-byte HID output reports:
// [0x02, 0x07, key, is_last, len_lo, len_hi, page_lo, page_hi] + up to 1016
// bytes of JPEG payload, zero-padded to fill the report.
const IMAGE_REPORT_LEN: usize = 1024;
const IMAGE_REPORT_HEADER_LEN: usize = 8;
const IMAGE_REPORT_PAYLOAD_LEN: usize = IMAGE_REPORT_LEN - IMAGE_REPORT_HEADER_LEN;

/// Owns the HID handle to a physical Stream Deck and is the only thing that
/// speaks its wire protocol
pub struct StreamDeck {
    device: HidDevice,
    /// Per-key state from the last report, so polling can fire only on the
    /// release->press edge instead of on every report while a key is held.
    pressed: [bool; KEY_COUNT as usize],
}

impl StreamDeck {
    pub const KEY_COUNT: u8 = KEY_COUNT;

    /// Retries opening the device until it succeeds
    /// Logs once per distinct failure, not on every attempt.
    pub fn open_with_retry(hid: &HidApi) -> Self {
        let mut last_error: Option<String> = None;
        loop {
            match hid.open(VENDOR_ID, PRODUCT_ID) {
                Ok(device) => {
                    return Self {
                        device,
                        pressed: [false; KEY_COUNT as usize],
                    };
                }
                Err(e) => {
                    let message = e.to_string();
                    if last_error.as_deref() != Some(message.as_str()) {
                        eprintln!("Stream Deck not found ({e}); waiting for it to connect...");
                        last_error = Some(message);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }
    }

    /// Rescans for HID devices and tries to open the Stream Deck, for use
    /// after a read error that suggests it was unplugged.
    pub fn try_reopen(hid: &mut HidApi) -> Option<Self> {
        hid.refresh_devices().ok()?;
        let device = hid.open(VENDOR_ID, PRODUCT_ID).ok()?;
        device.set_blocking_mode(false).ok()?;
        Some(Self {
            device,
            pressed: [false; KEY_COUNT as usize],
        })
    }

    pub fn set_blocking_mode(&self, blocking: bool) -> anyhow::Result<()> {
        self.device.set_blocking_mode(blocking)?;
        Ok(())
    }

    /// Reads the next input report (non-blocking) and returns newly-pressed
    /// key indices — release->press edges only, not every report while a key
    /// is held down. `None` means the read timed out (no report available).
    fn poll_pressed_keys(&mut self) -> hidapi::HidResult<Option<Vec<u8>>> {
        let mut buf = [0u8; 512];
        let n = self.device.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }

        // buf[0] = report ID (0x01); buf[1..4] = header bytes to skip;
        // buf[4..n] = one byte per key, 0x01 = pressed, 0x00 = released.
        let key_states = &buf[INPUT_REPORT_HEADER_LEN..n];
        let mut newly_pressed = Vec::new();
        for (key_index, &key_state) in key_states.iter().enumerate() {
            if key_index >= self.pressed.len() {
                break;
            }
            let is_pressed = key_state == 0x01;
            if is_pressed && !self.pressed[key_index] {
                // key_index < self.pressed.len() == KEY_COUNT, checked above.
                #[allow(clippy::cast_possible_truncation)]
                newly_pressed.push(key_index as u8);
            }
            self.pressed[key_index] = is_pressed;
        }
        Ok(Some(newly_pressed))
    }

    /// Blocks forever, polling for key presses and invoking `on_key_press`
    /// for each one (release->press edge only). Automatically reconnects on
    /// read errors — replacing `*device.lock()` with the new handle so pushes
    /// made concurrently from other threads keep working — and calls
    /// `on_reconnect` afterward so the caller can redraw whatever should be
    /// on screen (a freshly (re)connected device starts blank).
    pub fn poll_keys(
        device: &Mutex<Self>,
        mut hid: HidApi,
        mut on_key_press: impl FnMut(u8),
        mut on_reconnect: impl FnMut(),
    ) {
        // Tracks the last-seen read error so a disconnected device (which
        // errors on every read instead of timing out) logs once instead of spamming.
        let mut last_error: Option<String> = None;

        loop {
            let poll_result = {
                let mut d = device.lock().unwrap();
                d.poll_pressed_keys()
            };

            match poll_result {
                Ok(None) => {
                    // timeout — no state change, keep polling
                    last_error = None;
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Ok(Some(newly_pressed)) => {
                    last_error = None;
                    for key_id in newly_pressed {
                        on_key_press(key_id);
                    }
                }
                Err(e) => {
                    let message = e.to_string();
                    if last_error.as_deref() != Some(message.as_str()) {
                        eprintln!("HID read error: {e}");
                        last_error = Some(message);
                    }

                    if let Some(reopened) = Self::try_reopen(&mut hid) {
                        println!("Stream Deck reconnected");
                        *device.lock().unwrap() = reopened;
                        on_reconnect();
                        last_error = None;
                    }

                    // A disconnected device errors on every read instead of
                    // timing out like an idle one does, so without this sleep
                    // the loop would busy-spin as fast as the OS returns errors.
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }

    /// Pushes a pre-encoded JPEG (see [`crate::push_image::ICON_SIZE`] for the
    /// expected dimensions) onto `key`'s screen, chunked into fixed-size
    /// output reports as the device expects.
    pub fn push_key_image(&self, key: u8, jpeg: &[u8]) -> anyhow::Result<()> {
        let mut page = 0usize;
        let mut remaining = jpeg.len();

        while remaining > 0 {
            let chunk_len = remaining.min(IMAGE_REPORT_PAYLOAD_LEN);
            let sent = page * IMAGE_REPORT_PAYLOAD_LEN;
            let is_last = chunk_len == remaining;

            // chunk_len and page are split into wire-protocol low/high bytes;
            // both stay well within u16 range for any realistic icon size.
            #[allow(clippy::cast_possible_truncation)]
            let mut packet = vec![
                0x02,
                0x07,
                key,
                u8::from(is_last),
                (chunk_len & 0xff) as u8,
                (chunk_len >> 8) as u8,
                (page & 0xff) as u8,
                (page >> 8) as u8,
            ];
            packet.extend_from_slice(&jpeg[sent..sent + chunk_len]);
            packet.resize(IMAGE_REPORT_LEN, 0);

            self.device.write(&packet)?;

            remaining -= chunk_len;
            page += 1;
        }

        Ok(())
    }
}
