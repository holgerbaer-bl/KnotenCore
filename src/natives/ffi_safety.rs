//! Sprint 173: FFI Safety Guard Utilities
//!
//! These functions enforce null-pointer equivalence, string validity,
//! and use-after-free safety at every FFI boundary. The codebase uses
//! pure safe Rust (no raw C pointers), but these guards provide
//! defense-in-depth and serve as the canonical validation pattern
//! should a C-ABI bridge be added in the future.

/// Validate a handle ID (i64) — returns `Some(usize)` if valid.
/// Negative IDs are the safe-Rust equivalent of null pointers.
/// Logs a structured warning via `eprintln!` on invalid input.
#[inline]
pub fn validate_handle(handle_id: i64, fn_name: &str) -> Option<usize> {
    if handle_id < 0 {
        eprintln!(
            "[FFI Safety] Null-handle rejected in {}: handle={}",
            fn_name, handle_id
        );
        None
    } else {
        Some(handle_id as usize)
    }
}

/// Validate a string reference — rejects empty strings (null-equivalent)
/// and strings containing null bytes. Returns `Some(&str)` if valid.
#[inline]
pub fn validate_string<'a>(s: &'a str, fn_name: &str) -> Option<&'a str> {
    if s.is_empty() {
        eprintln!("[FFI Safety] Empty string rejected in {}", fn_name);
        return None;
    }
    if s.contains('\0') {
        eprintln!("[FFI Safety] Null-byte in string rejected in {}", fn_name);
        return None;
    }
    Some(s)
}

/// Guard against use-after-free: logs a warning if the entity was
/// already freed. Callers should pass `true` when the remove was
/// idempotent (key already absent).
#[inline]
pub fn guard_remove_entity(fn_name: &str, entity_id: usize, already_removed: bool) {
    if already_removed {
        eprintln!(
            "[FFI Safety] Idempotent remove: entity {} already freed in {}",
            entity_id, fn_name
        );
    }
}
