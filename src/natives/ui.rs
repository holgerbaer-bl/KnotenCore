// Legacy UI Module (Minifb removed in Sprint 51)
// Function signatures kept to satisfy FFI bounds, but now act as no-ops.

// Sprint 118: Thread-safe buffer for UITextInput state binding.
// The evaluator reads/writes this buffer via ui_text_input_get/set.
// Future egui integration will push keyboard events into this buffer.
pub static UI_TEXT_INPUT_BUFFER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Sprint 118: Returns the current text-input value.
pub fn ui_text_input_get() -> String {
    UI_TEXT_INPUT_BUFFER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Sprint 118: Overwrites the text-input buffer with `val`.
pub fn ui_text_input_set(val: String) {
    *UI_TEXT_INPUT_BUFFER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = val;
}

pub fn ui_init_window(_width: i64, _height: i64, _title: String) -> bool {
    eprintln!("[KnotenCore UI] Legacy UI module deprecated. Use Registry WGPU context instead.");
    false
}

pub fn ui_clear(_color: i64) {}

pub fn ui_draw_rect(_x: i64, _y: i64, _w: i64, _h: i64, _color: i64) {}

pub fn ui_draw_text(_x: i64, _y: i64, _text: String, _color: i64) {}

pub fn ui_present() -> bool {
    false
}

pub fn ui_is_key_down(_key_name: String) -> bool {
    false
}

pub fn ui_get_key_pressed() -> String {
    String::new()
}
