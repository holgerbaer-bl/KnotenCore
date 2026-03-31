# Changelog: KnotenCore Engine

**Vision:** A high-performance, general-purpose hybrid language (JIT/AOT) with native WGPU rendering and deterministic ARC memory management.

## [v1.0.22] - Sprint 123: AI-Readiness Delta-Review v2.0 & Hotfixes (2026-03-31)
Resolves critical inconsistencies found during the AI-Readiness Delta-Review, strictly adhering to the Rust Source-of-Truth.
- **`docs/LANGUAGE_REFERENCE/error_catalog.json`** *(new)*: Created structured self-healing loop error catalog covering `ERR_UNKNOWN_NODE`, `ERR_ARITY_MISMATCH`, `ERR_INVALID_HANDLE`, `ERR_IO_PERMISSION`, `ERR_NET_PERMISSION`, and `ERR_JSON_PARSE`.
- **`llm.md`**: Added a decision routing table mapping Execution Nodes (`Direct AST Node` vs `ExternCall`) to "Key Constraints", deprecating `NativeCall`.
- **`README.md`**: Added explicit `// Neural DSL (.knoten) — NOT JSON-AST` language annotations to all code blocks to avoid AST hallucination.
- **`node_types.json`**: Fixed `UISetStyle` to correctly reflect arity 4 to 6 (btn_idle and btn_hover are optional).
- **`native_functions.json`**: Added `registry_draw_entity` and `registry_draw_cylinder` based on true function signatures.
- **`AGENT_EXTENSION_MANUAL.md`**: Marked `DrawSprite` as a hypothetical example.

### Documentation Corrections
- **Pre-Flight Conflict Resolved**: Instruction requested adding a `Neg` node to `nod_grammar.ebnf` and `node_types.json`. Pre-Flight check confirmed `Neg(Box<Node>)` does **NOT** exist in `src/ast.rs` (`MathUn` only has `Sin`, `Cos`, `Abs`). The Rust source won. The documentation was explicitly NOT updated.
- **Pre-Flight Conflict Resolved**: Instruction requested verifying `registry_window_render_frame`. Pre-Flight check confirmed it does **NOT** exist in `src/natives/(bridge|registry).rs`. It was NOT added to the function registry, and the Sprint 105 loop in `README.md` was correctly marked as Legacy API.

## [v1.0.21] - Sprint 122: AI-Readiness Foundation Phase 1 Completion (2026-03-30)
Completes the machine-readable AI-Readiness reference stack with a native function registry and explicit anti-pattern guard-rails.
- **`docs/LANGUAGE_REFERENCE/native_functions.json`** *(new)*: Machine-readable registry of every FFI function exposed via `ExternCall`. 30+ entries across 6 modules (`registry`, `ui`, `fs`, `net`, `json`, `time`). Each entry documents: parameter names + types, return type, required permission flags, and a live JSON AST call example. AI agents **must only call functions listed here**.
- **`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`** *(new)*: 10 explicit DO/DON'T patterns covering: wrong node names (`Let`, `Var`), bare scalar values, hallucinated function names, malformed `ExternCall` structure, raw object literals, `UITextInput` state-binding errors, missing permission flags, and invented return fields.
- **`llm.md`**: Routing table extended with direct links to both new files, completing the 5-document Phase 1 reference stack.
- **`README.md`**: AI-Readiness section updated to "Sprints 121–122", with the full 5-entry reference table and an updated agent directive.
- **Git**: Committed and pushed cleanly to `origin/main` via standard (non-force) push. No divergence.

## [v1.0.20] - Sprint 121: AI-Readiness Foundation — EBNF & JSON Schemas (2026-03-30)
Establishes a hallucination-resistant, machine-readable language specification for autonomous AI agents.
- **`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`** *(new)*: Normative EBNF grammar covering every `Node` variant derived directly from `src/ast.rs`. Eliminates structural ambiguity for LLM code generation.
- **`docs/LANGUAGE_REFERENCE/node_types.json`** *(new)*: Full Draft-07 JSON Schema with `"additionalProperties": false` enforced on every object node. Covers all 60+ node variants — hallucinated fields are rejected at runtime by the schema validator.
- **`llm.md`**: Redesigned from a tutorial document to a lean **routing hub**. All AI agents are now directed to `LANGUAGE_REFERENCE/` for authoritative source-of-truth references. Retained: security sandbox table, 4-touchpoint extension checklist, fault format, and key constraints for code generation.
- **`README.md`**: Added **🤖 AI-Readiness Foundation** section as a top-level feature, with a reference table linking to the EBNF, JSON Schema, and agent guide.
- **Git**: Committed and pushed cleanly to `origin/main` via standard (non-force) push. No divergence.

## [v1.0.19] - Sprint 120: UI Layouts (HBox/VBox) & UILabel (2026-03-29)
Finalized the structural layout foundation for the `egui` native integration.
- **AST Extension**: Introduced `Node::UIHBox(Vec<Node>)`, `Node::UIVBox(Vec<Node>)`, and completed `Node::UILabel(Box<Node>)` support in `src/ast.rs`, `src/parser.rs`, and the evaluation engines.
- **Evaluation Loop Synchronization**: Implemented `resolve_ui_nodes` in `src/executor.rs` to deeply evaluate variables before streaming the UI payload across thread boundaries. `UILabel` natively tracks text inputs in real-time.
- **Egui Render Context**: Built a native recursive iterator mapping (`render_egui_node`) inside `src/window.rs` binding to `ui.horizontal()` and `ui.vertical()`.
- **Form Demo Update**: Upgraded `examples/form_demo.nod` to nest the interactive text buffer alongside the button inside a native `UIHBox` and dynamically reflect the string back within a `UILabel`.

## [v1.0.18] - Sprint 119: Visual egui Text Rendering & Pipeline (2026-03-29)
Established the foundational architecture for rendering `egui` inside the isolated `winit`/`wgpu` hardware loop natively.
- **`src/window.rs`**: Built the `RegistryWindowState` extensions directly managing the Egui context trinity (`egui::Context`, `egui_winit::State`, `egui_wgpu::Renderer`).
- **`src/natives/registry.rs`**: Introduced `RenderCommand::UpdateUI` specifically serving to transport AST node snapshots natively across thread boundaries from the background VM into the UI loop.
- **`src/executor.rs`**: Refactored `Node::UIWindow` iteration to seamlessly clone internal bodies synchronously over to the Render Channel, solving immediate-mode syncs dynamically.
- **`UITextInput` Binding**: Re-tooled the inner evaluation matching within `RedrawRequested` to literally execute `ui.text_edit_singleline` targeting the thread-locked `UI_TEXT_INPUT_BUFFER`. Interactive string modifications now render perfectly at 60FPS.

## [v1.0.17] - Sprint 118: UITextInput Widget & egui State Binding (2026-03-28)
Complete the interactive UI surface by wiring a thread-safe string buffer between the AST executor and the egui rendering pipeline.
- **`src/natives/ui.rs`**: Introduced `static UI_TEXT_INPUT_BUFFER: Mutex<String>` as the ground truth for text input state, plus `ui_text_input_get()` and `ui_text_input_set()` public helpers.
- **`src/natives/bridge.rs`**: Registered `"ui_text_input_get"` and `"ui_text_input_set"` FFI handlers in the `"ui"` bridge module, exposing the buffer to any `.nod` script via `ExternCall`.
- **`src/executor.rs`**: Upgraded `Node::UITextInput` from a dead no-op stub to a stateful implementation: on the first call the seed value (script variable) populates the buffer; all subsequent calls return the live buffer value. Enables the idiomatic `text = UITextInput(text)` assignment pattern.
- **`stdlib/ui.nod`**: Added `fn UITextInput(initial_text)` stdlib entry delegating to `ui_text_input_get` natively.
- **`examples/form_demo.nod`**: Added interactive form example demonstrating text input + submit button.
- **Test 55** (`integration_tests.rs`): `test_55_ui_text_input_seed_and_read` — verifies the seed/read round-trip returns `RelType::Str`.

## [v1.0.16] - Sprint 117: core/time.nod & CPU Throttling (2026-03-26)
Introduced standard hardware synchronization enabling efficient 60 FPS pacing via the `std::thread` FFI.
- **`core/time.nod`**: New standard module exposing `sleep(ms)` tightly coupled to `std::thread::sleep` natively, drastically reducing uncapped CPU cycling inside immediate-mode render loops.

## [v1.0.15] - Sprint 116: UI Rendering Pipeline Documentation (2026-03-25)
A structural documentation iteration explicitly codifying the theoretical boundaries between the AST executor loop mapping into the asynchronous `winit` WGPU immediate-mode layout frontend (`egui`) natively.
- **`llm.md`**: Implemented a comprehensive guide abstracting layout pipelines strictly for external agents evaluating future UI node implementations seamlessly without tracking raw OS threads defensively.
- **`README.md`**: Integrated an Option 1 idiomatic minimal UI block demonstrating immediate-mode layout processing elegantly over WGPU natively.
## [v1.0.14] - Sprint 115: Native JSON Parsing & core/json.nod (2026-03-24)
Further scaling the standard library by tightly coupling `serde_json` into the VM's native engine mapping loop.
- **`core/json.nod`**: New standard module exposing `parse(payload)` and `stringify(obj)` mapping raw dynamic nested networks abstractly into structured iterators.
- **FFI Data Serialization**: Introduced strict mathematical traversal logic natively inside `src/natives/fs.rs` avoiding JSON parsing failures causing unexpected panics gracefully escaping directly into `ExecResult::Fault` chains.

## [v1.0.13] - Sprint 114: Zero-Trust Networking & core/net.nod (2026-03-24)
Upgraded KnotenCore's execution paradigm by integrating deterministic, blocking HTTP network request infrastructure without violating the isolated VM pipeline.
- **`core/net.nod`**: Provided the `fetch(url)` standard module explicitly mapped to Rust's lightweight `ureq` client natively bridging via the FFI registry.
- **`--allow-net` Gateway**: Engineered a strict Zero-Trust flag enforcement preventing unauthorized network egress seamlessly verified by structural sandbox panic testing.
## [v1.0.12] - Sprint 113: Open Source Onboarding & Good First Issues (2026-03-23)
Established the formal foundation to transition the engine towards the global open-source community by producing strictly curated and heavily isolated "Good First Issues."
- **`CONTRIBUTING.md`**: Implemented strict local setup, architecture compilation (`cargo build`), and deterministic Sandbox regression evaluations (`cargo test --lib`) specifically for external onboarding.
- **`docs/good_first_issues.md`**: Curated strictly bounded execution tracks isolating external modifications strictly to the Standard Library (`core/string.nod`, `core/fs.nod`) bypassing immediate structural adjustments to the overarching AOT evaluation tree compiler.
## [v1.0.11] - Sprint 112: StdLib Expansion - Strings & File System (2026-03-23)
Expanded the Standard Library ecosystem to support enterprise-grade data processing pipelines by surfacing string manipulation and synchronous filesystem I/O wrappers.
- **`core/string.nod`**: Maps native C-string operations directly into script context (`len`, `contains`, `split`).
- **`core/fs.nod`**: Provides `read_text(path)` for idiomatic filesystem intake natively governed by the AOT Security Permissions Sandbox (`--allow-read`).
- **`core/array.nod`**: Exposes array utilities (e.g., `length(arr)`) fixing underlying module-evaluator routing bugs inside the FFI Bridge. 

---

## [v1.0.10] - Sprint 111: The Standard Library (StdLib) Core (2026-03-22)
Established the official overarching Standard Library. By leveraging the AOT infrastructure from Sprint 110, we've deployed isolated `core/` modules resolving elegantly bypassing directory trees.

### Added — The Standard Library (`core/`)
- **`core/math.nod`**: Bundles universally expected math logic explicitly written in native code natively evaluated (`abs`, `min`, `max`, `clamp`).
- **`core/system.nod`**: Safely packages generic OS integrations. Includes `is_pressed(key)` as an idiomatic functional wrapper abstracting explicit `registry_is_key_pressed` FFI.
- **Global Path Resolution**: Extended the `Compiler` AOT file-resolver natively detecting `"core/"` module directives rendering execution flawlessly ubiquitous globally. 

---

## [v1.0.9] - Sprint 110: AOT Module Linking and Import System (2026-03-22)
Transformed KnotenCore into an enterprise-scalable General-Purpose Language by introducing Ahead-Of-Time Module Linking. The compiler now seamlessly bridges segmented scripts.

### Added — AOT Module Linking
- **`Token::KeywordImport`**: Expanded the Lexer and Parser natively analyzing `import "module.nod";` directly via the AST as a `Node::Import(String)`.
- **Recursive AOT Composition**: The Bytecode Compiler now intercepts `Node::Import`, reading from the local disk and resolving the absolute path. It instantly links the lexed child AST instructions synchronously into the primary execution stack without dropping context. 
- **Circular Dependency Protection**: The engine maintains an `imported_files` HashSet, actively blocking recursive `A imports B -> B imports A` end-of-memory cascades during AOT transpilation.

---

## [v1.0.8] - Sprint 109: Lock-Free Input & Zero Allocations (2026-03-22)
Refactored the MVP Input Handling system into a massively performant, lock-free architecture utilizing zero heap allocations.

### Added — High-Performance Input Architecture
- **Lock-Free State Array**: Replaced the previous `OnceLock<Arc<Mutex<HashSet<String>>>>` with a `static GLOBAL_KEYS: [AtomicBool; 256]`. Hardware inputs are tracked without locking synchronization or heap fragmentation.
- **Zero-Allocation Event Loop**: In `src/window.rs`, `VirtualKeyCode` inputs are deterministically mapped to fixed integer indices (0-255). Winit states are stored directly using `Ordering::Relaxed`.
- **High-Performance FFI Bridge**: `registry_is_key_pressed` in the FFI bridge now parses the AST string arguments exactly once natively mapping to index positions, securely executing $O(1)$ lock-free boolean load calls. String cloning and collection allocations are entirely eradicated.

---

## [v1.0.7] - Sprint 108: Input Handling & Interactivity (2026-03-22)
Transformed the engine from a static renderer into an interactive 3D script host by coupling Winit keyboard events with the FFI bridge.

### Added — Interactivity
- **Global Input State**: Implemented a thread-safe Singleton `OnceLock<Arc<Mutex<HashSet<String>>>>` in `registry.rs` to track physical keyboard states across all OS events.
- **FFI Input Hook**: Registered `registry_is_key_pressed(key: String)` in the `.nod` VM bridge. Scripts can now poll specific keys (e.g. `"W"`, `"A"`, `"UP"`, `"SPACE"`) with zero block delays.
- **WASD Example**: Added `examples/interactive_loop.nod`, a real-time WGPU loop where the entity's coordinates are driven natively by keyboard inputs without JavaScript bindings.

### Fixed & Maintained
- Safely purged duplicate legacy `registry_is_key_pressed(i64)` implementations.
- Engine successfully retains absolute Math-Proven Zero Warnings (`cargo clippy --lib`).

---

## [v1.0.6] - Sprint 107: True Zero Warnings and Full Stack Hygiene (2026-03-20)
Mathematically proven zero warnings and complete VM stack safety.

### Fixed — Strict Stack Hygiene
- **Universal `ok_or_else`**: Eliminated the last 6 `unwrap_or(Void)` variants in non-critical VM paths (`SetLocal`, `GetLocal`, `StringLength`, `StringContainsChars`, `StringSplit`, `ArrayContains`).
- Every single stack pop now enforces an explicit Underflow check (`.ok_or_else(|| "Stack underflow")?`).

### Fixed — Clippy Eradication
- **`collapsible_if` Refactor**: Authored a custom Python AST regex tool to merge over 20 nested `if let` chains within the FFI Bridge (`bridge.rs`) into single lines utilizing Rust 1.65 let-chaining (`if ... && let ...`).
- **`E0382` Move Refactoring**: Replaced `contains_key` + `insert` with an in-place `get_mut` assignment sequence in `executor.rs`, eliminating an E0382 compiler block while satisfying Clippy.
- **Slice Optimization**: Adapted WGPU drawing routines (`fill_poly()`, `iso_render()`) to accept `&mut [u32]` slices instead of forcing `&mut Vec<u32>` object references.
- **`cargo clippy` Warning Count**: **Exactly 0**.

---

## [v1.0.5] - Sprint 106: Zero Warnings & Strict Compliance (2026-03-19)
Systematic hardening pass: every VM operation now fails loudly on misuse.

### Fixed — Security
- **`OpReadFile` full enforcement**: Executing `read_file` without `--allow-read` now returns `Err("Permission Denied: allow_fs_read is false (VM: ReadFile)")` instead of silently pushing `Void`. The VM halts immediately — no data leaks possible.

### Fixed — Stack Safety
- **`SetGlobal`, `Print`, `Return`**: Replaced `.unwrap_or(Void)` with `.ok_or_else(|| "Stack underflow")?`. Corrupt bytecode can no longer hide by operating on phantom `Void` values.
- **`Less` / `Greater` type errors**: Comparing incompatible types now returns `Err("Invalid types for Less/Greater comparison")` instead of silently returning `false`.

### Fixed — CLI
- **`--transpile` flag**: No longer silently ignored. Prints `'--transpile' is not yet connected to the VM pipeline` and exits with code 1.

### Fixed — Cosmetics
- `let mut previous_local_count` → `let previous_local_count` in `compiler.rs` (Clippy: `unused_mut`).
- `cargo fix --lib` applied 4 unused-import fixes (`registry.rs`, `window.rs`).

---

## [v1.0.4] - Sprint 105: The Visual Game Loop (2026-03-19)
Connected the Dictionary VM, Control Flow Bytecode, and FFI Bridge into a real animation loop.

### Added
- **`registry_draw_entity(win, x, y)`** in `registry.rs` + `bridge.rs`: Simplistic 2D entity rendering hook that projects a sphere onto the 3D camera plane at a fixed Z depth. Callable from `.nod` scripts via `OpExternCall`.
- **`Node::While` bytecode compilation**: The `Compiler` now transpiles `while` loops into a jump-backpatch pattern (`JumpIfFalse` exit + `Jump` loop-back). Previously only supported in the AST tree-walking interpreter.
- **`Node::NativeCall` bytecode compilation**: `NativeCall` is now aliased with `Call` in the compiler match arm — any call not in the known builtins list falls through to `OpExternCall` automatically.
- **`examples/game_loop.nod`**: Full scripted game loop demonstrating Dictionary state (`player.x`/`player.speed`), per-frame FFI render calls, and stack-safe bounded frame cap (3600 frames).

### Compiler Improvements
- Removed a temporary `println!` debug line inadvertently left in the compiler fallback arm.

---

## [v1.0.3] - Sprint 104: Audit Fixes — Thread Safety, Stack Hygiene, Security Enforcement (2026-03-19)
Hardened the Bytecode VM against critical issues identified in the Sprint 103 architecture audit.

### Fixed — Thread Safety
- **`RelType::Dict` → `Arc<Mutex>`**: Replaced `Rc<RefCell<HashMap>>` with `std::sync::Arc<std::sync::Mutex<HashMap>>` in `executor.rs` and all VM handler sites (`machine.rs`). The `unsafe impl Send for ExecutionEngine` is now sound. Updated `Display` impl and `AllocateDict`/`SetProperty`/`GetProperty` opcodes to use `.lock().unwrap()`.
- **Manual `PartialEq` for `RelType`**: Removed `PartialEq` from the derive macro (Mutex doesn't implement it) and added a hand-written impl. Dict equality uses `Arc::ptr_eq` — two Dict values are equal if and only if they point to the same allocation.

### Fixed — Stack Hygiene
- **`OpCode::Pop`**: Added new opcode to the ISA (`opcode.rs`) and implemented in the VM dispatch loop.
- **Compiler emits `Pop` after `PropertySet`**: The compiler (`compiler.rs`) now appends `OpCode::Pop` after every `Node::PropertySet` emission, discarding the dict reference that `SetProperty` re-pushes onto the stack. Prevents unbounded stack growth on repeated property assignments.

### Fixed — Strict Error Handling
- **Binary ops use `.ok_or_else(…)?`**: All seven binary/comparison opcodes (`Add`, `Subtract`, `Multiply`, `Divide`, `Equal`, `Less`, `Greater`) and `JumpIfFalse` now return `Err("Stack underflow in …")` on an empty stack instead of silently producing `Void`.

### Fixed — Clippy Cleanup
- Auto-applied Clippy suggestions to `run_knc.rs` (1 fix for unused `mut`). Manual fixes applied for remaining lib warnings where auto-fix was too aggressive.

---

## [v1.0.2] - Sprint 103: Implement Dictionaries and Property Access (2026-03-19)
Integrated a pure Native `Key-Value` Object structure directly onto the AOT constraints. The Bytecode Machine supports complete interior mutability arrays evaluating structural logic natively.

### Added — Dictionaries & Objects
- **`Token::Colon` & Lexer Integration**: Hardened the AST mapping logic inside `parser.rs`. The native parser implicitly supports defining `ObjectLiteral` values utilizing `name: "Hero"` representations.
- **Reference Semantics (`RelType::Dict`)**: Overhauled the core memory references isolating `Dict` instances around `Rc<RefCell<>>` memory tracking. This natively bypasses aggressive cloning during recursive struct modifications, enabling true "pass-by-reference" for functional variables (`take_damage(entity)`).
- **OpCode ISA (`OpAllocateDict`, `OpGetProperty`, `OpSetProperty`)**: Scaled the Arithmetic Logic Unit handling explicit reference mutations directly from the flat stack. Execution loops properly mutate and repush the modified references sequentially preventing stack overflows.
- **Compiler Routines (`vm/compiler.rs`)**: Wired `Node::ObjectLiteral`, `Node::PropertyGet`, and `Node::PropertySet` mapping into explicit stack evaluations scaling arbitrary tree manipulations safely onto the linear memory logic.
- **Test Application (`examples/struct_test.nod`)**: Designed a simulated combat structure mapping `player.hp = 100` before assigning `take_damage(player, 35)`. Validated absolute pointer mutation capabilities reflecting identical Global scopes flawlessly!

---

## [v1.0.1] - Sprint 101: Implement FFI Bridge and Native External Calls (2026-03-19)
Integrated a pure AOT architecture bypass directly into the Node AST transverser, mapping unregistered function calls into the `ExecutionEngine::bridge` structures.

### Added — FFI Integration
- **`OpExternCall` Instruction**: Upgraded the `OpCode` library allowing dynamic lookup across the constants pool. The VM natively handles dynamically shifting stack arguments for unmapped subroutines by popping lengths implicitly calculated by the compiler.
- **`BridgeModule` Injection**: Advanced the core runner loop (`VM::run`) taking `bridge: Option<&dyn BridgeModule>`. When encountering external calls, the Virtual Machine dynamically extracts namespace prefixes (e.g., `registry_*`, `ui_*`, `fs_*`) and seamlessly triggers native Rust Subroutines within the hardware context layer, evaluating explicit Sandboxing. 
- **AST Compiler Fallback (`compiler.rs`)**: Overhauled the `Node::Call` execution tree, transforming unsupported identifiers dynamically into `OpExternCall` emissions. This scales arbitrary Node scripting commands flawlessly into OS routines.
- **Test Application (`examples/vm_graphics_test.nod`)**: Developed a completely self-sustaining GUI window entirely managed by the hardware Engine VM relying solely on the Native FFI instruction set, omitting the AST internal tree interpreter.

---

## [v1.0.0] - Sprint 100: Milestone: Sandbox File I/O and Array Operations (2026-03-19)
Achieved a monumental milestone. The Bytecode VM has bypassed pure computational boundaries to interface with OS file systems directly within the AOT transpiler while securely tethered to the Engine's hardware Sandbox.

### Added — Sandbox I/O & Array Operations (Milestone 100)
- **`OpReadFile`**: Empowered the Virtual Machine `src/vm/machine.rs` to read the physical hard drive. The instruction requires an explicit reference to `crate::executor::AgentPermissions`. It strictly evaluates `--allow-read` and maps the path string exclusively through `ExecutionEngine::validate_fs_path()` to block arbitrary `../../` traversal attempts securely before placing the contents atop the VM stack.
- **Array Mappings (`OpStringSplit` & `OpArrayContains`)**: Designed native `RelType::Array` generation within the AOT Engine. `str_split` fragments strings into Array collections natively, while `arr_contains` allows postfix execution logic to iterate internally.
- **Compiler Bridging (`src/vm/compiler.rs`)**: Synchronized internal AST identifiers `read_file`, `str_split`, and `arr_contains` directly into Bytecode.
- **`examples/password_evaluator.nod` Upgrade**: Showcased the milestone by upgrading the password evaluator. The AOT engine retrieves a dynamically generated `examples/blacklist.txt` file off the SSD, arrays it structurally, scans the target string locally, and zeros the security score natively without an Interpreter intermediary.

### Compliance
- Authored Sprint 100 commit checkpoint directly via AG workflow. Commit message: `Feat: Sprint 100 - Milestone: Sandbox File I/O and Array Operations`.

---

## [v0.99.0] - Sprint 99: Implement String Operations and Password Evaluator (2026-03-19)

### Added — Architecture (Parallel Feature)
- **String Operations (`vm/opcode.rs` & `vm/machine.rs`)**: Forged `OpStringLength` and `OpStringContainsChars` inside the AOT Execution Backend. The String comparisons rely on native Character validation sets directly across the stack. 
- **Variable Storage**: Expanded the Virtual Machine architecture to inherently support explicit global contexts by wrapping a lightweight `HashMap<String, RelType>` connected to `OpSetGlobal` and `OpGetGlobal`. The VM natively maps constants into runtime globals.
- **Compiler Routines (`vm/compiler.rs`)**: Integrated `Node::Assign` and `Node::Identifier` parsing, dynamically converting them into precise Global assignments. AOT translates internal `str_len(ident)` and `str_contains(ident, chars)` AST signatures locally.
- **`examples/password_evaluator.nod`**: Synthesized a realistic knoten application leveraging logic tracking, branching, and string operations to allocate password security scores natively via the AOT Engine. 

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 99 - Implement String Operations and Password Evaluator`.

---

## [v0.98.0] - Sprint 98: Establish Community Standards and Security Policy (2026-03-19)

### Added — Architecture & Community
- **`SECURITY.md`**: Formalized the engine's strict Sandbox boundaries (FFI restrictions / OS path validations) and established a private reporting route for critical vulnerabilities.
- **`CONTRIBUTING.md`**: Outlined the architectural pillars of the new v1.0.0-alpha framework, explicitly directing contributors to the Bytecode VM `src/vm/` AOT transpiler, Reverse Polish Notation constraints, and Stack Machine dispatch logic.
- **`CODE_OF_CONDUCT.md`**: Integrated the Contributor Covenant.
- **GitHub Templates (`.github/`)**: Created `ISSUE_TEMPLATE/bug_report.md` specifying OS and environment prerequisites alongside `.github/PULL_REQUEST_TEMPLATE.md` enforcing structural checks regarding FFI/VM modifications.

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 98 - Establish Community Standards and Security Policy`.

---

## [v0.97.0] - Sprint 97: Implement Control Flow and Branching (2026-03-15)

### Added — Architecture (Parallel Feature)
- **Logical Operations (`vm/opcode.rs`)**: Integrated `OpEqual`, `OpGreater`, and `OpLess` directly into the Arithmetic Logic Unit, allowing inline boolean evaluations on the execution stack.
- **Instruction Pointer Flow (`vm/machine.rs`)**: Augmented the `VM::run` loop to support `OpJump(usize)` and `OpJumpIfFalse(usize)`. The system now dynamically mutates the `ip` to break linear execution upon encountering branch conditions.
- **Compiler Backpatching (`vm/compiler.rs`)**: Re-tooled `compile_node` to parse `Node::If` trees. The AOT compiler natively emits placeholder jump instructions, compiles internal TRUE/FALSE blocks, measures byte offsets dynamically, and backpatches exact length markers over the placeholders before finalizing the instruction pool. 

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 97 - Implement Control Flow and Branching`.

---

## [v0.96.0] - Sprint 96: Implement VM Execution Loop and Stack Dispatcher (2026-03-15)

### Added — Architecture (Parallel Feature)
- **Stack Machine Dispatcher (`vm/machine.rs`)**: Validated the `VM::run` environment. The interpreter natively identifies `OpConstant(index)` pointers, pulls absolute values identically from the `constants` array pool, and drives them immediately to the `stack`.
- **ALU Resolution**: Defined explicit pops within the engine's operation matchers (`OpAdd`, `OpSubtract`, `OpMultiply`, `OpDivide`). The machine honors strict RPN compliance by extracting Right nodes before Left nodes and performing fast, localized mathematical processing before re-pushing the sum natively.
- **I/O & Halt**: Formalized `OpPrint` (pops top stack element to stdout) and `OpReturn` (disengages the while execution loop).
- **Execution Proof**: Embedded inline unit tests simulating pre-compiled arrays `10 + 5` and `(10 - 2) * 3`. Proved bytecode processes instantly without recursive branching overhead.

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 96 - Implement VM Execution Loop and Stack Dispatcher`.

---

## [v0.95.0] - Sprint 95: Implement AST-to-Bytecode Compilation (2026-03-15)

### Added — Architecture (Parallel Feature)
- **AST Translation Pipeline (`vm/compiler.rs`)**: Implemented `compile_node` to recursively parse `ast::Node` trees. Translates standard literal primitives and binary operations directly into linear instruction sets matching the Reverse Polish Notation (RPN) specification natively understood by the machine loop. Left Node eval -> Right Node eval -> Operator.
- **Constant Deduplication**: Augmented the compiler to check memory addresses dynamically. Recurring Strings, Floats, or Integers are now uniquely mapped into the `constants` pool vector, completely stopping memory ballooning when running intensely iterative ASTs.
- **Compiler Validation Tests**: Bootstrapped inline unit tests explicitly simulating AST logic loops and validating flat bytecode array emission and deduplicated constant tracking.

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 95 - Implement AST-to-Bytecode Compilation`.

---

## [v0.94.0] - Sprint 94: Initialize Bytecode VM and Compiler Architecture (2026-03-15)

### Added — Architecture (Parallel Feature)
- **OpCode ISA (`vm/opcode.rs`)**: Established the foundational machine language enum `OpCode` defining `Constant(usize)`, block math operations, and execution flow.
- **AOT Compiler (`vm/compiler.rs`)**: Implemented the `Compiler` struct responsible for flattening arbitrary JSON AST node structures directly into robust arrays of linear OpCodes. Mapped literals (Int, Float, Str) natively into a structured `constants` pool vector, massively reducing tree-allocation overheads.
- **Bytecode Machine (`vm/machine.rs`)**: Built the `VM` evaluator core operating via instruction pointer (`ip`) and a high-speed pre-allocated stack (`Vec<RelType>`), stripping away the recursive latency native to the old `ExecutionEngine` interpreter. 
- Integrated sub-modules seamlessly inside `src/vm/mod.rs` alongside the existing storage systems. 

### Compliance
- Git commit cleanly pushed by autonomous agent. Commit message: `Feat: Sprint 94 - Initialize Bytecode VM and Compiler Architecture`.

---

## [v0.90.0] - Sprint 90: Day 1 Patch & Architecture Polish (2026-03-14)

### Changed — Architecture & Hygiene
- **Zero-Latency Event-Loop (window.rs / registry.rs / run_knc.rs)**: Removed the polling-based `mpsc::channel` connecting the `ExecutionEngine` to `winit`. Replaced entirely with Winit's native `EventLoopProxy<RenderCommand>` and `EventLoopBuilder::with_user_event()`, resolving the 1-frame rendering latency / stuttering.
- **Windows UNC Path Fix (executor.rs)**: Swapped out `std::fs::canonicalize` for `dunce::canonicalize`. This cleanly strips the problematic `\\?\` prefix from Windows paths across `validate_fs_path` and `validate_fs_path_write`, unbreaking CWD validation logic on Windows.
- **Clippy Polish (vm.rs / parser.rs)**: Addressed compilation warnings by deriving `Default` for `VM` and `VMCompiler`, resolving nested combinatorial conditions (collapsible `if` lets) within the Fetch parser, and clearing out dead/unused `Mat4`/`Vec3` and `Window` imports.

### Compliance
- Git commit pushed by autonomous agent. Commit message: `Fix: Sprint 90 - EventLoopProxy Latency Fix, Windows dunce Pathing and Code Hygiene`.
- Re-verified all 54 knoten integration tests and successfully compiled Windows targets.

---

## [v0.89.0] - Sprint 89: Zero-Day Fixes & Release Candidate (2026-03-14)

### Fixed — Security & Stability
- **FFI Security Lockdown (FINDING 2)**: Extended `validate_fs_path` and `validate_fs_path_write` from `executor.rs` to secure `fs_read_file`, `registry_texture_load`, and `registry_file_create`. Prevents directory traversal attacks via `../../` escapes.
- **WGPU Surface Panic (FINDING 3)**: Fixed `RedrawRequested` panic in `window.rs` on window resize/minimize by implementing a proper match on `surface.get_current_texture()`, handling `Outdated` and `Lost` by explicitly reconfiguring the surface.
- **Anti-Zombie Protocol & Cleanups (FINDING 4)**: Introduced `RenderCommand::ExitEventLoop` to ensure the winit EventLoop in `run_knc.rs` shuts down gracefully exactly when the background AST executor thread finishes. removed unused variable block in `registry_fill_color`.
- **Test-Suite Fix (FINDING 1)**: Replaced hardcoded float literals (3.14/3.1415) in `tests/integration_tests.rs` with `std::f32::consts::PI` to resolve `clippy::approx_constant` warnings and reinstate test validity.

### Added — Documentation
- Refreshed documentation across `README.md`, `llm.md`, and `changelog.md` prioritizing production-ready release state.

### Compliance
- Git commit pushed by autonomous agent. Commit message: `Fix: Sprint 89 - Pre-Release Audit Fixes and Graceful Shutdown`.

## [v0.88.0] - Sprint 88: Targeted Code Optimization (2026-03-14)
Resolved targeted performance bottlenecks in array and map manipulation during evaluation. Ensured stricter adherence to expected error propagation formats across both JIT and VM pipelines.

### Changed — Performance & Stability
- **`ExecutionEngine` (executor.rs)**: Introduced direct, zero-clone mutation functions (`mutate_map_insert`, `mutate_array_set`, `mutate_array_push`) to drastically reduce the $O(N)$ allocation penalty of deep-cloning collections just to add or modify a single element. Memory overhead is significantly reduced for large vectors and dictionaries.
- **`Evaluator` (evaluator.rs)**: Upgraded AST array push, set, and object property assignments to utilize the zero-clone `mutate_*` functions, preserving referential integrity on evaluation instead of re-allocating.
- **`VM` (vm.rs)**: Converted `VM::execute` return signature to `Result<RelType, String>`, cleanly propagating mathematical faults like `#Division by zero` backward to the caller instead of silently swallowing the error or yielding `0`.

### Compliance
- Git commit pushed by autonomous agent. Commit message: `Opt: Sprint 88 - Targeted Array/Map Zero-Clone Optimization and VM Fault Propagation`.
- Successfully validated against all 54 integration test oracles.
- Updated core developer-facing and user-facing documentation per policy.

---

## [v0.87.0] - Sprint 87: Documentation & Release Polish (2026-03-13)

### Changed — Documentation
- **`README.md`**: Fully rewritten as professional release documentation. Removed all internal sprint references. Reorganized into canonical feature sections: Thread-Safe Sandbox, WGPU Hardware Rendering, JIT/AOT Execution, Automatic ARC, Structured Fault Reporting, and Unified Physics.
- **`llm.md`**: Fully rewritten as a clean AI Agent Reference. All "Sprint XX:" section headers replaced with descriptive, feature-based headings. All technical content (code examples, security rules, ARC patterns, JSON snippets) preserved and verified against current engine state.
- **`AGENT_EXTENSION_MANUAL.md`**: Removed all internal sprint-labeled section headers. Technical extension instructions remain intact.

### Compliance
- Git commit pushed by autonomous agent. Commit message: `Docs: Sprint 87 - Professionalize Documentation and Remove Sprint History`.

---

## [v0.86.0] - Sprint 86: WGPU Pipeline Forging (2026-03-11)
Completed the real WGPU rendering pipeline per Audit v7 findings.

### Added — Camera Command (FINDING-5)
- **`RenderCommand::SetCamera { window_id: usize, view_proj: [[f32;4];4] }`** added to the enum in `registry.rs`.
- **`registry_set_camera(fov, x, y, z)`** now computes a real `perspective_rh × look_at_rh` matrix via `glam` and sends `SetCamera` (broadcasts to window_id=0 = all windows).
- **`registry_set_camera_for_window(win_id, fov, x, y, z)`** — new Rust function + bridge entry that sends `SetCamera` to a specific window identified by handle id.

### Added — Camera UBO Write (FINDING-4)
- **`SetCamera` handler in `window.rs`** writes the 64-byte view-proj matrix into `camera_buffer` via `queue.write_buffer`. The `RedrawRequested` handler still writes a sane fallback each frame so frames render even before a camera command is sent.

### Fixed — State Management & Resize (FINDING-3 & 7)
- **`config: wgpu::SurfaceConfiguration`** field added to `RegistryWindowState` and stored at window creation.
- **`WindowEvent::Resized`** now mutates `state.config.width`/`height` and calls `state.surface.configure(&state.device, &state.config)` — no more temporary one-off config object with potentially wrong fields.
- **Camera UBO aspect** fixed to use actual `state.width / state.height` each frame instead of a hardcoded `16:9`.

---

## [v0.85.0] - Sprint 85: Real Renderer Port (2026-03-11)
Replaced the partially-fake 3D rendering pipeline with a fully correct WGPU implementation.

### Fixed — Rendering (Real, Not Fake)
- **FINDING-4 — Vertex Layout**: Added `pub normal: [f32; 3]` to `RegistryVertex` in `registry.rs`. Layout now matches `mesh3d.wgsl`: `@location(0) position`, `@location(1) normal`, `@location(2) uv`.
- **FINDING-4 — Geometry Normals**: `generate_uv_sphere` now outputs correct outward normals (position = normal for unit sphere). `generate_cylinder` outputs upward/downward cap normals and outward side normals.
- **FINDING-4 — Missing Cube Geometry**: Added `generate_cube()` function producing a 24-vertex, 36-index unit cube with per-face flat normals. `registry_draw_cube` now sends `AddMesh` the first time it is called, so the cube actually appears.
- **FINDING-1 — Pipeline Vertex Stride**: Changed vertex buffer layout in `window.rs` from `size_of::<VoxelVertex>()` (32 bytes, now unused here) to `size_of::<RegistryVertex>()` (32 bytes, correct). The struct sizes happen to match, but the attribute layout (position/normal/uv vs position/uv) was wrong.
- **FINDING-3 — Camera Bind Group**: The `camera_bgl` declares 4 bindings (0=uniform, 1=diffuse tex, 2=sampler, 3=normal map). `camera_bind_group` previously only filled binding 0 — GPU validation would fail or garbage data rendered. Now all 4 bindings are satisfied with the white 1×1 default texture/sampler as fallbacks.
- **FINDING-3 — Camera UBO Content**: The camera buffer was zero-filled every frame (no matrix written). `RedrawRequested` now writes a real `perspective_rh × look_at_rh` view-projection matrix at offset 0 of the 240-byte `MeshUniforms` buffer.
- **FINDING-2 — Resize Surface Format**: `WindowEvent::Resized` used hardcoded `Bgra8UnormSrgb` instead of the stored `state.surface_format`. Fixed to use `state.surface_format`.
- **Init Order Fix**: Default texture+sampler are now created *before* the camera bind group so their views can be referenced in entries 1/2/3.

### Fixed — Camera Buffer Size
- Camera buffer resized from 80 bytes (`Mat4 + Vec4`) to 240 bytes (full `MeshUniforms`: `mat4 + 3×vec4 + 4×PointLight`).

### Removed — Zombie Code (executor.rs)
Deleted ~20 dead WGPU fields from `ExecutionEngine` that were never read after the Sprint 72 architecture migration to `window.rs`:
`device`, `queue`, `surface_format`, `depth_texture_view`, `current_canvas_view`, `current_canvas_frame`, `default_texture_view`, `default_sampler`, `shaders`, `render_pipelines`, `voxel_pipeline`, `voxel_vbo`, `voxel_ibo`, `voxel_instances`, `voxel_bind_group`, `voxel_atlas_bind_group`, `voxel_ubo`, `voxel_instance_buffer`, `mesh_cache`, `frame_encoder`, `mesh_ubo`, `meshes`, `textures`, `canvas_mesh_pipeline`.
Also deleted the now-unused `MeshBuffers` struct.

---

## [v0.83.0] - Sprint 83: Emergency Security & Architecture Fix (2026-03-11)
Closed 6 critical findings from Audit Round 6. Full sandbox hardening pass.

### Fixed — Security
- **FINDING-03 — Network Sandbox Escape**: `Node::Fetch` now checks `allow_network` permission before dispatching to `AsyncBridge`. Without the `--allow-network` flag the engine returns `ExecResult::Fault` immediately, preventing silently unrestricted outbound HTTP calls.
- **FINDING-05 — FS Path Traversal (Directory Escape)**: All four filesystem operations (`FileRead`, `FileWrite`, `FSRead`, `FSWrite`) now validate and canonicalize the supplied path. Paths that resolve outside the current working directory are rejected with `Security: Path escape detected`, closing the `../../etc/passwd`-class sandbox escape.
- **FINDING-09 — `set_var` Scope Pollution**: Refactored `set_var` in `executor.rs`. When a variable is not found in any call stack frame, it is now created in the global `self.memory` instead of silently pushing into the innermost `StackFrame`. This eliminates the bug where first-time assignments inside function calls were invisibly dropped on return.

### Fixed — VM Hardening
- **FINDING-06 — VM `panic!()` Calls**: Replaced all 5 `panic!("VM TypeError: ...")` calls and all naked `.unwrap()` calls on `stack.pop()` in `VM::execute` with safe `unwrap_or(RelType::Void)` fallbacks. Type mismatches now push `RelType::Void` onto the stack and execution continues, instead of aborting the process.

### Fixed — Architecture
- **FINDING-07 — Unsound `unsafe impl Sync`**: Removed `unsafe impl Sync for ExecutionEngine`. `ExecutionEngine` contains `cpal::Stream` which is explicitly `!Sync`. Since the engine is single-owner per thread, only `Send` is required. The `unsafe impl Send` (already correct) is retained.
- **FINDING-01 — `release_handles` FnDef Analysis**: Confirmed via code analysis that `release_handles` is correctly a no-op. Rust's drop glue on `RelType::FnDef(_, _, Box<Node>)` handles recursive deallocation automatically. Added a comprehensive documentation comment explaining why the no-op is correct and safe, preventing future well-intentioned but incorrect attempts to add manual recursion.

### Added
- **`--allow-network` CLI Flag**: Added to `run_knc` binary. Required to use `Node::Fetch`. Mirrors the existing `--allow-read` / `--allow-write` pattern.
- **`validate_fs_path` / `validate_fs_path_write`**: Two internal static helpers on `ExecutionEngine` implementing secure path resolution. Read-paths use `std::fs::canonicalize` (requires file to exist). Write-paths normalize `..` components manually without requiring the target to exist, then verify the result is inside the working directory.


## [v0.80.0] - Sprint 80: Security Lockdown (ExternCall Bypass)
Addressed a critical security vulnerability where `ExternCall` and native I/O operations could bypass the engine's permission system.

### Changed
- **`NativeModule` & `BridgeModule` Traits**: Updated handles to accept `AgentPermissions`, ensuring all native extensions are permission-aware.

- **`CoreBridge` Validation**: Integrated strict `FS_READ` and `FS_WRITE` checks into the FFI bridge for `registry` and `fs` operations.
- **ExternCall Interception**: Added a pro-active security layer in `executor.rs` that validates function calls before they reach the FFI bridge.

### Fixed
- **Sandbox Bypass**: Closed the vulnerability allowing unauthorized file system access via `ExternCall`.
- **Structured Error Reporting**: Permission denials now return formal `ExecResult::Fault` messages with specific context (e.g., `"Permission Denied: FS_READ"`).

---

## [v0.78.0] - Sprint 78: Error Tracing Foundation
Introduced a structured error reporting mechanism to provide deep diagnostic context for runtime failures, enabling future self-healing capabilities for AI agents.

### Changed
- **`ExecResult::Fault` Structure**: Expanded from a simple string to a struct containing both an error message (`msg`) and an AST node context (`node`).
- **Enhanced Diagnostics**: Systematically updated the evaluator, executor, renderer, and all native modules (Math, IO, Bridge) to report the specific node or function where an error occurred (e.g., `Node::MathDiv`, `Native::IO::ReadFile`).
- **Improved Pattern Matching**: Re-engineered the internal error handling and delegation logic in `evaluator.rs` to support the new structured fault data.

### Added
- **Validation Suite (Phase 2)**: Introduced `tests/intentional_crash.knoten` to verify the new error structure.
- **Testing Section**: Added formal validation instructions to `README.md` and `llm.md` for deterministic engine verification.

---

## [v0.77.0] - Sprint 77: Unified Physics (The Collision Sprint)
Unified the physics engine by integrating generic AABB (Axis-Aligned Bounding Box) collision logic directly into the FPS camera movement, replacing the previous hardcoded voxel-only restriction.

### Added
- **`Node::AddWorldAABB`**: New AST node allowing scripts to register arbitrary physical barriers and invisible collision volumes.
- **Unified Collision Resolution**: Re-engineered camera movement in `window.rs` to check for intersections against all registered world boxes.
- **Dynamic Camera AABB**: The player's environment presence is now defined by a standard bounding volume (AABB), ensuring consistent interaction with custom geometry.

### Fixed
- **Physics Disconnect**: Resolved the gap between the AST `CheckCollision` system and the actual hardware camera movement.

---

## [v0.76.0] - Sprint 76: Async, Natives & Security (The Hardening)
Completed the connectivity for asynchronous operations and native modules while introducing a strict security sandbox.

### Added
- **Asynchronous Bridge (`async_bridge.rs`)**: Restored `Node::Fetch` and `Node::Extract` functionality with background worker thread spawning.
- **Security Sandboxing**: Implemented `Deny-by-Default` for file system access. Added CLI flags `--allow-read` and `--allow-write` to `run_knc`.
- **Automatic ARC Handle Management**: Introduced `NativeHandle` struct with custom `Drop` logic, automating native resource cleanup across the JIT evaluator.

### Fixed
- **Handle Leakages**: Resolved recursive "hanging handles" by leveraging Rust's ownership system for DSL-level resources.
- **Borrow-Checker Conflicts**: Re-engineered the `AsyncBridge` polling mechanism in `executor.rs` to safely evaluate callbacks without holding internal state references.

---

## [v0.58.0] - Sprint 58: Neural Syntax (Agent-to-Agent DSL)
Replaced verbose JSON AST with a high-density, closure-based DSL designed for maximum AI parsing efficiency and token compression.

### Added
- **High-Density Parser (`parser.rs`)**: Custom zero-dependency Lexer and recursive descent AST parser specifically reading `.knoten` files.
- **Knoten-Transpiler (`dsl_emitter.rs`, `knoten_upgrade.rs`)**: AST formatting engine that auto-upgrades existing `.nod`/`.json` trees into their perfectly equivalent `.knoten` DSL syntax.
- **Cross-Platform Compilation Workflow**: Introduced `.github/workflows/release.yml` automating binary releases (<5MB) for macOS, Linux, and Windows.

### Changed
- `run_knc` automatically executes `serde_json` or the new `Parser` based on file extension (`.nod`/`.json` vs `.knoten`).

---

## [v0.57.0] - Sprint 57: State & Scroll (The Technical Deep Dive)
Introduced infinite scrolling capabilities, aggressive binary slimming, and drafted the next-generation Knoten-DSL.

### Added
- **`UIScrollArea(String, Box<Node>)`**: Native implementation of `egui::ScrollArea`. Eliminates the previous UI cap limitations by enabling dynamic, scrollable lists of unbounded depth.
- **Knoten-DSL Draft (`KNOTEN_DSL_DRAFT.md`)**: Proposed human-readable, closure-based curly brace syntax to replace raw JSON AST authoring.

### Changed
- **Binary Slimming (`Cargo.toml`)**: Reconfigured `[profile.release]` with `lto = "fat"`, `codegen-units = 1`, `opt-level = "z"`, and `strip = true` to aggressively condense the final binary footprint towards the <5MB objective.

---

## [v0.56.0] - Sprint 56: The Grid Layout Update
Introduced native Egui Grid support for high-precision UI distributions.

### Added
- **`UIGrid(i64, String, Box<Node>)`**: Implemented `egui::Grid` wrapper with autonomous `end_row()` management. Optimized for uniform 2D layouts (calculators, dashboards).
- **Auto-Row Management**: The executor now tracks column counts within `UIGrid` blocks and triggers row termination automatically after N elements.

---

## [v0.55.0] - Sprint 55: The UI Hardening Sprint
Resolved critical UI type inconsistencies and introduced native horizontal layout and fullscreen panel nodes.

### Fixed
- **`UIButton` Type Mismatch (#1):** `UIButton` now returns `RelType::Bool` instead of `RelType::Int`. Direct use as an `If`-condition now works natively without type coercion.
- **`RelType::Display` Annotation (#2):** `Display` now renders pure human-readable values (`42`, `true`, `hello`). Debugging output (with type tags) has been moved to the `Debug` trait. The internal `execute()` test harness uses `Debug` to keep test suite unbroken.
- **Egui Depth Buffer (#6):** Confirmed that the Egui 2D render pass uses `depth_view_opt` which resolves to `None` for 2D-only renderering. No Z-test on UI passes.
- **Windows EventLoop (#7):** Confirmed `with_any_thread(true)` fix already in place from Sprint 54.

### Added
- **`UIHorizontal(Box<Node>)`**: Renders child nodes side-by-side in a single egui horizontal layout row. Enables button grids, toolbars, and multi-column forms.
- **`UIFullscreen(Box<Node>)`**: Renders a borderless, zero-title-bar panel covering the entire canvas. Ideal for immersive game HUDs and full-screen overlay UIs.

---

## [v0.81.0] - Sprint 81: Primitive Resurrection & Mat4Mul
Restored 3D primitive geometry generation and implemented the essential matrix multiplication logic for advanced 3D scenes.

### Added
- **Restored Primitives**: Re-implemented vertex/index generation for `Sphere` (UV-mapped) and `Cylinder` in `registry.rs`.
- **`Node::Mat4Mul`**: Fully implemented 4x4 matrix multiplication in `evaluator.rs` for `RelType::Array` containing 16 elements.
- **Background Mesh Transfer**: Introduced `RenderCommand::AddMesh` to asynchronously send generated geometry from the executor thread to the main renderer thread.
- **Renderer Cache Integration**: `window.rs` now correctly handles `AddMesh` and draws primitives using the shared `geometry_cache`.

### Fixed
- **Placeholder Primitives**: Replaced no-op drawings with real geometric rendering.
- **Matrix Logic**: Restored the missing `Mat4Mul` implementation in the JIT evaluator.

---

## [0.80.0] - Sprint 80: The Thread-Safety Revolution
### Added
- **Native 3D Primitives (Cube, Cylinder)**: Expanded the registry with `registry_draw_cube` and `registry_draw_cylinder` for efficient geometry generation.
- **Native 3D Primitives (Sphere)**: Implemented `registry_draw_sphere` in the core registry.
- **Global UI Style Engine**: Bound a new AST node `UISetStyle` manipulating the global `egui::Context` rendering frame. Modifiable metrics include Window Rounding, Item Spacing, RGBA Accent coloring, and Background Panel shading, perfect for rendering Glassmorphism and Flat Design.
- **The Ultimate Constant**: Bound `registry_get_ultimate_answer` returning 42 natively via the FFI.
- **AOT & JIT Node Integration**: Upgraded the `executor.rs` stack and `optimizer.rs` counting arrays to safely recurse into all new stylistic nodes.

---

## [v0.54.0] - Sprint 54: The Styling & Persistence Update
Introduced panic-resilient File I/O mappings and dynamic EGUI stylistic overrides powered natively by the JSON.

### Added
- **File I/O Persistence**: Engineered `registry_read_file` and `registry_write_file` using `std::fs` natively, with robust error catching to prevent runtime panics within the ARC registry.

---

## [v0.53.0] - Sprint 53: The Kinetic Update (Input System)
Successfully implemented a universally applicable, thread-safe input handling system that bridges both game-engine inputs and software application inputs natively.

### Added
- **Global `InputState`:** Implemented as an `Arc<Mutex>` resolving all `DeviceEvent` and `WindowEvent` hooks from winit 0.30 via a new `pump_app_events` method on the main EventLoop.
- **Physical Keys (Gaming):** Maintained via `winit::keyboard::KeyCode` mapped in a `HashSet` for instantaneous queries over WASD / Arrow Keys.
- **Mouse Motion (3D/FPS):** Raw optical sensor deltas (`DeviceEvent::MouseMotion`) gracefully accumulate per-frame into `mouse_dx` and `mouse_dy`, completely untied from the UI cursor.
- **Text Typing (Software):** Automatically respects shift/caps keyboard contexts via `event.logical_key`, yielding the active `last_char` in exact u32 unicode form for native text-editing.
- **FFI & ARC Synchronization:** Added 4 new endpoints in `bridge.rs` (`registry_is_key_pressed`, `registry_get_mouse_delta_x/y`, `registry_get_last_char`), all casting safely to the `RelType` schema.

### Changed
- **Thread-Safe Resets:** Ensured exact VSync rendering intervals within `registry_window_update` before pumping the upcoming frame, protecting accumulation values during complex AST script loops.

---

## [v0.52.0] - Sprint 52: The 3D Hallway Flex
Extended the bare-metal WGPU integration from 2D billboarding/UI to true 3D spatial rendering. 

### Added
- **Camera Buffer:** Added a `wgpu::Buffer` and dedicated `BindGroup` for the Camera Uniform to feed the projection-view matrix.
- **Z-Buffer Depth Ordering:** Instantiated a `wgpu::TextureFormat::Depth32Float` texture attachment configured with `CompareFunction::Less` to correctly sort overlapping geometric quads.
- **GLAM Matrix Math:** Introduced `glam` dependency to dynamically assemble the camera `perspective_rh_gl()` and object `Mat4::from_scale_rotation_translation`.
- **PushConstants:** Upgraded pipeline layout to inject 64-byte `model_matrix` push constants per draw-call, avoiding dynamic uniform buffers.
- **Native AST Bindings:** Added `registry_set_camera` to mathematically orbit the scene camera, and refactored `registry_draw_quad_3d` to accept floating point coordinates (x, y, z, sx, sy).

---

## [v0.50.0] - Sprint 50: The Great ARC Reforging
Resolved critical memory vulnerabilities identified during the external security audit.

### Fixed
- **Core ARC Safety:** `registry_free` now safely wraps `registry_release` instead of removing handles, properly honoring the `ref_count`.
- **Panic Protection:** Fixed blind mutex locks (`unwrap()`), replacing them with `unwrap_or_else(|e| e.into_inner())` to prevent fatal panic poisoning.
- **RelType Clone-Bug:** `RelType` now properly manages its deep structure via a manual `Clone` implementation, guaranteeing that cloning inherently bumps its ARC `ref_count`.
- **AOT Memory Tracking:** The AOT transpiler now tracks `is_handle` block by block. Overwriting a native handle explicitly injects a `registry::registry_release()` natively into the compiled Rust output block, resolving all loop-based memory leaks.

---

## [v0.48.0] - Sprint 48: The Lexicon
Empowered KnotenCore with nested Key-Value dictionaries mapped to the standard Rust `HashMap`.

### Added
- **Native Maps:** Added `Type::Map` and corresponding AST Node variants (`MapCreate`, `MapSet`, `MapGet`, `MapHasKey`).
- **Deep ARC Integration:** JIT and AOT engines inherently support iterative de-allocation for maps. AOT intercepts assigned combinations utilizing Maps holding handles, iterating over inner keys to statically inject `registry_release` during scope exit.
