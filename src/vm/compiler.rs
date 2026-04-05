use crate::ast::Node;
use crate::executor::RelType;
use crate::vm::opcode::OpCode;

#[derive(Default)]
pub struct Compiler {
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub functions: std::collections::HashMap<String, usize>,
    pub locals: Vec<std::collections::HashMap<String, usize>>,
    pub current_local_count: usize,
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
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Add);
                true
            }
            Node::Sub(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Subtract);
                true
            }
            Node::Mul(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Multiply);
                true
            }
            Node::Div(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Divide);
                true
            }
            Node::Eq(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Equal);
                true
            }
            Node::Gt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Greater);
                true
            }
            Node::Lt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Less);
                true
            }
            Node::Lte(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::LessEqual);
                true
            }
            Node::Gte(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::GreaterEqual);
                true
            }
            Node::NotEq(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::NotEqual);
                true
            }
            Node::And(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::And);
                true
            }
            Node::Or(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Or);
                true
            }
            Node::Not(expr) => {
                if !self.compile_node(expr) { return false; }
                self.instructions.push(OpCode::Not);
                true
            }
            Node::Block(stmts) => {
                for stmt in stmts {
                    if !self.compile_node(stmt) { return false; }
                }
                true
            }
            Node::If(cond, then_block, else_block) => {
                if !self.compile_node(cond) { return false; }
                let jump_if_false_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0)); // Placeholder

                if !self.compile_node(then_block) { return false; }

                if let Some(else_branch) = else_block {
                    let jump_idx = self.instructions.len();
                    self.instructions.push(OpCode::Jump(0)); // Placeholder

                    // Backpatch JumpIfFalse to jump here (start of else block)
                    self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len());

                    if !self.compile_node(else_branch) { return false; }

                    // Backpatch unconditional Jump to jump past the else block
                    self.instructions[jump_idx] = OpCode::Jump(self.instructions.len());
                } else {
                    // Backpatch JumpIfFalse to jump past the then block
                    self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len());
                }
                true
            }
            Node::While(cond, body) => {
                let loop_start_idx = self.instructions.len();
                if !self.compile_node(cond) { return false; }
                
                let jump_if_false_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0)); // Placeholder
                
                if !self.compile_node(body) { return false; }
                
                self.instructions.push(OpCode::Jump(loop_start_idx)); // Loop back
                self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len()); // Exit loop
                true
            }
            Node::Assign(ident, expr) => {
                if !self.compile_node(expr) { return false; }
                
                if !self.locals.is_empty() {
                    let idx = if let Some(idx) = self.resolve_local(ident) {
                        idx
                    } else {
                        // Declare new local
                        let idx = self.current_local_count;
                        self.locals.last_mut().unwrap().insert(ident.clone(), idx);
                        self.current_local_count += 1;
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
                    && let Some(idx) = self.resolve_local(ident) {
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
                        if args.len() != 1 { return false; }
                        if !self.compile_node(&args[0]) { return false; }
                        self.instructions.push(OpCode::StringLength);
                        true
                    }
                    "str_contains" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // String source
                        if !self.compile_node(&args[1]) { return false; } // Pattern / char class
                        self.instructions.push(OpCode::StringContainsChars);
                        true
                    }
                    "str_split" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // Target String
                        if !self.compile_node(&args[1]) { return false; } // Delimiter
                        self.instructions.push(OpCode::StringSplit);
                        true
                    }
                    "arr_contains" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // Array
                        if !self.compile_node(&args[1]) { return false; } // Search String
                        self.instructions.push(OpCode::ArrayContains);
                        true
                    }
                    "read_file" => {
                        if args.len() != 1 { return false; }
                        if !self.compile_node(&args[0]) { return false; }
                        self.instructions.push(OpCode::ReadFile);
                        true
                    }
                    _ => {
                        if let Some(&target_ip) = self.functions.get(name) {
                            for arg in args {
                                if !self.compile_node(arg) { return false; }
                            }
                            self.instructions.push(OpCode::Call(target_ip, args.len()));
                            true
                        } else {
                            // FFI ExternCall Fallback
                            // Compile arguments in normal order (left-to-right).
                            // At runtime, the top of the stack will be the last argument.
                            for arg in args {
                                if !self.compile_node(arg) { return false; }
                            }
                            let name_idx = self.add_constant(RelType::Str(name.clone()));
                            self.instructions.push(OpCode::ExternCall { name_idx, arg_count: args.len() });
                            true
                        }
                    }
                }
            }
            Node::Print(expr) => {
                if !self.compile_node(expr) { return false; }
                self.instructions.push(OpCode::Print);
                true
            }
            Node::Return(expr) => {
                if !self.compile_node(expr) { return false; }
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
                    self.locals.last_mut().unwrap().insert(arg.clone(), self.current_local_count);
                    self.current_local_count += 1;
                }
                
                if !self.compile_node(body) { return false; }
                
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
                    if !self.compile_node(v) { return false; } // Push Value
                    self.instructions.push(OpCode::SetProperty); // Mutate and push Object reference
                }
                true
            }
            Node::PropertyGet(obj_node, prop_name) => {
                if !self.compile_node(obj_node) { return false; } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                self.instructions.push(OpCode::GetProperty); // Pushes Extracted Value
                true
            }
            Node::PropertySet(obj_node, prop_name, value_node) => {
                if !self.compile_node(obj_node) { return false; } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                if !self.compile_node(value_node) { return false; } // Push Value
                self.instructions.push(OpCode::SetProperty); // Pushes Modified Object
                self.instructions.push(OpCode::Pop);          // Discard dict ref — stack hygiene
                true
            }
            Node::Import(file_path) => {
                let path = if file_path.starts_with("core/") || file_path.starts_with("core\\") {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file_path)
                } else {
                    self.current_dir.join(file_path)
                };

                let Ok(abs_path) = std::fs::canonicalize(&path) else {
                    eprintln!("AOT Compiler Error: Cannot resolve import path: {}", path.display());
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
                        eprintln!("AOT Compiler Error: Cannot read file {}: {}", abs_path.display(), e);
                        return false;
                    }
                };
                
                let mut parser = crate::parser::Parser::new(&source);
                let child_ast = parser.parse();
                
                // Track execution context (isolate Local variables) but share Functions and Globals
                let old_dir = self.current_dir.clone();
                if let Some(parent) = abs_path.parent() {
                    self.current_dir = parent.to_path_buf();
                }
                
                let success = self.compile_node(&child_ast);
                self.current_dir = old_dir;
                
                if !success {
                    eprintln!("AOT Compiler Error: Failed to link imported file {}", abs_path.display());
                }
                success
            }
            Node::ArrayCreate(items) => {
                for item in items {
                    if !self.compile_node(item) { return false; }
                }
                self.instructions.push(OpCode::ArrayCreate(items.len()));
                true
            }
            Node::ArrayGet(arr_node, idx_node) => {
                if !self.compile_node(arr_node) { return false; }
                if !self.compile_node(idx_node) { return false; }
                self.instructions.push(OpCode::ArrayGet);
                true
            }
            Node::ArraySet(arr_node, idx_node, val_node) => {
                if !self.compile_node(arr_node) { return false; }
                if !self.compile_node(idx_node) { return false; }
                if !self.compile_node(val_node) { return false; }
                self.instructions.push(OpCode::ArraySet);
                true
            }
            Node::ArrayPush(arr_node, val_node) => {
                if !self.compile_node(arr_node) { return false; }
                if !self.compile_node(val_node) { return false; }
                self.instructions.push(OpCode::ArrayPush);
                true
            }
            Node::ArrayLen(arr_node) => {
                if !self.compile_node(arr_node) { return false; }
                self.instructions.push(OpCode::ArrayLen);
                true
            }
            Node::Concat(left, right) => {
                if !self.compile_node(left) { return false; }
                if !self.compile_node(right) { return false; }
                self.instructions.push(OpCode::Concat);
                true
            }
            Node::ToString(expr) => {
                if !self.compile_node(expr) { return false; }
                self.instructions.push(OpCode::ToString);
                true
            }
            Node::FileRead(path_expr) => {
                if !self.compile_node(path_expr) { return false; }
                self.instructions.push(OpCode::ReadFile);
                true
            }
            Node::FileWrite(path_expr, data_expr) => {
                if !self.compile_node(path_expr) { return false; }
                if !self.compile_node(data_expr) { return false; }
                self.instructions.push(OpCode::WriteFile);
                true
            }
            Node::ExternCall { module, function, args } => {
                for arg in args {
                    if !self.compile_node(arg) { return false; }
                }
                let mod_idx = self.add_constant(RelType::Str(module.clone()));
                let fn_idx = self.add_constant(RelType::Str(function.clone()));
                self.instructions.push(OpCode::NativeExternCall { module_idx: mod_idx, func_idx: fn_idx, arg_count: args.len() });
                true
            }
            Node::UILabel(text_node) => {
                if !self.compile_node(text_node) { return false; }
                self.instructions.push(OpCode::UILabel);
                true
            }
            Node::UIButton(text_node) => {
                if !self.compile_node(text_node) { return false; }
                self.instructions.push(OpCode::UIButton);
                true
            }
            Node::UIHorizontal(body) => {
                let count = if let Node::Block(children) = &**body {
                    for child in children { if !self.compile_node(child) { return false; } }
                    children.len()
                } else {
                    if !self.compile_node(body) { return false; }
                    1
                };
                self.instructions.push(OpCode::UIHBox(count));
                true
            }
            Node::UIHBox(children) => {
                for child in children { if !self.compile_node(child) { return false; } }
                self.instructions.push(OpCode::UIHBox(children.len()));
                true
            }
            Node::UIVBox(children) => {
                for child in children { if !self.compile_node(child) { return false; } }
                self.instructions.push(OpCode::UIVBox(children.len()));
                true
            }
            Node::UIWindow(id_str, title_node, body_node) => {
                if !self.compile_node(title_node) { return false; }
                let count = if let Node::Block(children) = &**body_node {
                    for child in children { if !self.compile_node(child) { return false; } }
                    children.len()
                } else {
                    if !self.compile_node(body_node) { return false; }
                    1
                };
                let id_idx = self.add_constant(RelType::Str(id_str.clone()));
                self.instructions.push(OpCode::UIWindow(id_idx, count));
                true
            }
            _ => false,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Node;

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
}
