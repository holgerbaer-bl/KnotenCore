# Good First Issues - KnotenCore StdLib

> *Maintainers: Copy the Markdown blocks below directly into GitHub Issues.*

---

## Issue 1: Extend String Manipulation (`to_upper`, `trim`)

### 🎯 Objective
Expand the Standard Library by implementing the `to_upper(str)` and `trim(str)` functionality within the `core/string.nod` module.

### Context
KnotenCore delegates intensive operations to native Rust implementations via its FFI Bridge (`src/natives/bridge.rs`). You will write the native Rust Opcodes, map them through the VM evaluator, and expose them as idiomatic functions in the StdLib.

### 🛠️ Implementation Steps
1. **Rust FFI Implementation (`src/natives/fs.rs` or `src/natives/bridge.rs`)**
   - In `src/natives/bridge.rs`, locate the `fs` module match block.
   - Add two new match arms: `"str_to_upper"` and `"str_trim"`.
   - Ensure you validate the correct number of arguments (1 String arg) and defensively return `ExecResult::Fault` on layout mismatch.
   - Return the evaluated string natively using Rust's `to_uppercase()` and `trim()`.

2. **KnotenCore Wrapper (`core/string.nod`)**
   - Add the following wrapper functions:
     ```javascript
     fn to_upper(str) {
         return str_to_upper(str);
     }
     
     fn trim(str) {
         return str_trim(str);
     }
     ```

### ✅ Acceptance Criteria
- [ ] `cargo build` and `cargo clippy --lib` pass flawlessly with 0 warnings.
- [ ] `cargo test --lib` passes.
- [ ] Writing a test script (`examples/test_string.nod`) importing `core/string.nod` successfully converts `" hello "` to `"HELLO"`.

---

## Issue 2: Expand Math Capabilities (`power`)

### 🎯 Objective
Empower the Math module by implementing `power(base, exp)` within `/core/math.nod`. 

### Context
The native `math.nod` module currently implements `min`, `max`, `clamp`, and `abs` natively via loops. However, computing complex exponents requires routing a native callback natively to Rust to leverage F16/F64 speed.

### 🛠️ Implementation Steps
1. **Rust FFI Implementation (`src/natives/bridge.rs`)**
   - Currently, FFI math functions reside securely inside the `global` namespace fallback (or you can create a `math` module block if one doesn't exist).
   - Add `"math_pow"` expecting 2 arguments (Base: Float, Exp: Float).
   - Implement the calculation safely via `f64::powf(base, exp)` inside the Bridge and return it as `ExecResult::Value(RelType::Float(ans))`.

2. **KnotenCore Wrapper (`core/math.nod`)**
   - Add the idiomatic wrapper:
     ```javascript
     fn power(base, exp) {
         return math_pow(base, exp);
     }
     ```

### ✅ Acceptance Criteria
- [ ] `cargo build` and `cargo clippy --lib` pass flawlessly.
- [ ] `cargo test --lib` passes.
- [ ] Successfully printing `power(2.0, 3.0)` yields `8.0` in a `.nod` sandbox script.

---

## Issue 3: Safe File Existence Check (`file_exists`)

### 🎯 Objective
Expose a secure API to dynamically verify if a file exists before attempting to read it via `file_exists(path)` inside `core/fs.nod`.

### Context
KnotenCore operates an advanced Zero-Trust Execution Sandbox. Any filesystem checks natively interacting with the host OS *must* ensure the `--allow-read` flag is evaluated before the check succeeds.

### 🛠️ Implementation Steps
1. **Rust FFI Implementation (`src/natives/bridge.rs` and `src/natives/fs.rs`)**
   - Inside `src/natives/bridge.rs` under the `"fs"` module match, add `"fs_file_exists"`.
   - **Crucial Security Check**: Verify the `permissions.allow_fs_read` property. If false, aggressively return `ExecResult::Fault` explicitly denying access.
   - If allowed, use Rust's `std::path::Path::new(&path).exists()` and return the boolean result.

2. **KnotenCore Wrapper (`core/fs.nod`)**
   - Add the idiomatic function wrapper:
     ```javascript
     fn file_exists(path) {
         return fs_file_exists(path);
     }
     ```

### ✅ Acceptance Criteria
- [ ] `cargo clippy --lib` passes with 0 warnings.
- [ ] Executing a script locally testing `file_exists("core/fs.nod")` correctly returns `true`.
- [ ] Executing the exact same script **without** the `--allow-read` payload CLI tag generates a fatal `Permission Denied` Sandbox VM Fault.
