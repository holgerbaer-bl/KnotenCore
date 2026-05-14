// Sprint 118 / Sprint 162: Thread-safe UI state for the Retained-Mode egui integration.
//
// Architecture (Sprint 162):
//   VM Thread                      Main / WGPU Thread
//   ---------                      ------------------
//   registry_ui_set(win, nodes) →  UpdateUI command → state.ui_tree stored
//                                  about_to_wait → request_redraw every frame
//                                  RedrawRequested → egui renders ui_tree autonomously
//                                  UIButton clicked → UI_BUTTON_EVENTS[label] = true
//   registry_ui_poll_button(win, label) ← reads & clears UI_BUTTON_EVENTS[label]
//   registry_ui_read_text(key)          ← reads UI_TEXT_BUFFERS[key]

use std::collections::HashMap;
use std::sync::Mutex;

// ── Legacy single-buffer (Sprint 118 compat) ─────────────────────
pub static UI_TEXT_INPUT_BUFFER: Mutex<String> = Mutex::new(String::new());

/// Sprint 118: Returns the current text-input value (single-buffer legacy path).
pub fn ui_text_input_get() -> String {
    UI_TEXT_INPUT_BUFFER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Sprint 118: Overwrites the single text-input buffer with `val`.
pub fn ui_text_input_set(val: String) {
    *UI_TEXT_INPUT_BUFFER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = val;
}

// ── Sprint 162: Multi-key text input buffers ──────────────────────
/// Keyed text buffers for multiple concurrent UITextInput widgets.
/// Key is the widget's `id` label (e.g. `"username"`, `"password"`).
pub static UI_TEXT_BUFFERS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Returns the current value for a keyed text input widget.
/// Returns empty string if the key has never been written.
pub fn ui_text_read(key: &str) -> String {
    let guard = UI_TEXT_BUFFERS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_ref() {
        map.get(key).cloned().unwrap_or_default()
    } else {
        String::new()
    }
}

/// Called by the egui render thread to persist text edits back to the shared store.
pub fn ui_text_write(key: &str, val: String) {
    let mut guard = UI_TEXT_BUFFERS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(key.to_string(), val);
}

// ── Sprint 162: Button event queue ───────────────────────────────
/// Clicked-flag store keyed by button label text.
/// The egui render thread sets the flag; the VM polls and clears it.
pub static UI_BUTTON_EVENTS: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

/// Signal that a button was clicked. Called from the render thread.
pub fn ui_button_signal(label: &str) {
    let mut guard = UI_BUTTON_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(label.to_string(), true);
}

/// Poll (and clear) the click state for a button label.
/// Returns `true` once per click event.
pub fn ui_button_poll(label: &str) -> bool {
    let mut guard = UI_BUTTON_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.remove(label).unwrap_or(false)
    } else {
        false
    }
}

// ── Legacy no-op stubs (backward compat) ─────────────────────────
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
