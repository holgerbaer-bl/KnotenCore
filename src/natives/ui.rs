use minifb::{Key, Window, WindowOptions};
use std::cell::UnsafeCell;

/// Wrapper to make Window usable from the main thread in a static.
/// SAFETY: KnotenCore's UI module is always called from the main thread.
struct SendWindow(UnsafeCell<Option<WindowState>>);
unsafe impl Send for SendWindow {}
unsafe impl Sync for SendWindow {}

struct WindowState {
    window: Window,
    buffer: Vec<u32>,
    width: usize,
    height: usize,
}

static UI_STATE: SendWindow = SendWindow(UnsafeCell::new(None));

fn with_ui<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut WindowState) -> R,
{
    // SAFETY: single-threaded access from KnotenCore executor
    unsafe {
        let state = &mut *UI_STATE.0.get();
        state.as_mut().map(f)
    }
}

// ── Public FFI Functions ─────────────────────────────────────────

/// Initializes a window with the given width, height, and title.
pub fn ui_init_window(width: i64, height: i64, title: String) -> bool {
    let w = width as usize;
    let h = height as usize;
    let buf = vec![0x222222; w * h];

    let win = Window::new(&title, w, h, WindowOptions::default());
    match win {
        Ok(mut window) => {
            window.set_target_fps(30);
            let state = WindowState {
                window,
                buffer: buf,
                width: w,
                height: h,
            };
            // SAFETY: single-threaded
            unsafe {
                *UI_STATE.0.get() = Some(state);
            }
            true
        }
        Err(_) => false,
    }
}

/// Clears the framebuffer to a solid color (0xRRGGBB).
pub fn ui_clear(color: i64) {
    with_ui(|s| {
        let c = color as u32;
        for px in s.buffer.iter_mut() {
            *px = c;
        }
    });
}

/// Draws a filled rectangle at (x, y) with given width, height, and color.
pub fn ui_draw_rect(x: i64, y: i64, w: i64, h: i64, color: i64) {
    with_ui(|s| {
        let c = color as u32;
        let bw = s.width;
        let bh = s.height;
        for dy in 0..h as usize {
            for dx in 0..w as usize {
                let px = x as usize + dx;
                let py = y as usize + dy;
                if px < bw && py < bh {
                    s.buffer[py * bw + px] = c;
                }
            }
        }
    });
}

/// Draws text at (x, y) using a built-in 5x7 pixel font. Color is 0xRRGGBB.
pub fn ui_draw_text(x: i64, y: i64, text: String, color: i64) {
    with_ui(|s| {
        let c = color as u32;
        let bw = s.width;
        let bh = s.height;
        let mut cx = x as usize;
        let cy = y as usize;

        for ch in text.chars() {
            let glyph = get_glyph(ch);
            for row in 0..7usize {
                for col in 0..5usize {
                    if glyph[row] & (1 << (4 - col)) != 0 {
                        let px = cx + col;
                        let py = cy + row;
                        if px < bw && py < bh {
                            s.buffer[py * bw + px] = c;
                        }
                    }
                }
            }
            cx += 6; // 5px char + 1px spacing
        }
    });
}

/// Flushes the buffer to the window. Returns false if the window was closed.
pub fn ui_present() -> bool {
    with_ui(|s| {
        let buf = s.buffer.clone();
        s.window.update_with_buffer(&buf, s.width, s.height).is_ok()
            && s.window.is_open()
            && !s.window.is_key_down(Key::Escape)
    })
    .unwrap_or(false)
}

/// Checks if a specific key is currently pressed.
pub fn ui_is_key_down(key_name: String) -> bool {
    with_ui(|s| {
        if let Some(key) = name_to_key(&key_name) {
            s.window.is_key_down(key)
        } else {
            false
        }
    })
    .unwrap_or(false)
}

/// Returns the last pressed key as a string, or empty string if none.
pub fn ui_get_key_pressed() -> String {
    with_ui(|s| {
        let keys = s.window.get_keys_pressed(minifb::KeyRepeat::No);
        if let Some(first) = keys.first() {
            key_to_name(*first)
        } else {
            String::new()
        }
    })
    .unwrap_or_default()
}

// ── Internal helpers ─────────────────────────────────────────────

fn name_to_key(name: &str) -> Option<Key> {
    match name {
        "0" => Some(Key::Key0),
        "1" => Some(Key::Key1),
        "2" => Some(Key::Key2),
        "3" => Some(Key::Key3),
        "4" => Some(Key::Key4),
        "5" => Some(Key::Key5),
        "6" => Some(Key::Key6),
        "7" => Some(Key::Key7),
        "8" => Some(Key::Key8),
        "9" => Some(Key::Key9),
        "Plus" => Some(Key::Equal),
        "Minus" => Some(Key::Minus),
        "Asterisk" => Some(Key::Key8),
        "Slash" => Some(Key::Slash),
        "Enter" => Some(Key::Enter),
        "Backspace" => Some(Key::Backspace),
        "Escape" => Some(Key::Escape),
        "Period" => Some(Key::Period),
        "C" => Some(Key::C),
        _ => None,
    }
}

fn key_to_name(key: Key) -> String {
    match key {
        Key::Key0 | Key::NumPad0 => "0",
        Key::Key1 | Key::NumPad1 => "1",
        Key::Key2 | Key::NumPad2 => "2",
        Key::Key3 | Key::NumPad3 => "3",
        Key::Key4 | Key::NumPad4 => "4",
        Key::Key5 | Key::NumPad5 => "5",
        Key::Key6 | Key::NumPad6 => "6",
        Key::Key7 | Key::NumPad7 => "7",
        Key::Key8 | Key::NumPad8 => "8",
        Key::Key9 | Key::NumPad9 => "9",
        Key::Equal => "Plus",
        Key::Minus | Key::NumPadMinus => "Minus",
        Key::Slash | Key::NumPadSlash => "Slash",
        Key::NumPadAsterisk => "Asterisk",
        Key::Enter | Key::NumPadEnter => "Enter",
        Key::Backspace => "Backspace",
        Key::Escape => "Escape",
        Key::Period | Key::NumPadDot => "Period",
        Key::C => "C",
        _ => "",
    }
    .to_string()
}

/// Minimal 5x7 pixel font — ASCII subset for calculator display.
fn get_glyph(ch: char) -> [u8; 7] {
    match ch {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '*' => [
            0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '=' => [
            0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'n' => [
            0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001,
        ],
        'o' => [
            0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        't' => [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
        ],
        'e' => [
            0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110,
        ],
        'a' => [
            0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111,
        ],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'c' => [
            0b00000, 0b00000, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'u' => [
            0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101,
        ],
        'r' => [
            0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000,
        ],
        _ => [
            0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
        ],
    }
}
