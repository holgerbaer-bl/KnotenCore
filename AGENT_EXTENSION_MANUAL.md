# KnotenCore — Agent Extension Manual

> **System Instruction for LLM Code Agents**
>
> This document teaches you, an AI coding assistant, how to extend KnotenCore with new native operations. Follow these steps precisely.

---

## Architecture Overview

KnotenCore programs are JSON-encoded Abstract Syntax Trees (AST). The runtime interprets them (JIT) or compiles them to standalone Rust binaries (AOT). All OS resources are managed through a deterministic ARC registry.

To add a new native operation (e.g. `AudioPlay`, `DrawSprite`), you must update exactly **4 touchpoints**:

| # | File | Role |
|---|------|------|
| 1 | `src/ast.rs` | Define the AST node variant |
| 2 | `src/natives/registry.rs` | Implement the native Rust function |
| 3 | `src/executor.rs` | Wire the node into the JIT evaluator |
| 4 | `src/compiler/codegen.rs` | Wire the node into the AOT transpiler |

---

## Step 1: Define the AST Node (`src/ast.rs`)

Add your new variant to the `Node` enum. Use `Box<Node>` for expression arguments.

```rust
// src/ast.rs — inside pub enum Node { ... }

// Example: A hypothetical DrawSprite node
DrawSprite(Box<Node>, Box<Node>, Box<Node>), // TextureHandle, X, Y
```

If your node returns a **Handle** (an OS resource), also check `Type` enum — `Type::Handle` already covers this.

---

## Step 2: Implement the Native Function (`src/natives/registry.rs`)

If you need a new resource type, add it to `NativeHandle`:

```rust
pub enum NativeHandle {
    // ... existing variants ...
    MyNewResource(MyResourceStruct),
}
```

Then implement the public function:

```rust
pub fn registry_my_function(arg1: i64, arg2: String) -> i64 {
    // 1. Create or acquire the resource
    let resource = MyResourceStruct::new(arg1, &arg2);

    // 2. Allocate a unique handle ID
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap();
    let id = *id_guard;
    *id_guard += 1;

    // 3. Insert into the ARC registry
    with_registry(|registry| {
        registry.insert(id, RegistryEntry {
            handle: NativeHandle::MyNewResource(resource),
            ref_count: 1,
        });
    });

    id as i64 // Return the handle ID
}
```

**ARC Rule**: When `registry_release` drops the ref_count to 0, the `NativeHandle` variant is removed from the HashMap. Rust's `Drop` trait handles deallocation. If your resource needs explicit cleanup, add match arms in `registry_release`.

---

## Step 3: Wire into the JIT Evaluator (`src/executor.rs`)

Find the main `match node { ... }` block in the `evaluate()` method and add your node:

```rust
Node::DrawSprite(tex_node, x_node, y_node) => {
    let tex = self.evaluate(tex_node);
    let x = self.evaluate(x_node);
    let y = self.evaluate(y_node);
    match (tex, x, y) {
        (
            ExecResult::Value(RelType::Handle(tex_id)),
            ExecResult::Value(RelType::Int(x_val)),
            ExecResult::Value(RelType::Int(y_val)),
        ) => {
            registry::registry_draw_sprite(tex_id, x_val, y_val);
            ExecResult::Value(RelType::Void)
        }
        _ => ExecResult::Fault("DrawSprite: invalid arguments".to_string()),
    }
}
```

**Important**: Also update `validator.rs` and `optimizer.rs` to handle the new node (add match arms for traversal/counting).

---

## Step 4: Wire into the AOT Transpiler (`src/compiler/codegen.rs`)

Add a match arm in the `generate()` method:

```rust
Node::DrawSprite(tex, x, y) => {
    format!(
        "registry::registry_draw_sprite({}, {}, {})",
        self.generate(tex, false),
        self.generate(x, false),
        self.generate(y, false)
    )
}
```

If the function **returns a Handle**, update the `is_handle_expr()` method to recognize it:

```rust
Node::NativeCall(fn_name, _) => {
    matches!(fn_name.as_str(),
        "registry_my_function"
        | ... // add your function name here
    )
}
```

---

## JSON AST Format

The JSON representation for calling your new node:

```json
{
  "DrawSprite": [
    { "Identifier": "my_texture" },
    { "IntLiteral": 100 },
    { "IntLiteral": 200 }
  ]
}
```

Or via `NativeCall` (no AST change needed, only registry function):

```json
{
  "NativeCall": ["registry_my_function", [
    { "IntLiteral": 42 },
    { "StringLiteral": "resource_name" }
  ]]
}
```

---

## Checklist

- [ ] Added `Node::YourNode` variant in `ast.rs`
- [ ] Added `NativeHandle::YourType` (if needed) in `registry.rs`
- [ ] Implemented `pub fn registry_your_fn()` in `registry.rs`
- [ ] Added match arm in `executor.rs` → `evaluate()`
- [ ] Added match arm in `codegen.rs` → `generate()`
- [ ] Updated `validator.rs` → `check_node()` match
- [ ] Updated `optimizer.rs` → `count_nodes()` and `optimize()` matches
- [ ] Run `cargo test` — all 54+ tests green
- [ ] Created example `.nod` script to verify
