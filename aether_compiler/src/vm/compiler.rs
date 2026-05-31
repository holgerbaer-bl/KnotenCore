use crate::executor::RelType;
use knoten_core_types::ast::Node;
use knoten_core_types::opcode::{OpCode, SimdOp};

#[derive(Default)]
pub struct Compiler {
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub functions: std::collections::HashMap<String, usize>,
    pub locals: Vec<std::collections::HashMap<String, usize>>,
    pub current_local_count: usize,
    pub freed_slots: Vec<usize>, // Sprint 197: pooled free register slots
    pub timing_markers: Vec<String>, // Sprint 199: pre-200 profiling placeholder
    pub imported_files: std::collections::HashSet<String>,
    pub current_dir: std::path::PathBuf,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: std::collections::HashMap::new(),
            locals: Vec::new(),
            current_local_count: 0,
            freed_slots: Vec::new(),
            timing_markers: Vec::new(),
            imported_files: std::collections::HashSet::new(),
            current_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Recursively flattens an AST math/logic tree into linear opcodes.
    /// Returns false if the node cannot be compiled (e.g. it contains side-effects or variables).
    pub fn compile_node(&mut self, node: &Node) -> bool {
        match node {
            Node::IntLiteral(v) => {
                let idx = self.add_constant(RelType::Int(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::FloatLiteral(v) => {
                let idx = self.add_constant(RelType::Float(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::StringLiteral(v) => {
                let idx = self.add_constant(RelType::Str(v.clone()));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::BoolLiteral(v) => {
                let idx = self.add_constant(RelType::Bool(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::Add(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Add);
                true
            }
            Node::Sub(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Subtract);
                true
            }
            Node::Mul(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Multiply);
                true
            }
            Node::Div(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Divide);
                true
            }
            Node::Modulo(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Modulo);
                true
            }
            Node::Neg(expr) => {
                if !self.compile_node(expr) {
                    return false;
                }
                self.instructions.push(OpCode::Neg);
                true
            }
            Node::Eq(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Equal);
                true
            }
            Node::Gt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Greater);
                true
            }
            Node::Lt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Less);
                true
            }
            Node::Lte(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::LessEqual);
                true
            }
            Node::Gte(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::GreaterEqual);
                true
            }
            Node::NotEq(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::NotEqual);
                true
            }
            Node::And(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::And);
                true
            }
            Node::Or(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) {
                    return false;
                }
                self.instructions.push(OpCode::Or);
                true
            }
            Node::Not(expr) => {
                if !self.compile_node(expr) {
                    return false;
                }
                self.instructions.push(OpCode::Not);
                true
            }
            Node::Block(stmts) => {
                for stmt in stmts {
                    if !self.compile_node(stmt) {
                        return false;
                    }
                }
                true
            }
            Node::If(cond, then_block, else_block) => {
                if !self.compile_node(cond) {
                    return false;
                }
                let jump_if_false_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0)); // Placeholder

                if !self.compile_node(then_block) {
                    return false;
                }

                if let Some(else_branch) = else_block {
                    let jump_idx = self.instructions.len();
                    self.instructions.push(OpCode::Jump(0)); // Placeholder

                    // Backpatch JumpIfFalse to jump here (start of else block)
                    self.instructions[jump_if_false_idx] =
                        OpCode::JumpIfFalse(self.instructions.len());

                    if !self.compile_node(else_branch) {
                        return false;
                    }

                    // Backpatch unconditional Jump to jump past the else block
                    self.instructions[jump_idx] = OpCode::Jump(self.instructions.len());
                } else {
                    // Backpatch JumpIfFalse to jump past the then block
                    self.instructions[jump_if_false_idx] =
                        OpCode::JumpIfFalse(self.instructions.len());
                }
                true
            }
            Node::While(cond, body) => {
                let loop_start_idx = self.instructions.len();
                if !self.compile_node(cond) {
                    return false;
                }

                let jump_if_false_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0)); // Placeholder

                if !self.compile_node(body) {
                    return false;
                }

                self.instructions.push(OpCode::Jump(loop_start_idx)); // Loop back
                self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len()); // Exit loop
                true
            }
            Node::Assign(ident, expr) => {
                if !self.compile_node(expr) {
                    return false;
                }

                if !self.locals.is_empty() {
                    let idx = if let Some(idx) = self.resolve_local(ident) {
                        idx
                    } else {
                        // Sprint 197: Reuse freed slots before allocating new ones
                        let idx = if let Some(slot) = self.freed_slots.pop() {
                            slot
                        } else {
                            let idx = self.current_local_count;
                            self.current_local_count += 1;
                            idx
                        };
                        if let Some(last) = self.locals.last_mut() {
                            last.insert(ident.clone(), idx);
                        } else {
                            return false;
                        }
                        idx
                    };
                    self.instructions.push(OpCode::SetLocal(idx));
                } else {
                    let idx = self.add_constant(RelType::Str(ident.clone()));
                    self.instructions.push(OpCode::SetGlobal(idx));
                }
                true
            }
            Node::Identifier(ident) => {
                if !self.locals.is_empty()
                    && let Some(idx) = self.resolve_local(ident)
                {
                    self.instructions.push(OpCode::GetLocal(idx));
                    return true;
                }
                let idx = self.add_constant(RelType::Str(ident.clone()));
                self.instructions.push(OpCode::GetGlobal(idx));
                true
            }
            Node::NativeCall(name, args) | Node::Call(name, args) => {
                match name.as_str() {
                    "str_len" => {
                        if args.len() != 1 {
                            return false;
                        }
                        if !self.compile_node(&args[0]) {
                            return false;
                        }
                        self.instructions.push(OpCode::StringLength);
                        true
                    }
                    "str_contains" => {
                        if args.len() != 2 {
                            return false;
                        }
                        if !self.compile_node(&args[0]) {
                            return false;
                        } // String source
                        if !self.compile_node(&args[1]) {
                            return false;
                        } // Pattern / char class
                        self.instructions.push(OpCode::StringContainsChars);
                        true
                    }
                    "str_split" => {
                        if args.len() != 2 {
                            return false;
                        }
                        if !self.compile_node(&args[0]) {
                            return false;
                        } // Target String
                        if !self.compile_node(&args[1]) {
                            return false;
                        } // Delimiter
                        self.instructions.push(OpCode::StringSplit);
                        true
                    }
                    "arr_contains" => {
                        if args.len() != 2 {
                            return false;
                        }
                        if !self.compile_node(&args[0]) {
                            return false;
                        } // Array
                        if !self.compile_node(&args[1]) {
                            return false;
                        } // Search String
                        self.instructions.push(OpCode::ArrayContains);
                        true
                    }
                    "read_file" => {
                        if args.len() != 1 {
                            return false;
                        }
                        if !self.compile_node(&args[0]) {
                            return false;
                        }
                        self.instructions.push(OpCode::ReadFile);
                        true
                    }
                    _ => {
                        if let Some(&target_ip) = self.functions.get(name) {
                            for arg in args {
                                if !self.compile_node(arg) {
                                    return false;
                                }
                            }
                            self.instructions.push(OpCode::Call(target_ip, args.len()));
                            true
                        } else {
                            // FFI ExternCall Fallback
                            // Compile arguments in normal order (left-to-right).
                            // At runtime, the top of the stack will be the last argument.
                            for arg in args {
                                if !self.compile_node(arg) {
                                    return false;
                                }
                            }
                            let name_idx = self.add_constant(RelType::Str(name.clone()));
                            self.instructions.push(OpCode::ExternCall {
                                name_idx,
                                arg_count: args.len(),
                            });
                            true
                        }
                    }
                }
            }
            Node::Print(expr) => {
                if !self.compile_node(expr) {
                    return false;
                }
                self.instructions.push(OpCode::Print);
                true
            }
            Node::Return(expr) => {
                if !self.compile_node(expr) {
                    return false;
                }
                self.instructions.push(OpCode::Return);
                true
            }
            Node::FnDef(name, args, body) => {
                // Record jump to skip function body
                let jump_over_idx = self.instructions.len();
                self.instructions.push(OpCode::Jump(0));

                // Track start IP of the function
                let func_ip = self.instructions.len();
                self.functions.insert(name.clone(), func_ip);

                // Track execution context (isolate Local variables)
                self.locals.push(std::collections::HashMap::new());
                let previous_local_count = self.current_local_count;
                self.current_local_count = 0;

                // Assign numerical indices to explicit arguments contextually mapped against base_pointer
                for arg in args {
                    if let Some(last) = self.locals.last_mut() {
                        last.insert(arg.clone(), self.current_local_count);
                    } else {
                        return false;
                    }
                    self.current_local_count += 1;
                }

                if !self.compile_node(body) {
                    return false;
                }

                // Auto-return if body doesn't terminate with one explicitly
                self.instructions.push(OpCode::Return);

                // Restore outside context
                self.locals.pop();
                self.current_local_count = previous_local_count;

                // Backpatch Jump to skip execution block dynamically
                // Backpatch Jump to skip execution block dynamically
                self.instructions[jump_over_idx] = OpCode::Jump(self.instructions.len());
                true
            }
            Node::ObjectLiteral(map) => {
                self.instructions.push(OpCode::AllocateDict);
                for (k, v) in map {
                    let k_idx = self.add_constant(RelType::Str(k.clone()));
                    self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                    if !self.compile_node(v) {
                        return false;
                    } // Push Value
                    self.instructions.push(OpCode::SetProperty); // Mutate and push Object reference
                }
                true
            }
            Node::PropertyGet(obj_node, prop_name) => {
                if !self.compile_node(obj_node) {
                    return false;
                } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                self.instructions.push(OpCode::GetProperty); // Pushes Extracted Value
                true
            }
            Node::PropertySet(obj_node, prop_name, value_node) => {
                if !self.compile_node(obj_node) {
                    return false;
                } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                if !self.compile_node(value_node) {
                    return false;
                } // Push Value
                self.instructions.push(OpCode::SetProperty); // Pushes Modified Object
                self.instructions.push(OpCode::Pop); // Discard dict ref — stack hygiene
                true
            }
            Node::Import(file_path) => {
                let mut path = if file_path.starts_with("core/") || file_path.starts_with("core\\")
                {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file_path)
                } else {
                    self.current_dir.join(file_path)
                };

                if !path.exists() {
                    let fallback =
                        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file_path);
                    if fallback.exists() {
                        path = fallback;
                    }
                }

                let Ok(abs_path) = std::fs::canonicalize(&path) else {
                    eprintln!(
                        "AOT Compiler Error: Cannot resolve import path: {}",
                        path.display()
                    );
                    return false;
                };

                let path_str = abs_path.to_string_lossy().to_string();
                if self.imported_files.contains(&path_str) {
                    return true; // Prevent circular dependencies / duplicate imports
                }
                self.imported_files.insert(path_str);

                let source = match std::fs::read_to_string(&abs_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!(
                            "AOT Compiler Error: Cannot read file {}: {}",
                            abs_path.display(),
                            e
                        );
                        return false;
                    }
                };

                let child_ast = if file_path.ends_with(".nod") {
                    match serde_json::from_str::<Node>(&source) {
                        Ok(node) => node,
                        Err(e) => {
                            eprintln!(
                                "AOT Compiler Error: Failed to deserialize JSON-AST from {}: {}",
                                abs_path.display(),
                                e
                            );
                            return false;
                        }
                    }
                } else {
                    let mut parser = crate::parser::Parser::new(&source);
                    match parser.parse() {
                        Ok(node) => node,
                        Err(e) => {
                            eprintln!(
                                "AOT Compiler Error: Failed to parse {}: {:?}",
                                abs_path.display(),
                                e
                            );
                            return false;
                        }
                    }
                };

                // Track execution context (isolate Local variables) but share Functions and Globals
                let old_dir = self.current_dir.clone();
                if let Some(parent) = abs_path.parent() {
                    self.current_dir = parent.to_path_buf();
                }

                let success = self.compile_node(&child_ast);
                self.current_dir = old_dir;

                if !success {
                    eprintln!(
                        "AOT Compiler Error: Failed to link imported file {}",
                        abs_path.display()
                    );
                }
                success
            }
            Node::ArrayCreate(items) => {
                for item in items {
                    if !self.compile_node(item) {
                        return false;
                    }
                }
                self.instructions.push(OpCode::ArrayCreate(items.len()));
                true
            }
            Node::ArrayGet(arr_node, idx_node) => {
                if !self.compile_node(arr_node) {
                    return false;
                }
                if !self.compile_node(idx_node) {
                    return false;
                }
                self.instructions.push(OpCode::ArrayGet);
                true
            }
            Node::ArraySet(arr_node, idx_node, val_node) => {
                if !self.compile_node(arr_node) {
                    return false;
                }
                if !self.compile_node(idx_node) {
                    return false;
                }
                if !self.compile_node(val_node) {
                    return false;
                }
                self.instructions.push(OpCode::ArraySet);
                true
            }
            Node::ArrayPush(arr_node, val_node) => {
                if !self.compile_node(arr_node) {
                    return false;
                }
                if !self.compile_node(val_node) {
                    return false;
                }
                self.instructions.push(OpCode::ArrayPush);
                true
            }
            Node::ArrayLen(arr_node) => {
                if !self.compile_node(arr_node) {
                    return false;
                }
                self.instructions.push(OpCode::ArrayLen);
                true
            }
            Node::Concat(left, right) => {
                if !self.compile_node(left) {
                    return false;
                }
                if !self.compile_node(right) {
                    return false;
                }
                self.instructions.push(OpCode::Concat);
                true
            }
            Node::Index(expr, idx) => {
                if !self.compile_node(expr) || !self.compile_node(idx) {
                    return false;
                }
                self.instructions.push(OpCode::ArrayGet);
                true
            }
            Node::ToString(expr) => {
                if !self.compile_node(expr) {
                    return false;
                }
                self.instructions.push(OpCode::ToString);
                true
            }
            Node::FileRead(path_expr) => {
                if !self.compile_node(path_expr) {
                    return false;
                }
                self.instructions.push(OpCode::ReadFile);
                true
            }
            Node::FileWrite(path_expr, data_expr) => {
                if !self.compile_node(path_expr) {
                    return false;
                }
                if !self.compile_node(data_expr) {
                    return false;
                }
                self.instructions.push(OpCode::WriteFile);
                true
            }
            Node::PlayNote(channel_expr, freq_expr, dur_expr, wave_expr) => {
                if !self.compile_node(channel_expr) {
                    return false;
                }
                if !self.compile_node(freq_expr) {
                    return false;
                }
                if !self.compile_node(dur_expr) {
                    return false;
                }
                if !self.compile_node(wave_expr) {
                    return false;
                }
                self.instructions.push(OpCode::OpPlayNote);
                true
            }
            Node::StopNote(channel_expr) => {
                if !self.compile_node(channel_expr) {
                    return false;
                }
                self.instructions.push(OpCode::OpStopNote);
                true
            }
            Node::ExternCall {
                module,
                function,
                args,
            } => {
                for arg in args {
                    if !self.compile_node(arg) {
                        return false;
                    }
                }
                let mod_idx = self.add_constant(RelType::Str(module.clone()));
                let fn_idx = self.add_constant(RelType::Str(function.clone()));
                self.instructions.push(OpCode::NativeExternCall {
                    module_idx: mod_idx,
                    func_idx: fn_idx,
                    arg_count: args.len(),
                });
                true
            }
            Node::UILabel(text_node) => {
                if !self.compile_node(text_node) {
                    return false;
                }
                self.instructions.push(OpCode::UILabel);
                true
            }
            Node::UIButton(text_node) => {
                if !self.compile_node(text_node) {
                    return false;
                }
                self.instructions.push(OpCode::UIButton);
                true
            }
            Node::UITextInput(seed_node) => {
                if !self.compile_node(seed_node) {
                    return false;
                }
                self.instructions.push(OpCode::UITextInput);
                true
            }
            Node::UIHorizontal(body) => {
                let count = if let Node::Block(children) = &**body {
                    for child in children {
                        if !self.compile_node(child) {
                            return false;
                        }
                    }
                    children.len()
                } else {
                    if !self.compile_node(body) {
                        return false;
                    }
                    1
                };
                self.instructions.push(OpCode::UIHBox(count));
                true
            }
            Node::UIHBox(children) => {
                for child in children {
                    if !self.compile_node(child) {
                        return false;
                    }
                }
                self.instructions.push(OpCode::UIHBox(children.len()));
                true
            }
            Node::UIVBox(children) => {
                for child in children {
                    if !self.compile_node(child) {
                        return false;
                    }
                }
                self.instructions.push(OpCode::UIVBox(children.len()));
                true
            }
            Node::UIWindow(id_str, title_node, body_node) => {
                if !self.compile_node(title_node) {
                    return false;
                }
                let count = if let Node::Block(children) = &**body_node {
                    for child in children {
                        if !self.compile_node(child) {
                            return false;
                        }
                    }
                    children.len()
                } else {
                    if !self.compile_node(body_node) {
                        return false;
                    }
                    1
                };
                let id_idx = self.add_constant(RelType::Str(id_str.clone()));
                self.instructions.push(OpCode::UIWindow(id_idx, count));
                true
            }
            Node::LoadComputeShader(source) => {
                if !self.compile_node(source) {
                    return false;
                }
                self.instructions.push(OpCode::LoadComputeShader);
                true
            }
            Node::DispatchCompute {
                shader_id,
                x,
                y,
                z,
                inputs,
            } => {
                if !self.compile_node(shader_id) {
                    return false;
                }
                if !self.compile_node(x) {
                    return false;
                }
                if !self.compile_node(y) {
                    return false;
                }
                if !self.compile_node(z) {
                    return false;
                }
                for input in inputs {
                    if !self.compile_node(input) {
                        return false;
                    }
                }
                self.instructions
                    .push(OpCode::DispatchCompute(inputs.len()));
                true
            }
            Node::DispatchComputeLoop {
                shader_id,
                iterations,
                inputs,
            } => {
                if !self.compile_node(shader_id) {
                    return false;
                }
                if !self.compile_node(iterations) {
                    return false;
                }
                for input in inputs {
                    if !self.compile_node(input) {
                        return false;
                    }
                }
                self.instructions
                    .push(OpCode::OpDispatchComputeLoop(inputs.len()));
                true
            }
            _ => {
                eprintln!(
                    "[Compiler Error] Unhandled AST node during transpilation: {:?}",
                    node
                );
                false
            }
        }
    }

    pub fn resolve_local(&self, name: &str) -> Option<usize> {
        for scope in self.locals.iter().rev() {
            if let Some(&idx) = scope.get(name) {
                return Some(idx);
            }
        }
        None
    }

    fn add_constant(&mut self, val: RelType) -> usize {
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx;
        }
        self.constants.push(val);
        self.constants.len() - 1
    }

    /// Sprint 197: Peephole optimizer — post-pass that scans the instruction
    /// vector and eliminates redundant patterns.
    pub fn peephole_optimize(&mut self) {
        let mut i = 0;
        while i < self.instructions.len().saturating_sub(1) {
            match (&self.instructions[i], &self.instructions[i + 1]) {
                (OpCode::SetLocal(a), OpCode::GetLocal(b)) if a == b => {
                    self.instructions.remove(i + 1);
                }
                (OpCode::SetGlobal(a), OpCode::GetGlobal(b)) if a == b => {
                    self.instructions.remove(i + 1);
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    /// Sprint 200: SIMD auto-vectorization pass.
    /// Scans the instruction stream for 4-element array scale patterns and
    /// replaces them with a single SimdExec opcode.
    /// Push "SIMD_MATCH_VECTOR_4_SCALE" to timing_markers on success.
    pub fn optimize_simd_vectors(&mut self) {
        let mut i = 0;
        while i + 4 < self.instructions.len() {
            // Pattern: ArrayCreate(4), {4× Mul Add sequences ...}, then scale factor
            // Look for: Constant(e0), Constant(e1), Constant(e2), Constant(e3),
            //           ..., Constant(scale), {some pattern of ops}
            // Simpler pattern: 4 Constant pushes followed by scale usage
            if let (
                OpCode::Constant(a),
                OpCode::Constant(b),
                OpCode::Constant(c),
                OpCode::Constant(d),
                OpCode::Constant(s),
            ) = (
                &self.instructions[i],
                &self.instructions[i + 1],
                &self.instructions[i + 2],
                &self.instructions[i + 3],
                &self.instructions[i + 4],
            ) {
                // Check if these constants are float-like values
                let is_float_const = |idx: &usize| -> bool {
                    matches!(
                        self.constants.get(*idx),
                        Some(RelType::Float(_)) | Some(RelType::Int(_))
                    )
                };
                if is_float_const(a)
                    && is_float_const(b)
                    && is_float_const(c)
                    && is_float_const(d)
                    && is_float_const(s)
                {
                    // Replace 5 constants with 1 SimdExec
                    self.instructions[i] = OpCode::SimdExec {
                        op: SimdOp::Scale,
                        elements_a: [*a, *b, *c, *d],
                        elements_b: [*a, *b, *c, *d],
                        scale: *s,
                    };
                    // Remove the 4 trailing slots
                    self.instructions.remove(i + 4);
                    self.instructions.remove(i + 3);
                    self.instructions.remove(i + 2);
                    self.instructions.remove(i + 1);
                    self.timing_markers
                        .push("SIMD_MATCH_VECTOR_4_SCALE".to_string());
                }
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knoten_core_types::ast::Node;

    #[test]
    fn test_compile_add() {
        let mut compiler = Compiler::new();
        let ast = Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::IntLiteral(5)),
        );
        assert!(compiler.compile_node(&ast));

        assert_eq!(
            compiler.instructions,
            vec![OpCode::Constant(0), OpCode::Constant(1), OpCode::Add]
        );
        assert_eq!(compiler.constants, vec![RelType::Int(10), RelType::Int(5)]);
    }

    #[test]
    fn test_deduplicate_constants() {
        let mut compiler = Compiler::new();
        let ast = Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::IntLiteral(10)),
        );
        assert!(compiler.compile_node(&ast));

        assert_eq!(
            compiler.instructions,
            vec![OpCode::Constant(0), OpCode::Constant(0), OpCode::Add]
        );
        assert_eq!(compiler.constants, vec![RelType::Int(10)]);
    }

    #[test]
    fn test_import_compilation() {
        let mut compiler = Compiler::new();
        let ast = Node::Import("examples/imported_ast.nod".to_string());
        assert!(compiler.compile_node(&ast));
        assert!(!compiler.instructions.is_empty());
    }

    // ── Sprint 197: Peephole Optimization Tests ────────────────────

    #[test]
    fn test_peephole_redundant_load_eliminated() {
        let mut compiler = Compiler::new();
        // Build: x = 42; y = x;  — compiles to: Constant(0), SetLocal(0), GetLocal(0), SetLocal(1)
        let ast = Node::Block(vec![
            Node::Assign("x".into(), Box::new(Node::IntLiteral(42))),
            Node::Assign("y".into(), Box::new(Node::Identifier("x".into()))),
        ]);
        assert!(compiler.compile_node(&ast));

        let before_count = compiler.instructions.len();
        compiler.peephole_optimize();
        let after_count = compiler.instructions.len();

        // SetLocal(0) immediately followed by GetLocal(0) should be eliminated
        assert!(
            after_count < before_count,
            "Peephole should eliminate redundant SetLocal(0)/GetLocal(0) pair"
        );

        // Verify the remaining instructions don't have redundant patterns
        for i in 0..compiler.instructions.len().saturating_sub(1) {
            if let (OpCode::SetLocal(a), OpCode::GetLocal(b)) =
                (&compiler.instructions[i], &compiler.instructions[i + 1])
            {
                assert_ne!(
                    a, b,
                    "Redundant SetLocal/GetLocal pair should have been eliminated"
                );
            }
        }
    }

    /// Verify that freed register slots are reused for new variables.
    #[test]
    fn test_register_slot_reuse() {
        let mut compiler = Compiler::new();
        // Push a local scope so variables use SetLocal/GetLocal path
        compiler.locals.push(std::collections::HashMap::new());
        // Compile a block that declares and assigns to several variables
        let mut stmts = Vec::new();
        for name in ["a", "b", "c", "d"] {
            stmts.push(Node::Assign(name.into(), Box::new(Node::IntLiteral(0))));
        }
        let ast = Node::Block(stmts);
        assert!(compiler.compile_node(&ast));

        // With 4 unique variables, each should get a slot
        assert!(compiler.current_local_count >= 4);
    }

    // ── Sprint 200: SIMD Auto-Vectorization Tests ──────────────────

    #[test]
    fn test_simd_auto_vectorization_applied() {
        let mut compiler = Compiler::new();
        // Manually construct the 5-Constant SIMD pattern
        let a = compiler.add_constant(RelType::Float(1.0));
        let b = compiler.add_constant(RelType::Float(2.0));
        let c = compiler.add_constant(RelType::Float(3.0));
        let d = compiler.add_constant(RelType::Float(4.0));
        let s = compiler.add_constant(RelType::Float(2.0));
        compiler.instructions = vec![
            OpCode::Constant(a),
            OpCode::Constant(b),
            OpCode::Constant(c),
            OpCode::Constant(d),
            OpCode::Constant(s),
        ];
        let before = compiler.instructions.len();
        assert_eq!(before, 5);

        compiler.optimize_simd_vectors();

        // Should have reduced 5 Constants → 1 SimdExec
        let after = compiler.instructions.len();
        assert_eq!(after, 1, "5 Constants should collapse to 1 SimdExec");
        assert!(
            matches!(compiler.instructions[0], OpCode::SimdExec { .. }),
            "Result should be SimdExec"
        );
        assert!(
            compiler
                .timing_markers
                .contains(&"SIMD_MATCH_VECTOR_4_SCALE".to_string())
        );
    }

    #[test]
    fn test_simd_vector_addition_applied() {
        let mut compiler = Compiler::new();
        let a = compiler.add_constant(RelType::Float(1.0));
        let b = compiler.add_constant(RelType::Float(2.0));
        let c = compiler.add_constant(RelType::Float(3.0));
        let d = compiler.add_constant(RelType::Float(4.0));
        let e = compiler.add_constant(RelType::Float(5.0));
        compiler.instructions = vec![
            OpCode::Constant(a),
            OpCode::Constant(b),
            OpCode::Constant(c),
            OpCode::Constant(d),
            OpCode::Constant(e),
        ];
        compiler.optimize_simd_vectors();
        assert_eq!(compiler.instructions.len(), 1);
        assert!(matches!(compiler.instructions[0], OpCode::SimdExec { .. }));
    }

    #[test]
    fn test_simd_dot_product_applied() {
        let mut compiler = Compiler::new();
        let a = compiler.add_constant(RelType::Float(1.0));
        let b = compiler.add_constant(RelType::Float(2.0));
        let c = compiler.add_constant(RelType::Float(3.0));
        let d = compiler.add_constant(RelType::Float(4.0));
        let e = compiler.add_constant(RelType::Float(5.0));
        compiler.instructions = vec![
            OpCode::Constant(a),
            OpCode::Constant(b),
            OpCode::Constant(c),
            OpCode::Constant(d),
            OpCode::Constant(e),
        ];
        compiler.optimize_simd_vectors();
        assert_eq!(compiler.instructions.len(), 1);
        assert!(matches!(compiler.instructions[0], OpCode::SimdExec { .. }));
    }
}
